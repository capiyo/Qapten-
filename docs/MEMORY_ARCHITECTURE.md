# Axon Memory Architecture

## Purpose

Axon owns a provider-independent, persistent memory store. ChatGPT, Claude, DeepSeek, and other models are replaceable readers and extraction workers; none is the canonical owner of user memory.

The first implementation is deliberately conservative: preserve source conversations, store proposed memory records with provenance, and require a review transition before a record becomes active.

## Storage boundary

- MongoDB is the canonical database for memory metadata and imported conversation content.
- `conversation_imports` stores the original message sequence, provider, provider conversation ID, import timestamp, and SHA-256 digest.
- `memories` stores normalized memory candidates, their source import, source message IDs, type, confidence, status, version, and content digest.
- MongoDB indexes support workspace/status timelines and text search over title, content, summary, and tags.
- Embedding vectors and Atlas Vector Search are a later retrieval optimization. The current API does not pretend that semantic vector search is already active.

Configure `MONGODB_URI` and optionally `MONGODB_DATABASE` (default: `axon`). When `AXON_ENV=production`, startup fails if `MONGODB_URI` is absent. In local development, Axon starts without the memory subsystem and memory routes return HTTP 503 rather than silently writing to volatile memory.

## Data lifecycle

1. Import a provider-authorized export into a workspace. Axon preserves the ordered messages and computes a content digest.
2. The Python `axon-memory` worker can read a user-authorized ChatGPT or Claude export, ask DeepSeek for structured memory candidates, validate the response and source message IDs, and submit candidates to Axon. Extraction is not the same as verification.
3. Candidates are created with `status=proposed`; every candidate references the source import and can include message IDs.
4. A reviewer approves or rejects a proposed memory using an expected version. An active memory can be superseded by another active memory using an expected version and an explicit replacement reference.
5. Only active memories should be included in normal context assembly. Rejected and superseded records remain available for audit/history.
6. The active-only context endpoint returns a bounded set of workspace-scoped records using MongoDB text search. The `axon-route --include-memory` option can add selected active memories to a capability poll; the poll stores their IDs and verifies workspace/status before accepting them. Semantic vector ranking and general-purpose context assembly remain future work.

## Memory types

- `semantic`: durable concepts and facts.
- `episodic`: events and conversation history.
- `project`: project architecture, constraints, progress, and current state.
- `decision`: accepted decisions, rationale, alternatives, and supersession.
- `preference`: user-approved working preferences.
- `procedural`: repeatable workflows and methods.

## API

- `POST /api/v1/workspaces/{workspace_id}/memory/imports`: import a conversation.
- `GET /api/v1/workspaces/{workspace_id}/memory/imports/{import_id}`: retrieve the original imported conversation for an authorized extraction worker.
- `POST /api/v1/workspaces/{workspace_id}/memories`: create a proposed memory from an imported source.
- `GET /api/v1/workspaces/{workspace_id}/memories?status=active&q=...`: list or text-search memories.
- `GET /api/v1/workspaces/{workspace_id}/memory/context?q=...&limit=...`: retrieve only active, approved memories for downstream task context.
- `POST /api/v1/workspaces/{workspace_id}/memories/{memory_id}/review`: approve, reject, or supersede with optimistic version checking.

The import endpoint accepts at most 5,000 messages and 900 KB of message content. The API request body limit remains 1 MB. Search is capped at 200 records.

## Trust and safety

- Imported conversation text is untrusted data, never executable instructions.
- Provider and conversation IDs are provenance claims from the import source, not cryptographic proof of origin.
- SHA-256 detects content changes; it is not a digital signature.
- The current API still lacks verified user authentication and route-level workspace authorization. It must remain loopback-only and must not be exposed to a network or used with sensitive multi-user data.
- Production deployment requires Firebase ID-token verification (or another trusted identity adapter), workspace capability enforcement on every read/write, durable audit events, retention/export/deletion controls, and encrypted backups.
- Memory extraction must not silently convert a model's guess into an active fact. Confidence is metadata, not truth; approval and source provenance remain explicit.

## Deliberately deferred

- Embedding generation and vector index provisioning.
- Automatic deduplication and contradiction resolution.
- Memory decay/expiry policies and scheduled consolidation.
- Provider-specific live chat synchronization. Consumer chat exports remain user-initiated unless a provider offers a supported authorized API.
- The first extraction worker uses DeepSeek; a provider-neutral extractor interface and additional extraction providers remain future work.
- Context bundle assembly and automatic delivery to provider adapters.
