import pytest

from axon_connector.models import AxonEnvelope
from axon_connector.providers.claude import parse_claude_export
from axon_connector.transfer import transfer_to_deepseek


class FakeDeepSeek:
    async def analyze(self, envelope, instruction):
        return "review complete"


def make_envelope():
    conversation = parse_claude_export(
        {"id": "conv-1", "messages": [{"role": "user", "content": "Analyze this."}]}
    )
    return AxonEnvelope.from_conversation(conversation, "workspace-1", "actor-1")


@pytest.mark.asyncio
async def test_transfer_requires_explicit_approval():
    with pytest.raises(PermissionError, match="explicit user approval"):
        await transfer_to_deepseek(
            make_envelope(), "Review", FakeDeepSeek(), approved=False
        )


@pytest.mark.asyncio
async def test_approved_transfer_returns_provider_response():
    result = await transfer_to_deepseek(
        make_envelope(), "Review", FakeDeepSeek(), approved=True
    )
    assert result == "review complete"
