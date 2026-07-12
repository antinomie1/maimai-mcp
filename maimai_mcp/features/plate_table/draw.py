"""Plate table / progress drawing — original classes, coords untouched."""

from __future__ import annotations

import time
from pathlib import Path

from ...core.image.plate_table import DrawPlateProgress, DrawPlateTable
from ...core.io_image import default_out_path
from ...core.merge.models import PlayedResult, ServiceName
from ...result import FeatureResult, image_result


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
    t0 = time.perf_counter()
    table = DrawPlateTable(
        service,
        play_result,
        plan=plan,
        version=version,
        version_name=version_name,
        page=page,
    )
    return image_result(
        table.draw(),
        path,
        text=f"{version}{plan}完成表",
        t0=t0,
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
    t0 = time.perf_counter()
    table = DrawPlateProgress(
        service,
        play_result,
        plan=plan,
        version=version,
        version_name=version_name,
        page=page,
    )
    return image_result(
        table.draw(),
        path,
        text=f"{version}{plan}进度",
        t0=t0,
    )
