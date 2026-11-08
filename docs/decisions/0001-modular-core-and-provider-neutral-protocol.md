# ADR 0001: Modular core and provider-neutral protocol

- **Status:** Accepted for the initial implementation
- **Date:** 2026-11-08

## Context

Qapten must connect agents that differ in provider, model, interface, and available integration method. Prematurely splitting every responsibility into independent services would increase operational complexity before the collaboration workflow is proven.

## Decision

1. Build the first vertical slice as a modular Rust/Axum application.
2. Define a versioned provider-neutral exchange protocol before provider adapters.
3. Keep C++/Drogon as an optional integration/edge service, introduced only when a measured throughput, protocol, or isolation need exists.
4. Keep Zig for isolated performance-critical components only after profiling.
5. Use MongoDB as the initial persistence target, with explicit aggregate versions, immutable artifacts, append-only audit records, and idempotent commands.
6. Build React web and React Native clients over the same domain API.

## Consequences

- The first release has fewer moving parts and one authoritative policy boundary.
- Drogon and Zig remain deliberate extension points, not mandatory dependencies.
- Provider integrations cannot define the core domain model.
- A future service extraction must preserve protocol and authorization semantics and be justified by operational evidence.
