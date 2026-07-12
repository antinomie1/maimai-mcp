"""FastMCP server entry for maimai tools."""

from __future__ import annotations

from mcp.server.fastmcp import FastMCP

from .tools import register_all

mcp = FastMCP("maimai_mcp")
register_all(mcp)


def main() -> None:
    """Run stdio MCP server."""
    mcp.run(transport="stdio")


if __name__ == "__main__":
    main()
