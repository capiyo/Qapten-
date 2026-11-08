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
- [x] Proposal validation rejects stale/future safety-state observations.
- [x] Development-only simulator telemetry API with simulated-source enforcement, event deduplication, monotonic per-device sequences, freshness checks, and 500-event per-device in-memory retention.
- [x] Simulator telemetry ingress schema and shared TypeScript request contract.
- [x] Initial open-source project files: MIT license, contribution guidance, and security policy.

## Not implemented

- Persistent MongoDB adapter, schema migrations, and durable event/audit storage.
- Cryptographic device enrollment, persistent replay/idempotency enforcement, and device gateway.
- Authentication, user identity, workspace membership, and real authorization.
- Artifact content hashing, classification enforcement, context bundle read API, and retention controls.
- Full decision records, audit timeline, permission management, and revocation UI.
- Agent provider APIs, browser-assisted capture, webhooks, and autonomous agent execution.
- C++/Drogon service and Zig utilities (intentionally deferred until justified).
- Production deployment, secrets management, and operational monitoring.
- Published standalone npm package for shared frontend contracts.

## Runtime limitations

The current API uses an in-process memory store. All records disappear when the process exits. Simulator telemetry is bounded to 500 retained events per device; event-ID deduplication is limited to that retained window. This is not durable replay protection. It has no authentication or real authorization. It refuses non-loopback binding by default; the explicit override is for isolated development only. Do not expose it to a network or use it for sensitive information.

## Verification

GitHub Actions run [42491980806](https://github.com/capiyo/Qapten-/actions/runs/42491980806) passed all three jobs on commit `417224d2fb9d8f3f5e2e578bc933a743920ebf57`: Rust formatting, Clippy, and tests; React web production build; and React Native TypeScript validation. The simulator telemetry API, freshness validation, and protocol contracts are included in that run. This verifies CI builds and tests only; no physical Android/iOS build, hardware integration, or production deployment has been performed.

## Architecture direction

- `frontend/` is the client layer: web, mobile, and frontend-only shared TypeScript contracts.
- `backend/` is the server layer: Axum currently owns API behavior and authoritative domain state.
- `contracts/` is the language-neutral, versioned protocol boundary.
- Axum remains the initial orchestration authority. MongoDB is the planned persistence layer, subject to schema and transaction design.
- Drogon and Zig remain evidence-driven extension points, not duplicated orchestration services.