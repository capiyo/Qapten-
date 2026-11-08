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
