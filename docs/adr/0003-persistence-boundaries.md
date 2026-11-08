# ADR 0003: Persistence boundaries for desktop and hosted Axon

- Status: Accepted for implementation planning
- Date: 2026-11-08

## Context

Axon's current Axum application stores workspaces, agents, artifacts, tasks, audit events, and simulator telemetry in process-local memory. The Tauri desktop host starts that API inside the desktop process. This is useful for development, but every record is lost when the application exits. The desktop client therefore cannot yet serve as a dependable personal workspace.

Axon also needs a shared service for collaboration across devices and independently running agents. A single embedded database is not a substitute for that shared service.

## Decision

Use two persistence roles with a shared domain and migration discipline:

1. **Desktop/local-first store: SQLite.** The desktop host owns a database in Tauri's per-application data directory. The database must be created with restrictive filesystem permissions where supported, use foreign keys, WAL mode where appropriate, and explicit numbered migrations. The API must initialize storage before accepting requests. The database must not be placed in the repository, current working directory, or web asset directory.
2. **Hosted/shared store: MongoDB.** The hosted Axum service may use MongoDB for multi-user/shared state after identity, authorization, transaction boundaries, indexes, retention, backup, and operational policies are specified. Desktop synchronization is an explicit authenticated protocol, not direct sharing of the local database file.
3. **Storage boundary:** HTTP handlers should depend on domain-facing storage operations rather than know whether the implementation is in-memory, SQLite, or MongoDB. Keep the in-memory adapter for deterministic tests; it must not be mistaken for production persistence.
4. **Audit integrity:** audit records must be committed atomically with the mutation they describe. The durable implementation should use an append-only sequence and a cryptographic hash chain over a canonical event encoding. A hash chain detects modification; it does not by itself prove actor identity or prevent an attacker with database write access from rewriting the entire chain. Verified actor identity and external anchoring remain separate controls.
5. **Synchronization:** local IDs remain stable across sync. Define idempotency keys, conflict/version semantics, deletion/tombstone behavior, retry policy, and conflict resolution before enabling bidirectional sync. Do not silently treat last-write-wins as safe for task lifecycle or audit events.

## Required acceptance tests

- Create a workspace, agent, artifact, task, task transition, audit event, and simulator telemetry event; close and reopen the store; verify all supported records and ordering are restored.
- Verify schema migrations from an empty database and from each supported prior schema version.
- Verify failed multi-record mutations roll back without orphaned artifacts or missing audit events.
- Verify concurrent task updates preserve optimistic version checks.
- Verify audit sequence continuity and hash-chain validation; detect a modified, missing, or reordered event.
- Verify duplicate telemetry event IDs and replayed per-device sequences remain rejected after restart.
- Verify the desktop database is created under the application data directory and the API remains loopback-only.
- Keep production startup fail-closed if migration or integrity validation fails.

## Security boundaries

SQLite persistence does not add authentication. Local desktop access is still limited to the local user account and the app's current capability policy. Do not store provider credentials in application tables; use the operating system credential vault after that integration is reviewed. Do not expose the loopback API to a network. The current in-memory API remains development-only until authenticated identity and route-level authorization are enforced.

## Consequences

This avoids conflating personal offline state with shared collaboration state. It also makes transaction semantics and audit durability explicit before implementation. The cost is that two storage adapters and a sync protocol must be maintained; domain behavior and migration tests should be shared to limit drift.
