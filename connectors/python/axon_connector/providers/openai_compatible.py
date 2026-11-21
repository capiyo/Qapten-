"""Generic OpenAI-compatible chat-completions adapter for configured model providers."""

from __future__ import annotations

import os
from dataclasses import dataclass
from typing import Any

import httpx


class ModelProviderError(RuntimeError):
    """Raised when a configured model endpoint cannot complete a request."""


@dataclass(frozen=True, slots=True)
class ModelConfig:
    model_id: str
    provider: str
    display_name: str
    model: str
    base_url: str
    api_key_env: str
    capabilities: tuple[str, ...]
    estimated_cost_per_1k_tokens: float | None = None

    @property
    def api_key(self) -> str:
        value = os.getenv(self.api_key_env)
        if not value:
            raise ValueError(f"Environment variable {self.api_key_env} is not configured.")
        return value


class OpenAICompatibleClient:
    def __init__(self, config: ModelConfig, timeout_seconds: float = 45.0) -> None:
        self.config = config
        self.timeout_seconds = timeout_seconds

    async def complete(
        self,
        messages: list[dict[str, str]],
        *,
        max_tokens: int = 1200,
    ) -> str:
        api_key = self.config.api_key
        try:
            async with httpx.AsyncClient(timeout=self.timeout_seconds) as client:
                response = await client.post(
                    f"{self.config.base_url.rstrip('/')}/chat/completions",
                    headers={
                        "Authorization": f"Bearer {api_key}",
                        "Content-Type": "application/json",
                    },
                    json={
                        "model": self.config.model,
                        "messages": messages,
                        "max_tokens": max_tokens,
                        "stream": False,
                    },
                )
                response.raise_for_status()
                body: dict[str, Any] = response.json()
        except httpx.HTTPStatusError as exc:
            raise ModelProviderError(
                f"{self.config.model_id} returned HTTP {exc.response.status_code}."
            ) from None
        except httpx.HTTPError as exc:
            raise ModelProviderError(
                f"{self.config.model_id} request failed: {type(exc).__name__}."
            ) from None
        except ValueError:
            raise ModelProviderError(f"{self.config.model_id} returned invalid JSON.") from None

        try:
            content = body["choices"][0]["message"]["content"]
        except (KeyError, IndexError, TypeError):
            raise ModelProviderError(
                f"{self.config.model_id} response did not contain a message."
            ) from None
        if not isinstance(content, str) or not content.strip():
            raise ModelProviderError(f"{self.config.model_id} returned an empty message.")
        return content.strip()
