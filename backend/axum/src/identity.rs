//! Workspace-scoped authorization policy primitives.
//!
//! This module evaluates grants for an already authenticated principal. It does
//! not authenticate callers, parse bearer tokens, or establish device identity.
//! Callers must only construct AuthenticatedPrincipal from a verified identity
//! boundary. Missing, expired, revoked, or mismatched grants are denied.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalKind {
    Human,
    Agent,
    Service,
    Device,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PrincipalRef {
    pub kind: PrincipalKind,
    pub id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    WorkspaceRead,
    WorkspaceManage,
    AgentRegister,
    AgentManage,
    ArtifactRead,
    ArtifactCreate,
    ArtifactClassify,
    TaskRead,
    TaskCreate,
    TaskAssign,
    TaskReview,
    DecisionPropose,
    DecisionAccept,
    AuditRead,
    DeviceEnroll,
    DeviceTelemetryRead,
    DeviceTelemetryWrite,
    DeviceCommandPropose,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityGrant {
    pub grant_id: Uuid,
    pub subject: PrincipalRef,
    pub workspace_id: Uuid,
    pub capabilities: Vec<Capability>,
    pub issued_by: PrincipalRef,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// A principal produced by a trusted authentication adapter.
/// Constructing this value from request JSON or an unverified header is unsafe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    pub principal: PrincipalRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationDenial {
    NoMatchingGrant,
    CapabilityNotGranted,
    GrantNotYetValid,
    GrantExpired,
    GrantRevoked,
}

/// Evaluate one workspace-scoped capability. This function is deliberately
/// deny-by-default and does not support implicit/global administrator grants.
pub fn authorize(
    principal: &AuthenticatedPrincipal,
    workspace_id: Uuid,
    capability: Capability,
    grants: &[CapabilityGrant],
    now: DateTime<Utc>,
) -> Result<(), AuthorizationDenial> {
    let matching = grants
        .iter()
        .filter(|grant| grant.subject == principal.principal && grant.workspace_id == workspace_id);
    let relevant: Vec<_> = matching.collect();
    if relevant.is_empty() {
        return Err(AuthorizationDenial::NoMatchingGrant);
    }

    for grant in &relevant {
        if grant.revoked_at.is_some_and(|revoked_at| revoked_at <= now)
            || grant.issued_at > now
            || grant.expires_at.is_some_and(|expires_at| expires_at <= now)
        {
            continue;
        }
        if grant.capabilities.contains(&capability) {
            return Ok(());
        }
    }

    if relevant
        .iter()
        .all(|grant| grant.revoked_at.is_some_and(|at| at <= now))
    {
        return Err(AuthorizationDenial::GrantRevoked);
    }
    if relevant.iter().all(|grant| grant.issued_at > now) {
        return Err(AuthorizationDenial::GrantNotYetValid);
    }
    if relevant
        .iter()
        .all(|grant| grant.expires_at.is_some_and(|at| at <= now))
    {
        return Err(AuthorizationDenial::GrantExpired);
    }
    Err(AuthorizationDenial::CapabilityNotGranted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn fixture() -> (AuthenticatedPrincipal, CapabilityGrant, Uuid, DateTime<Utc>) {
        let now = Utc::now();
        let principal = AuthenticatedPrincipal {
            principal: PrincipalRef {
                kind: PrincipalKind::Human,
                id: "human-1".into(),
            },
        };
        let workspace_id = Uuid::new_v4();
        let grant = CapabilityGrant {
            grant_id: Uuid::new_v4(),
            subject: principal.principal.clone(),
            workspace_id,
            capabilities: vec![Capability::ArtifactRead, Capability::TaskCreate],
            issued_by: PrincipalRef {
                kind: PrincipalKind::Human,
                id: "owner-1".into(),
            },
            issued_at: now - Duration::minutes(1),
            expires_at: Some(now + Duration::minutes(5)),
            revoked_at: None,
        };
        (principal, grant, workspace_id, now)
    }

    #[test]
    fn allows_only_explicit_capability_in_matching_workspace() {
        let (principal, grant, workspace, now) = fixture();
        assert_eq!(
            authorize(
                &principal,
                workspace,
                Capability::ArtifactRead,
                std::slice::from_ref(&grant),
                now
            ),
            Ok(())
        );
        assert_eq!(
            authorize(
                &principal,
                workspace,
                Capability::AuditRead,
                std::slice::from_ref(&grant),
                now
            ),
            Err(AuthorizationDenial::CapabilityNotGranted)
        );
        assert_eq!(
            authorize(
                &principal,
                Uuid::new_v4(),
                Capability::ArtifactRead,
                &[grant],
                now
            ),
            Err(AuthorizationDenial::NoMatchingGrant)
        );
    }

    #[test]
    fn denies_expired_future_and_revoked_grants() {
        let (principal, mut grant, workspace, now) = fixture();
        grant.expires_at = Some(now - Duration::seconds(1));
        assert_eq!(
            authorize(
                &principal,
                workspace,
                Capability::ArtifactRead,
                std::slice::from_ref(&grant),
                now
            ),
            Err(AuthorizationDenial::GrantExpired)
        );
        grant.expires_at = None;
        grant.issued_at = now + Duration::seconds(1);
        assert_eq!(
            authorize(
                &principal,
                workspace,
                Capability::ArtifactRead,
                std::slice::from_ref(&grant),
                now
            ),
            Err(AuthorizationDenial::GrantNotYetValid)
        );
        grant.issued_at = now - Duration::minutes(1);
        grant.revoked_at = Some(now - Duration::seconds(1));
        assert_eq!(
            authorize(
                &principal,
                workspace,
                Capability::ArtifactRead,
                &[grant],
                now
            ),
            Err(AuthorizationDenial::GrantRevoked)
        );
    }

    #[test]
    fn a_valid_grant_can_satisfy_a_capability_when_another_is_revoked() {
        let (principal, grant, workspace, now) = fixture();
        let mut revoked = grant.clone();
        revoked.grant_id = Uuid::new_v4();
        revoked.revoked_at = Some(now - Duration::seconds(1));
        assert_eq!(
            authorize(
                &principal,
                workspace,
                Capability::ArtifactRead,
                &[revoked, grant],
                now
            ),
            Ok(())
        );
    }

    #[test]
    fn principal_kind_is_part_of_identity() {
        let (mut principal, grant, workspace, now) = fixture();
        principal.principal.kind = PrincipalKind::Agent;
        assert_eq!(
            authorize(
                &principal,
                workspace,
                Capability::ArtifactRead,
                &[grant],
                now
            ),
            Err(AuthorizationDenial::NoMatchingGrant)
        );
    }
}
