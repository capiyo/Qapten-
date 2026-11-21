use crate::{
    audit::{AuditEvent, AuditStore},
    model::{Agent, Artifact, Task, Workspace},
    robotics::TelemetryEvent,
    storage::{EntityKind, Storage, StorageError, StorageMutation, StoredEntity},
};
use chrono::Utc;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Default)]
pub struct TelemetryStore {
    pub events_by_device: HashMap<String, Vec<TelemetryEvent>>,
    pub last_sequence_by_device: HashMap<String, u64>,
    pub event_ids: HashSet<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub workspaces: Arc<RwLock<HashMap<Uuid, Workspace>>>,
    pub agents: Arc<RwLock<HashMap<Uuid, Agent>>>,
    pub artifacts: Arc<RwLock<HashMap<Uuid, Artifact>>>,
    pub tasks: Arc<RwLock<HashMap<Uuid, Task>>>,
    /// Bounded in-memory simulator stream; never treated as durable device telemetry.
    pub telemetry: Arc<RwLock<TelemetryStore>>,
    /// Runtime cache backed by the append-only audit table when SQLite is configured.
    pub audit: Arc<RwLock<AuditStore>>,
    storage: Option<Arc<dyn Storage>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            workspaces: Arc::default(),
            agents: Arc::default(),
            artifacts: Arc::default(),
            tasks: Arc::default(),
            telemetry: Arc::default(),
            audit: Arc::default(),
            storage: None,
        }
    }
}

impl AppState {
    pub async fn with_storage(storage: Arc<dyn Storage>) -> Result<Self, StorageError> {
        let state = Self {
            storage: Some(storage.clone()),
            ..Self::default()
        };

        for entity in storage.list_entities(EntityKind::Workspace, None).await? {
            match entity {
                StoredEntity::Workspace(item) => {
                    state.workspaces.write().await.insert(item.id, item);
                }
                _ => return Err(StorageError::InvalidStoredEntityKind),
            }
        }
        for entity in storage.list_entities(EntityKind::Agent, None).await? {
            match entity {
                StoredEntity::Agent(item) => {
                    state.agents.write().await.insert(item.id, item);
                }
                _ => return Err(StorageError::InvalidStoredEntityKind),
            }
        }
        for entity in storage.list_entities(EntityKind::Artifact, None).await? {
            match entity {
                StoredEntity::Artifact(item) => {
                    state.artifacts.write().await.insert(item.id, item);
                }
                _ => return Err(StorageError::InvalidStoredEntityKind),
            }
        }
        for entity in storage.list_entities(EntityKind::Task, None).await? {
            match entity {
                StoredEntity::Task(item) => {
                    state.tasks.write().await.insert(item.id, item);
                }
                _ => return Err(StorageError::InvalidStoredEntityKind),
            }
        }

        let workspace_ids: Vec<_> = state.workspaces.read().await.keys().copied().collect();
        let mut audit = state.audit.write().await;
        for workspace_id in workspace_ids {
            audit.restore(storage.list_audit_events(workspace_id).await?);
        }
        drop(audit);

        Ok(state)
    }

    pub async fn persist_entity(
        &self,
        entity: StoredEntity,
        related_entities: Vec<StoredEntity>,
        workspace_id: Option<Uuid>,
        event_type: &str,
        entity_type: &str,
        entity_id: &str,
        metadata: Value,
    ) -> Result<(), StorageError> {
        let event = AuditEvent {
            sequence: 0,
            event_id: Uuid::new_v4(),
            workspace_id,
            actor: "system:local-development-unattributed".into(),
            event_type: event_type.into(),
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            occurred_at: Utc::now(),
            metadata,
        };

        if let Some(storage) = &self.storage {
            storage
                .apply(StorageMutation {
                    entity,
                    related_entities,
                    audit: event.clone(),
                })
                .await?;
        }

        self.audit.write().await.append_event(event);
        Ok(())
    }

    pub fn storage_is_durable(&self) -> bool {
        self.storage.is_some()
    }
}
