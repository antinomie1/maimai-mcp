"""Session-level state for the MCP server process."""

from __future__ import annotations

from dataclasses import dataclass, field

from maimai_mcp.config import maiconfig


@dataclass
class SessionState:
    """In-process defaults so tools can omit repeated identity args."""

    default_qq: int | None = field(default_factory=lambda: maiconfig.default_qq)
    default_username: str | None = field(
        default_factory=lambda: maiconfig.default_username
    )
    bootstrapped: bool = False
    last_song_ids: list[int] = field(default_factory=list)
    last_player_label: str | None = None

    def set_identity(
        self, *, qq: int | None = None, username: str | None = None
    ) -> None:
        if qq is not None:
            self.default_qq = qq
        if username is not None:
            self.default_username = username.strip() or None

    def snapshot(self) -> dict:
        return {
            "default_qq": self.default_qq,
            "default_username": self.default_username,
            "bootstrapped": self.bootstrapped,
            "last_song_ids": list(self.last_song_ids),
            "last_player_label": self.last_player_label,
        }


# One session per stdio process
session = SessionState()
