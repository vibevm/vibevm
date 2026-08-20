# precompact-snapshot.ps1 — PreCompact hook (matchers: auto, manual).
# PreCompact output cannot reach the model, so this writes a durable
# breadcrumb to disk instead: when compaction happened and what the working
# tree looked like at that exact moment. The post-compact agent (and the
# owner, tuning the 90% threshold) reads the log; the release plan's
# prediction P5 is checked against it. ASCII only (PS 5.1 + BOM-less files).
$ErrorActionPreference = 'SilentlyContinue'
$repo = $env:CLAUDE_PROJECT_DIR
if (-not $repo) { $repo = 'C:\Users\olegc\git\v\vibevm' }
$log = Join-Path $repo '.claude\compact-log.txt'
$stamp = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
$head = git -C $repo log --oneline -1
$status = (git -C $repo status --short | Select-Object -First 25) -join "`n"
if (-not $status) { $status = '(working tree clean)' }
Add-Content -Path $log -Value "=== compaction at $stamp ===`nHEAD: $head`n$status`n"
exit 0
