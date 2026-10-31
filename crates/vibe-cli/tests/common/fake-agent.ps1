# A fake coding agent for the PROP-064 acceptance fixture, the PowerShell
# twin of fake-agent.sh: it records the arguments, the hook environment, the
# working directory, the prompt and the context under $env:FAKE_AGENT_RECORD,
# prints one transcript line, and answers as $env:FAKE_AGENT_STATUS
# (ok | fail | none) with exit status $env:FAKE_AGENT_EXIT. Start it as
# `pwsh -NoProfile -File fake-agent.ps1 …`.
$ErrorActionPreference = 'Stop'
$record = $env:FAKE_AGENT_RECORD
if (-not $record) { throw 'FAKE_AGENT_RECORD names the record directory' }
New-Item -ItemType Directory -Force -Path $record | Out-Null
Add-Content -Path (Join-Path $record 'calls') -Value 'call'
Set-Content -Path (Join-Path $record 'args') -Value $args
$names = 'VIBE_HOOK_STEP', 'VIBE_HOOK_RUN', 'VIBE_HOOK_PROMPT_FILE', 'VIBE_HOOK_CONTEXT_FILE',
  'VIBE_WORK', 'VIBE_SOURCES', 'VIBE_OUT', 'VIBE_PACKAGE', 'VIBE_APP_ROOT', 'FAKE_AGENT_NAME'
$lines = foreach ($name in $names) { "$name=$([Environment]::GetEnvironmentVariable($name))" }
Set-Content -Path (Join-Path $record 'env') -Value $lines
Set-Content -Path (Join-Path $record 'cwd') -Value (Get-Location).Path
Copy-Item -Path $env:VIBE_HOOK_PROMPT_FILE -Destination (Join-Path $record 'prompt.md')
Copy-Item -Path $env:VIBE_HOOK_CONTEXT_FILE -Destination (Join-Path $record 'context.json')
Write-Output "fake agent ran step $($env:VIBE_HOOK_STEP)"
$status = if ($env:FAKE_AGENT_STATUS) { $env:FAKE_AGENT_STATUS } else { 'ok' }
if ($status -ne 'none') {
  $answer = @{
    protocol = 'vibe-agent-hook-result/1'
    status = $status
    message = "fake agent answered $status"
    files_changed = @('a.txt')
  } | ConvertTo-Json -Compress
  Set-Content -Path (Join-Path $env:VIBE_HOOK_RUN 'result.json') -Value $answer
}
$code = if ($env:FAKE_AGENT_EXIT) { [int]$env:FAKE_AGENT_EXIT } else { 0 }
exit $code
