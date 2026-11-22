$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
& node (Join-Path $root 'tooling/local-site/run.mjs') @args
$childExit = $LASTEXITCODE
if ($null -eq $childExit) { $childExit = 1 }
exit $childExit
