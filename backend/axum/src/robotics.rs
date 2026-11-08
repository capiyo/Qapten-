//! Robotics domain validation only. This module deliberately has no actuator or
//! device-network client: accepted proposals remain records for later review.
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    MobileRobot,
    RobotArm,
    Drone,
    Sensor,
    Simulator,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistration {
    pub device_id: String,
    pub workspace_id: String,
    pub device_type: DeviceKind,
    pub manufacturer: String,
    pub model: String,
    pub identity_key_fingerprint: String,
    pub capabilities: Vec<String>,
    pub firmware_version: Option<String>,
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SafetyMode {
    Disabled,
    Manual,
    Assisted,
    Autonomous,
    Maintenance,
    Fault,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyState {
    pub device_id: String,
    pub emergency_stop: bool,
    pub interlocks_healthy: bool,
    pub operating_mode: SafetyMode,
    pub observed_at: DateTime<Utc>,
    pub source: String,
    pub fault_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProposedOperation {
    MoveBase,
    SetJointTarget,
    Stop,
    Home,
    Custom,
}

/// An intent record, never a command to an actuator.
fn default_simulation_only() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandProposal {
    pub proposal_id: String,
    pub device_id: String,
    pub requester_id: String,
    pub operation: ProposedOperation,
    pub sequence: u64,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub idempotency_key: String,
    pub reason: String,
    pub parameters: serde_json::Value,
    #[serde(default = "default_simulation_only")]
    pub simulation_only: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct SafetyLimits {
    pub max_velocity_mps: f64,
    pub max_command_age_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalRejection {
    DeviceMismatch,
    NotAuthorized,
    EmergencyStopActive,
    InterlocksUnhealthy,
    UnsafeOperatingMode,
    ExpiredOrInvalidWindow,
    StaleSequence,
    InvalidIdempotencyKey,
    VelocityOutOfBounds,
    InvalidParameters,
    InvalidPolicyLimits,
}

pub fn validate_proposal(
    proposal: &CommandProposal,
    authenticated_device_id: &str,
    requester_authorized: bool,
    safety: &SafetyState,
    last_accepted_sequence: u64,
    now: DateTime<Utc>,
    limits: SafetyLimits,
) -> Result<(), ProposalRejection> {
    if proposal.device_id != authenticated_device_id || safety.device_id != authenticated_device_id
    {
        return Err(ProposalRejection::DeviceMismatch);
    }
    if !requester_authorized {
        return Err(ProposalRejection::NotAuthorized);
    }
    if safety.emergency_stop {
        return Err(ProposalRejection::EmergencyStopActive);
    }
    if !safety.interlocks_healthy {
        return Err(ProposalRejection::InterlocksUnhealthy);
    }
    if !matches!(
        safety.operating_mode,
        SafetyMode::Manual | SafetyMode::Assisted
    ) {
        return Err(ProposalRejection::UnsafeOperatingMode);
    }
    if !limits.max_velocity_mps.is_finite()
        || limits.max_velocity_mps < 0.0
        || limits.max_command_age_seconds < 0
    {
        return Err(ProposalRejection::InvalidPolicyLimits);
    }
    if proposal.expires_at <= proposal.issued_at
        || proposal.expires_at <= now
        || proposal.issued_at > now
        || now.signed_duration_since(proposal.issued_at)
            > Duration::seconds(limits.max_command_age_seconds)
    {
        return Err(ProposalRejection::ExpiredOrInvalidWindow);
    }
    if proposal.sequence <= last_accepted_sequence {
        return Err(ProposalRejection::StaleSequence);
    }
    if proposal.idempotency_key.trim().len() < 16 {
        return Err(ProposalRejection::InvalidIdempotencyKey);
    }
    if !proposal.parameters.is_object() {
        return Err(ProposalRejection::InvalidParameters);
    }
    if let Some(value) = proposal.parameters.get("linear_velocity_mps") {
        let velocity = value.as_f64().ok_or(ProposalRejection::InvalidParameters)?;
        if !velocity.is_finite() {
            return Err(ProposalRejection::InvalidParameters);
        }
        if velocity.abs() > limits.max_velocity_mps {
            return Err(ProposalRejection::VelocityOutOfBounds);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn fixture() -> (CommandProposal, SafetyState, DateTime<Utc>, SafetyLimits) {
        let now = Utc::now();
        let proposal = CommandProposal {
            proposal_id: "proposal-1".into(),
            device_id: "robot-1".into(),
            requester_id: "agent-1".into(),
            operation: ProposedOperation::MoveBase,
            sequence: 7,
            issued_at: now - Duration::seconds(2),
            expires_at: now + Duration::seconds(10),
            idempotency_key: "idempotency-key-0001".into(),
            reason: "Navigate to waypoint".into(),
            parameters: serde_json::json!({"linear_velocity_mps": 0.4}),
            simulation_only: true,
        };
        let safety = SafetyState {
            device_id: "robot-1".into(),
            emergency_stop: false,
            interlocks_healthy: true,
            operating_mode: SafetyMode::Assisted,
            observed_at: now,
            source: "simulator".into(),
            fault_codes: vec![],
        };
        (
            proposal,
            safety,
            now,
            SafetyLimits {
                max_velocity_mps: 0.5,
                max_command_age_seconds: 5,
            },
        )
    }

    #[test]
    fn accepts_fresh_authorized_bounded_proposal() {
        let (proposal, safety, now, limits) = fixture();
        assert_eq!(
            validate_proposal(&proposal, "robot-1", true, &safety, 6, now, limits),
            Ok(())
        );
    }

    #[test]
    fn rejects_unauthorized_requester() {
        let (proposal, safety, now, limits) = fixture();
        assert_eq!(
            validate_proposal(&proposal, "robot-1", false, &safety, 6, now, limits),
            Err(ProposalRejection::NotAuthorized)
        );
    }

    #[test]
    fn emergency_stop_overrides_valid_proposal() {
        let (proposal, mut safety, now, limits) = fixture();
        safety.emergency_stop = true;
        assert_eq!(
            validate_proposal(&proposal, "robot-1", true, &safety, 6, now, limits),
            Err(ProposalRejection::EmergencyStopActive)
        );
    }

    #[test]
    fn rejects_stale_sequence_and_expired_proposal() {
        let (mut proposal, safety, now, limits) = fixture();
        assert_eq!(
            validate_proposal(&proposal, "robot-1", true, &safety, 7, now, limits),
            Err(ProposalRejection::StaleSequence)
        );
        proposal.expires_at = now - Duration::seconds(1);
        assert_eq!(
            validate_proposal(&proposal, "robot-1", true, &safety, 6, now, limits),
            Err(ProposalRejection::ExpiredOrInvalidWindow)
        );
    }

    #[test]
    fn rejects_out_of_bounds_and_non_finite_velocity() {
        let (mut proposal, safety, now, limits) = fixture();
        proposal.parameters = serde_json::json!({"linear_velocity_mps": 0.8});
        assert_eq!(
            validate_proposal(&proposal, "robot-1", true, &safety, 6, now, limits),
            Err(ProposalRejection::VelocityOutOfBounds)
        );
        proposal.parameters = serde_json::json!({"linear_velocity_mps": "not-a-number"});
        assert_eq!(
            validate_proposal(&proposal, "robot-1", true, &safety, 6, now, limits),
            Err(ProposalRejection::InvalidParameters)
        );
    }

    #[test]
    fn rejects_bad_interlocks_and_device_identity() {
        let (proposal, mut safety, now, limits) = fixture();
        safety.interlocks_healthy = false;
        assert_eq!(
            validate_proposal(&proposal, "robot-1", true, &safety, 6, now, limits),
            Err(ProposalRejection::InterlocksUnhealthy)
        );
        assert_eq!(
            validate_proposal(&proposal, "other-device", true, &safety, 6, now, limits),
            Err(ProposalRejection::DeviceMismatch)
        );
    }
}
