# vibe:codex-subagents launcher epoch=1 owner=org.vibevm.world/codex-subagents
$ErrorActionPreference = 'Stop'
$WorkerArgs = @($args)
function Refuse([string]$Message) {
    [Console]::Error.WriteLine("codex-worker: $Message")
    exit 2
}
if ($WorkerArgs.Count -lt 1 -or $WorkerArgs[0] -notin @('login', 'status', 'interactive', 'exec')) {
    Refuse 'usage: codex-worker {login|status|interactive|exec} [Codex arguments]'
}
$mode = $WorkerArgs[0]
$forward = @()
if ($WorkerArgs.Count -gt 1) { $forward = $WorkerArgs[1..($WorkerArgs.Count - 1)] }
$workerHome = $env:CODEX_WORKER_HOME
$absolute = if ([IO.Path]::DirectorySeparatorChar -eq '\') {
    $workerHome -match '^(?:[A-Za-z]:[\\/]|[\\/]{2}[^\\/]+[\\/][^\\/]+)'
} else { $workerHome -like '/*' }
if ([string]::IsNullOrWhiteSpace($workerHome) -or -not $absolute) {
    Refuse 'CODEX_WORKER_HOME must name an explicit absolute isolated directory'
}
$requested = if ($env:CODEX_WORKER_BIN) { $env:CODEX_WORKER_BIN } else { 'codex' }
$command = Get-Command -Name $requested -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if ($null -eq $command) { Refuse 'Codex CLI executable was not found' }
$model = if ($env:CODEX_WORKER_MODEL) { $env:CODEX_WORKER_MODEL } else { 'gpt-6.1-sol' }
$effort = if ($env:CODEX_WORKER_EFFORT) { $env:CODEX_WORKER_EFFORT } else { 'medium' }
if ($effort -notin @('low', 'medium', 'high', 'xhigh', 'max')) { Refuse 'unsupported requested reasoning effort' }
$configNext = $false
$modelGiven = $false
$sandboxGiven = $false
$jsonGiven = $false
$colorGiven = $false
foreach ($argument in $forward) {
    if ($argument -eq '--') { break }
    if ($argument -in @('-m', '--model') -or $argument.StartsWith('--model=') -or
        ($argument.StartsWith('-m') -and $argument.Length -gt 2)) { $modelGiven = $true }
    if ($argument -in @('-s', '--sandbox') -or $argument.StartsWith('--sandbox=') -or
        ($argument.StartsWith('-s') -and $argument.Length -gt 2)) { $sandboxGiven = $true }
    if ($argument -eq '--json') { $jsonGiven = $true }
    if ($argument -eq '--color' -or $argument.StartsWith('--color=')) { $colorGiven = $true }
    $value = ''
    if ($configNext) { $value = $argument; $configNext = $false }
    elseif ($argument -in @('-c', '--config')) { $configNext = $true }
    elseif ($argument.StartsWith('--config=')) { $value = $argument.Substring(9) }
    elseif ($argument.StartsWith('-c') -and $argument.Length -gt 2) { $value = $argument.Substring(2) }
    elseif ($mode -eq 'exec' -and ($argument -in @('-p', '--profile') -or
            $argument.StartsWith('--profile=') -or ($argument.StartsWith('-p') -and $argument.Length -gt 2))) {
        Refuse 'quiet exec does not load configuration profiles'
    }
    $key = ($value.Split('=')[0] -replace '\s', '')
    $key = $key.Replace('"', '').Replace("'", '')
    if ($key -in @('cli_auth_credentials_store', 'forced_login_method')) {
        Refuse 'account-selection settings belong to this launcher'
    }
    if ($mode -eq 'exec' -and $key -eq 'project_doc_max_bytes') {
        Refuse 'quiet exec fixes project_doc_max_bytes=0'
    }
}
New-Item -ItemType Directory -Path $workerHome -Force | Out-Null
$workerHome = (Resolve-Path -LiteralPath $workerHome).ProviderPath
# Resolve-Path alone does not resolve junction targets. Refuse linked ancestry
# before Codex opens credentials rather than accidentally sharing primary auth.
$directory = [IO.DirectoryInfo]::new($workerHome)
while ($null -ne $directory) {
    if (($directory.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        Refuse 'worker state directory must not have linked/reparse ancestry'
    }
    $directory = $directory.Parent
}
$userHome = [Environment]::GetFolderPath('UserProfile')
$primary = Join-Path $userHome '.codex'
if (Test-Path -LiteralPath $primary -PathType Container) {
    $primary = (Resolve-Path -LiteralPath $primary).ProviderPath
    # The primary .codex can itself be a junction to an ordinary directory.
    # Resolve that chain too; never compare only the alias spelling.
    for ($hop = 0; $hop -lt 8; $hop++) {
        $primaryItem = Get-Item -LiteralPath $primary -Force
        if (($primaryItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -eq 0) { break }
        $targets = @($primaryItem.Target)
        if ($targets.Count -ne 1 -or [string]::IsNullOrWhiteSpace($targets[0])) {
            Refuse 'primary state alias target cannot be resolved safely'
        }
        $target = $targets[0]
        if (-not [IO.Path]::IsPathRooted($target)) { $target = Join-Path $primaryItem.Parent.FullName $target }
        $primary = (Resolve-Path -LiteralPath $target).ProviderPath
    }
    $directory = [IO.DirectoryInfo]::new($primary)
    while ($null -ne $directory) {
        if (($directory.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            Refuse 'primary state has unresolved linked/reparse ancestry'
        }
        $directory = $directory.Parent
    }
    if ($workerHome.Equals($primary, [StringComparison]::OrdinalIgnoreCase)) {
        Refuse 'worker state must differ from the default Codex home'
    }
}
$childArgs = @('--no-daemon', '-c', 'cli_auth_credentials_store="file"', '-c', 'forced_login_method="chatgpt"')
switch ($mode) {
    login { $childArgs += 'login' }
    status { $childArgs += @('login', 'status') }
    interactive {
        if (-not $modelGiven) { $childArgs += @('--model', $model) }
        $childArgs += @('-c', "model_reasoning_effort=`"$effort`"")
    }
    exec { $childArgs += @('exec', '--strict-config', '--ignore-user-config', '-c', 'project_doc_max_bytes=0',
        '-c', "model_reasoning_effort=`"$effort`"", '-c', 'approval_policy="never"')
        if (-not $modelGiven) { $childArgs += @('--model', $model) }
        if (-not $sandboxGiven) { $childArgs += @('--sandbox', 'workspace-write') }
        if (-not $jsonGiven) { $childArgs += '--json' }
        if (-not $colorGiven) { $childArgs += @('--color', 'never') }
    }
}
$previous = @{}
foreach ($entry in [Environment]::GetEnvironmentVariables('Process').GetEnumerator()) {
    if (($entry.Key -like 'CODEX_*' -or $entry.Key -like 'OPENAI_*') -and $entry.Key -ne 'CODEX_CA_CERTIFICATE') {
        $previous[$entry.Key] = $entry.Value
    }
}
$childExit = 1
try {
    foreach ($name in $previous.Keys) { [Environment]::SetEnvironmentVariable($name, $null, 'Process') }
    [Environment]::SetEnvironmentVariable('CODEX_HOME', $workerHome, 'Process')
    & $command.Source @childArgs @forward
    $childExit = $LASTEXITCODE
} finally {
    [Environment]::SetEnvironmentVariable('CODEX_HOME', $null, 'Process')
    foreach ($name in $previous.Keys) { [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process') }
}
exit $childExit
