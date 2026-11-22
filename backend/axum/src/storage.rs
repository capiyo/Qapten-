use crate::{
    audit::AuditEvent,
    model::{Agent, Artifact, Task, Workspace},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::{str::FromStr, sync::Arc, time::Duration};
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
    #[error("stored entity kind does not match its table")]
    InvalidStoredEntityKind,
    #[error("identity storage is unavailable")]
    IdentityStorageUnavailable,
    #[error("Firebase UID is already bound to a different Firebase project")]
    IdentityProjectMismatch,
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
    pub related_entities: Vec<StoredEntity>,
    pub audit: AuditEvent,
}

#[derive(Debug, Clone)]
pub struct UserWorkspace {
    pub user_id: Uuid,
    pub workspace: Workspace,
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

    async fn ensure_firebase_user(
        &self,
        project_id: &str,
        uid: &str,
    ) -> Result<UserWorkspace, StorageError>;

    async fn is_workspace_member(
        &self,
        user_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<bool, StorageError>;

    async fn list_user_workspaces(&self, user_id: Uuid) -> Result<Vec<Workspace>, StorageError>;

    async fn create_workspace_for_user(
        &self,
        user_id: Uuid,
        name: String,
        actor: String,
    ) -> Result<Workspace, StorageError>;
}

#[derive(Clone)]
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .busy_timeout(Duration::from_secs(10));
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn connect_in_memory() -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?
            .busy_timeout(Duration::from_secs(10));
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
        let mut transaction = self.pool.begin().await?;
        for entity in std::iter::once(mutation.entity).chain(mutation.related_entities) {
            let payload = serde_json::to_string(&entity)?;
            let workspace_id = entity.workspace_id().map(|id| id.to_string());
            let created_at = entity.created_at().to_rfc3339();
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
        }

        let audit = mutation.audit;
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

