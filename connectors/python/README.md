# AXON Python connector

The Python connector is the provider-adapter boundary for explicit, inspectable AI context handoffs.

## Current Claude → DeepSeek path

1. Export a conversation from Claude using the provider's supported export controls.
2. Select one conversation JSON file from that export.
3. Run the connector in preview mode. It reports source, message count and context size without sending content.
4. Add `--print-context` to inspect the exact normalized AXP envelope before transfer. Treat this output as private conversation data.
5. Re-run with `--approve-transfer` to send that selected context to DeepSeek through its API.
6. DeepSeek's response is returned to the terminal. AXON API persistence and UI handoff integration are a subsequent integration step.

Claude's consumer chat history is not exposed by the standard Anthropic Messages API. This implementation therefore imports user-authorized export files; it does not scrape logged-in browser pages, inspect cookies, automate private endpoints, or bypass provider controls.

## Setup

Requires Python 3.11+.

```bash
cd connectors/python
python -m venv .venv
# Windows: .venv\Scripts\activate
# macOS/Linux: source .venv/bin/activate
pip install -e ".[dev]"
```

Set the DeepSeek API key in the environment. Do not put it in source code or commit it.

```powershell
$env:DEEPSEEK_API_KEY="your-key"
```

```bash
export DEEPSEEK_API_KEY="your-key"
```

## Preview a conversation

```bash
axon-bridge ./conversation.json \
  --workspace-id YOUR_WORKSPACE_ID \
  --actor-id YOUR_AXON_ACTOR_PROVENANCE_ID \
  --instruction "Independently review the architecture and identify risks." \
  --print-context
```

Preview is the default. No provider request is made without explicit approval. The context preview can contain private conversation data; do not paste it into public logs or tickets.

## Approve the handoff

```bash
axon-bridge ./conversation.json \
  --workspace-id YOUR_WORKSPACE_ID \
  --actor-id YOUR_AXON_ACTOR_PROVENANCE_ID \
  --instruction "Independently review the architecture and identify risks." \
  --approve-transfer
```

The `--actor-id` is provenance metadata only. It is not authentication and must not be trusted as an identity claim. When integrated into AXON, the backend must derive the actor from a verified Firebase ID token and authorize the workspace and context transfer server-side. Never trust a client-provided actor ID as proof of identity.

## Configuration

- `DEEPSEEK_API_KEY`: required for approved transfers.
- `DEEPSEEK_MODEL`: optional model override; defaults to `deepseek-chat`.
- DeepSeek endpoint defaults to `https://api.deepseek.com`.

## Current boundaries

- Claude export import only; no browser scraping or direct Claude chat-history API.
- DeepSeek API call is implemented.
- AXP context payload includes a SHA-256 digest for integrity checking; this digest detects payload changes but is not a signature or proof of origin.
- Explicit approval is required; preview is the default.
- The connector currently returns DeepSeek output locally. It does not yet write artifacts/tasks into the AXON backend or create an authenticated provider connection.
- Attachments, images, tool-use blocks and very large conversations are not yet transferred.
- Add server-side Firebase identity verification, workspace authorization, encrypted secret management, durable transfer audit and retention policy before multi-user/production use.


## Persistent memory extraction (ChatGPT or Claude)

The `axon-memory` command imports one selected conversation from a user-authorized ChatGPT `conversations.json` export (using `--conversation-id`) or one Claude conversation JSON file, asks DeepSeek to extract durable memory candidates, validates the returned JSON and source message references, and can store the source plus proposed memories in Axon's local MongoDB-backed API.

Preview the source without sending it to a provider:

```bash
axon-memory ./conversations.json --provider chatgpt --conversation-id CHAT_ID --workspace-id WORKSPACE_UUID
```

Approve sending the selected conversation to DeepSeek and inspect proposed candidates:

```bash
axon-memory ./conversation.json --provider chatgpt --workspace-id WORKSPACE_UUID --approve-provider-transfer
```

Approve both provider transfer and local persistence:

```bash
axon-memory ./conversation.json --provider chatgpt --workspace-id WORKSPACE_UUID --approve-provider-transfer --approve-persist
```

Set `MONGODB_URI` and `MONGODB_DATABASE` for the Axon API, and set `DEEPSEEK_API_KEY` for extraction. The persistence client only accepts a local HTTP Axon API address. Extracted records are saved with `proposed` status; they are not active context until reviewed through Axon's memory review endpoint.

The extractor does not obey instructions embedded in imported conversations, validates model output, rejects invented source message IDs, and excludes credentials and sensitive personal data in its extraction instructions. This is a conservative first pass, not a guarantee that a model will never produce a false or sensitive candidate. Review every candidate before activation. The API currently has no authentication, so keep it bound to loopback and do not expose it to a network.


## Multi-model capability majority routing

The `axon-route` command implements the capability poll workflow. Configure up to 30 OpenAI-compatible model endpoints in a private copy of `models.example.json`. Each entry identifies a model, provider, endpoint, model-specific capability tags, and the environment variable containing its API key. Do not put provider keys in the JSON file.

Set the configured key environment variables, start Axon with MongoDB enabled, and preview the task:

```bash
axon-route --task "Design a responsive dashboard" \
  --capability visual_design \
  --workspace-id WORKSPACE_UUID \
  --models ./models.json
```

After checking the eligible model list and task brief, approve provider sharing:

```bash
axon-route --task "Design a responsive dashboard" \
  --capability visual_design \
  --workspace-id WORKSPACE_UUID \
  --models ./models.json \
  --approve-provider-sharing
```

The worker asks eligible models to nominate a candidate in parallel, persists the valid nominations in Axon, and requires a strict majority. A tie or plurality does not select a winner. When a winner exists, only that model receives the execution request. The result is returned to the terminal; task-response artifact persistence and provider-specific adapters for non-OpenAI-compatible APIs remain future work.

This is an explicit data-sharing workflow: the task brief is sent to every eligible voter after `--approve-provider-sharing`. Use capability tags to prevent irrelevant models from participating. API keys are read from environment variables, and Axon's API must remain local until authentication and workspace authorization are implemented.
