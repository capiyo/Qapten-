# Axon Axum API

Initial executable API foundation.

## Run

From the repository root:

```sh
cargo run -p axon-api
```

The server binds to `127.0.0.1:8080` by default.

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

## Important limitations

This first slice uses an in-process memory store, including the bounded simulator telemetry stream. Data disappears when the process exits. The simulator telemetry endpoint is not a physical-device gateway and must not be exposed to a network. It has no authentication or workspace membership enforcement yet, so it refuses non-loopback network binding by default. Do not expose it to a network or use it for sensitive information.

The next implementation step is persistent storage plus authentication and authorization before any shared or remote deployment.
