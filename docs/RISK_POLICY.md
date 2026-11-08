# Risk Policy

Axon moves information and proposed work across agent boundaries. Its primary risks are unauthorized disclosure, corrupted context, false provenance, and unintended action.

## Security invariants

1. A workspace is a hard authorization boundary.
2. An agent sees only artifacts explicitly shared with it and still authorized at read time.
3. Provider credentials and browser session cookies are never stored as ordinary agent metadata.
4. Imported content is untrusted data, even when it claims to be a system instruction or policy.
5. No model output can grant itself permissions or approve its own consequential action.
6. External actions require explicit authorization and a separate execution capability.
7. Every access, export, handoff, permission change, and decision transition is auditable.
8. Revocation prevents future access; it cannot make a recipient unlearn content already legitimately received. The UI must communicate this limitation.

## Threats and controls

| Threat | Required control |
|---|---|
| Cross-workspace data leakage | Server-side authorization on every read and write; negative isolation tests |
| Prompt injection in imported artifacts | Treat content as data; isolate instructions; never elevate privileges from payload text |
| Secret leakage | Secret detection/redaction, classification, preview-before-share, deny-by-default for secret labels |
| Forged provenance | Authenticated capture actor, immutable metadata, content hashes, source confidence |
| Malicious or compromised agent | Per-agent grants, bounded context, revocation, rate limits, audit |
| Infinite agent loops | Hop budgets, idempotency, deduplication, explicit approval for recursive delegation |
| Duplicate or replayed handoffs | Idempotency keys, expiry, state/version checks |
| Provider policy violations | Use official APIs or user-authorized capture; no bypassing access controls |
| Accidental destructive action | Read-only by default; human approval for external writes and irreversible actions |

## Data minimization

Collect only data needed for the selected task. Context bundles must be narrow and recipient-specific. Provide retention and deletion controls. Do not assume that because two agents belong to one person they should share every artifact.

## Human control

The initial release is advisory and coordination-focused. It does not autonomously deploy code, transfer money, send messages, change account settings, or perform other consequential external actions. Such capabilities require a separate permission model, preview, confirmation, execution log, and revocation mechanism.

## Incident handling

Record security events without copying unnecessary sensitive payloads into logs. Provide a way to revoke agent grants and invalidate active handoff tokens. Preserve enough audit evidence to investigate an incident while respecting configured retention and deletion requirements.
