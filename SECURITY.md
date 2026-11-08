# Security Policy

## Supported status

Axon is an early-stage project. The current foundation is a local development prototype, not a production-ready hosted service.

## Reporting a vulnerability

Do not publish exploitable vulnerability details in a public issue. Use GitHub's private vulnerability reporting feature for this repository when available. If private reporting is unavailable, contact the repository maintainers privately through a verified contact channel listed on the repository profile.

Include the affected commit or version, impact, reproduction steps, and any suggested mitigation. Do not include real credentials, private user data, or unredacted conversation content.

## Current security limitations

- The API uses process-local in-memory storage.
- Authentication, user identity, workspace membership, and authorization are not implemented.
- The API binds to loopback by default and refuses non-loopback binding unless an explicit development override is set.
- Manual agent registration does not establish a provider connection.
- Do not expose the current API to the internet or use it with sensitive or production data.

A successful CI run is not a security audit or production-readiness certification.
