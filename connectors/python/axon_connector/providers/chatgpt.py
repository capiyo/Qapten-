"""Import one conversation object from a user-authorized ChatGPT data export."""

from __future__ import annotations

import json
from datetime import UTC, datetime
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


def _timestamp(value: Any) -> str | None:
    if not isinstance(value, (int, float)):
        return None
    try:
        return datetime.fromtimestamp(value, UTC).isoformat()
    except (OverflowError, OSError, ValueError):
        return None


def parse_chatgpt_export(data: dict[str, Any]) -> ImportedConversation:
    """Normalize one conversation from conversations.json into provider-neutral messages."""
    conversation_id = data.get("id") or data.get("conversation_id")
    if not isinstance(conversation_id, str) or not conversation_id.strip():
        raise ValueError("ChatGPT export is missing a conversation ID.")

    mapping = data.get("mapping")
    raw_messages: list[dict[str, Any]] = []
    if isinstance(mapping, dict):
        for node in mapping.values():
            if not isinstance(node, dict):
                continue
            message = node.get("message")
            if isinstance(message, dict):
                raw_messages.append(message)
    elif isinstance(data.get("messages"), list):
        raw_messages = [
            item for item in data["messages"] if isinstance(item, dict)
        ]
    else:
        raise TypeError("ChatGPT export must contain a mapping object or messages array.")

    def sort_key(message: dict[str, Any]) -> float:
        value = message.get("create_time") or message.get("created_at")
        return float(value) if isinstance(value, (int, float)) else 0.0

    messages: list[ConversationMessage] = []
    for item in sorted(raw_messages, key=sort_key):
        author = item.get("author")
        role = author.get("role") if isinstance(author, dict) else item.get("role")
        if role not in {"system", "user", "assistant"}:
            continue
        content = item.get("content")
        if isinstance(content, dict):
            content = content.get("parts") or content.get("text")
        text = _message_text(content).strip()
        if not text:
            continue
        messages.append(
            ConversationMessage(
                role=role,
                content=text,
                message_id=str(item.get("id") or "") or None,
                created_at=_timestamp(item.get("create_time") or item.get("created_at")),
            )
        )

    return ImportedConversation(
        provider="chatgpt",
        conversation_id=conversation_id,
        title=str(data.get("title") or "Imported ChatGPT conversation"),
        messages=tuple(messages),
    )


def load_chatgpt_export(path: str | Path) -> ImportedConversation:
    """Load one conversation JSON object from a user-authorized ChatGPT export."""
    source = Path(path)
    with source.open("r", encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise TypeError("Expected a single ChatGPT conversation JSON object.")
    return parse_chatgpt_export(data)
