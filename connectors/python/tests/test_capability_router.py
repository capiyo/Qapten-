import json

import pytest

from axon_connector.capability_router import (
    CapabilityRoutingError,
    _decode_nomination,
    load_model_catalog,
)


def _model(index: int) -> dict[str, object]:
    return {
        "id": f"provider-{index}:model-{index}",
        "provider": f"provider-{index}",
        "display_name": f"Model {index}",
        "model": f"remote-model-{index}",
        "base_url": "https://provider.invalid/v1",
        "api_key_env": f"MODEL_{index}_KEY",
        "capabilities": ["visual_design"],
        "estimated_cost_per_1k_tokens": 0.001,
    }


def test_loads_a_ten_model_catalog(tmp_path) -> None:
    catalog = tmp_path / "models.json"
    catalog.write_text(json.dumps({"models": [_model(index) for index in range(10)]}), encoding="utf-8")

    models = load_model_catalog(catalog)

    assert len(models) == 10
    assert models[0].model_id == "provider-0:model-0"
    assert models[0].capabilities == ("visual_design",)


def test_rejects_duplicate_model_ids(tmp_path) -> None:
    catalog = tmp_path / "models.json"
    catalog.write_text(json.dumps([_model(1), _model(1), _model(2)]), encoding="utf-8")

    with pytest.raises(ValueError, match="Duplicate model ID"):
        load_model_catalog(catalog)


def test_decodes_only_eligible_non_self_nominations() -> None:
    candidate_ids = {"model-a", "model-b", "model-c"}

    result = _decode_nomination(
        '{"nominated_model_id":"model-b","rationale":"Best visual output capability."}',
        "model-a",
        candidate_ids,
    )

    assert result["nominated_model_id"] == "model-b"
    with pytest.raises(CapabilityRoutingError, match="self-nomination"):
        _decode_nomination(
            '{"nominated_model_id":"model-a","rationale":"I am the best."}',
            "model-a",
            candidate_ids,
        )
    with pytest.raises(CapabilityRoutingError, match="outside the eligible catalog"):
        _decode_nomination(
            '{"nominated_model_id":"unknown","rationale":"Unknown model."}',
            "model-a",
            candidate_ids,
        )
