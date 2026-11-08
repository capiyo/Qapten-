# Qapten Exchange Protocol (QEP) — Initial Design

**Status:** Draft design; not yet implemented or frozen.

## Goals

QEP defines a provider-neutral envelope for exchanging artifacts, context, tasks, handoffs, decisions, and feedback. It is designed for explicit, auditable collaboration across agents with different capabilities and trust levels.

## Envelope

Every exchanged record should carry:

- `protocol_version`: schema version, independent of application release.
- `record_id`: globally unique immutable identifier.
- `workspace_id`: security boundary.
- `record_type`: artifact, context_bundle, task, handoff, decision, feedback, or event.
- `created_at`: UTC timestamp.
- `actor`: human or agent identity that created the record.
- `source`: provider, capture method, and original reference when available.
- `classification`: visibility and sensitivity label.
- `payload`: type-specific validated data.
- `integrity`: content hash and canonicalization algorithm where applicable.

## Identity

An agent identity is a Qapten principal, not a provider login. It includes a stable Qapten ID, display name, provider identifier, declared capabilities, connection mode, and workspace-scoped grants. Never store provider passwords or browser session cookies as agent identity.

## Artifact

Artifacts are immutable snapshots. Corrections create a new artifact with a relationship to the previous version rather than silently rewriting history. Minimum metadata: media type, content hash, source, captured-by principal, capture time, size, and classification.

## Context bundle

A context bundle is an explicit, versioned manifest of artifact and decision references. It includes intended recipient, purpose, expiry, and policy version. The receiver must still pass current authorization checks when reading referenced content.

## Task and handoff lifecycle

Proposed states:

```text
draft -> ready -> offered -> acknowledged -> in_progress
                                      |             |
                                      v             v
                                   rejected     submitted
                                                   |
                                          review_required
                                            /       \
                                        accepted  changes_requested
                                            |
                                         completed

Any non-terminal task may be cancelled by an authorized actor.
Expired offers become expired; they are not silently accepted.
```

A handoff specifies the objective, constraints, expected output, recipient, context bundle, expiry, and idempotency key. Acknowledgement is a distinct event from completion. A submitted response does not automatically become an accepted decision.

## Decision record

A decision includes statement, status (proposed, accepted, rejected, superseded), rationale, alternatives, evidence references, proposer, reviewer, timestamps, and optional supersession target. Only an authorized human or configured policy may accept a decision with consequential effects.

## Loop prevention

Every task has a parent/root task reference, hop count, maximum hop budget, and deduplication key. Repeated equivalent handoffs must be detected. Agents cannot autonomously create unbounded successor tasks. Cycles require explicit user approval or a bounded policy.

## Compatibility

Schemas are versioned. Readers must reject unsupported major versions and preserve unknown optional fields where safe. Breaking changes require a new major protocol version and migration notes.

## Security requirements

- Validate size, type, schema, and content encoding.
- Treat payload text as untrusted and potentially prompt-injection-bearing.
- Enforce workspace, recipient, and artifact-level authorization.
- Never interpret artifact content as permission to call tools or execute actions.
- Record import, export, view, handoff, acknowledgement, decision, and revocation events.
