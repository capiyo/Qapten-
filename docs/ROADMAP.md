# Roadmap

The roadmap is outcome-driven. A phase is complete only when its acceptance criteria are tested and documented.

## Phase 0 — Architecture and repository foundation
- [x] Define product boundary and principles.
- [x] Define domain vocabulary and initial service boundaries.
- [x] Draft exchange protocol and security invariants.
- [x] Add initial service and client skeletons (build verification pending).
- [x] Add CI workflow for Rust formatting, Clippy, Rust tests, web build, and mobile TypeScript check (all jobs passed in the verified run).

## Phase 1 — Two-agent manual collaboration
- [ ] Workspace and membership model.
- [ ] Human authentication integration and workspace authorization.
- [ ] Manual agent registration (no provider credentials).
- [ ] Artifact import with provenance, preview, classification, and content limits.
- [ ] Context bundle manifest and recipient-specific sharing.
- [x] Initial task/handoff creation and acknowledgement/review state transitions in the API (response artifact capture and UI remain open).
- [ ] Decision proposals and explicit human acceptance.
- [ ] Audit timeline and revocation.
- [ ] Web UI for the complete workflow.

**Acceptance:** A user can move a task between two manually represented assistants without manually reconstructing the entire context, and can inspect exactly what was shared and what changed.

## Phase 2 — Mobile and browser-assisted capture
- [x] React Native workspace creation/listing and basic agent, artifact, and task workflows.
- [ ] Mobile context selection, response submission, and review controls.
- [ ] Browser-assisted capture with user initiation, preview, and explicit submit.
- [ ] Local-first draft capture with safe synchronization.
- [ ] Export formats suitable for providers without APIs.

**Acceptance:** Capture is user-visible, permissioned, reversible before submission, and does not read private provider sessions in the background.

## Phase 3 — Official provider integrations
- [ ] Provider adapter interface.
- [ ] First official API integration after provider capability and policy review.
- [ ] Webhook verification and replay protection.
- [ ] Capability-aware routing and rate-limit handling.

**Acceptance:** Provider integrations are optional adapters; manual workflows continue to work if a provider API is unavailable.

## Phase 4 — Multi-agent collaboration
- [ ] Agent capability registry.
- [ ] Bounded delegation and task graph.
- [ ] Conflict detection and decision comparison.
- [ ] Feedback and evaluation loops.
- [ ] Loop budgets, scheduling, and failure recovery.

**Acceptance:** Multi-agent tasks are bounded, explainable, attributable, and interruptible by the user.

## Phase 5 — Project engineering workspaces
- [ ] Repository-aware context adapters.
- [ ] Read-only codebase analysis and review workflows.
- [ ] Controlled patch proposals and test evidence.
- [ ] Optional GitHub integration with least-privilege scopes.

**Acceptance:** Axon can coordinate analysis and review for Funspot and Qoin without silently changing their repositories or deployment settings.

## Robotics foundation — protocol draft

- [x] Define AXP device registration, telemetry, command proposal, and safety-state schemas.
- [x] Add shared TypeScript contract types.
- [x] Add deterministic Rust proposal validation and safety rejection tests.
- [x] Document the no-actuator boundary and future gateway requirements.
- [ ] Add cryptographic device enrollment, durable replay/idempotency storage, and append-only robotics audit events.
- [ ] Add simulator and hardware-in-the-loop test harness before any physical integration.
- [ ] Implement a separately secured device gateway only after safety review.

**Acceptance:** AI-generated intent remains non-executable; device identity, authorization, freshness, sequence, interlocks, emergency-stop state, and configured motion bounds are checked before any future dispatch authorization. No hardware dispatch is part of this phase.

## Explicit non-goals for the first release

- A model marketplace or benchmark leaderboard.
- Unrestricted autonomous agents.
- Background scraping of logged-in provider sessions.
- Automatic deployment or production changes.
- A premature microservice fleet.
