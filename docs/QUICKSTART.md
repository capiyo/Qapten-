# Local Quickstart

This guide runs the current manual-collaboration prototype locally. It is not a production deployment guide.

## 1. Start the API

From the repository root:

```sh
cargo run -p qapten-api
```

The API listens on `http://127.0.0.1:8080`. It uses process-local memory and has no authentication. Keep it on loopback.

## 2. Start the web client

In a second terminal:

```sh
cd frontend/apps/web
npm install
npm run dev
```

Open the Vite URL printed in the terminal (normally `http://127.0.0.1:5173`). The dev server proxies API requests to the local Axum process.

## 3. Start the mobile client

```sh
cd frontend/apps/mobile
npm install
npx expo start
```

For an Android emulator, set `EXPO_PUBLIC_QAPTEN_API_URL=http://10.0.2.2:8080` before starting Expo. For a physical Android device connected over USB, keep the API bound to loopback and run:

```sh
adb reverse tcp:8080 tcp:8080
```

Then set `EXPO_PUBLIC_QAPTEN_API_URL=http://127.0.0.1:8080`. Do not enable remote API binding on an untrusted network.

## 4. Manual API workflow

Create a workspace:

```sh
curl -X POST http://127.0.0.1:8080/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{"name":"Funspot engineering"}'
```

Copy the returned workspace UUID into `WORKSPACE_ID`, then register two manually represented agents:

```sh
curl -X POST "http://127.0.0.1:8080/api/v1/workspaces/$WORKSPACE_ID/agents" \
  -H "Content-Type: application/json" \
  -d '{"display_name":"Claude reviewer","provider":"claude"}'

curl -X POST "http://127.0.0.1:8080/api/v1/workspaces/$WORKSPACE_ID/agents" \
  -H "Content-Type: application/json" \
  -d '{"display_name":"ChatGPT implementer","provider":"chatgpt"}'
```

Capture a piece of context as an artifact:

```sh
curl -X POST "http://127.0.0.1:8080/api/v1/workspaces/$WORKSPACE_ID/artifacts" \
  -H "Content-Type: application/json" \
  -d '{"title":"Bug report","content":"Describe the issue and relevant evidence.","source_agent_id":null}'
```

Create a task using the assigned agent UUID and any selected artifact UUIDs:

```sh
curl -X POST "http://127.0.0.1:8080/api/v1/workspaces/$WORKSPACE_ID/tasks" \
  -H "Content-Type: application/json" \
  -d '{"title":"Review the issue","objective":"Investigate the cause and return evidence-backed recommendations.","assignee_agent_id":"AGENT_UUID","context_artifact_ids":["ARTIFACT_UUID"]}'
```

The workspace UI can create and advance tasks. To simulate the manual handoff, acknowledge the task, move it to `in_progress`, paste the receiving assistant's response into the task response field, and submit it. Qapten stores that response as a linked artifact. Then move the task into review, accept it or request changes, and complete it when accepted.

## Known prototype constraints

- Data is lost when the API process stops.
- Agent names/provider labels do not connect to provider APIs.
- The person using the UI performs the copy/paste and confirms the response source.
- Authentication, workspace membership, audit persistence, durable storage, and external actions are not implemented.
- Never place secrets or sensitive personal data in this prototype.
