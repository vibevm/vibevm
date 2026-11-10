# Register the discipline's MCP server with the agents on this machine.
# See register-mcp.sh for why this contribution is optional (B-188).
$ErrorActionPreference = 'Stop'
$vibe = if ($env:VIBE_EXECUTABLE) { $env:VIBE_EXECUTABLE } else { 'vibe' }
& $vibe mcp install
exit $LASTEXITCODE
