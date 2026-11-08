use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

/// In-memory development audit record. Actor attribution is explicitly local
/// and unauthenticated until a verified principal is supplied by middleware.
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub sequence: u64,
    pub event_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub actor: String,
    pub event_type: String,
    pub entity_type: String,
    pub entity_id: String,
    pub occurred_at: DateTime<Utc>,
    pub metadata: Value,
}

#[derive(Debug, Default)]
pub struct AuditStore {
    next_sequence: u64,
    events: Vec<AuditEvent>,
}

impl AuditStore {
    pub fn append(
        &mut self,
        workspace_id: Option<Uuid>,
        event_type: impl Into<String>,
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
        metadata: Value,
    ) -> AuditEvent {
        self.next_sequence += 1;
        let event = AuditEvent {
            sequence: self.next_sequence,
            event_id: Uuid::new_v4(),
            workspace_id,
            actor: "system:local-development-unattributed".into(),
            event_type: event_type.into(),
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            occurred_at: Utc::now(),
            metadata,
        };
        self.events.push(event.clone());
        event
    }

    pub fn list_workspace(&self, workspace_id: Uuid) -> Vec<AuditEvent> {
        self.events
            .iter()
            .filter(|event| event.workspace_id == Some(workspace_id))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_sequence_is_monotonic_and_workspace_scoped() {
        let mut store = AuditStore::default();
        let workspace = Uuid::new_v4();
        let first = store.append(Some(workspace), "workspace.created", "workspace", workspace.to_string(), Value::Null);
        let second = store.append(Some(workspace), "task.created", "task", "task-1", Value::Null);
        store.append(Some(Uuid::new_v4()), "workspace.created", "workspace", "other", Value::Null);
        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);
        assert_eq!(store.list_workspace(workspace).len(), 2);
        assert!(store.list_workspace(workspace).iter().all(|event| event.actor == "system:local-development-unattributed"));
    }
}
