# Account and data deletion design

**Status:** design only. No deletion endpoint, background worker, or destructive migration is implemented in Phase A.

## Identity and ownership

- A Firebase UID maps to one Axon user through the unique (provider, subject) identity pair.
- The user owns workspaces through explicit membership records; the personal workspace is not inferred from the UID.
- A deletion request must resolve the Firebase UID to the Axon user and enumerate every workspace membership before any data is removed.

## Proposed deletion workflow

1. **Request and re-authenticate.** Require a fresh Firebase authentication event (the exact auth_time window remains a product/security decision), an explicit confirmation, and an idempotency key. Do not accept a UID supplied by the client as the deletion target.
2. **Record intent atomically.** Create an account-deletion request and an outbox job in one transaction. Move the account into a deletion-pending state so new sessions and new writes can be denied. Repeated requests return the existing request status.
3. **Offer export first.** If the user requests an export, generate it before destructive work, deliver it through a short-lived authenticated download, and expire the export automatically.
4. **Revoke access.** Revoke Firebase refresh tokens / disable the Firebase account through the trusted Admin API. Do not rely only on client sign-out.
5. **Delete application data idempotently.** Delete the user's memberships and owned workspace data, including agents, artifacts, tasks, and future chats, messages, approved memories, memory events, resume-brief caches, attachments, and user-scoped simulator data. Remove connected-provider credentials and cached copies where those features exist. Shared resources must be handled by ownership and retention policy rather than blindly cascading across other members.
6. **Delete the Firebase identity.** Delete the Firebase Auth user after application-side deletion is durably recorded. Retry transient provider failures from the outbox; never require the user to keep a valid session while a deletion job is running.
7. **Verify completion.** Re-run the deletion inventory, record a completion state, invalidate outstanding exports and sessions, and retain only the minimum records permitted by the approved retention policy.

## Audit-log and erasure policy

The audit table is append-only. Audit metadata must not contain access tokens, message bodies, prompts, artifact contents, email addresses, or other pasted user text. Audit actor identifiers are Axon user IDs, not Firebase UIDs.

Before implementing deletion, decide and document the legal retention treatment for immutable audit rows. The current schema stores the Axon actor UUID in those rows, so deleting the users row does not erase that historical identifier. The preferred design is to retain only a non-reversible/pseudonymous actor reference where a documented security or legal retention basis requires it, while deleting the identity mapping and all operational user data. If strict erasure is required for a category of audit data, the audit architecture and immutability policy must be reconciled explicitly before launch; do not silently mutate the append-only log.

## Reliability and safety requirements

- Use a durable deletion-request state machine (for example: requested, pending, running, completed, failed_retryable).
- Make each deletion step idempotent and checkpointed; a crash must resume rather than leave a partially deleted account.
- Use an outbox for Firebase revocation/deletion calls and retry with bounded exponential backoff.
- Prevent new writes once deletion is pending.
- Define backup retention and expiry. Deleted data may remain in encrypted backups only until the documented backup-retention window expires; restoration procedures must re-apply completed deletion tombstones.
- Keep the request status and support correlation ID free of message content, tokens, and unnecessary personal data.
- Add tests for concurrent requests, retries, partial failures, shared-workspace ownership, export expiry, and proof that deleted text cannot reappear in memory briefs or caches.

No deletion behavior is enabled by this design document.