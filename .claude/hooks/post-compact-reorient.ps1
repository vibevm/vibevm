# post-compact-reorient.ps1 — SessionStart hook (compact/resume).
# Stdout is injected into the resumed Claude context. ASCII literals only:
# Windows PowerShell 5.1 otherwise decodes a BOM-less script as ANSI.
$ErrorActionPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$repo = $env:CLAUDE_PROJECT_DIR
if (-not $repo) { $repo = 'C:\Users\olegc\git\v\vibevm' }
$repoSlash = ($repo -replace '\\', '/').TrimEnd('/')
$selector = (git -C $repo symbolic-ref -q HEAD)
if (-not $selector) { $selector = (git -C $repo rev-parse HEAD) }
$steward = Join-Path ([Environment]::GetFolderPath('UserProfile')) '.vibe\steward'
$context = $null
$contextRoot = Join-Path $steward 'contexts'
if (Test-Path -LiteralPath $contextRoot) {
  foreach ($candidate in Get-ChildItem -LiteralPath $contextRoot -Directory) {
    $binding = Join-Path $candidate.FullName 'binding.toml'
    if (-not (Test-Path -LiteralPath $binding)) { continue }
    $bindingText = Get-Content -Raw -Encoding UTF8 -LiteralPath $binding
    $rootNeedle = 'worktree_root = "' + $repoSlash + '"'
    $selectorNeedle = 'revision_selector = "' + $selector + '"'
    if ($bindingText.Contains($rootNeedle) -and $bindingText.Contains($selectorNeedle)) {
      if ($context) {
        Write-Output 'STEWARD ERROR: duplicate exact context bindings; stop before central mutation.'
        exit 0
      }
      $context = $candidate.FullName
    }
  }
}

Write-Output '=== POST-COMPACT / RESUME REORIENTATION ==='
Write-Output 'Files and empirical commands outrank the compaction summary.'
Write-Output 'Re-read AGENTS/CLAUDE, generated STATIC + INDEX entries, relevant specs,'
Write-Output 'then the complete user-local stewardship plan before continuing.'
Write-Output ''
if ($context) {
  Write-Output ('--- stewardship context: ' + $context + ' ---')
  $settings = Join-Path $context 'settings.toml'
  $custody = Join-Path $context 'custody.toml'
  $plan = Join-Path $context 'plan.toml'
  if (Test-Path -LiteralPath $settings) {
    Get-Content -Encoding UTF8 -LiteralPath $settings |
      Where-Object { $_ -match '^(interaction_mode|planning_profile|updated_at)\s*=' }
  }
  if (Test-Path -LiteralPath $custody) {
    Get-Content -Encoding UTF8 -LiteralPath $custody |
      Where-Object { $_ -match '^(epoch|state|holder_id|session_id|heartbeat_at|offer_id)\s*=' }
  }
  if (Test-Path -LiteralPath $plan) {
    Get-Content -Encoding UTF8 -LiteralPath $plan |
      Where-Object { $_ -match '^(plan_id|revision|current_node|updated_at)\s*=' } |
      Select-Object -First 4
    Write-Output ('READ THE WHOLE PLAN: ' + $plan)
  }
} else {
  Write-Output 'STEWARD WARNING: no exact local context binding; resolve/create one before central work.'
}
Write-Output '--- git status --short (first 60 lines) ---'
git -C $repo status --short | Select-Object -First 60
Write-Output '--- last 8 commits ---'
git -C $repo log --oneline -8
Write-Output '--- worktrees ---'
git -C $repo worktree list
Write-Output 'Before spawning or deleting anything, inspect live processes and retained artifacts.'
exit 0
