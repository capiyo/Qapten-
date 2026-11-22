# Axon Axum API

Initial executable API foundation.

## Run

Set the Firebase Auth project ID before starting the API:

```sh
FIREBASE_PROJECT_ID=<your-axon-firebase-project-id> cargo run -p axon-api
```

The server binds to `127.0.0.1:8080` by default. Only `GET /health` is public. Every other route requires a Firebase **ID token** in the `Authorization: Bearer <id-token>` header; Firebase custom tokens are not accepted.

Remote binding is disabled unless both `AXON_ALLOW_REMOTE_BIND=true` and `AXON_TLS_TERMINATED=true` are set behind a trusted HTTPS-terminating proxy. The Axum listener itself does not terminate TLS.

The simulator routes are disabled by default. Enable them only in a development environment with both `AXON_ENV=development` and `AXON_ENABLE_SIMULATOR=true`. Simulator data is isolated by authenticated Axon user.

Firebase client sign-in is performed by the Firebase SDK; Axon does not expose a password or sign-in endpoint. The API throttles failed bearer-token attempts to 20 per source IP per rolling minute. Do not log or forward ID tokens to application logs.

## Endpoints

- `GET /health`
- `POST /api/v1/sim/devices/{device_id}/telemetry` — accepts simulated telemetry only; enforces event-ID uniqueness, monotonic per-device sequence, and observation freshness.
- `GET /api/v1/sim/devices/{device_id}/telemetry` — returns the bounded in-memory simulator event stream.
- `GET /api/v1/workspaces`
- `POST /api/v1/workspaces` — JSON: `{"name":"My workspace"}`
- `GET /api/v1/workspaces/{workspace_id}/agents`
- `POST /api/v1/workspaces/{workspace_id}/agents` — JSON: `{"display_name":"Reviewer","provider":"claude"}`
- `GET /api/v1/workspaces/{workspace_id}/artifacts`
- `POST /api/v1/workspaces/{workspace_id}/artifacts` — JSON: `{"title":"Investigation","content":"Captured response","source_agent_id":null}`
- `GET /api/v1/workspaces/{workspace_id}/tasks`
- `POST /api/v1/workspaces/{workspace_id}/tasks` — JSON: `{"title":"Review the fixture status bug","objective":"Inspect the event timing logic and report evidence.","assignee_agent_id":"<agent-uuid>","context_artifact_ids":[]}`
- `POST /api/v1/tasks/{task_id}/submit` — captures a response as a linked artifact while the task is in progress.
- `POST /api/v1/tasks/{task_id}/transitions` — JSON: `{"expected_version":1,"status":"acknowledged","note":"I accept this handoff."}`

## Authentication and persistence

- Firebase ID tokens are verified using RS256, the configured Firebase project ID as audience, the project-specific issuer, expiry, subject and a 60-second clock-skew allowance.
- Google signing keys are cached according to their cache headers. Unknown key IDs trigger a serialized, cooldown-limited refresh to support rotation without allowing arbitrary key IDs to amplify outbound requests.
- A verified Firebase UID maps to a provider/subject user row. The first successful authentication idempotently provisions one personal workspace and an owner membership in SQLite.
- Workspace and task resource paths are authorized against membership before handlers run. Missing and foreign resources both return `404`.
- Every mutation audit event carries the Axon user UUID as actor. Audit metadata excludes tokens, message bodies, artifact contents, and user-entered labels.
- Failed authentication attempts are throttled per direct peer IP. Firebase remains responsible for throttling its own client sign-in flows.
- Simulator telemetry is bounded, in-memory development data and is never treated as physical-device telemetry.
- Account and data deletion is documented in `docs/ACCOUNT_DELETION_DESIGN.md`; it is not implemented.
