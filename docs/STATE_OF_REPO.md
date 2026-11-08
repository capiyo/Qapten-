# State of Repository

**Last updated:** 2026-11-08

## Repository

- Repository: `capiyo/Qapten-`
- Default branch: `main`
- Visibility: public
- Active implementation branch: `foundation/axum-web-skeleton`
- Review: [Draft PR #1](https://github.com/capiyo/Qapten-/pull/1)

## Implemented on the draft branch

- [x] Product purpose, design principles, architecture, protocol draft, risk policy, roadmap, and architecture decision record.
- [x] Rust workspace and Axum API application.
- [x] Health, workspace, manual agent registration/listing, and artifact capture/listing endpoints.
- [x] Task creation/listing, selected context artifact references, and version-checked lifecycle transitions.
- [x] Manual task response submission as a linked artifact.
- [x] React + TypeScript + Vite workspace screen for creating workspaces, registering agents, capturing artifacts, attaching context to tasks, submitting responses, and reviewing/advancing tasks.
- [x] React Native + Expo workspace creation/listing, agent registration, artifact capture, and task creation/listing.
- [x] GitHub Actions workflow for Rust formatting, Clippy, tests, and web build.

## Not implemented

- Persistent MongoDB adapter, schema migrations, and durable event/audit storage.
- Authentication, user identity, workspace membership, and real authorization.
- Artifact content hashing, classification enforcement, context bundle read API, and retention controls.
- Full decision records, audit timeline, permission management, and revocation UI.
- Agent provider APIs, browser-assisted capture, webhooks, and autonomous agent execution.
- Mobile context selection, response submission, review transitions, and full handoff history.
- C++/Drogon service and Zig utilities (intentionally deferred until justified).
- Production deployment, secrets management, and operational monitoring.

## Runtime limitations

The current API uses an in-process memory store. All records disappear when the process exits. It has no authentication or real authorization. It refuses non-loopback binding by default; the explicit override is for isolated development only. Do not expose it to a network or use it for sensitive information.

## Verification

The GitHub connector was used to commit source files and open the draft PR. It did not run a local compiler or test suite. GitHub Actions checks must complete before this foundation is considered build-verified. The React Native app has not been built on Android or iOS.

## Architecture direction

Axum remains the initial orchestration authority. MongoDB is the planned persistence layer, subject to schema and transaction design. React and React Native are separate clients over the same API. Drogon and Zig remain evidence-driven extension points, not duplicated orchestration services.
