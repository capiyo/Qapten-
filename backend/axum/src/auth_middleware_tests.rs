#[cfg(test)]
mod tests {
    use crate::{
        firebase_auth::FirebaseVerifier,
        model::{Task, TaskStatus},
        storage::SqliteStorage,
        AppState,
    };
    use axum::{
        body::Body,
        http::{header::AUTHORIZATION, Request, StatusCode},
    };
    use chrono::Utc;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn test_app() -> (axum::Router, AppState, Uuid) {
        let storage = SqliteStorage::connect_in_memory().await.unwrap().shared();
        let state = AppState::with_storage(storage)
            .await
            .unwrap()
            .with_firebase_verifier(FirebaseVerifier::test_verifier(), true);
        let first = state
            .ensure_firebase_user("axon-test-project", "owner-user")
            .await
            .unwrap();
        state
            .workspaces
            .write()
            .await
            .insert(first.workspace.id, first.workspace.clone());
        let task_id = Uuid::new_v4();
        state.tasks.write().await.insert(
            task_id,
            Task {
                id: task_id,
                workspace_id: first.workspace.id,
                title: "Private task".into(),
                objective: "Test resource isolation".into(),
                assignee_agent_id: Uuid::new_v4(),
                context_artifact_ids: Vec::new(),
                response_artifact_id: None,
                status: TaskStatus::Offered,
                version: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                transitions: Vec::new(),
            },
        );
        (crate::app(state.clone()), state, task_id)
    }

    fn request(method: &str, path: &str, token: Option<String>) -> Request<Body> {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            builder = builder.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        builder.body(Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn every_registered_api_route_rejects_missing_and_expired_tokens() {
        let (app, state, task_id) = test_app().await;
        let workspace_id = state
            .ensure_firebase_user("axon-test-project", "owner-user")
            .await
            .unwrap()
            .workspace
            .id;
        let routes: Vec<(&str, String)> = vec![
            ("GET", "/api/v1/workspaces".into()),
            ("POST", "/api/v1/workspaces".into()),
            ("GET", format!("/api/v1/workspaces/{workspace_id}/audit")),
            ("GET", "/api/v1/sim/devices/simulator-1/telemetry".into()),
            ("POST", "/api/v1/sim/devices/simulator-1/telemetry".into()),
            ("GET", format!("/api/v1/workspaces/{workspace_id}/agents")),
            ("POST", format!("/api/v1/workspaces/{workspace_id}/agents")),
            (
                "GET",
                format!("/api/v1/workspaces/{workspace_id}/artifacts"),
            ),
            (
                "POST",
                format!("/api/v1/workspaces/{workspace_id}/artifacts"),
            ),
            ("GET", format!("/api/v1/workspaces/{workspace_id}/tasks")),
            ("POST", format!("/api/v1/workspaces/{workspace_id}/tasks")),
            ("POST", format!("/api/v1/tasks/{task_id}/submit")),
            ("POST", format!("/api/v1/tasks/{task_id}/transitions")),
        ];
        for (method, path) in routes {
            let missing = app
                .clone()
                .oneshot(request(method, &path, None))
                .await
                .unwrap();
            assert_eq!(
                missing.status(),
                StatusCode::UNAUTHORIZED,
                "missing token: {method} {path}"
            );
            let expired = app
                .clone()
                .oneshot(request(
                    method,
                    &path,
                    Some(FirebaseVerifier::test_token("expired-user", true)),
                ))
                .await
                .unwrap();
            assert_eq!(
                expired.status(),
                StatusCode::UNAUTHORIZED,
                "expired token: {method} {path}"
            );
        }
    }

    #[tokio::test]
    async fn another_users_workspace_and_task_resources_are_not_found() {
        let (app, state, task_id) = test_app().await;
        let workspace_id = state
            .ensure_firebase_user("axon-test-project", "owner-user")
            .await
            .unwrap()
            .workspace
            .id;
        let other_user_token = FirebaseVerifier::test_token("other-user", false);
        let paths = [
            format!("/api/v1/workspaces/{workspace_id}/audit"),
            format!("/api/v1/workspaces/{workspace_id}/agents"),
            format!("/api/v1/workspaces/{workspace_id}/artifacts"),
            format!("/api/v1/workspaces/{workspace_id}/tasks"),
            format!("/api/v1/tasks/{task_id}/submit"),
            format!("/api/v1/tasks/{task_id}/transitions"),
        ];
        for path in paths {
            let method = if path.contains("/submit") || path.contains("/transitions") {
                "POST"
            } else {
                "GET"
            };
            let response = app
                .clone()
                .oneshot(request(method, &path, Some(other_user_token.clone())))
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::NOT_FOUND,
                "cross-user access: {method} {path}"
            );
        }
    }

    #[tokio::test]
    async fn health_is_the_only_public_route() {
        let (app, _, _) = test_app().await;
        let health = app
            .clone()
            .oneshot(request("GET", "/health", None))
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
        let api = app
            .oneshot(request("GET", "/api/v1/workspaces", None))
            .await
            .unwrap();
        assert_eq!(api.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn simulator_routes_are_hidden_when_development_flag_is_disabled() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap().shared();
        let state = AppState::with_storage(storage)
            .await
            .unwrap()
            .with_firebase_verifier(FirebaseVerifier::test_verifier(), false);
        let app = crate::app(state);
        let token = FirebaseVerifier::test_token("dev-user", false);
        let response = app
            .oneshot(request(
                "GET",
                "/api/v1/sim/devices/simulator-1/telemetry",
                Some(token),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
