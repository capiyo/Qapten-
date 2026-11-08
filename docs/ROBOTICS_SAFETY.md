# Robotics protocol and safety boundary (draft v1)

## Purpose

Axon coordinates people, software agents, simulators, and physical devices. Robotics is a first-class domain, but this implementation is a protocol and validation foundation only. It does not connect to hardware and cannot dispatch actuator commands.

## Trust boundary

1. Device identity must be established by a cryptographic credential at a future gateway. A device ID in JSON is only a lookup hint, never proof of identity.
2. Every proposal must be attributed to an authenticated requester and checked against workspace/device grants.
3. Models can create proposals, not approvals. Deterministic policy code makes authorization decisions outside the model.
4. Proposals are time-bounded, sequence-ordered, and idempotent records. Gateways must reject replay, stale sequence numbers, expired proposals, and mismatched device identities.
5. Device-local safety controllers and hardware interlocks remain authoritative. Axon cannot override them.
6. Emergency stop is local and independent of Axon, network connectivity, cloud services, and model availability. A remote stop request may supplement but never replace the physical stop path.
7. A validated proposal is still not dispatch authorization. A future gateway must perform a second device-local check immediately before actuation.

## Initial AXP records

- Device registration schema: registration metadata and identity-key fingerprint. Production registration must bind the fingerprint to a verified key challenge.
- Telemetry event schema: sequenced observations with observation and receipt times. Consumers must reject duplicate event IDs and detect sequence gaps.
- Command proposal schema: non-executable intent. It intentionally has no actuator URL, transport field, or executable payload format.
- Safety state schema: emergency-stop, interlock, operating mode, and fault state.

## Deterministic validation implemented

The Rust domain module validates authenticated device identity, explicit requester authorization, inactive emergency stop, healthy interlocks, manual/assisted operating mode, valid issue/expiry window, maximum command age, strictly increasing sequence, minimum idempotency-key length, and finite velocity within configured bounds.

The validator is pure domain logic. It does not authenticate cryptographic keys, persist replay state, approve a proposal, or communicate with a robot. A successful validation result must never be interpreted as permission to actuate.

## Future gateway requirements (not implemented)

- Mutual TLS or device-bound public-key challenge and rotation.
- Workspace/device authorization with least-privilege capability grants.
- Durable idempotency and monotonic sequence store.
- Signed, short-lived dispatch authorization bound to device, proposal hash, policy version, and expiry.
- Device-local limit enforcement, watchdog, collision avoidance, geofencing, and hardware interlocks.
- Independent physical emergency stop and verified safe-state behavior after loss of power/network.
- Tamper-evident audit log and telemetry retention controls.
- Simulation/HIL testing, fault injection, safety case, and per-platform review before any hardware pilot.

No physical device should be connected to this API. No actuator endpoint is exposed.
