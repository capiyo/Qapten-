# Shared frontend contracts

This package contains TypeScript representations of the Qapten API payloads consumed by both frontend clients.

- Web and mobile import from this package instead of defining parallel copies of API entities.
- The language-neutral, versioned protocol source of truth remains under `/contracts`.
- Update this package and the protocol schemas together when a public API payload changes.
- Do not put API calls, business logic, authorization decisions, or provider SDKs in this package.

The package is currently source-imported by relative path; it is not yet published as an independently versioned npm package.
