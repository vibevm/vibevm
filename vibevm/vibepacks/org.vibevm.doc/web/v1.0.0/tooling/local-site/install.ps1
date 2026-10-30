# source_build recipe for Windows
# (spec://org.vibevm.core/vibevm/common/PROP-059#source-build): copy the
# package tree from $env:VIBE_PACKAGE into $env:VIBE_OUT\site without build
# output, then write bin\vibevm-doc.cmd. VIBE_OUT is a staging path, so the
# wrapper finds the site relative to itself (%~dp0); no absolute path is baked in.
$ErrorActionPreference = 'Stop'
$omit = @('.vite', 'dist', 'dist-embedded', 'node_modules', 'playwright-report',
  'server', 'server-embedded', 'target', 'test-results', 'tmp')
function Copy-Tree([string]$From, [string]$To) {
  New-Item -ItemType Directory -Force -Path $To | Out-Null
  foreach ($item in Get-ChildItem -LiteralPath $From -Force) {
    if ($omit -contains $item.Name) { continue }
    $target = Join-Path $To $item.Name
    if ($item.PSIsContainer) { Copy-Tree $item.FullName $target }
    else { Copy-Item -LiteralPath $item.FullName -Destination $target }
  }
}
Copy-Tree $env:VIBE_PACKAGE (Join-Path $env:VIBE_OUT 'site')
$bin = Join-Path $env:VIBE_OUT 'bin'
New-Item -ItemType Directory -Force -Path $bin | Out-Null
$wrapper = "@echo off`r`nnode `"%~dp0..\site\tooling\local-site\run.mjs`" %*`r`n"
Set-Content -LiteralPath (Join-Path $bin 'vibevm-doc.cmd') -Value $wrapper -Encoding ASCII -NoNewline
