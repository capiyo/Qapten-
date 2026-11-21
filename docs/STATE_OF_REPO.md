# State of Repository

**Last updated:** 2026-11-08

**Product identity:** Axon (working name; public trademark/domain clearance not yet completed).

## Repository

- Repository: `capiyo/Qapten-`
- Default branch: `main`
- Visibility: public
- Active implementation branch: `foundation/axum-web-skeleton`
- Review: [Draft PR #1](https://github.com/capiyo/Qapten-/pull/1)
- License: MIT (see [LICENSE](../LICENSE))

## Implemented on the draft branch

- [x] Product purpose, design principles, architecture, protocol draft, risk policy, roadmap, and architecture decision record.
- [x] Rust workspace and Axum API application.
- [x] Health, workspace, manual agent registration/listing, and artifact capture/listing endpoints.
- [x] Task creation/listing, selected context artifact references, and version-checked lifecycle transitions.
- [x] Manual task response submission as a linked artifact.
- [x] React + TypeScript + Vite web workspace for creating workspaces, registering agents, capturing artifacts, attaching context to tasks, submitting responses, and reviewing/advancing tasks.
- [x] React Native + Expo workspace for workspace creation/listing, agent registration, artifact capture, context selection, task creation, response submission, and review transitions.
- [x] Shared TypeScript API contract types consumed by web and mobile.
- [x] GitHub Actions checks for Rust formatting, Clippy, tests, web production build, and mobile TypeScript validation.
- [x] AXP robotics device-registration, telemetry, non-executable command-proposal, and safety-state schemas.
- [x] Shared TypeScript robotics contracts and pure Rust proposal validation tests.
- [x] Robotics safety boundary documented; no hardware dispatch or actuator endpoint.
- [x] Workspace-scoped identity and capability grant domain primitives with deny-by-default policy evaluation and expiry/revocation tests.
- [x] AXP identity grant schema and shared TypeScript principal/capability contracts.
- [x] Process-local append-only audit event model, workspace audit read endpoint, and mutation events for workspace, agent, artifact, and task lifecycle changes.
- [x] Proposal validation rejects stale/future safety-state observations.
- [x] Development-only simulator telemetry API with simulated-source enforcement, event deduplication, monotonic per-device sequences, freshness checks, and 500-event per-device in-memory retention.
- [x] Simulator telemetry ingress schema and shared TypeScript request contract.
- [x] Initial open-source project files: MIT license, contribution guidance, and security policy.

- [x] Initial Python Claude-export to DeepSeek API connector with AXP normalization, explicit approval gating, CLI preview, and unit tests.
- [x] Python connector Ruff/pytest CI job.

## Not implemented

- Persistent MongoDB adapter, schema migrations, and durable event/audit storage.
- Cryptographic device enrollment, persistent replay/idempotency enforcement, and device gateway.
- Authentication adapters, verified caller identity, workspace membership persistence, grant issuance/revocation APIs, and route-level authorization enforcement.
- Artifact content hashing, classification enforcement, context bundle read API, and retention controls.
- Durable tamper-evident audit storage, verified actor attribution, full decision records, permission management, and revocation UI.
- Firebase ID-token verification and server-side route authorization.
- AXON backend API integration for Python transfer persistence and response artifact creation.
- Direct Claude chat-history API access; current import uses user-authorized exports, not browser scraping.
- Attachment/image/tool-use transfer and resumable large-context transfer.
- Additional agent provider APIs, webhooks, and autonomous agent execution.
- C++/Drogon service and Zig utilities (intentionally deferred until justified).
- Production deployment, secrets management, and operational monitoring.
- Published standalone npm package for shared frontend contracts.

## Python connector limitations

The Python connector imports one user-authorized Claude conversation export and calls DeepSeek's API after explicit approval. Preview can display the exact AXP context; the payload carries a SHA-256 digest for change detection, not a signature or proof of origin. Its standalone actor ID is provenance metadata, not authentication. The transfer is not yet persisted in the AXON backend, and DeepSeek output is returned locally rather than stored as a task response artifact. Do not commit provider API keys. Production integration must verify Firebase ID tokens and enforce workspace/capability authorization on the backend.

## Runtime limitations

The current API uses an in-process memory store. All records, including the audit stream, disappear when the process exits. The development audit stream is append-only within one process but is neither durable nor tamper-evident. Simulator telemetry is bounded to 500 retained events per device; event-ID deduplication is limited to that retained window. This is not durable replay protection. It has no authentication or route-level authorization enforcement. The identity module evaluates grants only for principals supplied by a future trusted authentication adapter; it does not establish identity. It refuses non-loopback binding by default; the explicit override is for isolated development only. Do not expose it to a network or use it for sensitive information.

## Memory and capability-routing work on `feature/axon-memory-consensus-routing`

- [x] MongoDB-backed conversation import collection with message preservation, source metadata, and SHA-256 content digest.
- [x] MongoDB-backed memory collection with workspace/status indexes and text search over title, content, summary, and tags.
- [x] Typed memory kinds: semantic, episodic, project, decision, preference, and procedural.
- [x] Proposed-by-default memory lifecycle with approve/reject/supersede transitions and optimistic version checks.
- [x] Memory import, create, list/search, and review API routes.
- [x] Strict-majority capability nomination engine with fixed electorate/candidate set, explicit external-sharing approval, one ballot per voter, self-vote rejection, and no-winner outcome for ties or plurality.
- [x] MongoDB persistence for capability polls and nominations with optimistic version checks.
- [x] Capability poll API: create, retrieve, nominate, and finalize.
- [x] Generic OpenAI-compatible provider adapter driven by a local model catalog and environment-held API keys.
- [x] Python `axon-route` worker: parallel model nominations, persisted strict-majority result, and winner-only task execution after explicit provider-sharing approval.
- [x] Python `axon-memory` worker: ChatGPT/Claude export import, DeepSeek candidate extraction, provenance validation, and proposed-memory persistence after separate approvals.
- [x] AXP JSON Schemas for memory records and capability polls.
- [x] CI triggers include feature branches and pull requests targeting foundation branches.

## Remaining gaps in this slice

- The Python `axon-memory` CLI imports ChatGPT/Claude exports, asks DeepSeek for structured memory candidates after explicit provider-transfer approval, validates candidate structure and source message IDs, and can persist the source plus proposed records to local Axon. A provider-neutral extraction interface, additional extractors, and a review UI remain to be implemented.
- MongoDB persistence is currently scoped to the memory subsystem. Existing workspaces, agents, tasks, robotics telemetry, and audit records still use process-local storage.
- Model catalog discovery, provider price/availability refresh, per-voter retries and adaptive timeouts, and provider-specific non-OpenAI-compatible adapters are not implemented.
- Winner task results are returned to the CLI; they are not yet captured as durable Axon task response artifacts.
- Memory extraction currently uses DeepSeek only; additional extraction providers, automatic scheduling, and a review UI remain future work.
- Vector embeddings, vector-index provisioning, semantic retrieval, context assembly, deduplication, and contradiction resolution remain future work.
- The API still has no verified authentication or route-level workspace authorization. MongoDB-backed memory must remain loopback-only and is not safe for multi-user production deployment yet.

## Verification

GitHub Actions run [42500672881](https://github.com/capiyo/Qapten-/actions/runs/42500672881) passed all four jobs on commit `5c199a8f364d5272f9525703f4e591a820652ef4`: Rust formatting, Clippy and tests; React web production build; React Native TypeScript validation; and Python connector Ruff/pytest. The Python suite verifies Claude export normalization, AXP provenance and payload hashing, preview-only behavior, explicit transfer approval, and context-size validation. This verifies the current branch head at the time of the run. No physical Android/iOS build, hardware integration, or production deployment has been performed.

## Architecture direction

- `frontend/` is the client layer: web, mobile, and frontend-only shared TypeScript contracts.
- `backend/` is the server layer: Axum currently owns API behavior and authoritative domain state.
- `contracts/` is the language-neutral, versioned protocol boundary.
- Axum remains the initial orchestration authority. MongoDB is the planned persistence layer, subject to schema and transaction design.
- Drogon and Zig remain evidence-driven extension points, not duplicated orchestration services.