use crate::{
    audit::AuditEvent,
    model::{Agent, Artifact, Task, Workspace},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::{str::FromStr, sync::Arc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite storage error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("SQLite migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("record serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("stored audit event contains an invalid identifier")]
    InvalidStoredIdentifier,
    #[error("stored audit event contains an invalid timestamp")]
    InvalidStoredTimestamp,
    #[error("stored audit sequence is negative")]
    InvalidStoredSequence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Workspace,
    Agent,
    Artifact,
    Task,
}

impl EntityKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Agent => "agent",
            Self::Artifact => "artifact",
            Self::Task => "task",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "record", rename_all = "snake_case")]
pub enum StoredEntity {
    Workspace(Workspace),
    Agent(Agent),
    Artifact(Artifact),
    Task(Task),
}

impl StoredEntity {
    fn kind(&self) -> EntityKind {
        match self {
            Self::Workspace(_) => EntityKind::Workspace,
            Self::Agent(_) => EntityKind::Agent,
            Self::Artifact(_) => EntityKind::Artifact,
            Self::Task(_) => EntityKind::Task,
        }
    }

    fn id(&self) -> Uuid {
        match self {
            Self::Workspace(record) => record.id,
            Self::Agent(record) => record.id,
            Self::Artifact(record) => record.id,
            Self::Task(record) => record.id,
        }
    }

    fn workspace_id(&self) -> Option<Uuid> {
        match self {
            Self::Workspace(_) => None,
            Self::Agent(record) => Some(record.workspace_id),
            Self::Artifact(record) => Some(record.workspace_id),
            Self::Task(record) => Some(record.workspace_id),
        }
    }

    fn created_at(&self) -> DateTime<Utc> {
        match self {
            Self::Workspace(record) => record.created_at,
            Self::Agent(record) => record.created_at,
            Self::Artifact(record) => record.captured_at,
            Self::Task(record) => record.created_at,
        }
    }
}

/// A single atomic write: the entity change and its audit event either both
/// commit or neither does. Audit rows are immutable after insertion.
#[derive(Debug, Clone)]
pub struct StorageMutation {
    pub entity: StoredEntity,
    pub audit: AuditEvent,
}

#[async_trait]
pub trait Storage: Send + Sync {
    async fn apply(&self, mutation: StorageMutation) -> Result<(), StorageError>;

    async fn list_entities(
        &self,
        kind: EntityKind,
        workspace_id: Option<Uuid>,
    ) -> Result<Vec<StoredEntity>, StorageError>;

    async fn list_audit_events(&self, workspace_id: Uuid) -> Result<Vec<AuditEvent>, StorageError>;
}

