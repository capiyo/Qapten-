use crate::{
    model::{
        Agent, Artifact, ConnectionMode, CreateArtifact, CreateWorkspace, RegisterAgent, Workspace,
    },
    AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct Health {
    status: &'static str,
    service: &'static str,
    storage: &'static str,
    authentication: &'static str,
}

pub async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        service: "qapten-api",
        storage: "development_memory_only",
        authentication: "not_implemented_local_only",
    })
}

#[derive(Serialize)]
pub struct ApiError {
    error: String,
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
    Json(input): Json<CreateWorkspace>,
) -> Result<(StatusCode, Json<Workspace>), (StatusCode, Json<ApiError>)> {
    let name = input.name.trim();
    if name.is_empty() || name.len() > 120 {
        return Err(bad_request(
            "Workspace name must contain between 1 and 120 characters.",
        ));
    }

    let workspace = Workspace {
        id: Uuid::new_v4(),
        name: name.to_owned(),
        created_at: Utc::now(),
    };
    state
        .workspaces
        .write()
        .await
        .insert(workspace.id, workspace.clone());

    Ok((StatusCode::CREATED, Json(workspace)))
}

pub async fn list_workspaces(State(state): State<AppState>) -> Json<Vec<Workspace>> {
    let mut items: Vec<_> = state.workspaces.read().await.values().cloned().collect();
    items.sort_by_key(|item| item.created_at);
    Json(items)
}

pub async fn register_agent(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Json(input): Json<RegisterAgent>,
) -> Result<(StatusCode, Json<Agent>), (StatusCode, Json<ApiError>)> {
    if !state
        .workspaces
        .read()
        .await
        .contains_key(&workspace_id)
    {
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
    state.agents.write().await.insert(agent.id, agent.clone());

    Ok((StatusCode::CREATED, Json(agent)))
}

pub async fn create_artifact(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Json(input): Json<CreateArtifact>,
) -> Result<(StatusCode, Json<Artifact>), (StatusCode, Json<ApiError>)> {
    if !state
        .workspaces
        .read()
        .await
        .contains_key(&workspace_id)
    {
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
        .artifacts
        .write()
        .await
        .insert(artifact.id, artifact.clone());

    Ok((StatusCode::CREATED, Json(artifact)))
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(
            value["authentication"],
            "not_implemented_local_only"
        );
    }

    #[tokio::test]
    async fn workspace_creation_validates_and_persists_in_process() {
        let state = AppState::default();
        let response = create_workspace(
            State(state.clone()),
            Json(CreateWorkspace {
                name: "Funspot engineering".into(),
            }),
        )
        .await
        .unwrap();

        assert_eq!(response.0, StatusCode::CREATED);
        assert_eq!(state.workspaces.read().await.len(), 1);
    }
}

pub async fn list_agents(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<Agent>>, (StatusCode, Json<ApiError>)> {
    if !state
        .workspaces
        .read()
        .await
        .contains_key(&workspace_id)
    {
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
) -> Result<Json<Vec<Artifact>>, (StatusCode, Json<ApiError>)> {
    if !state
        .workspaces
        .read()
        .await
        .contains_key(&workspace_id)
    {
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
