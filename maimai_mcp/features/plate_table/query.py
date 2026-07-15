"""Plate completion / progress query."""

from __future__ import annotations

from ...constants import PLATE_CN, VERSION_MAP
from ...core.database.qq import User
from ...core.domain import get_player_result_ref
from ...core.errors import ValidationError, handle_errors
from ...core.merge.models import PlayedResult
from ...core.user import resolve_player


@handle_errors
async def query_plate(
    version: str,
    plan: str,
    qq: int | None = None,
    *,
    username: str | None = None,
    source: str | None = None,
) -> tuple[User, list[PlayedResult], str, str, str]:
    ver = PLATE_CN.get(version, version)
    if f"{ver}{plan}" == "真将":
        raise ValidationError("真系没有真将哦。")
    mapping = VERSION_MAP.get(ver)
    if not mapping:
        raise ValidationError(f"无法识别版本「{version}」")
    _version, version_name = mapping
    ref = await resolve_player(
        qq,
        username,
        require_lxns_auth=not bool((username or "").strip()),
        source=source,
    )
    assert ref is not None
    play_result = await get_player_result_ref(ref, _version)
    return ref.user, play_result, ver, version_name, plan
