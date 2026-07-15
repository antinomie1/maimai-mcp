#!/usr/bin/env bash
# Launch MCP Inspector against maimai_mcp (stdio).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PYTHONPATH="$ROOT"
PYTHON="${PYTHON:-python3}"

if [[ ! -f "$ROOT/maimai_mcp/.env" ]]; then
  echo "WARN: missing maimai_mcp/.env (copy .env.example, set STATIC_PATH)" >&2
fi

echo "Repo: $ROOT"
echo "Starting: $PYTHON -m maimai_mcp"
exec npx --yes "@modelcontextprotocol/inspector" "$PYTHON" -m maimai_mcp
