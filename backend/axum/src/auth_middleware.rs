use crate::{state::AppState, storage::UserWorkspace};
use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request, Response, StatusCode},
    middleware::Next,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub firebase_uid: String,
    pub project_id: String,
    pub personal_workspace_id: Uuid,
}

impl From<UserWorkspace> for AuthenticatedUser {
    fn from(value: UserWorkspace) -> Self {
        Self {
            user_id: value.user_id,
            firebase_uid: String::new(),
            project_id: String::new(),
            personal_workspace_id: value.workspace.id,
        }
    }
}

pub async fn authenticate(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response<Body> {
    if request.uri().path() == "/health" {
        return next.run(request).await;
    }

    let Some(verifier) = state.firebase_verifier.as_ref() else {
        return response(StatusCode::SERVICE_UNAVAILABLE);
    };
    let Some(value) = request.headers().get(AUTHORIZATION).and_then(|v| v.to_str().ok()) else {
        return response(StatusCode::UNAUTHORIZED);
    };
    let Some(token) = value.strip_prefix("Bearer ") else {
        return response(StatusCode::UNAUTHORIZED);
    };
    let identity = match verifier.verify(token).await {
        Ok(identity) => identity,
        Err(_) => return response(StatusCode::UNAUTHORIZED),
    };
    let account = match state.ensure_firebase_user(&identity.project_id, &identity.uid).await {
        Ok(account) => account,
        Err(_) => return response(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let user = Arc::new(AuthenticatedUser {
        user_id: account.user_id,
        firebase_uid: identity.uid,
        project_id: identity.project_id,
        personal_workspace_id: account.workspace.id,
    });

    let path: Vec<&str> = request.uri().path().split('/').collect();
    if path.get(3) == Some(&"workspaces") {
        if let Some(workspace_text) = path.get(4) {
            let Ok(workspace_id) = Uuid::parse_str(workspace_text) else {
                return response(StatusCode::NOT_FOUND);
            };
            let member = state.is_workspace_member(user.user_id, workspace_id).await.unwrap_or(false);
            if !member {
                return response(StatusCode::NOT_FOUND);
            }
        }
    } else if path.get(3) == Some(&"tasks") {
        if let Some(task_text) = path.get(4) {
            let Ok(task_id) = Uuid::parse_str(task_text) else {
                return response(StatusCode::NOT_FOUND);
            };
            let workspace_id = state.tasks.read().await.get(&task_id).map(|task| task.workspace_id);
            let Some(workspace_id) = workspace_id else {
                return response(StatusCode::NOT_FOUND);
            };
            let member = state.is_workspace_member(user.user_id, workspace_id).await.unwrap_or(false);
            if !member {
                return response(StatusCode::NOT_FOUND);
            }
        }
    } else if path.get(3) == Some(&"sim") && !state.simulator_enabled {
        return response(StatusCode::NOT_FOUND);
    }

    request.extensions_mut().insert(user);
    next.run(request).await
}

fn response(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(match status {
            StatusCode::UNAUTHORIZED => r#"{\"error\":\"Authentication required.\"}"#,
            StatusCode::NOT_FOUND => r#"{\"error\":\"Resource not found.\"}"#,
            StatusCode::SERVICE_UNAVAILABLE => r#"{\"error\":\"Authentication is not configured.\"}"#,
            _ => r#"{\"error\":\"The request could not be completed.\"}"#,
        }))
        .expect("static authentication response is valid")
}
