"""Explicitly approved conversation-to-Axon-memory extraction command."""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import sys

import httpx

from .memory import extract_memory_candidates, persist_memory_candidates
from .models import AxonEnvelope
from .providers.chatgpt import load_chatgpt_export
from .providers.claude import load_claude_export
from .providers.deepseek import DeepSeekClient, DeepSeekError


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="axon-memory")
    parser.add_argument("conversation_export", help="Path to one exported conversation JSON file.")
    parser.add_argument("--provider", choices=("chatgpt", "claude"), required=True)
    parser.add_argument("--workspace-id", required=True)
    parser.add_argument(
        "--actor-id",
        default="local-memory-extractor",
        help="AXP provenance label only; this is not an authenticated identity.",
    )
    parser.add_argument("--model", default=os.getenv("DEEPSEEK_MODEL", "deepseek-chat"))
    parser.add_argument("--api-url", default=os.getenv("AXON_API_URL", "http://127.0.0.1:8080"))
    parser.add_argument("--max-context-chars", type=int, default=120_000)
    parser.add_argument(
        "--approve-provider-transfer",
        action="store_true",
        help="Explicitly approve sending the selected conversation to DeepSeek for extraction.",
    )
    parser.add_argument(
        "--approve-persist",
        action="store_true",
        help="Explicitly approve storing the source conversation and proposed memories in local Axon.",
    )
    return parser


async def _run(args: argparse.Namespace) -> int:
    try:
        if args.max_context_chars <= 0:
            raise ValueError("--max-context-chars must be greater than zero.")
        if args.approve_persist and not args.approve_provider_transfer:
            raise ValueError("--approve-persist also requires --approve-provider-transfer.")

        if args.provider == "chatgpt":
            conversation = load_chatgpt_export(args.conversation_export)
        else:
            conversation = load_claude_export(args.conversation_export)

        envelope = AxonEnvelope.from_conversation(
            conversation,
            workspace_id=args.workspace_id,
            actor_id=args.actor_id,
        )
        serialized = json.dumps(envelope.to_dict(), ensure_ascii=False)
        if len(serialized) > args.max_context_chars:
            raise ValueError(
                f"Context is {len(serialized)} characters; limit is {args.max_context_chars}."
            )

        print(
            f"Source: {conversation.provider} export | {conversation.title}\n"
            f"Messages: {len(conversation.messages)} | Context: {len(serialized)} chars"
        )
        if not args.approve_provider_transfer:
            print(
                "Preview only. No conversation has been sent to an AI provider or stored. "
                "Re-run with --approve-provider-transfer to extract memory candidates."
            )
            return 0

        extractor = DeepSeekClient(model=args.model)
        candidates = await extract_memory_candidates(extractor, envelope, conversation)
        print("\n--- Proposed memories (not yet persisted) ---\n")
        print(
            json.dumps(
                [
                    {
                        "kind": item.kind,
                        "title": item.title,
                        "summary": item.summary,
                        "confidence": item.confidence,
                        "message_ids": list(item.message_ids),
                    }
                    for item in candidates
                ],
                ensure_ascii=False,
                indent=2,
            )
        )

        if not args.approve_persist:
            print(
                "\nExtraction complete. Nothing was stored. Review the candidates and "
                "re-run with --approve-persist to save them as proposed memories."
            )
            return 0

        import_id, stored = await persist_memory_candidates(
            conversation,
            candidates,
            workspace_id=args.workspace_id,
            api_url=args.api_url,
        )
        print(
            f"\nStored source import {import_id}; created {len(stored)} proposed memories. "
            "They remain inactive until reviewed in Axon."
        )
        return 0
    except (
        OSError,
        ValueError,
        PermissionError,
        DeepSeekError,
        httpx.HTTPError,
        json.JSONDecodeError,
    ) as exc:
        print(f"axon-memory: {exc}", file=sys.stderr)
        return 2


def main() -> None:
    raise SystemExit(asyncio.run(_run(build_parser().parse_args())))


if __name__ == "__main__":
    main()
