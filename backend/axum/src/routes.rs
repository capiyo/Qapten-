use crate::{
    audit::AuditEvent,
    model::{
        Agent, Artifact, ConnectionMode, CreateArtifact, CreateWorkspace, RegisterAgent, Workspace,
    },
    storage::StoredEntity,
    AppState,
};
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;
use std::sync::Arc;
use crate::auth_middleware::AuthenticatedUser;

type ApiResult<T> = Result<T, (StatusCode, Json<ApiError>)>;

#[derive(Serialize)]
pub struct Health {
    status: &'static str,
    service: &'static str,
    storage: &'static str,
    authentication: &'static str,
}

pub async fn list_audit_events(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<Vec<AuditEvent>>> {
    if !state.workspaces.read().await.contains_key(&workspace_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "Workspace not found.".into(),
            }),
        ));
    }
    Ok(Json(state.list_audit_events(workspace_id).await.map_err(|_| storage_failure())?))
}

pub async fn health(State(state): State<AppState>) -> Json<Health> {
    Json(Health {
        status: "ok",
        service: "axon-api",
        storage: if state.storage_is_durable() {
            "persistent"
        } else {
            "development_memory_only"
        },
        authentication: if state.firebase_verifier.is_some() { "firebase_id_token" } else { "not_configured" },
    })
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    error: String,
}

fn storage_failure() -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError {
            error: "The change could not be committed. Please retry.".into(),
        }),
    )
}

fn bad_request(message: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError {
            error: message.into(),
        }),
    )
}

pub async fn create_workspace(
    State(state): State<AppState>,
    Extension(user): Extension<Arc<AuthenticatedUser>>,
    Json(input): Json<CreateWorkspace>,
) -> ApiResult<(StatusCode, Json<Workspace>)> {
    let name = input.name.trim();
    if name.is_empty() || name.len() > 120 {
        return Err(bad_request(
            "Workspace name must contain between 1 and 120 characters.",
        ));
    }

    let workspace = state
        .create_workspace_for_user(
            user.user_id,
            name.to_owned(),
            format!("human:{}", user.user_id),
        )
        .await
        .map_err(|_| storage_failure())?;
    state.workspaces.write().await.insert(workspace.id, workspace.clone());

    Ok((StatusCode::CREATED, Json(workspace)))
}

pub async fn list_workspaces(
    State(state): State<AppState>,
    Extension(user): Extension<Arc<AuthenticatedUser>>,
) -> ApiResult<Json<Vec<Workspace>>> {
    let items = state.list_user_workspaces(user.user_id).await.map_err(|_| storage_failure())?;
    Ok(Json(items))
}

pub async fn register_agent(
    State(state): State<AppState>,
    Extension(user): Extension<Arc<AuthenticatedUser>>,
    Path(workspace_id): Path<Uuid>,
    Json(input): Json<RegisterAgent>,
) -> ApiResult<(StatusCode, Json<Agent>)> {
    if !state.workspaces.read().await.contains_key(&workspace_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "Workspace not found.".into(),
            }),
        ));
    }

    let display_name = input.display_name.trim();
    let provider = input.provider.trim();
    if display_name.is_empty()
        || display_name.len() > 100
        || provider.is_empty()
        || provider.len() > 80
    {
        return Err(bad_request(
            "Agent display name and provider must be non-empty and within their length limits.",
        ));
    }

    let agent = Agent {
        id: Uuid::new_v4(),
        workspace_id,
        display_name: display_name.to_owned(),
        provider: provider.to_owned(),
        connection_mode: ConnectionMode::Manual,
        created_at: Utc::now(),
    };
    state
        .persist_entity(
            StoredEntity::Agent(agent.clone()),
            Vec::new(),
            AuditEvent::attributed(
                format!("human:{}", user.user_id),
                Some(workspace_id),
                "agent.registered",
                "agent",
                agent.id.to_string(),
                serde_json::json!({"provider": agent.provider, "connection_mode": "manual"}),
            ),
        )
        .await
        .map_err(|_| storage_failure())?;
    state.agents.write().await.insert(agent.id, agent.clone());

    Ok((StatusCode::CREATED, Json(agent)))
}

