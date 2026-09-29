$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($env:VIBE_PROJECT_ROOT)) {
    throw 'vibe run did not supply VIBE_PROJECT_ROOT'
}
if ([string]::IsNullOrWhiteSpace($env:VIBE_EXECUTABLE)) {
    throw 'vibe run did not supply VIBE_EXECUTABLE'
}

if ($args -notcontains '--no-build') {
    $packageRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
    & node (Join-Path $packageRoot 'tooling\site-build\build.mjs')
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

if ($null -eq $packageRoot) {
    $packageRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
}
& node (Join-Path $packageRoot 'tooling\local-site\run.mjs') --no-build --no-install @args
$childExit = $LASTEXITCODE
if ($null -eq $childExit) { $childExit = 1 }
exit $childExit
