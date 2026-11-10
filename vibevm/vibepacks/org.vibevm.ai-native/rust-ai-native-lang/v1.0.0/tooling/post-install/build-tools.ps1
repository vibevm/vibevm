# The discipline tools, built in the slot the install just materialised.
# See build-tools.sh for why this is a declared contribution (B-188).
$ErrorActionPreference = 'Stop'
$vibe = if ($env:VIBE_EXECUTABLE) { $env:VIBE_EXECUTABLE } else { 'vibe' }
& $vibe bin build
exit $LASTEXITCODE
