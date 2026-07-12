"""Plate table / progress drawing — original classes, coords untouched."""

from __future__ import annotations

from pathlib import Path

from ...core.image.plate_table import DrawPlateProgress, DrawPlateTable
from ...core.io_image import default_out_path, save_image
from ...core.merge.models import PlayedResult, ServiceName
from ...result import FeatureResult


def draw_plate_table(
    service: ServiceName,
    play_result: list[PlayedResult],
    *,
    plan: str,
    version: str,
    version_name: str,
    page: int = 1,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"plate_{version}{plan}_p{page}", out)
    table = DrawPlateTable(
        service,
        play_result,
        plan=plan,
        version=version,
        version_name=version_name,
        page=page,
    )
    return FeatureResult(
        text=f"{version}{plan}完成表",
        image_path=save_image(table.draw(), path),
    )


def draw_plate_progress(
    service: ServiceName,
    play_result: list[PlayedResult],
    *,
    plan: str,
    version: str,
    version_name: str,
    page: int = 1,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"plate_prog_{version}{plan}_p{page}", out)
    table = DrawPlateProgress(
        service,
        play_result,
        plan=plan,
        version=version,
        version_name=version_name,
        page=page,
    )
    return FeatureResult(
        text=f"{version}{plan}进度",
        image_path=save_image(table.draw(), path),
    )
