use crate::{
    robotics::{validate_simulated_telemetry, TelemetryEvent, TelemetryIngest},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RoboticsApiError {
    error: String,
    code: &'static str,
}

fn rejection_status(
    rejection: crate::robotics::ProposalRejection,
) -> (StatusCode, Json<RoboticsApiError>) {
    let (status, code, message) = match rejection {
        crate::robotics::ProposalRejection::TelemetryDeviceMismatch => (
            StatusCode::BAD_REQUEST,
            "telemetry_device_mismatch",
            "The telemetry device identifier is invalid.",
        ),
        crate::robotics::ProposalRejection::TelemetryNotSimulated => (
            StatusCode::FORBIDDEN,
            "simulator_only",
            "This endpoint accepts simulated telemetry only.",
        ),
        crate::robotics::ProposalRejection::TelemetrySequenceReplay => (
            StatusCode::CONFLICT,
            "telemetry_sequence_replay",
            "Telemetry sequence must be strictly greater than the last accepted sequence.",
        ),
        crate::robotics::ProposalRejection::InvalidTelemetryEvent => (
            StatusCode::BAD_REQUEST,
            "invalid_telemetry_event",
            "Telemetry event ID is required and must not exceed 128 characters.",
        ),
        crate::robotics::ProposalRejection::StaleTelemetry => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "stale_telemetry",
            "Observed telemetry is older than the allowed simulator freshness window.",
        ),
        crate::robotics::ProposalRejection::FutureTelemetry => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "future_telemetry",
            "Observed telemetry exceeds the allowed future clock-skew window.",
        ),
        _ => (
            StatusCode::BAD_REQUEST,
            "invalid_telemetry",
            "Telemetry failed validation.",
        ),
    };
    (
        status,
        Json(RoboticsApiError {
            error: message.into(),
            code,
        }),
    )
}

/// Development-only simulator ingress. The service itself refuses non-loopback
/// binding until authentication is implemented.
pub async fn ingest_simulated_telemetry(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Json(input): Json<TelemetryIngest>,
) -> Result<(StatusCode, Json<TelemetryEvent>), (StatusCode, Json<RoboticsApiError>)> {
    let now = Utc::now();
    let mut store = state.telemetry.write().await;
    if store.event_ids.contains(&input.event_id) {
        return Err((
            StatusCode::CONFLICT,
            Json(RoboticsApiError {
                error: "Telemetry event ID has already been accepted.".into(),
                code: "duplicate_telemetry_event",
            }),
        ));
    }

    let last_sequence = store.last_sequence_by_device.get(&device_id).copied();
    validate_simulated_telemetry(&input, &device_id, last_sequence, now, 30, 2)
        .map_err(rejection_status)?;

    let event = TelemetryEvent {
        event_id: input.event_id.clone(),
        device_id: device_id.clone(),
        sequence: input.sequence,
        observed_at: input.observed_at,
        received_at: now,
        kind: input.kind,
        frame_id: input.frame_id,
        payload: input.payload,
        quality: Some(input.quality),
    };

    store.event_ids.insert(event.event_id.clone());
    store
        .last_sequence_by_device
        .insert(device_id.clone(), event.sequence);
    let events = store.events_by_device.entry(device_id).or_default();
    events.push(event.clone());
    if events.len() > 500 {
        let removed = events.remove(0);
        store.event_ids.remove(&removed.event_id);
    }

    Ok((StatusCode::ACCEPTED, Json(event)))
}

pub async fn list_simulated_telemetry(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Json<Vec<TelemetryEvent>> {
    let store = state.telemetry.read().await;
    Json(
        store
            .events_by_device
            .get(&device_id)
            .cloned()
            .unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn simulator_ingress_accepts_fresh_event_and_rejects_duplicate_id() {
        let state = AppState::default();
        let app = crate::app(state);
        let payload = serde_json::json!({
            "event_id": "sim-event-1",
            "sequence": 1,
            "observed_at": Utc::now().to_rfc3339(),
            "kind": "heartbeat",
            "payload": {},
            "quality": "simulated"
        });
        let request = || {
            Request::builder()
                .method("POST")
                .uri("/api/v1/sim/devices/simulator-1/telemetry")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap()
        };

        let accepted = app.clone().oneshot(request()).await.unwrap();
        assert_eq!(accepted.status(), StatusCode::ACCEPTED);

        let duplicate = app.oneshot(request()).await.unwrap();
        assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn simulator_ingress_rejects_measured_quality() {
        let app = crate::app(AppState::default());
        let payload = serde_json::json!({
            "event_id": "physical-event-1",
            "sequence": 1,
            "observed_at": Utc::now().to_rfc3339(),
            "kind": "heartbeat",
            "payload": {},
            "quality": "measured"
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/sim/devices/simulator-1/telemetry")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
