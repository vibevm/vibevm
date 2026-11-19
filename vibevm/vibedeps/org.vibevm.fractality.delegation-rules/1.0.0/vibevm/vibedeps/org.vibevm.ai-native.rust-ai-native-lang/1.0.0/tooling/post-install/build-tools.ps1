# The discipline tools, built in the slot the install just materialised.
# See build-tools.sh for why this is a declared contribution (B-188).
$ErrorActionPreference = 'Stop'
$vibe = if ($env:VIBE_EXECUTABLE) { $env:VIBE_EXECUTABLE } else { 'vibe' }
# Run from the owning project root: the lifecycle cwd is the target slot, but
# `bin build` takes the owning project's lock.
if ($env:VIBE_PROJECT_ROOT) { Set-Location -LiteralPath $env:VIBE_PROJECT_ROOT }
# Pass --assume-yes: the install that materialised this slot already authorised
# dependency installation, so the narrated build contribution needs no second
# prompt (PROP-054 `#INSTALL-IS-CONSENT`).
& $vibe bin build --assume-yes
exit $LASTEXITCODE
