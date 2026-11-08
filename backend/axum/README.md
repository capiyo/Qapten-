# Qapten Axum API

Initial executable API foundation.

## Run

From the repository root:

```sh
cargo run -p qapten-api
```

The server binds to `127.0.0.1:8080` by default.

## Endpoints

- `GET /health`
- `GET /api/v1/workspaces`
- `POST /api/v1/workspaces` — JSON: `{"name":"My workspace"}`
- `POST /api/v1/workspaces/{workspace_id}/agents` — JSON: `{"display_name":"Reviewer","provider":"claude"}`
- `POST /api/v1/workspaces/{workspace_id}/artifacts` — JSON: `{"title":"Investigation","content":"Captured response","source_agent_id":null}`
- `GET /api/v1/workspaces/{workspace_id}/tasks`
- `POST /api/v1/workspaces/{workspace_id}/tasks` — JSON: `{"title":"Review the fixture status bug","objective":"Inspect the event timing logic and report evidence.","assignee_agent_id":"<agent-uuid>","context_artifact_ids":[]}`
- `POST /api/v1/tasks/{task_id}/transitions` — JSON: `{"expected_version":1,"status":"acknowledged","note":"I accept this handoff."}`

## Important limitations

This first slice uses an in-process memory store. Data disappears when the process exits. It has no authentication or workspace membership enforcement yet, so it refuses non-loopback network binding by default. Do not expose it to a network or use it for sensitive information.

The next implementation step is persistent storage plus authentication and authorization before any shared or remote deployment.
