# Launch MCP Inspector against maimai_mcp (stdio).
# Requires: Node.js + npx, Python 3.10+, maimai_mcp/.env with STATIC_PATH
#
# Usage (from repo root):
#   .\scripts\run_inspector.ps1
#   .\scripts\run_inspector.ps1 -Python "py -3.11"

param(
    [string]$Python = "python",
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"

Write-Host "Repo: $RepoRoot"
$env:PYTHONPATH = $RepoRoot

$envFile = Join-Path $RepoRoot "maimai_mcp\.env"
if (-not (Test-Path $envFile)) {
    Write-Warning "Missing $envFile — copy maimai_mcp\.env.example and set STATIC_PATH"
}

# Prefer npx inspector; same command as MCP clients use
$serverCmd = $Python
$serverArgs = @("-m", "maimai_mcp")

Write-Host "Starting Inspector:"
Write-Host "  $serverCmd $($serverArgs -join ' ')"
Write-Host "  PYTHONPATH=$env:PYTHONPATH"
Write-Host ""

# Pass PYTHONPATH into the server subprocess (Inspector sanitizes env by default)
npx --yes "@modelcontextprotocol/inspector" `
  -e "PYTHONPATH=$RepoRoot" `
  -- $serverCmd @serverArgs
