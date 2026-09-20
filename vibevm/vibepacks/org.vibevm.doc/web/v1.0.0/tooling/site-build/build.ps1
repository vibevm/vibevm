$ErrorActionPreference = 'Stop'
$entry = Join-Path $env:VIBE_PROJECT_ROOT 'vibevm\vibepacks\org.vibevm.doc\web\v1.0.0\tooling\site-build\build.mjs'
$node = Get-Command node.exe -ErrorAction SilentlyContinue
if ($null -eq $node) {
    [Console]::Error.WriteLine("vibevm-doc build: node.exe was not found on lifecycle PATH: $env:PATH")
    exit 127
}
$env:VIBE_NODE_EXECUTABLE = $node.Source
$process = Start-Process -FilePath $node.Source -ArgumentList @("`"$entry`"") -NoNewWindow -Wait -PassThru
exit $process.ExitCode