pub async fn create_artifact(
    State(state): State<AppState>,
    Extension(user): Extension<Arc<AuthenticatedUser>>,
    Path(workspace_id): Path<Uuid>,
    Json(input): Json<CreateArtifact>,
) -> ApiResult<(StatusCode, Json<Artifact>)> {
    if !state.workspaces.read().await.contains_key(&workspace_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "Workspace not found.".into(),
            }),
        ));
    }

    let title = input.title.trim();
    if title.is_empty()
        || title.len() > 200
        || input.content.is_empty()
        || input.content.len() > 512_000
    {
        return Err(bad_request(
            "Artifact title must be 1–200 characters and content must be 1–512000 bytes.",
        ));
    }

    if let Some(agent_id) = input.source_agent_id {
        let agents = state.agents.read().await;
        if !agents
            .get(&agent_id)
            .is_some_and(|agent| agent.workspace_id == workspace_id)
        {
            return Err(bad_request(
                "Source agent must belong to the target workspace.",
            ));
        }
    }

    let artifact = Artifact {
        id: Uuid::new_v4(),
        workspace_id,
        title: title.to_owned(),
        content: input.content,
        source_agent_id: input.source_agent_id,
        captured_at: Utc::now(),
    };
    state
        .persist_entity(
            StoredEntity::Artifact(artifact.clone()),
            Vec::new(),
            AuditEvent::attributed(
                format!("human:{}", user.user_id),
                Some(workspace_id),
                "artifact.created",
                "artifact",
                artifact.id.to_string(),
                serde_json::json!({ "title": artifact.title, "source_agent_id": artifact.source_agent_id }),
            ),
        )
        .await
        .map_err(|_| storage_failure())?;
    state
        .artifacts
        .write()
        .await
        .insert(artifact.id, artifact.clone());

    Ok((StatusCode::CREATED, Json(artifact)))
}

pub async fn list_agents(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<Vec<Agent>>> {
    if !state.workspaces.read().await.contains_key(&workspace_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "Workspace not found.".into(),
            }),
        ));
    }

    let mut agents: Vec<_> = state
        .agents
        .read()
        .await
        .values()
        .filter(|agent| agent.workspace_id == workspace_id)
        .cloned()
        .collect();
    agents.sort_by_key(|agent| agent.created_at);
    Ok(Json(agents))
}

pub async fn list_artifacts(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<Vec<Artifact>>> {
    if !state.workspaces.read().await.contains_key(&workspace_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "Workspace not found.".into(),
            }),
        ));
    }

    let mut artifacts: Vec<_> = state
        .artifacts
        .read()
        .await
        .values()
        .filter(|artifact| artifact.workspace_id == workspace_id)
        .cloned()
        .collect();
    artifacts.sort_by_key(|artifact| artifact.captured_at);
    Ok(Json(artifacts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{SqliteStorage, Storage};
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_reports_development_constraints() {
        let response = crate::app(AppState::default())
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["storage"], "development_memory_only");
        assert_eq!(value["authentication"], "not_configured");
    }

    #[tokio::test]
    async fn workspace_and_audit_survive_runtime_reconstruction() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap().shared();
        let state = AppState::with_storage(storage.clone()).await.unwrap();
        let account = state.ensure_firebase_user("axon-test", "user-1").await.unwrap();
        let user = Arc::new(AuthenticatedUser {
            user_id: account.user_id,
            firebase_uid: "user-1".into(),
            project_id: "axon-test".into(),
            personal_workspace_id: account.workspace.id,
        });
        state.workspaces.write().await.insert(account.workspace.id, account.workspace);
        let created = create_workspace(
            State(state),
            Extension(user),
            Json(CreateWorkspace {
                name: "Persistent project".into(),
            }),
        )
        .await
        .unwrap();
        let workspace_id = created.1.id;

        let restored = AppState::with_storage(storage).await.unwrap();
        assert!(restored.workspaces.read().await.contains_key(&workspace_id));
        let events = restored.audit.read().await.list_workspace(workspace_id);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "workspace.created");
    }

    #[tokio::test]
    async fn workspace_creation_validates_and_persists_in_process() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap().shared();
        let state = AppState::with_storage(storage).await.unwrap();
        let account = state.ensure_firebase_user("axon-test", "user-2").await.unwrap();
        let user = Arc::new(AuthenticatedUser {
            user_id: account.user_id,
            firebase_uid: "user-2".into(),
            project_id: "axon-test".into(),
            personal_workspace_id: account.workspace.id,
        });
        state.workspaces.write().await.insert(account.workspace.id, account.workspace);
        let response = create_workspace(
            State(state.clone()),
            Extension(user),
            Json(CreateWorkspace {
                name: "Funspot engineering".into(),
            }),
        )
        .await
        .unwrap();

        assert_eq!(response.0, StatusCode::CREATED);
        assert_eq!(state.workspaces.read().await.len(), 2);
        let audit = storage.list_audit_events(response.1.id).await.unwrap();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].event_type, "workspace.created");
    }
}
