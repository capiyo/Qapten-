# Backend

The backend owns server-side behavior. Frontend clients must not access persistence directly or become authorities for permissions and workflow state.

## Current service

- `axum/`: the only implemented backend application. It exposes the HTTP API and currently owns the workspace, agent, artifact, and task lifecycle endpoints.
- Storage is currently process-local memory. MongoDB is planned but not implemented.
- Authentication and authorization are not implemented. The API binds to loopback by default and must not be exposed publicly.

## Intended internal boundaries

The first backend should remain a modular monolith rather than splitting into independently deployed services prematurely:

1. **Transport** — Axum routes, request parsing, response mapping, and API versioning.
2. **Application** — use cases and orchestration commands.
3. **Domain** — workspace, agent, artifact, task, decision, and policy rules.
4. **Ports** — repository, artifact storage, identity, clock, and provider interfaces.
5. **Adapters** — MongoDB, object storage, identity provider, and model-provider implementations.

The transport layer calls application use cases. Domain rules must not depend on Axum, MongoDB, React, or a specific model provider. Persistence adapters implement ports; they do not decide business policy.

## C++ and Zig

Drogon and Zig are not implemented services today. Add them only when a bounded responsibility and measured requirement justify a separate component. Axum remains the authoritative owner of authorization and task state transitions; no second service may silently duplicate those rules.

## Local development

See [Axum API README](axum/README.md) and the repository [quickstart](../docs/QUICKSTART.md).
