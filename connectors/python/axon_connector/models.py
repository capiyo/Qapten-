"""Provider-neutral conversation and AXP envelope models."""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
from datetime import UTC, datetime
from typing import Any
from uuid import uuid4


@dataclass(frozen=True, slots=True)
class ConversationMessage:
    role: str
    content: str
    message_id: str | None = None
    created_at: str | None = None

    def __post_init__(self) -> None:
        if self.role not in {"system", "user", "assistant"}:
            raise ValueError(f"Unsupported conversation role: {self.role}")
        if not self.content.strip():
            raise ValueError("Conversation message content cannot be empty.")


@dataclass(frozen=True, slots=True)
class ImportedConversation:
    provider: str
    conversation_id: str
    title: str
    messages: tuple[ConversationMessage, ...]
    capture_method: str = "manual_import"

    def __post_init__(self) -> None:
        if self.provider != "claude":
            raise ValueError("This importer currently accepts Claude conversations only.")
        if not self.conversation_id.strip():
            raise ValueError("Conversation ID is required.")
        if not self.messages:
            raise ValueError("Conversation must contain at least one message.")


@dataclass(frozen=True, slots=True)
class AxonEnvelope:
    workspace_id: str
    actor_id: str
    payload: dict[str, Any]
    record_id: str = field(default_factory=lambda: str(uuid4()))
    protocol_version: str = "1.0.0"
    record_type: str = "context_bundle"
    created_at: str = field(default_factory=lambda: datetime.now(UTC).isoformat())
    classification: str = "workspace"
    source_provider: str = "claude"
    capture_method: str = "manual_import"

    def to_dict(self) -> dict[str, Any]:
        return {
            "protocol_version": self.protocol_version,
            "record_id": self.record_id,
            "workspace_id": self.workspace_id,
            "record_type": self.record_type,
            "created_at": self.created_at,
            "actor": {"principal_type": "human", "principal_id": self.actor_id},
            "source": {
                "provider": self.source_provider,
                "capture_method": self.capture_method,
            },
            "classification": self.classification,
            "payload": self.payload,
        }

    @classmethod
    def from_conversation(
        cls, conversation: ImportedConversation, workspace_id: str, actor_id: str
    ) -> "AxonEnvelope":
        return cls(
            workspace_id=workspace_id,
            actor_id=actor_id,
            source_provider=conversation.provider,
            capture_method=conversation.capture_method,
            payload={
                "conversation_id": conversation.conversation_id,
                "title": conversation.title,
                "messages": [asdict(message) for message in conversation.messages],
            },
        )
