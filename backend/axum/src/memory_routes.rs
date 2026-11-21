use crate::{
    memory::{
        validate_import, validate_memory, CreateMemoryRequest, ImportConversationRequest,
        MemoryRecord, MemoryRepository, MemoryStatus, ReviewMemoryRequest,
    },
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct MemoryApiError {
    error: String,
}

type ApiResult<T> = Result<T, (StatusCode, Json<MemoryApiError>)>;

fn api_error(status: StatusCode, message: impl Into<String>) -> (StatusCode, Json<MemoryApiError>) {
    (status, Json(MemoryApiError { error: message.into() }))
}

fn repository(state: &AppState) -> ApiResult<&MemoryRepository> {
    state.memory.as_deref().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Durable memory is unavailable. Configure MONGODB_URI and restart Axon.",
        )
    })
}

pub async fn import_conversation(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Json(input): Json<ImportConversationRequest>,
) -> ApiResult<(StatusCode, Json<crate::memory::ConversationImport>)> {
    validate_import(&input).map_err(|message| api_error(StatusCode::BAD_REQUEST, message))?;
    let record = repository(&state)?
        .import_conversation(workspace_id, input)
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok((StatusCode::CREATED, Json(record)))
}

pub async fn create_memory(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Json(input): Json<CreateMemoryRequest>,
) -> ApiResult<(StatusCode, Json<MemoryRecord>)> {
    validate_memory(&input).map_err(|message| api_error(StatusCode::BAD_REQUEST, message))?;
    let record = repository(&state)?
        .create_memory(workspace_id, input)
        .await
        .map_err(|error| {
            let message = error.to_string();
            let status = if message.contains("Source conversation import was not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            api_error(status, message)
        })?;
    Ok((StatusCode::CREATED, Json(record)))
}

#[derive(Debug, Deserialize)]
pub struct ListMemoriesQuery {
    pub status: Option<MemoryStatus>,
    pub q: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_memories(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListMemoriesQuery>,
) -> ApiResult<Json<Vec<MemoryRecord>>> {
    if query.q.as_ref().is_some_and(|value| value.len() > 120) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Search query cannot exceed 120 characters.",
        ));
    }
    let memories = repository(&state)?
        .list_memories(
            workspace_id,
            query.status,
            query.q.as_deref(),
            query.limit.unwrap_or(50),
        )
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(memories))
}

pub async fn review_memory(
    State(state): State<AppState>,
    Path((workspace_id, memory_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<ReviewMemoryRequest>,
) -> ApiResult<Json<MemoryRecord>> {
    let record = repository(&state)?
        .review_memory(workspace_id, memory_id, input)
        .await
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, error.to_string()))?;
    record.map(Json).ok_or_else(|| {
        api_error(
            StatusCode::CONFLICT,
            "Memory was not found, is not in the required review state, or its version changed. \
             Reload before retrying.",
        )
    })
}

pub async fn get_import(
    State(state): State<AppState>,
    Path((workspace_id, import_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<crate::memory::ConversationImport>> {
    let record = repository(&state)?
        .get_import(workspace_id, import_id)
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    record.map(Json).ok_or_else(|| {
        api_error(StatusCode::NOT_FOUND, "Conversation import was not found in this workspace.")
    })
}

pub async fn context_memories(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListMemoriesQuery>,
) -> ApiResult<Json<Vec<MemoryRecord>>> {
    if query.q.as_ref().is_some_and(|value| value.len() > 120) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Search query cannot exceed 120 characters.",
        ));
    }
    let memories = repository(&state)?
        .list_memories(
            workspace_id,
            Some(MemoryStatus::Active),
            query.q.as_deref(),
            query.limit.unwrap_or(20),
        )
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(memories))
}
