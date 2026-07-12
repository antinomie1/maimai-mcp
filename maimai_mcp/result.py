"""Unified feature result contract (replaces MessageSegment)."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


def _serialize(obj: Any) -> Any:
    """Best-effort JSON-friendly conversion for pydantic / dataclass / Path."""
    if obj is None or isinstance(obj, (str, int, float, bool)):
        return obj
    if isinstance(obj, Path):
        return str(obj)
    if isinstance(obj, dict):
        return {str(k): _serialize(v) for k, v in obj.items()}
    if isinstance(obj, (list, tuple)):
        return [_serialize(v) for v in obj]
    if hasattr(obj, "model_dump"):
        return obj.model_dump()
    if hasattr(obj, "dict") and callable(obj.dict):
        try:
            return obj.dict()
        except Exception:
            pass
    if hasattr(obj, "__dict__") and not isinstance(obj, type):
        return {
            k: _serialize(v)
            for k, v in vars(obj).items()
            if not k.startswith("_")
        }
    return str(obj)


@dataclass
class FeatureResult:
    """Standard return value for feature query/draw entrypoints.

    Contract:
      - ``ok`` is True iff ``error`` is None
      - ``code`` is a stable machine-readable error id when failed
      - ``data`` holds structured payload (JSON-serializable via ``to_dict``)
      - ``image_path`` is the preferred image output; ``image_b64`` is optional
    """

    text: str | None = None
    data: Any = None
    image_path: Path | None = None
    image_b64: str | None = None
    error: str | None = None
    code: str | None = None
    extras: dict[str, Any] = field(default_factory=dict)

    @property
    def ok(self) -> bool:
        return self.error is None

    @classmethod
    def success(
        cls,
        *,
        text: str | None = None,
        data: Any = None,
        image_path: Path | str | None = None,
        image_b64: str | None = None,
        **extras: Any,
    ) -> FeatureResult:
        path = Path(image_path) if image_path is not None else None
        return cls(
            text=text,
            data=data,
            image_path=path,
            image_b64=image_b64,
            extras=extras or {},
        )

    @classmethod
    def failure(
        cls,
        message: str,
        *,
        code: str = "error",
        data: Any = None,
        **extras: Any,
    ) -> FeatureResult:
        return cls(error=message, code=code, data=data, extras=extras or {})

    def raise_if_error(self) -> FeatureResult:
        if self.error:
            from .core.errors import MaimaiError

            raise MaimaiError(self.error, code=self.code or "error")
        return self

    def to_dict(self) -> dict[str, Any]:
        return {
            "ok": self.ok,
            "text": self.text,
            "data": _serialize(self.data),
            "image_path": str(self.image_path) if self.image_path else None,
            "image_b64": self.image_b64,
            "error": self.error,
            "code": self.code,
            "extras": _serialize(self.extras),
        }

    def to_json(self, *, indent: int | None = 2) -> str:
        return json.dumps(self.to_dict(), ensure_ascii=False, indent=indent)

    def print(self, fmt: str = "text") -> int:
        """Print to stdout. Returns process exit code (0 ok, 1 error)."""
        if not self.ok:
            print(f"[error] ({self.code or 'error'}) {self.error}")
            return 1
        if fmt == "json":
            print(self.to_json())
        else:
            if self.text:
                print(self.text)
            if self.image_path:
                print(f"[image] {self.image_path}")
            if fmt == "text" and not self.text and not self.image_path and self.data is not None:
                print(self.to_json())
        return 0
