"""Convert FeatureResult / domain payloads to MCP tool response strings."""

from __future__ import annotations

import base64
import json
from pathlib import Path
from typing import Any

from maimai_mcp.result import FeatureResult, _serialize


def result_to_json(
    result: FeatureResult,
    *,
    include_image_b64: bool = False,
) -> str:
    payload = result.to_dict()
    if not include_image_b64:
        payload["image_b64"] = None
    elif result.image_path and Path(result.image_path).is_file():
        raw = Path(result.image_path).read_bytes()
        payload["image_b64"] = base64.b64encode(raw).decode("ascii")
    return json.dumps(payload, ensure_ascii=False, indent=2)


def ok_json(data: Any = None, *, text: str | None = None, **extras: Any) -> str:
    return FeatureResult.success(text=text, data=data, **extras).to_json()


def err_json(message: str, *, code: str = "error") -> str:
    return FeatureResult.failure(message, code=code).to_json()


def merge_results(*parts: FeatureResult | dict[str, Any], text: str | None = None) -> str:
    """Combine multiple FeatureResults into one JSON payload for workflows."""
    images: list[str] = []
    data: dict[str, Any] = {}
    texts: list[str] = []
    if text:
        texts.append(text)
    for i, p in enumerate(parts):
        if isinstance(p, FeatureResult):
            if p.error:
                return result_to_json(p)
            if p.text:
                texts.append(p.text)
            if p.image_path:
                images.append(str(p.image_path))
            if p.data is not None:
                data[f"part_{i}"] = _serialize(p.data)
        else:
            data[f"part_{i}"] = _serialize(p)
    fr = FeatureResult.success(
        text="\n".join(texts) if texts else None,
        data={**data, "image_paths": images},
        image_path=Path(images[0]) if images else None,
    )
    return result_to_json(fr)
