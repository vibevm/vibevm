$ErrorActionPreference = 'Stop'

$ProviderRoot = $env:VIBE_PACKAGE_DIR
if ($ProviderRoot.StartsWith('//?/') -or $ProviderRoot.StartsWith('\\?\')) {
    $ProviderRoot = $ProviderRoot.Substring(4)
}
$CmdPath = Join-Path $ProviderRoot 'scripts\test.cmd'
$env:VIBE_FIXTURE_PRESET_CMD = $CmdPath
try {
    $Command = '""!VIBE_FIXTURE_PRESET_CMD!""'
    $Process = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\cmd.exe') -ArgumentList @('/d', '/s', '/v:on', '/c', $Command) -NoNewWindow -Wait -PassThru
    $ExitCode = $Process.ExitCode
} finally {
    Remove-Item Env:VIBE_FIXTURE_PRESET_CMD -ErrorAction SilentlyContinue
}
if ($ExitCode -ne 0) { exit $ExitCode }
