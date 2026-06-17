# vibevm first-run — bootstrap the very first installation from a source
# checkout. Builds the current tree, installs it as your first VVM version,
# and puts `vibe` on PATH so a new shell can run it. See README.md
# "First run" and spec/common/PROP-019-version-manager.md.
#
# What it does, in order:
#   1. vibe man install         — build this checkout, publish it as
#                                 instance 1, make it the active version.
#   2. vibe man doctor --fix    — write the shims into ~/opt/bin and put
#                                 ~/opt/bin on PATH (durable; new shells).
#   3. vibe man ls              — show what is installed.
#
# This edits your durable user PATH. To try VVM WITHOUT touching ~/opt or
# PATH, skip this script and run:
#   $env:VIBEVM_INSTALL_ROOT = (New-Item -ItemType Directory `
#       (Join-Path $env:TEMP ([guid]::NewGuid()))).FullName
#   cargo run -p vibe-cli -- man install
$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
Set-Location $repoRoot

if (-not (Test-Path 'Cargo.toml') -or -not (Test-Path 'crates/vibe-cli')) {
    Write-Error 'first-run: run this from a vibevm source tree (Cargo.toml + crates/vibe-cli not found)'
    exit 1
}

function Invoke-Vibe {
    Write-Host "==> vibe $($args -join ' ')"
    & cargo run -q -p vibe-cli -- @args
    if ($LASTEXITCODE -ne 0) { throw "first-run: command failed: vibe $($args -join ' ')" }
}

Write-Host 'first-run: building this checkout and installing it as your first version...'
Invoke-Vibe man install

Write-Host ''
Write-Host 'first-run: writing shims and putting ~/opt/bin on PATH...'
Invoke-Vibe man doctor --fix --yes

Write-Host ''
Invoke-Vibe man ls

Write-Host ''
Write-Host 'first-run: done. Open a NEW terminal, then:'
Write-Host ''
Write-Host '    vibe man ls'
Write-Host ''
Write-Host 'From now on the loop is fast: `vibe man install` rebuilds, flips the'
Write-Host 'active version, and the next `vibe` in the same shell uses it -- no reload.'
