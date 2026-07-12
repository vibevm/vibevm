<#
  fractality — thin launcher (PowerShell), no global install.

  Runs the working-tree build of the CLI straight from the project tree,
  so there is nothing to install on PATH. State lives in the default
  global home (~/.fractality) — where a mission-control daemon and
  profiles.toml already live — unless FRACTALITY_HOME or the --home flag
  says otherwise.

  Usage:  .\fractality.ps1 mc status
          .\fractality.ps1 ps
          .\fractality.ps1 run --packet fractality\v0.1.0\spec\examples\hello-glm.toml
#>
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path

$bin = Join-Path $here 'fractality\v0.1.0\target\debug\fractality.exe'
if (-not (Test-Path $bin)) {
    Write-Error "fractality: binary not built at $bin — build it: cargo build -p fractality-cli (from fractality\v0.1.0)"
    exit 2
}

& $bin @args
exit $LASTEXITCODE
