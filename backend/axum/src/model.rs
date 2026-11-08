use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkspace {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Agent {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub display_name: String,
    pub provider: String,
    pub connection_mode: ConnectionMode,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionMode {
    Manual,
}

#[derive(Debug, Deserialize)]
pub struct RegisterAgent {
    pub display_name: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Artifact {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub title: String,
    pub content: String,
    pub source_agent_id: Option<Uuid>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateArtifact {
    pub title: String,
    pub content: String,
    pub source_agent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub title: String,
    pub objective: String,
    pub assignee_agent_id: Uuid,
    pub context_artifact_ids: Vec<Uuid>,
    pub response_artifact_id: Option<Uuid>,
    pub status: TaskStatus,
    pub version: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub transitions: Vec<TaskTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Offered,
    Acknowledged,
    InProgress,
    Submitted,
    ReviewRequired,
    Accepted,
    ChangesRequested,
    Completed,
    Rejected,
    Cancelled,
}

#[derive(Debug, Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub objective: String,
    pub assignee_agent_id: Uuid,
    #[serde(default)]
    pub context_artifact_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitTask {
    pub expected_version: u64,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct TransitionTask {
    pub expected_version: u64,
    pub status: TaskStatus,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskTransition {
    pub from: Option<TaskStatus>,
    pub to: TaskStatus,
    pub note: String,
    pub occurred_at: DateTime<Utc>,
}
