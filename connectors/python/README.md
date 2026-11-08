# AXON Python connector

The Python connector is the provider-adapter boundary for explicit, inspectable AI context handoffs.

## Current Claude → DeepSeek path

1. Export a conversation from Claude using the provider's supported export controls.
2. Select one conversation JSON file from that export.
3. Run the connector in preview mode. It reports source, message count and context size without sending content.
4. Re-run with `--approve-transfer` to send that selected context to DeepSeek.
5. DeepSeek's response is returned to the terminal. AXON API persistence and UI handoff integration are a subsequent integration step.

Claude's consumer chat history is not exposed by the standard Anthropic Messages API. This implementation therefore imports user-authorized export files; it does not scrape logged-in browser pages, inspect cookies, automate private endpoints, or bypass provider controls.

## Setup

Requires Python 3.11+.

```bash
cd connectors/python
python -m venv .venv
# Windows: .venv\\Scripts\\activate
# macOS/Linux: source .venv/bin/activate
pip install -e ".[dev]"
```

Set the DeepSeek API key in the environment. Do not put it in source code or commit it.

```bash
# PowerShell
$env:DEEPSEEK_API_KEY="your-key"

# Bash
export DEEPSEEK_API_KEY="your-key"
```

## Preview a conversation

```bash
axon-bridge ./conversation.json \
  --workspace-id YOUR_WORKSPACE_ID \
  --actor-id YOUR_VERIFIED_AXON_ACTOR_ID \
  --instruction "Independently review the architecture and identify risks."
```

Preview is the default. No provider request is made without explicit approval.

## Approve the handoff

```bash
axon-bridge ./conversation.json \
  --workspace-id YOUR_WORKSPACE_ID \
  --actor-id YOUR_VERIFIED_AXON_ACTOR_ID \
  --instruction "Independently review the architecture and identify risks." \
  --approve-transfer
```

The `--actor-id` is provenance metadata only in this standalone prototype. It is not authentication. When integrated into AXON, the backend must derive the actor from a verified Firebase ID token and authorize the workspace and context transfer server-side. Never trust a client-provided actor ID as proof of identity.

## Configuration

- `DEEPSEEK_API_KEY`: required for approved transfers.
- `DEEPSEEK_MODEL`: optional model override; defaults to `deepseek-chat`.
- DeepSeek endpoint defaults to `https://api.deepseek.com`.

## Current boundaries

- Claude export import only; no browser scraping or direct Claude chat-history API.
- DeepSeek API call is implemented.
- Context is sent as a bounded AXP envelope and treated as untrusted source material by the receiving model.
- Explicit approval is required; preview is the default.
- The connector currently returns DeepSeek output locally. It does not yet write artifacts/tasks into the AXON backend or create an authenticated provider connection.
- Attachments, images, tool-use blocks and very large conversations are not yet transferred.
- Add server-side Firebase identity verification, workspace authorization, encrypted secret management, durable transfer audit and retention policy before multi-user/production use.
