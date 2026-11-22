# Chats and Project Memory

Phase B adds workspace-scoped project conversations and reviewable, event-sourced memory.

## Data model

- `chats`: workspace-owned conversation metadata with creator `owner_id`, newest activity first, archive state and a monotonic memory version.
- `chat_messages`: ordered `user | assistant | system` messages. Assistant messages may carry `answered_by` to identify the responding agent/provider.
- `memory_events`: append-only event log. Each event stores a complete entry snapshot and one of `created | approved | edited | rejected | deleted`. The current memory view is rebuilt by replaying events in sequence order; there is no independently mutable memory-state table.
- `resume_brief_cache`: disposable derived data keyed by chat ID, memory version, last message ID and prompt version. Cache rows are deleted in the same transaction as message/memory mutations and lazily rebuilt on the next chat read.

Memory entry types are `decision | fact | constraint | open_task | note`; statuses are `proposed | approved | rejected`. Only approved entries are included in resume briefs. Editing an entry returns it to proposed status and clears its approval timestamp.

## API

All routes require a verified Firebase ID token and workspace membership. A resource outside the caller's workspace returns 404.

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/api/v1/workspaces/{workspace_id}/chats` | List non-archived chats, newest activity first |
| POST | `/api/v1/workspaces/{workspace_id}/chats` | Create a chat (`{ "title": "..." }`) |
| GET | `/api/v1/workspaces/{workspace_id}/chats/{chat_id}` | Read chat, messages, memory and resume brief |
| POST | `/api/v1/workspaces/{workspace_id}/chats/{chat_id}/messages` | Append a message (`role`, `content`, optional `answered_by`) |
| PATCH | `/api/v1/workspaces/{workspace_id}/chats/{chat_id}/archive` | Archive a chat |
| POST | `/api/v1/workspaces/{workspace_id}/chats/{chat_id}/memory` | Propose memory (`type`, `text`, optional `reason` and `source_message_id`) |
| POST | `/api/v1/workspaces/{workspace_id}/chats/{chat_id}/memory/{entry_id}/approve` | Approve a proposal |
| POST | `/api/v1/workspaces/{workspace_id}/chats/{chat_id}/memory/{entry_id}/reject` | Reject a proposal |
| POST | `/api/v1/workspaces/{workspace_id}/chats/{chat_id}/memory/{entry_id}/edit` | Edit text/reason; edited entries return to proposed |
| POST | `/api/v1/workspaces/{workspace_id}/chats/{chat_id}/memory/{entry_id}/delete` | Append a deletion event (does not erase history) |

## Resume brief

The brief is deterministic and bounded to 190 words. It uses the last six messages and approved decisions, open tasks, facts, notes and constraints. It always includes Where left off, Decisions, Open tasks and Next / constraints (with explicit empty-state values), and includes Facts when recorded. Proposed or rejected memory never enters the brief. It is rebuilt lazily when its cache key changes.

## Transaction and privacy guarantees

Each chat/message mutation and its audit record are committed in one SQLite transaction. Each memory mutation appends a memory event, increments the chat memory version, purges the derived brief cache and writes its audit event in one transaction. Audit metadata contains IDs, roles, types and statuses only; message and memory text is not copied into audit metadata or request logs. Memory-event update/delete triggers reject attempts to rewrite history.

The chat record includes its creator as `owner_id`; workspace membership remains the access boundary. The web and mobile clients expose chat creation, message history, resume briefs and memory proposal/approval. Axon does not fabricate an assistant response: assistant messages are persisted only when a real agent/provider response is supplied to the message API.
