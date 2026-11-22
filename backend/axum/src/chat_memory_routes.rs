use crate::{
    auth_middleware::AuthenticatedUser,
    chat_memory::{
        Chat, ChatDetail, CreateChat, CreateMemoryEntry, CreateMessage, EditMemoryEntry,
    },
    AppState,
};
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

type ApiResult<T> = Result<T, (StatusCode, Json<ApiError>)>;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

fn error(status: StatusCode, message: &str) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            error: message.into(),
        }),
    )
}

fn store(
    state: &AppState,
) -> Result<&crate::chat_memory::ChatMemoryStore, (StatusCode, Json<ApiError>)> {
    state.chat_memory.as_ref().ok_or_else(|| {
        error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Persistent chat storage is unavailable.",
        )
    })
}

fn actor(user: &AuthenticatedUser) -> String {
    format!("human:{}", user.user_id)
}

pub async fn list_chats(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<Vec<Chat>>> {
    let chats = store(&state)?
        .list_chats(workspace_id)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Could not load chats."))?;
    Ok(Json(chats))
}

pub async fn create_chat(
    State(state): State<AppState>,
    Extension(user): Extension<Arc<AuthenticatedUser>>,
    Path(workspace_id): Path<Uuid>,
    Json(input): Json<CreateChat>,
) -> ApiResult<(StatusCode, Json<Chat>)> {
    let title = input.title.trim();
    if title.is_empty() || title.len() > 160 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Chat title must contain 1–160 characters.",
        ));
    }
    let chat = store(&state)?
        .create_chat(workspace_id, user.user_id, &actor(&user), title.to_owned())
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Could not create chat."))?;
    Ok((StatusCode::CREATED, Json(chat)))
}

pub async fn get_chat(
    State(state): State<AppState>,
    Path((workspace_id, chat_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<ChatDetail>> {
    let detail = store(&state)?
        .get_chat(workspace_id, chat_id)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Could not load chat."))?;
    detail
        .map(Json)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "Chat not found."))
}

pub async fn append_message(
    State(state): State<AppState>,
    Extension(user): Extension<Arc<AuthenticatedUser>>,
    Path((workspace_id, chat_id)): Path<(Uuid, Uuid)>,
    Json(mut input): Json<CreateMessage>,
) -> ApiResult<(StatusCode, Json<crate::chat_memory::ChatMessage>)> {
    input.role = input.role.trim().to_ascii_lowercase();
    if !["user", "assistant", "system"].contains(&input.role.as_str()) {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Message role must be user, assistant, or system.",
        ));
    }
    if input.content.trim().is_empty() || input.content.len() > 256_000 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Message content must contain 1–256000 bytes.",
        ));
    }
    if input.role != "assistant" && input.answered_by.is_some() {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "answered_by is only valid for assistant messages.",
        ));
    }
    input.content = input.content.trim().to_owned();
    let message = store(&state)?
        .append_message(workspace_id, chat_id, &actor(&user), input)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Could not save message."))?;
    message
        .map(|item| (StatusCode::CREATED, Json(item)))
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "Chat not found."))
}

pub async fn archive_chat(
    State(state): State<AppState>,
    Extension(user): Extension<Arc<AuthenticatedUser>>,
    Path((workspace_id, chat_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let archived = store(&state)?
        .archive_chat(workspace_id, chat_id, &actor(&user))
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Could not archive chat."))?;
    if archived {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(error(StatusCode::NOT_FOUND, "Chat not found."))
    }
}

pub async fn create_memory_entry(
    State(state): State<AppState>,
    Extension(user): Extension<Arc<AuthenticatedUser>>,
    Path((workspace_id, chat_id)): Path<(Uuid, Uuid)>,
    Json(mut input): Json<CreateMemoryEntry>,
) -> ApiResult<(StatusCode, Json<crate::chat_memory::MemoryEntry>)> {
    input.entry_type = input.entry_type.trim().to_ascii_lowercase();
    if !["decision", "fact", "constraint", "open_task", "note"].contains(&input.entry_type.as_str())
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Memory type must be decision, fact, constraint, open_task, or note.",
        ));
    }
    if input.text.trim().is_empty() || input.text.len() > 4000 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Memory text must contain 1–4000 bytes.",
        ));
    }
    input.text = input.text.trim().to_owned();
    let entry = store(&state)?
        .create_memory_entry(workspace_id, chat_id, &actor(&user), input)
        .await
        .map_err(|_| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not create memory entry.",
            )
        })?;
    entry
        .map(|item| (StatusCode::CREATED, Json(item)))
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "Chat not found."))
}

pub async fn mutate_memory_entry(
    State(state): State<AppState>,
    Extension(user): Extension<Arc<AuthenticatedUser>>,
    Path((workspace_id, chat_id, entry_id, action)): Path<(Uuid, Uuid, Uuid, String)>,
    input: Option<Json<EditMemoryEntry>>,
) -> ApiResult<Json<crate::chat_memory::MemoryEntry>> {
    let action = action.to_ascii_lowercase();
    if !["approve", "reject", "edit", "delete"].contains(&action.as_str()) {
        return Err(error(StatusCode::NOT_FOUND, "Memory action not found."));
    }
    let edit = if action == "edit" {
        let Some(Json(value)) = input else {
            return Err(error(
                StatusCode::BAD_REQUEST,
                "An edited memory text is required.",
            ));
        };
        if value.text.trim().is_empty() || value.text.len() > 4000 {
            return Err(error(
                StatusCode::BAD_REQUEST,
                "Memory text must contain 1–4000 bytes.",
            ));
        }
        Some(EditMemoryEntry {
            text: value.text.trim().to_owned(),
            reason: value.reason,
        })
    } else {
        None
    };
    let entry = store(&state)?
        .mutate_memory_entry(
            workspace_id,
            chat_id,
            entry_id,
            &actor(&user),
            &action,
            edit,
        )
        .await
        .map_err(|_| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not update memory entry.",
            )
        })?;
    entry
        .map(Json)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "Memory entry not found."))
}
