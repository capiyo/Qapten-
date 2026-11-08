use crate::{
    model::{Artifact, CreateTask, SubmitTask, Task, TaskStatus, TaskTransition, TransitionTask},
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

type TaskResult<T> = Result<T, (StatusCode, Json<TaskApiError>)>;

#[derive(Serialize)]
pub struct TaskApiError {
    error: String,
    current_version: Option<u64>,
}

fn error(status: StatusCode, message: impl Into<String>) -> (StatusCode, Json<TaskApiError>) {
    (
        status,
        Json(TaskApiError {
            error: message.into(),
            current_version: None,
        }),
    )
}

pub async fn create_task(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Json(input): Json<CreateTask>,
) -> TaskResult<(StatusCode, Json<Task>)> {
    if !state
        .workspaces
        .read()
        .await
        .contains_key(&workspace_id)
    {
        return Err(error(StatusCode::NOT_FOUND, "Workspace not found."));
    }

    let title = input.title.trim();
    let objective = input.objective.trim();
    if title.is_empty() || title.len() > 160 || objective.is_empty() || objective.len() > 10_000 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Task title must be 1–160 characters and objective 1–10000 characters.",
        ));
    }

    let agents = state.agents.read().await;
    if !agents
        .get(&input.assignee_agent_id)
        .is_some_and(|agent| agent.workspace_id == workspace_id)
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Assignee agent must belong to the target workspace.",
        ));
    }
    drop(agents);

    let artifacts = state.artifacts.read().await;
    if input.context_artifact_ids.len() > 50
        || input.context_artifact_ids.iter().any(|id| {
            !artifacts
                .get(id)
                .is_some_and(|artifact| artifact.workspace_id == workspace_id)
        })
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Every context artifact must exist in this workspace; at most 50 references are allowed.",
        ));
    }
    drop(artifacts);

    let now = Utc::now();
    let task = Task {
        id: Uuid::new_v4(),
        workspace_id,
        title: title.to_owned(),
        objective: objective.to_owned(),
        assignee_agent_id: input.assignee_agent_id,
        context_artifact_ids: input.context_artifact_ids,
        response_artifact_id: None,
        status: TaskStatus::Offered,
        version: 1,
        created_at: now,
        updated_at: now.clone(),
        transitions: vec![TaskTransition {
            from: None,
            to: TaskStatus::Offered,
            note: "Task created and offered to the assigned agent.".into(),
            occurred_at: now,
        }],
    };

    state.tasks.write().await.insert(task.id, task.clone());
    Ok((StatusCode::CREATED, Json(task)))
}

