# State of Repository

**Last updated:** 2026-11-08

**Product identity:** Axon (working name; public trademark/domain clearance not yet completed).

## Repository

- Repository: `capiyo/Qapten-` (rename to `capiyo/axon` still pending in GitHub repository settings)
- Default branch: `main`
- Visibility: public
- Active implementation branch: `foundation/axon-desktop-shell` (desktop work based on `foundation/axum-web-skeleton`)
- Review: [Draft PR #1 — core foundation](https://github.com/capiyo/Qapten-/pull/1); [Draft PR #2 — desktop client](https://github.com/capiyo/Qapten-/pull/2)
- License: MIT (see [LICENSE](../LICENSE))

## Implemented on the draft branch

- [x] Initial Tauri 2 desktop shell for Windows, macOS, and Linux using the shared React/Vite web client.
- [x] Desktop-specific API endpoint configuration, minimum-privilege capability file, window sizing, and restrictive initial CSP.
- [x] Original Axon SVG identity mark shared by web and desktop; platform icon generation wired into the desktop packaging script.
- [x] Tauri setup starts the Axum API inside the desktop process and gracefully stops it when the application exits.
- [x] Local API binds to `127.0.0.1:8080`; requests are limited by a trusted Tauri/Vite origin allowlist, and the UI retries transient startup connection failures.
- [x] Desktop CI job validates Tauri icon generation, desktop-mode web build, Rust formatting, and desktop host compilation; all five jobs passed on commit `7497521579dbe310e009a4ed87c6053e1cecff71`.

## Desktop limitations

The Tauri shell and embedded API runtime are committed on `foundation/axon-desktop-shell`. Native installers have not yet been built or tested on all target platforms. The embedded API still uses in-memory development storage and has no authentication or route-level authorization. No local filesystem, shell, process, credential-vault, tray, updater, or unrestricted network permission is enabled. Desktop is an integrated development runtime, not yet a production release.

- [x] Product purpose, design principles, architecture, protocol draft, risk policy, roadmap, and architecture decision record.
- [x] Rust workspace and Axum API application.
- [x] Health, workspace, manual agent registration/listing, and artifact capture/listing endpoints.
- [x] Task creation/listing, selected context artifact references, and version-checked lifecycle transitions.
- [x] Manual task response submission as a linked artifact.
- [x] React + TypeScript + Vite web workspace for creating workspaces, registering agents, capturing artifacts, attaching context to tasks, submitting responses, and reviewing/advancing tasks.
- [x] React Native + Expo workspace for workspace creation/listing, agent registration, artifact capture, context selection, task creation, response submission, and review transitions.
- [x] Shared TypeScript API contract types consumed by web and mobile.
- [x] GitHub Actions checks for Rust formatting, Clippy, tests, web production build, mobile TypeScript validation, Python connector tests, and the Tauri desktop host.
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

- Persistent local SQLite adapter for desktop, server-side MongoDB adapter, schema migrations, and durable event/audit storage.
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

## Verification

The latest desktop branch run [42513876839](https://github.com/capiyo/Qapten-/actions/runs/42513876839) passed all five jobs on commit `7497521579dbe310e009a4ed87c6053e1cecff71`: Python connector tests, Rust checks, Tauri desktop shell check, mobile TypeScript check, and web typecheck/build. The Python suite verifies Claude export normalization, AXP provenance and payload hashing, preview-only behavior, explicit transfer approval, and context-size validation. No physical Android/iOS build, hardware integration, native installer verification, or production deployment has been performed.

## Architecture direction

- `frontend/` is the client layer: web, mobile, and frontend-only shared TypeScript contracts.
- `backend/` is the server layer: Axum currently owns API behavior and authoritative domain state.
- `contracts/` is the language-neutral, versioned protocol boundary.
- Axum remains the initial orchestration authority. SQLite is the planned embedded desktop store; MongoDB is the planned shared/server persistence layer. Both require explicit migrations and transaction design.
- Drogon and Zig remain evidence-driven extension points, not duplicated orchestration services.