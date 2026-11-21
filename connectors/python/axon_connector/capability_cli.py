"""Ask multiple models to nominate a specialist, then execute only the majority winner."""

from __future__ import annotations

import argparse
import asyncio
import json
import math
import os
import sys

import httpx

from .capability_router import (
    CapabilityRoutingError,
    load_model_catalog,
    run_majority_route,
)
from .providers.openai_compatible import ModelProviderError


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="axon-route")
    parser.add_argument("--task", required=True, help="The task brief to route and execute.")
    parser.add_argument(
        "--capability",
        required=True,
        help="Capability category, e.g. visual_design.",
    )
    parser.add_argument("--workspace-id", required=True)
    parser.add_argument("--models", required=True, help="Path to the JSON model catalog.")
    parser.add_argument(
        "--max-cost-per-1k-tokens",
        type=float,
        help="Optional estimated USD cost ceiling per 1000 tokens; models without a price are excluded.",
    )
    parser.add_argument(
        "--api-url",
        default=os.getenv("AXON_API_URL", "http://127.0.0.1:8080"),
    )
    parser.add_argument(
        "--include-memory",
        action="store_true",
        help="Include relevant approved Axon memories in the provider poll and winning task.",
    )
    parser.add_argument(
        "--approve-provider-sharing",
        action="store_true",
        help="Explicitly approve sending the task and any selected memories to every voter.",
    )
    return parser


async def _run(args: argparse.Namespace) -> int:
    try:
        if not args.task.strip() or len(args.task) > 10_000:
            raise ValueError("Task must contain 1–10000 characters.")
        if not args.capability.strip() or len(args.capability) > 120:
            raise ValueError("Capability must contain 1–120 characters.")
        if args.max_cost_per_1k_tokens is not None and (
            not math.isfinite(args.max_cost_per_1k_tokens)
            or args.max_cost_per_1k_tokens < 0
        ):
            raise ValueError("--max-cost-per-1k-tokens must be finite and non-negative.")

        models = load_model_catalog(args.models)
        available_models = tuple(
            model
            for model in models
            if os.getenv(model.api_key_env)
            and (args.capability in model.capabilities or "*" in model.capabilities)
            and (
                args.max_cost_per_1k_tokens is None
                or (
                    model.estimated_cost_per_1k_tokens is not None
                    and model.estimated_cost_per_1k_tokens <= args.max_cost_per_1k_tokens
                )
            )
        )
        if len(available_models) < 3:
            raise ValueError(
                f"Only {len(available_models)} models have configured API keys; "
                "at least 3 are required."
            )

        print(f"Capability: {args.capability}")
        print(f"Task: {args.task}")
        memory_label = "requested" if args.include_memory else "not included"
        print(f"Approved Axon memory context: {memory_label}")
        print("Eligible models:")
        for model in available_models:
            cost = (
                "unknown price"
                if model.estimated_cost_per_1k_tokens is None
                else f"${model.estimated_cost_per_1k_tokens:.6f}/1k tokens"
            )
            print(
                f"- {model.display_name} ({model.provider}) "
                f"[{model.model_id}] — {cost}"
            )

        if not args.approve_provider_sharing:
            print(
                "\nPreview only. No provider has received the task. "
                "Re-run with --approve-provider-sharing to start the majority poll."
            )
            return 0

        outcome = await run_majority_route(
            workspace_id=args.workspace_id,
            capability=args.capability,
            task_brief=args.task,
            models=available_models,
            api_url=args.api_url,
            include_memory=args.include_memory,
            provider_sharing_approved=args.approve_provider_sharing,
            max_cost_per_1k_tokens=args.max_cost_per_1k_tokens,
        )
        print("\n--- Majority poll ---")
        print(json.dumps(outcome["result"], ensure_ascii=False, indent=2))
        print(f"Poll ID: {outcome['poll_id']}")
        if outcome["memory_ids"]:
            print("Memory IDs included: " + ", ".join(outcome["memory_ids"]))
        if outcome["provider_failures"]:
            print("Provider failures:")
            for failure in outcome["provider_failures"]:
                print(f"- {failure}")

        if outcome["winner"] is None:
            print("No strict majority. Axon did not dispatch the task to any model.")
            return 0

        print("\n--- Selected model ---")
        print(json.dumps(outcome["winner"], ensure_ascii=False, indent=2))
        print("\n--- Task result ---\n")
        print(outcome["task_result"])
        return 0
    except (
        OSError,
        ValueError,
        CapabilityRoutingError,
        ModelProviderError,
        PermissionError,
        httpx.HTTPError,
    ) as exc:
        print(f"axon-route: {exc}", file=sys.stderr)
        return 2


def main() -> None:
    raise SystemExit(asyncio.run(_run(build_parser().parse_args())))


if __name__ == "__main__":
    main()
