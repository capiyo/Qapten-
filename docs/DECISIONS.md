# Axon decision log

This file is append-only. Add a new dated entry when a product or architecture decision is approved; do not rewrite or delete earlier decisions.

## 2026-11-21 — Product navigation

- The primary navigation has two tabs: Chats and News.
- Chats are projects. Opening an existing chat shows a short resume brief at the top.
- New chat is a floating action associated with the bottom navigation.
- Memory is a panel inside each chat, opened from the chat top bar. Memory is not a primary tab.
- A third Memory tab is not justified yet; revisit only if cross-project memory management becomes a frequent standalone workflow.

## 2026-11-21 — Funspot navigation reference

- Axon should follow Funspot's evenly spaced full-width bottom navigation reference, not invent a new navigation design.
- The center floating action is adapted to New chat.
- Reuse Funspot tokens and shared components where compatible with Axon's web and React Native stacks.

## 2026-11-21 — News isolation

- News is a read-only feed populated by one endpoint; the user owns and operates the fetching service.
- Required item fields: title, source, publish time and article link; image is optional.
- Source and publish time must always be displayed.
- Full articles open in the browser.
- News content is not sent to AI and is not saved to memory unless the user explicitly invokes an action such as Ask Axon about this.
- The exact endpoint response schema is pending explicit user agreement before News consumer coding.
- News must be paginated, cached on device and readable offline from the last fetched pages. Images are optional.
- Ads, if ever added, are separate and clearly labelled.

## 2026-11-21 — Persistence foundation

- SQLite is the first durable adapter behind a storage interface; MongoDB may be added later behind the same interface.
- Entity mutation and its audit event must commit atomically.
- Audit events are append-only.
- The database is the canonical source for durable project state; runtime maps are caches, not the source of truth.
- Authentication and route authorization must be completed before remote exposure or sensitive user data.
