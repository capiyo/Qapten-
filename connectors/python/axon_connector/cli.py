"""Command-line entry point for an explicit Claude-export to DeepSeek handoff."""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import sys

from .models import AxonEnvelope
from .providers.claude import load_claude_export
from .providers.deepseek import DeepSeekClient, DeepSeekError
from .transfer import transfer_to_deepseek


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="axon-bridge")
    parser.add_argument("claude_export", help="Path to one conversation JSON from a Claude export.")
    parser.add_argument("--workspace-id", required=True)
    parser.add_argument("--actor-id", required=True, help="Verified AXON actor ID supplied by AXON.")
    parser.add_argument("--instruction", required=True, help="What DeepSeek should do with this context.")
    parser.add_argument("--model", default=os.getenv("DEEPSEEK_MODEL", "deepseek-chat"))
    parser.add_argument("--approve-transfer", action="store_true")
    parser.add_argument("--max-context-chars", type=int, default=120_000)
    return parser


async def _run(args: argparse.Namespace) -> int:
    try:
        conversation = load_claude_export(args.claude_export)
        envelope = AxonEnvelope.from_conversation(
            conversation, workspace_id=args.workspace_id, actor_id=args.actor_id
        )
        serialized = json.dumps(envelope.to_dict(), ensure_ascii=False)
        if len(serialized) > args.max_context_chars:
            raise ValueError(
                f"Context is {len(serialized)} characters; limit is {args.max_context_chars}."
            )
        print(
            f"Source: Claude export | Conversation: {conversation.title}\n"
            f"Messages: {len(conversation.messages)} | Context size: {len(serialized)} chars"
        )
        if not args.approve_transfer:
            print("Preview only. Re-run with --approve-transfer to send this context to DeepSeek.")
            return 0

        client = DeepSeekClient(model=args.model)
        result = await transfer_to_deepseek(
            envelope, args.instruction, client, approved=True
        )
        print("\n--- DeepSeek response ---\n")
        print(result)
        return 0
    except (OSError, ValueError, PermissionError, DeepSeekError, json.JSONDecodeError) as exc:
        print(f"axon-bridge: {exc}", file=sys.stderr)
        return 2


def main() -> None:
    raise SystemExit(asyncio.run(_run(build_parser().parse_args())))


if __name__ == "__main__":
    main()
