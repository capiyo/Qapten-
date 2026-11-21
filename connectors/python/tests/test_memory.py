import asyncio
import json

import pytest

from axon_connector.memory import extract_memory_candidates, parse_memory_candidates
from axon_connector.models import AxonEnvelope, ConversationMessage, ImportedConversation


@pytest.fixture
def conversation() -> ImportedConversation:
    return ImportedConversation(
        provider="chatgpt",
        conversation_id="chat-1",
        title="Axon memory",
        messages=(
            ConversationMessage(
                role="user",
                content="Axon should own durable memory.",
                message_id="message-1",
            ),
            ConversationMessage(
                role="assistant",
                content="Use MongoDB as the source of truth.",
                message_id="message-2",
            ),
        ),
    )


def test_parses_and_validates_memory_candidates(conversation: ImportedConversation) -> None:
    response = json.dumps(
        {
            "memories": [
                {
                    "kind": "project",
                    "title": "Axon owns canonical memory",
                    "content": "Axon stores provider-independent memory in its own database.",
                    "summary": "Axon owns persistent memory independently of AI providers.",
                    "tags": ["axon", "memory"],
                    "confidence": 0.95,
                    "message_ids": ["message-1", "message-2"],
                }
            ]
        }
    )

    candidates = parse_memory_candidates(response, conversation)

    assert len(candidates) == 1
    assert candidates[0].kind == "project"
    assert candidates[0].confidence == 0.95
    assert candidates[0].message_ids == ("message-1", "message-2")


def test_rejects_memory_candidates_with_untrusted_source_references(
    conversation: ImportedConversation,
) -> None:
    response = json.dumps(
        {
            "memories": [
                {
                    "kind": "project",
                    "title": "Invented source",
                    "content": "A memory with an invalid source reference.",
                    "summary": "Invalid provenance.",
                    "tags": [],
                    "confidence": 0.8,
                    "message_ids": ["not-in-this-conversation"],
                }
            ]
        }
    )

    with pytest.raises(ValueError, match="outside the imported conversation"):
        parse_memory_candidates(response, conversation)


def test_rejects_non_finite_confidence(conversation: ImportedConversation) -> None:
    response = (
        '{"memories":[{"kind":"project","title":"T","content":"C",'
        '"summary":"S","tags":[],"confidence":1e999,"message_ids":[]}]}'
    )

    with pytest.raises(ValueError, match="finite number"):
        parse_memory_candidates(response, conversation)


def test_rejects_non_json_or_unexpected_model_output(conversation: ImportedConversation) -> None:
    with pytest.raises(ValueError, match="invalid JSON"):
        parse_memory_candidates("I think these are the memories.", conversation)



def test_memory_extraction_requires_explicit_provider_approval(
    conversation: ImportedConversation,
) -> None:
    class NeverCalledClient:
        async def analyze(self, envelope, instruction):
            raise AssertionError("Provider must not be called without approval.")

    envelope = AxonEnvelope.from_conversation(
        conversation,
        workspace_id="workspace-1",
        actor_id="local-test",
    )
    with pytest.raises(PermissionError, match="explicit user approval"):
        asyncio.run(
            extract_memory_candidates(
                NeverCalledClient(),
                envelope,
                conversation,
                approved=False,
            )
        )
