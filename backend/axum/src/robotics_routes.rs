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
        crate::robotics::ProposalRejection::TelemetryPayloadTooLarge => (
            StatusCode::PAYLOAD_TOO_LARGE,
            "telemetry_payload_too_large",
            "Telemetry payload must contain at most 128 fields and 64 KiB of JSON.",
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
    use crate::robotics::{TelemetryKind, TelemetryQuality};
    use chrono::Duration;

    fn input(event_id: &str, sequence: u64) -> TelemetryIngest {
        TelemetryIngest {
            event_id: event_id.into(),
            sequence,
            observed_at: Utc::now() - Duration::seconds(1),
            kind: TelemetryKind::Heartbeat,
            frame_id: None,
            payload: serde_json::Map::new(),
            quality: TelemetryQuality::Simulated,
        }
    }

    #[tokio::test]
    async fn simulator_ingress_accepts_then_rejects_duplicate_event_id() {
        let state = AppState::default();
        let accepted = ingest_simulated_telemetry(
            State(state.clone()),
            Path("sim-test".into()),
            Json(input("event-1", 1)),
        )
        .await
        .expect("first event should be accepted");
        assert_eq!(accepted.0, StatusCode::ACCEPTED);

        let duplicate = ingest_simulated_telemetry(
            State(state.clone()),
            Path("sim-test".into()),
            Json(input("event-1", 2)),
        )
        .await
        .expect_err("duplicate event id must be rejected");
        assert_eq!(duplicate.0, StatusCode::CONFLICT);
        assert_eq!(
            state.telemetry.read().await.last_sequence_by_device.get("sim-test"),
            Some(&1)
        );
    }

    #[tokio::test]
    async fn simulator_ingress_rejects_replayed_sequence_without_mutating_store() {
        let state = AppState::default();
        ingest_simulated_telemetry(
            State(state.clone()),
            Path("sim-test".into()),
            Json(input("event-1", 5)),
        )
        .await
        .expect("first event should be accepted");

        let replay = ingest_simulated_telemetry(
            State(state.clone()),
            Path("sim-test".into()),
            Json(input("event-2", 5)),
        )
        .await
        .expect_err("sequence replay must be rejected");
        assert_eq!(replay.0, StatusCode::CONFLICT);
        let store = state.telemetry.read().await;
        assert_eq!(store.events_by_device["sim-test"].len(), 1);
        assert_eq!(store.last_sequence_by_device["sim-test"], 5);
    }

    #[tokio::test]
    async fn simulator_ingress_retains_only_latest_five_hundred_events() {
        let state = AppState::default();
        for sequence in 1..=501 {
            ingest_simulated_telemetry(
                State(state.clone()),
                Path("sim-test".into()),
                Json(input(&format!("event-{sequence}"), sequence)),
            )
            .await
            .expect("monotonic simulator event should be accepted");
        }

        let store = state.telemetry.read().await;
        let events = &store.events_by_device["sim-test"];
        assert_eq!(events.len(), 500);
        assert_eq!(events.first().map(|event| event.sequence), Some(2));
        assert_eq!(events.last().map(|event| event.sequence), Some(501));
        assert!(!store.event_ids.contains("event-1"));
    }
}
