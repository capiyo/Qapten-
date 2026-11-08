use crate::model::{Agent, Artifact, Task, Workspace};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct AppState {
    pub workspaces: Arc<RwLock<HashMap<Uuid, Workspace>>>,
    pub agents: Arc<RwLock<HashMap<Uuid, Agent>>>,
    pub artifacts: Arc<RwLock<HashMap<Uuid, Artifact>>>,\n    pub tasks: Arc<RwLock<HashMap<Uuid, Task>>>,
}
