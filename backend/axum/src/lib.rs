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
    extract::{ConnectInfo, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use auth::{AuthError, AuthenticatedUser};
use serde::Serialize;
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};
use uuid::Uuid;
use std::net::SocketAddr;

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
    let Some(verifier) = state.firebase_verifier.as_ref() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let source_ip = request.extensions().get::<ConnectInfo<SocketAddr>>().map(|ConnectInfo(address)| address.ip());
    let Some(value) = request.headers().get(axum::http::header::AUTHORIZATION).and_then(|v| v.to_str().ok()) else {
        return if state.record_auth_failure(source_ip).await { unauthorized() } else { StatusCode::TOO_MANY_REQUESTS.into_response() };
    };
    let Some(token) = value.strip_prefix("Bearer ").filter(|v| !v.is_empty()) else {
        return if state.record_auth_failure(source_ip).await { unauthorized() } else { StatusCode::TOO_MANY_REQUESTS.into_response() };
    };
    let claims = match verifier.verify(token).await {
        Ok(claims) => claims,
        Err(AuthError::Unavailable) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Err(AuthError::InvalidToken | AuthError::MissingProject) => {
            return if state.record_auth_failure(source_ip).await { unauthorized() } else { StatusCode::TOO_MANY_REQUESTS.into_response() };
        },
    };
    let identity = match state.resolve_firebase_user(&claims.sub, claims.email.as_deref()).await {
        Ok(identity) => identity,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let principal = AuthenticatedUser { user_id: identity.user_id, firebase_uid: claims.sub };
    if path.starts_with("/api/v1/sim/") && !state.simulator_enabled {
        return StatusCode::NOT_FOUND.into_response();
    }

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


#[cfg(test)]
mod authentication_tests {
    use super::*;
    use crate::{
        auth::{local_test_token, local_test_verifier},
        model::{Task, TaskStatus, TaskTransition},
        storage::SqliteStorage,
    };
    use axum::{body::{to_bytes, Body}, http::Request};
    use chrono::Utc;
    use tower::ServiceExt;

    async fn test_app() -> (Router, AppState) {
        let storage = SqliteStorage::connect_in_memory().await.unwrap().shared();
        let verifier = local_test_verifier("axon-test-project");
        let state = AppState::with_storage_and_auth(storage, verifier, true).await.unwrap();
        (app(state.clone()), state)
    }

    fn protected_routes(workspace_id: Uuid, task_id: Uuid) -> Vec<(&'static str, String)> {
        vec![
            ("GET", "/api/v1/workspaces".into()),
            ("POST", "/api/v1/workspaces".into()),
            ("GET", format!("/api/v1/workspaces/{workspace_id}/audit")),
            ("GET", format!("/api/v1/workspaces/{workspace_id}/agents")),
            ("POST", format!("/api/v1/workspaces/{workspace_id}/agents")),
            ("GET", format!("/api/v1/workspaces/{workspace_id}/artifacts")),
            ("POST", format!("/api/v1/workspaces/{workspace_id}/artifacts")),
            ("GET", format!("/api/v1/workspaces/{workspace_id}/tasks")),
            ("POST", format!("/api/v1/workspaces/{workspace_id}/tasks")),
            ("POST", format!("/api/v1/tasks/{task_id}/submit")),
            ("POST", format!("/api/v1/tasks/{task_id}/transitions")),
            ("GET", "/api/v1/sim/devices/test-device/telemetry".into()),
            ("POST", "/api/v1/sim/devices/test-device/telemetry".into()),
        ]
    }

    async fn request_status(app: Router, method: &str, uri: &str, token: Option<&str>) -> StatusCode {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            builder = builder.header(axum::http::header::AUTHORIZATION, format!("Bearer {token}"));
        }
        app.oneshot(builder.body(Body::empty()).unwrap()).await.unwrap().status()
    }

    #[tokio::test]
    async fn every_protected_route_rejects_missing_expired_and_wrong_key_tokens() {
        let (app, _state) = test_app().await;
        let now = Utc::now().timestamp().max(0) as u64;
        let expired = local_test_token("axon-test-project", "expired-user", now - 120, false);
        let wrong_key = local_test_token("axon-test-project", "wrong-key-user", now + 600, true);
        for (method, uri) in protected_routes(Uuid::new_v4(), Uuid::new_v4()) {
            assert_eq!(request_status(app.clone(), method, &uri, None).await, StatusCode::UNAUTHORIZED, "missing token: {method} {uri}");
            assert_eq!(request_status(app.clone(), method, &uri, Some(&expired)).await, StatusCode::UNAUTHORIZED, "expired token: {method} {uri}");
            assert_eq!(request_status(app.clone(), method, &uri, Some(&wrong_key)).await, StatusCode::UNAUTHORIZED, "wrong signing key: {method} {uri}");
        }
    }

    #[tokio::test]
    async fn another_users_workspace_and_task_resources_are_hidden_as_not_found() {
        let (app, state) = test_app().await;
        let owner = state.resolve_firebase_user("owner-firebase-uid", None).await.unwrap();
        let task_id = Uuid::new_v4();
        let now = Utc::now();
        state.tasks.write().await.insert(task_id, Task {
            id: task_id,
            workspace_id: owner.workspace.id,
            title: "private task".into(),
            objective: "test cross-user access".into(),
            assignee_agent_id: Uuid::new_v4(),
            context_artifact_ids: Vec::new(),
            response_artifact_id: None,
            status: TaskStatus::Offered,
            version: 1,
            created_at: now,
            updated_at: now,
            transitions: vec![TaskTransition {
                from: None,
                to: TaskStatus::Offered,
                note: "created".into(),
                occurred_at: now,
            }],
        });
        let stranger_token = local_test_token(
            "axon-test-project",
            "different-firebase-uid",
            now.timestamp().max(0) as u64 + 600,
            false,
        );

        let protected_resources = [
            ("GET", format!("/api/v1/workspaces/{}/audit", owner.workspace.id)),
            ("GET", format!("/api/v1/workspaces/{}/agents", owner.workspace.id)),
            ("POST", format!("/api/v1/workspaces/{}/agents", owner.workspace.id)),
            ("GET", format!("/api/v1/workspaces/{}/artifacts", owner.workspace.id)),
            ("POST", format!("/api/v1/workspaces/{}/artifacts", owner.workspace.id)),
            ("GET", format!("/api/v1/workspaces/{}/tasks", owner.workspace.id)),
            ("POST", format!("/api/v1/workspaces/{}/tasks", owner.workspace.id)),
            ("POST", format!("/api/v1/tasks/{task_id}/submit")),
            ("POST", format!("/api/v1/tasks/{task_id}/transitions")),
        ];
        for (method, uri) in protected_resources {
            assert_eq!(
                request_status(app.clone(), method, &uri, Some(&stranger_token)).await,
                StatusCode::NOT_FOUND,
                "foreign resource leaked: {method} {uri}",
            );
        }

        let response = app.clone().oneshot(
            Request::builder()
                .uri("/api/v1/workspaces")
                .header(axum::http::header::AUTHORIZATION, format!("Bearer {stranger_token}"))
                .body(Body::empty()).unwrap(),
        ).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.as_array().unwrap().iter().all(|workspace| workspace["id"] != owner.workspace.id.to_string()));
    }
    #[tokio::test]
    async fn simulator_requires_authentication_and_remains_hidden_outside_development() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap().shared();
        let state = AppState::with_storage_and_auth(
            storage,
            local_test_verifier("axon-test-project"),
            false,
        ).await.unwrap();
        let router = app(state);
        let path = "/api/v1/sim/devices/test-device/telemetry";
        assert_eq!(request_status(router.clone(), "GET", path, None).await, StatusCode::UNAUTHORIZED);
        let now = Utc::now().timestamp().max(0) as u64;
        let token = local_test_token("axon-test-project", "sim-user", now + 600, false);
        assert_eq!(request_status(router, "GET", path, Some(&token)).await, StatusCode::NOT_FOUND);
    }

}