    async fn ensure_firebase_user(
        &self,
        project_id: &str,
        uid: &str,
    ) -> Result<UserWorkspace, StorageError> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();
        let candidate_user_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, auth_provider, provider_subject, created_at, firebase_project_id) VALUES (?, 'firebase', ?, ?, ?) ON CONFLICT(auth_provider, provider_subject) DO NOTHING"
        )
        .bind(candidate_user_id.to_string())
         .bind(uid)
        .bind(now.to_rfc3339())
        .bind(project_id)
        .execute(&mut *tx).await?;

        let row = sqlx::query("SELECT id, personal_workspace_id, firebase_project_id FROM users WHERE auth_provider = 'firebase' AND provider_subject = ?")
             .bind(uid)
            .fetch_one(&mut *tx).await?;
        let user_id_text: String = row.try_get("id")?;
        let user_id =
            Uuid::parse_str(&user_id_text).map_err(|_| StorageError::InvalidStoredIdentifier)?;
        let linked_workspace: Option<String> = row.try_get("personal_workspace_id")?;
        let stored_project: Option<String> = row.try_get("firebase_project_id")?;
        if stored_project.as_deref() != Some(project_id) {
            return Err(StorageError::IdentityProjectMismatch);
        }

        let workspace = if let Some(workspace_id) = linked_workspace {
            let workspace_id = Uuid::parse_str(&workspace_id)
                .map_err(|_| StorageError::InvalidStoredIdentifier)?;
            let row = sqlx::query(
                "SELECT payload_json FROM entities WHERE kind = 'workspace' AND id = ?",
            )
            .bind(workspace_id.to_string())
            .fetch_one(&mut *tx)
            .await?;
            let payload: String = row.try_get("payload_json")?;
            match serde_json::from_str::<StoredEntity>(&payload)? {
                StoredEntity::Workspace(workspace) => workspace,
                _ => return Err(StorageError::InvalidStoredEntityKind),
            }
        } else {
            let workspace = Workspace {
                id: Uuid::new_v4(),
                name: "Personal workspace".into(),
                created_at: now,
            };
            let entity = StoredEntity::Workspace(workspace.clone());
            let payload = serde_json::to_string(&entity)?;
            sqlx::query("INSERT INTO entities (kind, id, workspace_id, created_at, payload_json) VALUES ('workspace', ?, NULL, ?, ?)")
                .bind(workspace.id.to_string())
                .bind(workspace.created_at.to_rfc3339())
                .bind(payload)
                .execute(&mut *tx).await?;
            sqlx::query("UPDATE users SET personal_workspace_id = ? WHERE id = ? AND personal_workspace_id IS NULL")
                .bind(workspace.id.to_string())
                .bind(user_id.to_string())
                .execute(&mut *tx).await?;
            sqlx::query("INSERT INTO workspace_memberships (workspace_id, user_id, role, created_at) VALUES (?, ?, 'owner', ?) ON CONFLICT(workspace_id, user_id) DO NOTHING")
                .bind(workspace.id.to_string())
                .bind(user_id.to_string())
                .bind(now.to_rfc3339())
                .execute(&mut *tx).await?;
            let event = AuditEvent {
                sequence: 0,
                event_id: Uuid::new_v4(),
                workspace_id: Some(workspace.id),
                actor: format!("human:{user_id}"),
                event_type: "workspace.personal_created".into(),
                entity_type: "workspace".into(),
                entity_id: workspace.id.to_string(),
                occurred_at: now,
                metadata: serde_json::json!({"provisioned_by": "firebase_auth"}),
            };
            sqlx::query("INSERT INTO audit_events (event_id, workspace_id, actor, event_type, entity_type, entity_id, occurred_at, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(event.event_id.to_string())
                .bind(event.workspace_id.map(|id| id.to_string()))
                .bind(event.actor)
                .bind(event.event_type)
                .bind(event.entity_type)
                .bind(event.entity_id)
                .bind(event.occurred_at.to_rfc3339())
                .bind(serde_json::to_string(&event.metadata)?)
                .execute(&mut *tx).await?;
            workspace
        };
        tx.commit().await?;
        Ok(UserWorkspace { user_id, workspace })
    }

    async fn is_workspace_member(
        &self,
        user_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<bool, StorageError> {
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM workspace_memberships WHERE user_id = ? AND workspace_id = ?",
        )
        .bind(user_id.to_string())
        .bind(workspace_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(exists.is_some())
    }

    async fn list_user_workspaces(&self, user_id: Uuid) -> Result<Vec<Workspace>, StorageError> {
        let rows = sqlx::query(
            "SELECT e.payload_json FROM workspace_memberships m JOIN entities e ON e.kind = 'workspace' AND e.id = m.workspace_id WHERE m.user_id = ? ORDER BY m.created_at ASC"
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|row| {
                let payload: String = row.try_get("payload_json")?;
                match serde_json::from_str::<StoredEntity>(&payload)? {
                    StoredEntity::Workspace(workspace) => Ok(workspace),
                    _ => Err(StorageError::InvalidStoredEntityKind),
                }
            })
            .collect()
    }

    async fn create_workspace_for_user(
        &self,
        user_id: Uuid,
        name: String,
        actor: String,
    ) -> Result<Workspace, StorageError> {
        let mut tx = self.pool.begin().await?;
        let workspace = Workspace {
            id: Uuid::new_v4(),
            name,
            created_at: Utc::now(),
        };
        let entity = StoredEntity::Workspace(workspace.clone());
        sqlx::query("INSERT INTO entities (kind, id, workspace_id, created_at, payload_json) VALUES ('workspace', ?, NULL, ?, ?)")
            .bind(workspace.id.to_string())
            .bind(workspace.created_at.to_rfc3339())
            .bind(serde_json::to_string(&entity)?)
            .execute(&mut *tx).await?;
        sqlx::query("INSERT INTO workspace_memberships (workspace_id, user_id, role, created_at) VALUES (?, ?, 'owner', ?)")
            .bind(workspace.id.to_string())
            .bind(user_id.to_string())
            .bind(workspace.created_at.to_rfc3339())
            .execute(&mut *tx).await?;
        let event = AuditEvent {
            sequence: 0,
            event_id: Uuid::new_v4(),
            workspace_id: Some(workspace.id),
            actor,
            event_type: "workspace.created".into(),
            entity_type: "workspace".into(),
            entity_id: workspace.id.to_string(),
            occurred_at: workspace.created_at,
            metadata: serde_json::json!({"name": workspace.name}),
        };
        sqlx::query("INSERT INTO audit_events (event_id, workspace_id, actor, event_type, entity_type, entity_id, occurred_at, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(event.event_id.to_string())
            .bind(event.workspace_id.map(|id| id.to_string()))
            .bind(event.actor)
            .bind(event.event_type)
            .bind(event.entity_type)
            .bind(event.entity_id)
            .bind(event.occurred_at.to_rfc3339())
            .bind(serde_json::to_string(&event.metadata)?)
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(workspace)
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
            related_entities: Vec::new(),
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

    #[tokio::test]
    async fn firebase_first_login_is_idempotent_under_concurrent_requests() {
        let database_path =
            std::env::temp_dir().join(format!("axon-identity-{}.db", Uuid::new_v4()));
        let database_url = format!("sqlite://{}", database_path.display());
        let storage = SqliteStorage::connect(&database_url).await.unwrap();
        let (first, second) = tokio::join!(
            storage.ensure_firebase_user("axon-project", "same-firebase-uid"),
            storage.ensure_firebase_user("axon-project", "same-firebase-uid"),
        );
        let first = first.unwrap();
        let second = second.unwrap();
        assert_eq!(first.user_id, second.user_id);
        assert_eq!(first.workspace.id, second.workspace.id);
        assert!(storage
            .is_workspace_member(first.user_id, first.workspace.id)
            .await
            .unwrap());
        assert_eq!(
            storage
                .list_user_workspaces(first.user_id)
                .await
                .unwrap()
                .len(),
            1
        );
        let audit = storage.list_audit_events(first.workspace.id).await.unwrap();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].actor, format!("human:{}", first.user_id));
        drop(storage);
        std::fs::remove_file(database_path).unwrap();
    }
}
