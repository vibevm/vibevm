# precompact-snapshot.ps1 — PreCompact hook (auto/manual).
# PreCompact output cannot reach the model, so write a uniquely named LOCAL
# stewardship record. Never create a shared repository compact log.
$ErrorActionPreference = 'SilentlyContinue'
$repo = $env:CLAUDE_PROJECT_DIR
if (-not $repo) { $repo = 'C:\Users\olegc\git\v\vibevm' }
$repoSlash = ($repo -replace '\\', '/').TrimEnd('/')
$selector = (git -C $repo symbolic-ref -q HEAD)
if (-not $selector) { $selector = (git -C $repo rev-parse HEAD) }
$steward = Join-Path ([Environment]::GetFolderPath('UserProfile')) '.vibe\steward'
$recordDir = Join-Path $steward 'unbound-records'
$contextRoot = Join-Path $steward 'contexts'
if (Test-Path -LiteralPath $contextRoot) {
  foreach ($candidate in Get-ChildItem -LiteralPath $contextRoot -Directory) {
    $binding = Join-Path $candidate.FullName 'binding.toml'
    if (-not (Test-Path -LiteralPath $binding)) { continue }
    $bindingText = Get-Content -Raw -Encoding UTF8 -LiteralPath $binding
    if ($bindingText.Contains('worktree_root = "' + $repoSlash + '"') -and
        $bindingText.Contains('revision_selector = "' + $selector + '"')) {
      $recordDir = Join-Path $candidate.FullName 'records'
      break
    }
  }
}
New-Item -ItemType Directory -Force -Path $recordDir | Out-Null
$stamp = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ')
$nonce = [guid]::NewGuid().ToString('N').Substring(0, 12)
$record = Join-Path $recordDir ($stamp + '-' + $nonce + '-precompact.txt')
$head = git -C $repo log --oneline -1
$status = (git -C $repo status --short | Select-Object -First 80) -join "`n"
if (-not $status) { $status = '(working tree clean)' }
$body = "precompact_at=$stamp`nselector=$selector`nHEAD=$head`n$status`n"
[System.IO.File]::WriteAllText($record, $body, (New-Object System.Text.UTF8Encoding($false)))
exit 0
