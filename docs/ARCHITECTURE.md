# Architecture

## 1. System boundary

Axon is a collaboration control plane between people and AI agents. It stores shared work products and coordinates explicit handoffs. It does not replace the underlying model providers, silently inspect their private sessions, or assume that every agent has permission to see every workspace artifact.

The first release is a modular monolith around the orchestration domain. Splitting services before throughput or isolation requirements are measured would add operational cost without proving the core workflow.

## 2. Core domain

- **Workspace:** security and collaboration boundary for a person, project, or team.
- **Agent:** a named participant with provider, model/capability metadata, connection mode, and scoped permissions. An agent registration is not proof of a live provider connection.
- **Artifact:** immutable captured input or output with content hash, media type, source, author, capture time, project scope, and visibility.
- **Context bundle:** a versioned, bounded selection of artifacts and decisions prepared for a specific task or recipient.
- **Task:** a unit of work with requester, owner/recipient, objective, constraints, expected deliverable, status, and due/expiry policy.
- **Handoff:** an explicit transfer of a task and context bundle to another agent or human. It requires acknowledgement and has a traceable lifecycle.
- **Decision:** a proposed or accepted choice with rationale, alternatives, evidence references, author, status, and supersession links.
- **Feedback:** a human or agent assessment tied to a specific artifact, handoff, or decision.
- **Audit event:** append-only record of security-relevant and workflow-relevant actions.

## 3. Identity and authorization boundary

AXON distinguishes identity from authorization. A principal is a human, agent, service, or device; a capability grant binds that principal to an explicit set of capabilities within one workspace, with issue, expiry, and revocation timestamps. The Rust policy module is deny-by-default and evaluates these grants. It is not an authentication system: only a future trusted adapter may construct an authenticated principal from verified credentials. Request JSON, caller-supplied IDs, and unsigned headers must never be treated as proof of identity. Route handlers remain local-only until authentication and authorization middleware are implemented and tested.

## 4. Initial logical topology

```text
React Web / React Native
          |
     HTTPS / API
          |
   Rust Axum application
   |       |        |
Identity Workspace  Orchestration
   |       |        |
   +--- Domain services ---+
              |
       Repository layer
              |
     MongoDB (initial store)

Optional integration edge:
Axum <-> C++ Drogon
Only for bounded high-throughput or protocol-adapter workloads.

Optional measured utilities:
Rust <-> Zig
Only behind stable interfaces and benchmarks.
```

## 5. Responsibility boundaries

### Axum (initial system of record)
Owns authentication integration, authorization, workspace membership, agent registry, artifact metadata, context assembly, task/handoff state transitions, decision records, audit events, and API contracts.

### Drogon (not required for the first vertical slice)
May own stateless ingress, webhook normalization, or high-throughput adapter endpoints if load tests justify a separate C++ service. It must call the authoritative Axum domain API for policy-sensitive state changes. It must not maintain a second copy of authorization or task lifecycle rules.

### Zig (not required for the first vertical slice)
May implement isolated binary parsing, hashing, or other measured hot paths. No business policy or persistence ownership belongs here.

### Frontend
Web and mobile clients are views over the same API and domain model. They must not independently invent authorization rules or task state transitions.

## 6. Data and consistency

MongoDB is the initial persistence choice. Use immutable artifact payloads and append-only audit records; mutable aggregate documents hold current task/workspace state with explicit versions. Every state-changing command must be authorized, validated against the current version, and recorded with an audit event.

Cross-document workflows must be idempotent. Use stable operation IDs and unique indexes to prevent duplicate handoff acknowledgements and repeated event application. Do not claim multi-document atomicity without using a transaction and verifying deployment topology supports it.

Large artifact bodies may move to object storage later. The first implementation should define an ArtifactStore interface so metadata and payload storage can evolve independently.

## 7. Trust and provenance

Treat model output, imported text, filenames, and external metadata as untrusted content—not executable instructions. Preserve provenance and clearly separate:
- captured source material;
- agent-generated interpretation;
- human-verified facts;
- proposed decisions;
- accepted decisions.

Context bundles must be recipient-specific and purpose-limited. Secret-bearing artifacts should be rejected or redacted before sharing. Access checks happen at read time, not only when a bundle is created.

## 8. Integration progression

1. Manual import/export and structured handoffs.
2. Local browser-assisted capture initiated by the user, with preview and confirmation.
3. Official provider APIs and webhooks where available and authorized.
4. Delegated execution only after scoped credentials, approval gates, audit, and revocation are implemented.

## 9. Initial vertical slice

Implement one workspace, two manually registered agents, artifact capture, context bundle creation, task handoff, acknowledgement, response capture, decision proposal/acceptance, and an audit timeline. This validates the central value before adding provider-specific complexity.
