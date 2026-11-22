import asyncio

from axon_connector import cli
from axon_connector.models import ConversationMessage, ImportedConversation


def test_preview_does_not_construct_provider_client(monkeypatch, capsys):
    conversation = ImportedConversation(
        provider="claude",
        conversation_id="conversation-1",
        title="Review",
        messages=(ConversationMessage(role="user", content="Inspect this design."),),
    )
    monkeypatch.setattr(cli, "load_claude_export", lambda _path: conversation)

    def unexpected_client(**_kwargs):
        raise AssertionError("preview must not contact a provider")

    monkeypatch.setattr(cli, "DeepSeekClient", unexpected_client)
    args = cli.build_parser().parse_args(
        [
            "conversation.json",
            "--workspace-id",
            "workspace-1",
            "--actor-id",
            "actor-provenance",
            "--instruction",
            "Review the design.",
            "--print-context",
        ]
    )

    assert asyncio.run(cli._run(args)) == 0
    output = capsys.readouterr().out
    assert "Preview only" in output
    assert "Inspect this design." in output
    assert '"content_hash"' in output


def test_non_positive_context_limit_is_rejected(capsys):
    args = cli.build_parser().parse_args(
        [
            "conversation.json",
            "--workspace-id",
            "workspace-1",
            "--actor-id",
            "actor-provenance",
            "--instruction",
            "Review the design.",
            "--max-context-chars",
            "0",
        ]
    )

    assert asyncio.run(cli._run(args)) == 2
    assert "--max-context-chars must be greater than zero" in capsys.readouterr().err
