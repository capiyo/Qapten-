# Qapten with Funspot and Qoin

Qapten is intended to improve the engineering workflow across the user's projects without becoming a hidden dependency that can silently alter them.

## Example: Funspot

A task might ask agents to investigate why fixture state changes to `live` too early.

A Qapten workspace could hold:
- the reported behavior and reproduction steps;
- relevant frontend, backend, and scraper evidence;
- the agent that inspected each subsystem;
- competing explanations and their supporting evidence;
- a decision record identifying the selected fix;
- proposed patches and test results;
- a human acceptance record.

An agent reviewing the React Native client should receive only the relevant files, findings, and contracts—not every unrelated conversation or private artifact in the workspace.

## Example: Qoin

A Qoin workspace could coordinate work on a market-data adapter, deterministic signal logic, risk policy, or backtesting methodology.

Qapten should preserve:
- data source and timestamp provenance;
- algorithm and parameter versions;
- assumptions and limitations;
- backtest configuration and reproducibility metadata;
- independent review findings;
- accepted and rejected design decisions.

A model-generated interpretation must never be presented as verified market data or a validated performance result.

## Integration boundary

The first Qapten release should not require either project to depend on Qapten at runtime. Start with repository links, manually imported artifacts, and user-approved context bundles. Later, add read-only repository adapters and controlled patch proposals.

Any write access to Funspot or Qoin must be separately authorized, scoped to a repository and branch, previewed, and audited. Production deployment and secret management remain outside Qapten's authority unless a future explicit execution design is approved.

## Long-term direction

Qapten becomes the shared engineering memory and coordination layer:
- Funspot contributes real-world product, frontend, backend, and operations tasks.
- Qoin contributes data, quantitative research, testing, and risk-analysis tasks.
- Qapten provides the common handoff, provenance, review, and decision infrastructure.

The projects remain independently buildable and deployable.
