from axon_connector.providers.chatgpt import parse_chatgpt_export


def test_parses_chatgpt_mapping_into_ordered_messages() -> None:
    conversation = parse_chatgpt_export(
        {
            "id": "chat-123",
            "title": "Axon architecture",
            "mapping": {
                "assistant-node": {
                    "message": {
                        "id": "m2",
                        "author": {"role": "assistant"},
                        "create_time": 2,
                        "content": {"content_type": "text", "parts": ["Use MongoDB for memory."]},
                    }
                },
                "user-node": {
                    "message": {
                        "id": "m1",
                        "author": {"role": "user"},
                        "create_time": 1,
                        "content": {"content_type": "text", "parts": ["How should Axon remember?"]},
                    }
                },
            },
        }
    )

    assert conversation.provider == "chatgpt"
    assert conversation.conversation_id == "chat-123"
    assert [message.message_id for message in conversation.messages] == ["m1", "m2"]
    assert conversation.messages[0].role == "user"
    assert conversation.messages[1].content == "Use MongoDB for memory."


def test_rejects_chatgpt_export_without_conversation_id() -> None:
    import pytest

    with pytest.raises(ValueError, match="conversation ID"):
        parse_chatgpt_export({"mapping": {}})
