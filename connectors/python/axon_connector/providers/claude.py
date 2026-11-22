"""Import Claude conversation exports; this does not scrape browser sessions."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from ..models import ConversationMessage, ImportedConversation


def _message_text(value: Any) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, list):
        parts: list[str] = []
        for item in value:
            if isinstance(item, str):
                parts.append(item)
            elif isinstance(item, dict):
                text = item.get("text")
                if isinstance(text, str):
                    parts.append(text)
        return "\n".join(part for part in parts if part.strip())
    if isinstance(value, dict):
        text = value.get("text")
        return text if isinstance(text, str) else ""
    return ""


def _role(value: Any) -> str | None:
    role = str(value or "").lower()
    if role in {"human", "user"}:
        return "user"
    if role in {"assistant", "claude"}:
        return "assistant"
    if role == "system":
        return "system"
    return None


def parse_claude_export(data: dict[str, Any]) -> ImportedConversation:
    """Normalize a Claude export conversation object into provider-neutral messages."""
    conversation_id = data.get("uuid") or data.get("id") or data.get("conversation_id")
    if not isinstance(conversation_id, str) or not conversation_id.strip():
        raise ValueError("Claude export is missing a conversation ID/UUID.")

    raw_messages = data.get("chat_messages") or data.get("messages")
    if not isinstance(raw_messages, list):
        raise TypeError("Claude export must contain a chat_messages or messages array.")

    messages: list[ConversationMessage] = []
    for item in raw_messages:
        if not isinstance(item, dict):
            continue
        sender = item.get("sender") or item.get("role") or item.get("author")
        if isinstance(sender, dict):
            sender = sender.get("role") or sender.get("name")
        role = _role(sender)
        content = _message_text(item.get("text") or item.get("content"))
        if role and content.strip():
            messages.append(
                ConversationMessage(
                    role=role,
                    content=content.strip(),
                    message_id=str(item.get("uuid") or item.get("id") or "") or None,
                    created_at=str(item.get("created_at") or "") or None,
                )
            )

    return ImportedConversation(
        provider="claude",
        conversation_id=conversation_id,
        title=str(data.get("name") or data.get("title") or "Imported Claude conversation"),
        messages=tuple(messages),
    )


def load_claude_export(path: str | Path) -> ImportedConversation:
    """Load one conversation JSON file from a user-authorized Claude export."""
    source = Path(path)
    with source.open("r", encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise TypeError("Expected a single Claude conversation JSON object.")
    return parse_claude_export(data)
