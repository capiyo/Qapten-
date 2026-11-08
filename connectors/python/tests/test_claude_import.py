import hashlib
import json

import pytest

from axon_connector.models import AxonEnvelope
from axon_connector.providers.claude import load_claude_export, parse_claude_export


def test_parse_claude_export_normalizes_messages():
    conversation = parse_claude_export(
        {
            "uuid": "conv-123",
            "name": "Architecture discussion",
            "chat_messages": [
                {"uuid": "m1", "sender": "human", "text": "Review this design."},
                {"uuid": "m2", "sender": "assistant", "text": "Here is my review."},
                {"sender": "tool", "text": "internal tool output"},
            ],
        }
    )
    assert conversation.provider == "claude"
    assert conversation.conversation_id == "conv-123"
    assert [message.role for message in conversation.messages] == ["user", "assistant"]


def test_export_loader_reads_json(tmp_path):
    path = tmp_path / "conversation.json"
    path.write_text(
        json.dumps(
            {
                "id": "conv-1",
                "messages": [{"role": "user", "content": "Hello"}],
            }
        ),
        encoding="utf-8",
    )
    assert load_claude_export(path).messages[0].content == "Hello"


def test_missing_conversation_id_is_rejected():
    with pytest.raises(ValueError, match="conversation ID"):
        parse_claude_export({"messages": [{"role": "user", "content": "Hello"}]})


def test_axp_envelope_contains_provenance_workspace_and_integrity():
    conversation = parse_claude_export(
        {"id": "conv-1", "messages": [{"role": "user", "content": "Hello"}]}
    )
    envelope = AxonEnvelope.from_conversation(conversation, "workspace-1", "firebase-uid-1")
    record = envelope.to_dict()
    assert record["workspace_id"] == "workspace-1"
    assert record["actor"]["principal_id"] == "firebase-uid-1"
    assert record["source"]["provider"] == "claude"
    assert record["source"]["capture_method"] == "manual_import"
    canonical_payload = json.dumps(
        record["payload"], ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    assert record["integrity"] == {
        "algorithm": "sha-256",
        "content_hash": hashlib.sha256(canonical_payload).hexdigest(),
    }


def test_axp_envelope_rejects_empty_identity_scope():
    conversation = parse_claude_export(
        {"id": "conv-1", "messages": [{"role": "user", "content": "Hello"}]}
    )
    with pytest.raises(ValueError, match="Workspace ID"):
        AxonEnvelope.from_conversation(conversation, "", "actor-1")
