pub mod audit;
pub mod auth;
pub mod identity;
mod model;
pub mod robotics;
mod robotics_routes;
mod routes;
mod state;
pub mod storage;
mod task_routes;

pub use state::AppState;

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use auth::{AuthError, AuthenticatedUser};
use serde::Serialize;
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};
use uuid::Uuid;

#[derive(Serialize)]
struct AuthFailure {
    error: &'static str,
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, Json(AuthFailure { error: "Authentication required." })).into_response()
}

async fn authenticate(
    State(state): State<AppState>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if path == "/health" {
        return next.run(request).await;
    }
    if path.starts_with("/api/v1/sim/") && !state.simulator_enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(verifier) = state.firebase_verifier.as_ref() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let Some(value) = request.headers().get(axum::http::header::AUTHORIZATION).and_then(|v| v.to_str().ok()) else {
        return unauthorized();
    };
    let Some(token) = value.strip_prefix("Bearer ").filter(|v| !v.is_empty()) else {
        return unauthorized();
    };
    let claims = match verifier.verify(token).await {
        Ok(claims) => claims,
        Err(AuthError::Unavailable) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Err(AuthError::InvalidToken | AuthError::MissingProject) => return unauthorized(),
    };
    let identity = match state.resolve_firebase_user(&claims.sub, claims.email.as_deref()).await {
        Ok(identity) => identity,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let principal = AuthenticatedUser { user_id: identity.user_id, firebase_uid: claims.sub };

    // Resource paths are authorized before handlers run. Missing and foreign
    // resources intentionally share the same 404 response.
    let segments: Vec<&str> = path.split('/').collect();
    if segments.get(3) == Some(&"workspaces") {
        if let Some(raw_id) = segments.get(4) {
            let Ok(workspace_id) = Uuid::parse_str(raw_id) else {
                return StatusCode::NOT_FOUND.into_response();
            };
            match state.user_can_access_workspace(principal.user_id, workspace_id).await {
                Ok(true) => {}
                Ok(false) | Err(_) => return StatusCode::NOT_FOUND.into_response(),
            }
        }
    } else if segments.get(3) == Some(&"tasks") {
        if let Some(raw_id) = segments.get(4) {
            let Ok(task_id) = Uuid::parse_str(raw_id) else {
                return StatusCode::NOT_FOUND.into_response();
            };
            let task = state.tasks.read().await.get(&task_id).cloned();
            let Some(task) = task else { return StatusCode::NOT_FOUND.into_response(); };
            match state.user_can_access_workspace(principal.user_id, task.workspace_id).await {
                Ok(true) => {}
                Ok(false) | Err(_) => return StatusCode::NOT_FOUND.into_response(),
            }
        }
    }
    request.extensions_mut().insert(principal);
    next.run(request).await
}

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
        .route("/api/v1/tasks/:task_id/submit", post(task_routes::submit_task))
        .route("/api/v1/tasks/:task_id/transitions", post(task_routes::transition_task))
        .with_state(state.clone())
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(state, authenticate))
}
