"""Image I/O helpers: decode base64 payloads from drawing code, write PNG files."""

from __future__ import annotations

import base64
import re
from io import BytesIO
from pathlib import Path

from PIL import Image

from ..config import maiconfig


def ensure_output_dir(path: Path | None = None) -> Path:
    out = Path(path) if path else Path(maiconfig.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    return out


def strip_base64_prefix(data: str) -> str:
    """Accept raw base64 or ``base64://...`` / data-URI forms used by original code."""
    if data.startswith("base64://"):
        return data[len("base64://") :]
    if data.startswith("data:"):
        return data.split(",", 1)[-1]
    return data


def b64_to_image(data: str) -> Image.Image:
    raw = base64.b64decode(strip_base64_prefix(data))
    return Image.open(BytesIO(raw)).convert("RGBA")


def save_image(
    data: str | Image.Image | bytes,
    path: Path | str,
) -> Path:
    """Save image payload to disk. Returns resolved path."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)

    if isinstance(data, Image.Image):
        data.save(path, format="PNG")
        return path.resolve()

    if isinstance(data, bytes):
        path.write_bytes(data)
        return path.resolve()

    # str: base64 from original drawing helpers
    img = b64_to_image(data)
    img.save(path, format="PNG")
    return path.resolve()


def default_out_path(name: str, out: Path | str | None = None) -> Path:
    safe = re.sub(r"[^\w.\-]+", "_", name)
    if not safe.lower().endswith(".png"):
        safe = f"{safe}.png"
    base = Path(out) if out else ensure_output_dir()
    if base.suffix.lower() == ".png":
        return base
    return ensure_output_dir(base) / safe
