pub mod audit;
pub mod identity;
mod model;
pub mod robotics;
mod robotics_routes;
mod routes;
mod state;
mod task_routes;

pub use state::AppState;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(routes::health))
        .route(
            "/api/v1/workspaces/:workspace_id/audit",
            get(routes::list_audit_events),
        )
        .route(
            "/api/v1/sim/devices/:device_id/telemetry",
            get(robotics_routes::list_simulated_telemetry)
                .post(robotics_routes::ingest_simulated_telemetry),
        )
        .route(
            "/api/v1/workspaces",
            get(routes::list_workspaces).post(routes::create_workspace),
        )
        .route(
            "/api/v1/workspaces/:workspace_id/agents",
            get(routes::list_agents).post(routes::register_agent),
        )
        .route(
            "/api/v1/workspaces/:workspace_id/artifacts",
            get(routes::list_artifacts).post(routes::create_artifact),
        )
        .route(
            "/api/v1/workspaces/:workspace_id/tasks",
            get(task_routes::list_tasks).post(task_routes::create_task),
        )
        .route(
            "/api/v1/tasks/:task_id/submit",
            post(task_routes::submit_task),
        )
        .route(
            "/api/v1/tasks/:task_id/transitions",
            post(task_routes::transition_task),
        )
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(TraceLayer::new_for_http())
}
