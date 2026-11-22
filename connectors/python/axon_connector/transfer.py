"""Explicitly approved provider-to-provider context handoff."""

from __future__ import annotations

from .models import AxonEnvelope
from .providers.deepseek import DeepSeekClient


async def transfer_to_deepseek(
    envelope: AxonEnvelope,
    instruction: str,
    client: DeepSeekClient,
    *,
    approved: bool,
) -> str:
    """Send a selected context bundle only after an explicit approval signal."""
    if not approved:
        raise PermissionError("Context transfer requires explicit user approval.")
    return await client.analyze(envelope, instruction)
