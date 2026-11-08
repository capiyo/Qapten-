# Axon

**The connective infrastructure for intelligent systems.**

Axon connects humans, AI agents, software systems, autonomous services, and robots through a durable, permissioned coordination layer for context, knowledge, decisions, task handoffs, telemetry, evidence, and feedback.

It is not intended to be another model leaderboard or a thin chat aggregator. Its purpose is to make work portable across agent boundaries so that a person does not have to repeatedly copy context, decisions, and results between otherwise isolated tools.

## Why it exists

A developer may ask one assistant to investigate a bug, another to review an architecture, and a third to test a proposed change. Today, the person often becomes the integration bus: copying prompts, responses, files, and decisions between systems.

Axon aims to become that shared layer—with the user retaining control over what is captured, shared, and acted upon.

The first proving ground is collaboration between assistants (for example, Claude and ChatGPT) using explicit, user-authorized capture and handoff. Robotics and physical-device support will use a separate authenticated gateway, deterministic policy checks, local interlocks, and independent emergency-stop paths; model output alone never authorizes actuator commands. The same foundation is designed to support specialized agents, software services, connected devices, and robotics across projects such as Funspot and Qoin.

## Design principles

1. **User-controlled exchange.** No agent may read or act on data merely because another agent can. Sharing is explicit, scoped, revocable, and auditable.
2. **Evidence before authority.** Preserve source, author, timestamp, confidence, and supporting artifacts. A statement is not automatically a fact.
3. **Decisions are first-class.** Record what was decided, why, by whom, against which alternatives, and whether it was superseded.
4. **Tasks have ownership and state.** Handoffs require an explicit recipient, expected output, acknowledgement, and completion or rejection.
5. **Provider-neutral core.** Agent adapters translate provider-specific inputs and outputs into Axon's common protocol. The core must not depend on one model vendor.
6. **Visible internals.** Users can inspect context, provenance, permissions, handoffs, agent activity, and failures. Avoid opaque autonomous behavior.
7. **Incremental integration.** Start with manual import/export and structured handoffs; add browser-assisted capture and provider APIs as authorized capabilities become available.
8. **Safe by default.** No credential collection, session hijacking, hidden scraping, or bypassing provider controls.

## Initial architecture

- **Rust / Axum:** orchestration API, identity and authorization boundaries, workspace and task lifecycle, agent registry, context exchange, decision records, audit events.
- **C++ / Drogon:** high-throughput edge and integration services where profiling demonstrates a need. It must not duplicate orchestration policy.
- **Zig:** small, isolated performance-critical components only when a measured requirement justifies them.
- **MongoDB:** initial document/event persistence; schema and consistency boundaries are documented before implementation.
- **React (web) and React Native (mobile):** workspace, agent activity, handoffs, context review, permissions, and audit visibility.
- **Shared TypeScript packages:** UI primitives and API contracts where sharing is practical. Backend protocol definitions remain language-neutral and versioned.

The first milestone is a reliable, inspectable collaboration loop—not a large collection of integrations.

## First collaboration loop

1. A user creates a workspace and registers two agents.
2. The user imports or captures an agent response with its source and project scope.
3. Axon stores it as an immutable, attributed artifact.
4. The user creates a handoff referencing selected artifacts and a clear requested outcome.
5. The receiving agent works from that bounded context and returns a result.
6. The user reviews the result, accepts or rejects proposed decisions, and records feedback.
7. The workspace timeline shows the complete chain of evidence, handoffs, decisions, and revisions.

## Python provider connector

The initial Python bridge imports user-authorized Claude conversation exports, normalizes selected messages into an AXP context envelope, and sends the context to DeepSeek through its API only after explicit approval. It does not scrape logged-in browser sessions or use private endpoints. See [connectors/python/README.md](connectors/python/README.md).

## Getting started

See [docs/QUICKSTART.md](docs/QUICKSTART.md) for local setup and a manual two-agent handoff walkthrough. Current implementation limitations are recorded in [docs/STATE_OF_REPO.md](docs/STATE_OF_REPO.md).

## Frontend and backend boundary

Axon is divided into two application layers with a language-neutral protocol boundary:

- **Frontend (`frontend/`)**: React web, React Native mobile, and shared client-side TypeScript API types. It owns rendering and user interaction only.
- **Backend (`backend/`)**: server-side API, domain rules, orchestration, persistence, authorization, and provider integrations. Axum is the current authoritative backend application.
- **Protocol (`contracts/`)**: versioned schemas shared across languages and clients.

Both clients call the same backend API. They must not implement their own task lifecycle, authorization, or persistence rules. See [the architecture guide](docs/ARCHITECTURE.md) and [frontend guide](frontend/README.md).

## Open source

Axon is licensed under the [MIT License](LICENSE). See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution expectations and [SECURITY.md](SECURITY.md) for responsible vulnerability reporting.

## Repository map

```text
backend/
  axum/       Rust orchestration service
  drogon/     C++ edge/integration service (introduced when justified)
  zig/        isolated Zig utilities (introduced when justified)
frontend/
  apps/
    web/      React web application
    mobile/   React Native application
  packages/   shared frontend contracts and components
connectors/   provider-specific Python adapters
contracts/    versioned, language-neutral protocol schemas
infra/        local development and deployment definitions
docs/         architecture, decisions, security, and roadmap
tests/        protocol and cross-service test scenarios
```

Components are introduced in dependency order. A directory is not a promise that a service already exists.

## Current status

The repository is at its architecture and foundation stage. See [STATE_OF_REPO.md](docs/STATE_OF_REPO.md) and [ROADMAP.md](docs/ROADMAP.md) for verified implementation status and sequencing.

## Development status

No production deployment, provider integration, autonomous browser access, or external agent connection is implied by this initial repository foundation. Those capabilities require explicit implementation, testing, and user authorization.
