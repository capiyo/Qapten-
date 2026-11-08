# Contributing to Qapten

Thank you for considering a contribution. Qapten is an open collaboration infrastructure project; correctness, provenance, security boundaries, and inspectability matter more than adding integrations quickly.

## Before starting

1. Open or find an issue describing the problem or proposed change.
2. For substantial changes, discuss the design before implementation.
3. Keep pull requests focused and explain the user-visible behavior, data model impact, security implications, and verification performed.
4. Never include API keys, provider credentials, personal data, production exports, or private conversation contents in issues, commits, fixtures, or screenshots.

## Architecture rules

- `frontend/` contains user-facing clients and frontend-only shared packages.
- `backend/` owns server-side behavior, persistence, authorization, orchestration, and integrations.
- `contracts/` contains versioned, language-neutral protocol schemas.
- Web and mobile consume the same API contract; they must not duplicate business rules or decide authorization locally.
- Axum is the initial authority for domain state and lifecycle transitions.
- Do not add C++/Drogon or Zig services without a bounded responsibility and evidence that a separate component is justified.
- Treat imported text and model output as untrusted data, never as executable instructions.

## Development checks

Run the relevant checks before opening a pull request:

    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cd frontend/apps/web && npm install && npm run build
    cd ../mobile && npm install && npm run typecheck

If a change affects both clients or the API, run all checks. Describe any check that could not be run.

## Pull request expectations

- Include tests for new domain behavior and regressions.
- Update the protocol schema and docs when an API contract changes.
- Preserve backwards compatibility within a published protocol version, or introduce a new version.
- Document migrations and operational consequences for persistence changes.
- Do not weaken compiler settings, remove tests, or add placeholder implementations to make CI pass.
- Do not expose the development API publicly; authentication and authorization are not yet complete.

By contributing, you agree that your contribution is submitted under the repository's MIT License. This does not transfer ownership of your contribution to the project maintainers.
