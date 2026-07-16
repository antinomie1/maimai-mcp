"""Register all maimai MCP tools onto a FastMCP instance."""

from __future__ import annotations

from mcp.server.fastmcp import FastMCP


def register_all(mcp: FastMCP) -> None:
    from . import catalog, group_rank, player, records, tables, user, workflow

    catalog.register(mcp)
    player.register(mcp)
    group_rank.register(mcp)
    tables.register(mcp)
    user.register(mcp)
    records.register(mcp)
    workflow.register(mcp)