pub async fn list_tasks(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> TaskResult<Json<Vec<Task>>> {
    if !state
        .workspaces
        .read()
        .await
        .contains_key(&workspace_id)
    {
        return Err(error(StatusCode::NOT_FOUND, "Workspace not found."));
    }

    let mut tasks: Vec<_> = state
        .tasks
        .read()
        .await
        .values()
        .filter(|task| task.workspace_id == workspace_id)
        .cloned()
        .collect();
    tasks.sort_by(|left, right| left.created_at.cmp(&right.created_at));
    Ok(Json(tasks))
}

fn allowed_transition(from: &TaskStatus, to: &TaskStatus) -> bool {
    use TaskStatus::*;

    matches!(
        (from, to),
        (Offered, Acknowledged | Rejected | Cancelled)
            | (Acknowledged, InProgress | Cancelled)
            | (InProgress, Submitted | Cancelled)
            | (Submitted, ReviewRequired)
            | (ReviewRequired, Accepted | ChangesRequested)
            | (ChangesRequested, InProgress | Cancelled)
            | (Accepted, Completed)
    )
}

#[derive(Serialize)]
pub struct TaskSubmission {
    pub task: Task,
    pub artifact: Artifact,
}

pub async fn submit_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(input): Json<SubmitTask>,
) -> TaskResult<Json<TaskSubmission>> {
    let title = input.title.trim();
    let content = input.content.trim();
    if title.is_empty()
        || title.len() > 200
        || content.is_empty()
        || content.len() > 512_000
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Response title must be 1–200 characters and response content 1–512000 bytes.",
        ));
    }

    let mut tasks = state.tasks.write().await;
    let Some(task) = tasks.get_mut(&task_id) else {
        return Err(error(StatusCode::NOT_FOUND, "Task not found."));
    };
    if task.version != input.expected_version {
        return Err((
            StatusCode::CONFLICT,
            Json(TaskApiError {
                error: "Task version conflict. Reload the task before retrying.".into(),
                current_version: Some(task.version),
            }),
        ));
    }
    if task.status != TaskStatus::InProgress {
        return Err(error(
            StatusCode::CONFLICT,
            "A response can only be submitted while the task is in progress.",
        ));
    }

    let now = Utc::now();
    let artifact = Artifact {
        id: Uuid::new_v4(),
        workspace_id: task.workspace_id,
        title: title.to_owned(),
        content: content.to_owned(),
        source_agent_id: Some(task.assignee_agent_id),
        captured_at: now.clone(),
    };
    state
        .artifacts
        .write()
        .await
        .insert(artifact.id, artifact.clone());

    task.response_artifact_id = Some(artifact.id);
    task.status = TaskStatus::Submitted;
    task.version += 1;
    task.updated_at = now;
    task.transitions.push(TaskTransition {
        from: Some(TaskStatus::InProgress),
        to: TaskStatus::Submitted,
        note: format!(
            "Response captured as artifact {artifact_id}.",
            artifact_id = artifact.id
        ),
        occurred_at: now,
    });

    Ok(Json(TaskSubmission {
        task: task.clone(),
        artifact,
    }))
}

pub async fn transition_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(input): Json<TransitionTask>,
) -> TaskResult<Json<Task>> {
    let note = input.note.trim();
    if note.is_empty() || note.len() > 2_000 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "A transition note of 1–2000 characters is required.",
        ));
    }

    let mut tasks = state.tasks.write().await;
    let Some(task) = tasks.get_mut(&task_id) else {
        return Err(error(StatusCode::NOT_FOUND, "Task not found."));
    };

    if task.version != input.expected_version {
        return Err((
            StatusCode::CONFLICT,
            Json(TaskApiError {
                error: "Task version conflict. Reload the task before retrying.".into(),
                current_version: Some(task.version),
            }),
        ));
    }

    if !allowed_transition(&task.status, &input.status) {
        return Err(error(
            StatusCode::CONFLICT,
            format!(
                "Transition from {from:?} to {to:?} is not allowed.",
                from = task.status,
                to = input.status
            ),
        ));
    }

    let previous = task.status.clone();
    let now = Utc::now();
    task.status = input.status.clone();
    task.version += 1;
    task.updated_at = now;
    task.transitions.push(TaskTransition {
        from: Some(previous),
        to: input.status,
        note: note.to_owned(),
        occurred_at: now,
    });

    Ok(Json(task.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_lifecycle_rejects_skipping_review() {
        assert!(!allowed_transition(
            &TaskStatus::Submitted,
            &TaskStatus::Completed
        ));
        assert!(!allowed_transition(
            &TaskStatus::Offered,
            &TaskStatus::InProgress
        ));
    }

    #[test]
    fn task_lifecycle_supports_review_and_revision() {
        assert!(allowed_transition(
            &TaskStatus::Submitted,
            &TaskStatus::ReviewRequired
        ));
        assert!(allowed_transition(
            &TaskStatus::ReviewRequired,
            &TaskStatus::ChangesRequested
        ));
        assert!(allowed_transition(
            &TaskStatus::ChangesRequested,
            &TaskStatus::InProgress
        ));
        assert!(allowed_transition(
            &TaskStatus::Accepted,
            &TaskStatus::Completed
        ));
    }
}
