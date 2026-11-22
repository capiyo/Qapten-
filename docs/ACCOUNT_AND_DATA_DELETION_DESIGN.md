# Account and data deletion design

**Status:** design only. No deletion endpoint or destructive job is enabled by Phase A.

## Goals

- A user can request deletion of their Axon account and personal data.
- A request is authenticated and requires recent Firebase reauthentication.
- Firebase refresh tokens are revoked and the Firebase account is deleted through a trusted server-side Admin SDK integration.
- Workspace ownership, invitations, audit integrity and legal retention are handled explicitly rather than by cascading blind deletes.

## Proposed lifecycle

1. **Request:** authenticated user submits a deletion request after recent reauthentication. Create an idempotent deletion request with a request ID, requester user ID, timestamp and policy version.
2. **Quarantine:** mark the Axon user as deletion_pending; block new sessions and mutations. Revoke Firebase refresh tokens. Never accept a client-supplied UID as the deletion target.
3. **Workspace resolution:** for each workspace membership, determine whether the user is the sole owner, one of several owners, or a member. Require ownership transfer or an explicit workspace deletion policy before deleting a sole-owner workspace.
4. **Data erasure:** delete or irreversibly anonymize user-owned profile data, chat content, approved memory, attachments and other personal content according to the retention policy. Remove invitations and memberships created solely by the user where appropriate. Preserve shared workspace content owned by other members.
5. **Audit preservation:** audit rows remain append-only. Replace direct identity references with a non-reversible deletion subject/tombstone where retention law permits; retain only the minimum event metadata required for security and legal obligations. Never copy chat text into audit metadata.
6. **Identity removal:** remove the provider-subject mapping and user profile after all required cleanup jobs complete. Retain a minimal deletion tombstone to make retries idempotent and prevent accidental account resurrection during the retention window.
7. **Completion:** record completion time and outcome, then notify the user through a verified channel.

## Required safeguards

- Idempotent jobs, durable retry state, bounded retries and a dead-letter/manual-review path.
- Recent-authentication check and explicit confirmation.
- No deletion based on a UID or workspace ID supplied without resolving it from the authenticated principal.
- Transactional membership and ownership changes; content erasure can be chunked with durable checkpoints.
- Retention/legal-hold policy defined before implementation.
- Tests for concurrent requests, partial failures, sole-owner workspaces, shared workspaces, retry safety and audit immutability.
- No hard-delete endpoint until the ownership, retention and audit policies are approved.

## Phase A boundary

Phase A adds provider identity and membership foundations only. It does not implement, schedule or trigger account deletion.
