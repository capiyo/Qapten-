use crate::{
    capability::{CapabilityPoll, CapabilityVoteResult, ModelCandidate},
    memory::MemoryRepository,
    AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct CapabilityApiError {
    error: String,
}

type ApiResult<T> = Result<T, (StatusCode, Json<CapabilityApiError>)>;

fn api_error(
    status: StatusCode,
    message: impl Into<String>,
) -> (StatusCode, Json<CapabilityApiError>) {
    (status, Json(CapabilityApiError { error: message.into() }))
}

fn repository(state: &AppState) -> ApiResult<&MemoryRepository> {
    state.memory.as_deref().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Durable storage is unavailable. Configure MONGODB_URI and restart Axon.",
        )
    })
}

#[derive(Debug, Deserialize)]
pub struct CreateCapabilityPollRequest {
    pub capability: String,
    pub task_brief: String,
    pub electorate: Vec<String>,
    pub candidates: Vec<ModelCandidate>,
}

pub async fn create_poll(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Json(input): Json<CreateCapabilityPollRequest>,
) -> ApiResult<(StatusCode, Json<CapabilityPoll>)> {
    if input.capability.trim().is_empty()
        || input.capability.len() > 120
        || input.task_brief.trim().is_empty()
        || input.task_brief.len() > 10_000
        || input.electorate.len() > 30
        || input.candidates.len() > 50
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Capability, task brief, electorate, or candidate bounds are invalid.",
        ));
    }
    let poll = CapabilityPoll::new(
        workspace_id,
        input.capability,
        input.task_brief,
        input.electorate,
        input.candidates,
    )
    .map_err(|error| api_error(StatusCode::BAD_REQUEST, format!("{error:?}")))?;
    let poll = repository(&state)?
        .create_capability_poll(poll)
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok((StatusCode::CREATED, Json(poll)))
}

#[derive(Debug, Deserialize)]
pub struct SubmitNominationRequest {
    pub voter_model_id: String,
    pub nominated_model_id: String,
    pub rationale: String,
}

pub async fn submit_nomination(
    State(state): State<AppState>,
    Path((workspace_id, poll_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<SubmitNominationRequest>,
) -> ApiResult<Json<CapabilityPoll>> {
    if input.voter_model_id.trim().is_empty()
        || input.voter_model_id.len() > 160
        || input.nominated_model_id.trim().is_empty()
        || input.nominated_model_id.len() > 160
        || input.rationale.trim().is_empty()
        || input.rationale.len() > 2_000
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Voter, candidate, and rationale must be non-empty and within their size limits.",
        ));
    }
    let poll = repository(&state)?
        .submit_capability_nomination(
            workspace_id,
            poll_id,
            &input.voter_model_id,
            &input.nominated_model_id,
            &input.rationale,
        )
        .await
        .map_err(|error| {
            let message = error.to_string();
            let status = if message.contains("version conflict") {
                StatusCode::CONFLICT
            } else if message.contains("not found") {
                StatusCode::NOT_FOUND
            } else if message.contains("PollClosed") {
                StatusCode::CONFLICT
            } else {
                StatusCode::BAD_REQUEST
            };
            api_error(status, message)
        })?;
    Ok(Json(poll))
}

#[derive(Debug, Serialize)]
pub struct FinalizedCapabilityPoll {
    pub poll: CapabilityPoll,
    pub result: CapabilityVoteResult,
}

pub async fn finalize_poll(
    State(state): State<AppState>,
    Path((workspace_id, poll_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<FinalizedCapabilityPoll>> {
    let (poll, result) = repository(&state)?
        .finalize_capability_poll(workspace_id, poll_id)
        .await
        .map_err(|error| {
            let message = error.to_string();
            let status = if message.contains("not found") {
                StatusCode::NOT_FOUND
            } else if message.contains("conflict") || message.contains("PollClosed") {
                StatusCode::CONFLICT
            } else {
                StatusCode::BAD_REQUEST
            };
            api_error(status, message)
        })?;
    Ok(Json(FinalizedCapabilityPoll { poll, result }))
}

pub async fn get_poll(
    State(state): State<AppState>,
    Path((workspace_id, poll_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<CapabilityPoll>> {
    let poll = repository(&state)?
        .get_capability_poll(workspace_id, poll_id)
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    poll.map(Json).ok_or_else(|| {
        api_error(StatusCode::NOT_FOUND, "Capability poll was not found in this workspace.")
    })
}
