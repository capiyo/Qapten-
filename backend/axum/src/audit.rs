use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// In-memory development audit record. Actor attribution is explicitly local
/// and unauthenticated until a verified principal is supplied by middleware.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl AuditEvent {
    pub fn attributed(
        actor: impl Into<String>,
        workspace_id: Option<Uuid>,
        event_type: impl Into<String>,
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
        metadata: Value,
    ) -> Self {
        Self {
            sequence: 0,
            event_id: Uuid::new_v4(),
            workspace_id,
            actor: actor.into(),
            event_type: event_type.into(),
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            occurred_at: Utc::now(),
            metadata,
        }
    }

}

#[derive(Debug, Default)]
pub struct AuditStore {
    next_sequence: u64,
    events: Vec<AuditEvent>,
}

impl AuditStore {
    pub fn append_event(&mut self, mut event: AuditEvent) -> AuditEvent {
        self.next_sequence += 1;
        event.sequence = self.next_sequence;
        self.events.push(event.clone());
        event
    }

    pub fn restore(&mut self, events: Vec<AuditEvent>) {
        for event in events {
            self.next_sequence = self.next_sequence.max(event.sequence);
            self.events.push(event);
        }
        self.events.sort_by_key(|event| event.sequence);
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
        let first = store.append_event(AuditEvent::attributed(
            "user:actor-1", Some(workspace), "workspace.created", "workspace", workspace.to_string(), Value::Null,
        ));
        let second = store.append_event(AuditEvent::attributed(
            "user:actor-1", Some(workspace), "task.created", "task", "task-1", Value::Null,
        ));
        store.append_event(AuditEvent::attributed(
            "user:actor-2", Some(Uuid::new_v4()), "workspace.created", "workspace", "other", Value::Null,
        ));
        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);
        assert_eq!(store.list_workspace(workspace).len(), 2);
        assert!(store
            .list_workspace(workspace)
            .iter()
            .all(|event| event.actor == "user:actor-1"));
    }
}
