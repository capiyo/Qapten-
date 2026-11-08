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
    http::{header, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};

/// Build the Axon HTTP API with a strict local-client origin allowlist.
/// The API remains loopback-only until authenticated remote access is implemented.
pub fn app(state: AppState) -> Router {
    let allowed_origins = [
        HeaderValue::from_static("tauri://localhost"),
        HeaderValue::from_static("http://tauri.localhost"),
        HeaderValue::from_static("http://localhost:5173"),
        HeaderValue::from_static("http://127.0.0.1:5173"),
    ];

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
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(allowed_origins))
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers([header::CONTENT_TYPE]),
        )
}


#[cfg(test)]
mod cors_tests {
    use super::*;
    use axum::{
        body::Body,
        http::{header, Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn cors_allows_tauri_origins_and_rejects_unlisted_origins() {
        let router = app(AppState::default());
        let allowed = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/health")
                    .header(header::ORIGIN, "tauri://localhost")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(allowed.status(), StatusCode::OK);
        assert_eq!(
            allowed.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
            "tauri://localhost"
        );

        let denied = router
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/health")
                    .header(header::ORIGIN, "https://untrusted.example")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(denied
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none());
    }
}