#[derive(Clone)]
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn connect_in_memory() -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?;
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    pub fn shared(self) -> Arc<dyn Storage> {
        Arc::new(self)
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn apply(&self, mutation: StorageMutation) -> Result<(), StorageError> {
        let entity = mutation.entity;
        let audit = mutation.audit;
        let payload = serde_json::to_string(&entity)?;
        let workspace_id = entity.workspace_id().map(|id| id.to_string());
        let created_at = entity.created_at().to_rfc3339();

        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO entities (kind, id, workspace_id, created_at, payload_json) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(kind, id) DO UPDATE SET \
             workspace_id = excluded.workspace_id, \
             created_at = excluded.created_at, \
             payload_json = excluded.payload_json",
        )
        .bind(entity.kind().as_str())
        .bind(entity.id().to_string())
        .bind(workspace_id)
        .bind(created_at)
        .bind(payload)
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            "INSERT INTO audit_events \
             (event_id, workspace_id, actor, event_type, entity_type, entity_id, occurred_at, metadata_json) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(audit.event_id.to_string())
        .bind(audit.workspace_id.map(|id| id.to_string()))
        .bind(audit.actor)
        .bind(audit.event_type)
        .bind(audit.entity_type)
        .bind(audit.entity_id)
        .bind(audit.occurred_at.to_rfc3339())
        .bind(serde_json::to_string(&audit.metadata)?)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(())
    }

    async fn list_entities(
        &self,
        kind: EntityKind,
        workspace_id: Option<Uuid>,
    ) -> Result<Vec<StoredEntity>, StorageError> {
        let workspace_id = workspace_id.map(|id| id.to_string());
        let rows = sqlx::query(
            "SELECT payload_json FROM entities \
             WHERE kind = ? AND (? IS NULL OR workspace_id = ?) \
             ORDER BY created_at ASC, id ASC",
        )
        .bind(kind.as_str())
        .bind(workspace_id.as_deref())
        .bind(workspace_id.as_deref())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let payload: String = row.try_get("payload_json")?;
                Ok(serde_json::from_str(&payload)?)
            })
            .collect()
    }

    async fn list_audit_events(&self, workspace_id: Uuid) -> Result<Vec<AuditEvent>, StorageError> {
        let rows = sqlx::query(
            "SELECT sequence, event_id, workspace_id, actor, event_type, entity_type, \
             entity_id, occurred_at, metadata_json FROM audit_events \
             WHERE workspace_id = ? ORDER BY sequence ASC",
        )
        .bind(workspace_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let sequence: i64 = row.try_get("sequence")?;
                let event_id: String = row.try_get("event_id")?;
                let workspace_id: Option<String> = row.try_get("workspace_id")?;
                let occurred_at: String = row.try_get("occurred_at")?;
                let metadata: String = row.try_get("metadata_json")?;
                Ok(AuditEvent {
                    sequence: u64::try_from(sequence)
                        .map_err(|_| StorageError::InvalidStoredSequence)?,
                    event_id: Uuid::parse_str(&event_id)
                        .map_err(|_| StorageError::InvalidStoredIdentifier)?,
                    workspace_id: workspace_id
                        .map(|value| Uuid::parse_str(&value))
                        .transpose()
                        .map_err(|_| StorageError::InvalidStoredIdentifier)?,
                    actor: row.try_get("actor")?,
                    event_type: row.try_get("event_type")?,
                    entity_type: row.try_get("entity_type")?,
                    entity_id: row.try_get("entity_id")?,
                    occurred_at: DateTime::parse_from_rfc3339(&occurred_at)
                        .map_err(|_| StorageError::InvalidStoredTimestamp)?
                        .with_timezone(&Utc),
                    metadata: serde_json::from_str(&metadata)?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Workspace;
    use serde_json::json;

    fn mutation(workspace_id: Uuid, event_id: Uuid) -> StorageMutation {
        let now = Utc::now();
        StorageMutation {
            entity: StoredEntity::Workspace(Workspace {
                id: workspace_id,
                name: "Axon".into(),
                created_at: now,
            }),
            audit: AuditEvent {
                sequence: 0,
                event_id,
                workspace_id: Some(workspace_id),
                actor: "human:test-user".into(),
                event_type: "workspace.created".into(),
                entity_type: "workspace".into(),
                entity_id: workspace_id.to_string(),
                occurred_at: now,
                metadata: json!({"name": "Axon"}),
            },
        }
    }

    #[tokio::test]
    async fn sqlite_persists_entities_and_audit_events() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        let workspace_id = Uuid::new_v4();
        storage
            .apply(mutation(workspace_id, Uuid::new_v4()))
            .await
            .unwrap();

        let entities = storage
            .list_entities(EntityKind::Workspace, None)
            .await
            .unwrap();
        assert_eq!(entities.len(), 1);
        assert!(matches!(&entities[0], StoredEntity::Workspace(item) if item.id == workspace_id));

        let events = storage.list_audit_events(workspace_id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].sequence, 1);
        assert_eq!(events[0].actor, "human:test-user");
    }

    #[tokio::test]
    async fn entity_mutation_rolls_back_when_audit_insert_fails() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        let first_workspace = Uuid::new_v4();
        let duplicate_event = Uuid::new_v4();
        storage
            .apply(mutation(first_workspace, duplicate_event))
            .await
            .unwrap();

        let second_workspace = Uuid::new_v4();
        assert!(storage
            .apply(mutation(second_workspace, duplicate_event))
            .await
            .is_err());

        let entities = storage
            .list_entities(EntityKind::Workspace, None)
            .await
            .unwrap();
        assert_eq!(entities.len(), 1);
        assert!(
            matches!(&entities[0], StoredEntity::Workspace(item) if item.id == first_workspace)
        );
        assert!(storage
            .list_audit_events(second_workspace)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn audit_events_reject_update_and_delete() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        let workspace_id = Uuid::new_v4();
        storage
            .apply(mutation(workspace_id, Uuid::new_v4()))
            .await
            .unwrap();

        let update =
            sqlx::query("UPDATE audit_events SET actor = 'changed' WHERE workspace_id = ?")
                .bind(workspace_id.to_string())
                .execute(&storage.pool)
                .await;
        assert!(update.is_err());

        let delete = sqlx::query("DELETE FROM audit_events WHERE workspace_id = ?")
            .bind(workspace_id.to_string())
            .execute(&storage.pool)
            .await;
        assert!(delete.is_err());
    }
}
