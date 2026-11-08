mod model;
mod routes;
mod state;

pub use state::AppState;

use axum::{routing::{get, post}, Router};
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(routes::health))
        .route("/api/v1/workspaces", get(routes::list_workspaces).post(routes::create_workspace))
        .route("/api/v1/workspaces/:workspace_id/agents", post(routes::register_agent))
        .route("/api/v1/workspaces/:workspace_id/artifacts", post(routes::create_artifact))\n        .route("/api/v1/workspaces/:workspace_id/tasks", get(routes::list_tasks).post(routes::create_task))\n        .route("/api/v1/tasks/:task_id/transitions", post(routes::transition_task))
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(TraceLayer::new_for_http())
}
