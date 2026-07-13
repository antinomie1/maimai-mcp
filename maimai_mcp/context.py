"""Session-level state for the MCP server process."""

from __future__ import annotations

from dataclasses import dataclass, field

from maimai_mcp.config import maiconfig
from maimai_mcp.core.errors import ValidationError


GROUP_AS_QQ_MSG = (
    "看起来传入的是群号而不是玩家 QQ。"
    "请传发送者 QQ（或被查对象 QQ），群号请放到 group_id / maimai_set_identity.group_id。"
)


@dataclass
class SessionState:
    """In-process defaults so tools can omit repeated identity args.

    ``default_qq`` = player QQ only. ``group_id`` is never used for score queries.
    """

    default_qq: int | None = field(default_factory=lambda: maiconfig.default_qq)
    default_username: str | None = field(
        default_factory=lambda: maiconfig.default_username
    )
    group_id: int | None = None
    bootstrapped: bool = False
    last_song_ids: list[int] = field(default_factory=list)
    last_player_label: str | None = None

    def assert_player_qq(self, qq: int | None, *, also_group: int | None = None) -> None:
        """Reject when *qq* equals a known group id (common Agent mistake)."""
        if qq is None:
            return
        for gid in (also_group, self.group_id):
            if gid is not None and qq == gid:
                raise ValidationError(GROUP_AS_QQ_MSG)

    def set_identity(
        self,
        *,
        qq: int | None = None,
        username: str | None = None,
        group_id: int | None = None,
    ) -> None:
        if group_id is not None:
            self.group_id = group_id
        self.assert_player_qq(qq, also_group=group_id)
        if qq is not None:
            self.default_qq = qq
        if username is not None:
            self.default_username = username.strip() or None

    def snapshot(self) -> dict:
        return {
            "default_qq": self.default_qq,
            "default_username": self.default_username,
            "group_id": self.group_id,
            "bootstrapped": self.bootstrapped,
            "last_song_ids": list(self.last_song_ids),
            "last_player_label": self.last_player_label,
        }


# One session per stdio process
session = SessionState()
