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

    with pytest.raises(TypeError, match="conversation ID"):
        parse_chatgpt_export({"mapping": {}})



def test_loads_one_conversation_from_full_export_with_explicit_selection(tmp_path) -> None:
    import json

    import pytest

    from axon_connector.providers.chatgpt import load_chatgpt_export

    export = tmp_path / "conversations.json"
    export.write_text(
        json.dumps(
            [
                {"id": "chat-1", "title": "First", "mapping": {}},
                {
                    "id": "chat-2",
                    "title": "Second",
                    "mapping": {
                        "node": {
                            "message": {
                                "id": "m2",
                                "author": {"role": "user"},
                                "content": {"parts": ["Hello"]},
                            }
                        }
                    },
                },
            ]
        ),
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="multiple conversations"):
        load_chatgpt_export(export)

    selected = load_chatgpt_export(export, conversation_id="chat-2")
    assert selected.conversation_id == "chat-2"
    assert selected.title == "Second"
