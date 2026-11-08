# State of Repository

**Last updated:** 2026-11-08

## Verified repository state

- Repository: `capiyo/Qapten-`
- Default branch: `main`
- Visibility reported by GitHub: public
- Initial repository was empty before this foundation was written.
- Initial commit establishes README and product intent.
- Architecture, protocol, risk, and roadmap documents are being established as design artifacts.

## Implemented
- [x] Axum API skeleton with health, workspace, manual agent, artifact, and task endpoints.
- [x] In-memory task lifecycle with version-checked transitions and context artifact validation.
- [x] React web workspace creation screen connected to the local API.
- [x] React Native / Expo app shell with API health status.
- [x] GitHub Actions CI workflow for Rust checks and web build.

- [x] Product purpose and design principles in README.
- [x] Initial architecture boundaries.
- [x] Draft Qapten Exchange Protocol (QEP).
- [x] Initial risk policy.
- [x] Outcome-based roadmap.

## Not implemented yet
- No persistent database adapter; current API data is process-local memory.
- No authentication, user identity, or real authorization.
- No handoff response artifact/review UI in the frontend yet.
- No live agent provider connections or browser capture.

- No Axum service code.
- No Drogon service code.
- No Zig component.
- No React web application.
- No React Native application.
- No database schema or migrations.
- No authentication or authorization implementation.
- No provider API integration.
- No browser extension or browser capture implementation.
- No automated tests or CI pipeline.
- No deployment configuration.

## Architectural decision

Begin with a modular Rust/Axum orchestration core and a provider-neutral protocol. Keep C++/Drogon and Zig as later, evidence-driven components rather than adding empty services. Use React for web and React Native for mobile, with shared contracts where practical. MongoDB is the planned initial persistence layer, pending implementation-level schema and consistency review.

## Verification status

The API, web, and mobile foundations are committed on the open draft PR branch. No local build or test has been run through the GitHub connector. CI status must be checked before treating the code as build-verified. The API is a local-only, in-memory development prototype, not production-ready.
