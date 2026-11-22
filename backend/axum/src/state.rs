use crate::{
    auth::FirebaseVerifier,
    audit::{AuditEvent, AuditStore},
    model::{Agent, Artifact, Task, Workspace},
    robotics::TelemetryEvent,
    storage::{EntityKind, Storage, StorageError, StorageMutation, StoredEntity, UserWorkspace},
};
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

#[derive(Clone, Default)]
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
    pub firebase_verifier: Option<FirebaseVerifier>,
    pub simulator_enabled: bool,
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
        let mut audit_events = Vec::new();
        for workspace_id in workspace_ids {
            audit_events.extend(storage.list_audit_events(workspace_id).await?);
        }
        state.audit.write().await.restore(audit_events);

        Ok(state)
    }

    pub async fn with_storage_and_auth(
        storage: Arc<dyn Storage>,
        verifier: FirebaseVerifier,
        simulator_enabled: bool,
    ) -> Result<Self, StorageError> {
        let mut state = Self::with_storage(storage).await?;
        state.firebase_verifier = Some(verifier);
        state.simulator_enabled = simulator_enabled;
        Ok(state)
    }

    pub async fn resolve_firebase_user(
        &self,
        subject: &str,
        email: Option<&str>,
    ) -> Result<UserWorkspace, StorageError> {
        let storage = self.storage.as_ref().ok_or(StorageError::IdentityProvisioning)?;
        let identity = storage.resolve_or_create_firebase_user(subject, email).await?;
        self.workspaces.write().await.insert(identity.workspace.id, identity.workspace.clone());
        Ok(identity)
    }

    pub async fn user_can_access_workspace(
        &self,
        user_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<bool, StorageError> {
        match &self.storage {
            Some(storage) => storage.user_can_access_workspace(user_id, workspace_id).await,
            None => Ok(false),
        }
    }

    pub async fn list_user_workspace_ids(&self, user_id: Uuid) -> Result<Vec<Uuid>, StorageError> {
        match &self.storage {
            Some(storage) => storage.list_user_workspace_ids(user_id).await,
            None => Ok(Vec::new()),
        }
    }

    pub async fn create_workspace_for_user(
        &self,
        workspace: Workspace,
        user_id: Uuid,
        audit: AuditEvent,
    ) -> Result<(), StorageError> {
        let storage = self.storage.as_ref().ok_or(StorageError::IdentityProvisioning)?;
        storage.create_workspace_for_user(workspace, user_id, audit).await
    }

    pub async fn persist_entity(
        &self,
        entity: StoredEntity,
        related_entities: Vec<StoredEntity>,
        event: AuditEvent,
    ) -> Result<(), StorageError> {
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
