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
- [x] GitHub Actions checks for Rust formatting, Clippy, tests, web production build, and mobile TypeScript validation.\n- [x] AXP robotics device-registration, telemetry, non-executable command-proposal, and safety-state schemas.\n- [x] Shared TypeScript robotics contracts and pure Rust proposal validation tests.\n- [x] Robotics safety boundary documented; no hardware dispatch or actuator endpoint.
- [x] Initial open-source project files: MIT license, contribution guidance, and security policy.

## Not implemented

- Persistent MongoDB adapter, schema migrations, and durable event/audit storage.\n- Cryptographic device enrollment, persistent replay/idempotency enforcement, and device gateway.
- Authentication, user identity, workspace membership, and real authorization.
- Artifact content hashing, classification enforcement, context bundle read API, and retention controls.
- Full decision records, audit timeline, permission management, and revocation UI.
- Agent provider APIs, browser-assisted capture, webhooks, and autonomous agent execution.
- C++/Drogon service and Zig utilities (intentionally deferred until justified).
- Production deployment, secrets management, and operational monitoring.
- Published standalone npm package for shared frontend contracts.

## Runtime limitations

The current API uses an in-process memory store. All records disappear when the process exits. It has no authentication or real authorization. It refuses non-loopback binding by default; the explicit override is for isolated development only. Do not expose it to a network or use it for sensitive information.

## Verification

The GitHub Actions run for commit `4e65ecceca94bb8412ae2543fe245b89d80c7b4e` passed all jobs: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, the React web production build, and the React Native TypeScript check. Robotics changes after that commit require a fresh CI run. The first robotics CI attempt failed at Rust formatting; a formatting correction and schema-alignment fixes were committed afterward and require a fresh run. This verifies CI builds and tests only; no physical Android/iOS build or production deployment has been performed.

## Architecture direction

- `frontend/` is the client layer: web, mobile, and frontend-only shared TypeScript contracts.
- `backend/` is the server layer: Axum currently owns API behavior and authoritative domain state.
- `contracts/` is the language-neutral, versioned protocol boundary.
- Axum remains the initial orchestration authority. MongoDB is the planned persistence layer, subject to schema and transaction design.
- Drogon and Zig remain evidence-driven extension points, not duplicated orchestration services.