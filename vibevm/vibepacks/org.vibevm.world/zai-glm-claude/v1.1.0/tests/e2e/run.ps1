[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)] [string] $VibeExe,
    [Parameter(Mandatory = $true)] [string] $RegistryRoot,
    [string] $EvidenceRoot,
    [ValidateRange(1, 86400)] [int] $LiveTimeoutSeconds = 300,
    [switch] $Live
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
function Assert-True {
    param([bool] $Condition, [string] $Message)
    if (-not $Condition) { throw $Message }
}
function Resolve-RequiredPath {
    param([string] $Value, [string] $Label, [bool] $File)
    Assert-True ([System.IO.Path]::IsPathFullyQualified($Value)) "$Label must be absolute."
    $full = [System.IO.Path]::GetFullPath($Value)
    if ($File) {
        Assert-True ([System.IO.File]::Exists($full)) "$Label is not a file."
    }
    else {
        Assert-True ([System.IO.Directory]::Exists($full)) "$Label is not a directory."
    }
    return $full
}
function Write-Utf8 {
    param([string] $Path, [string] $Text)
    [System.IO.File]::WriteAllText($Path, $Text, $script:utf8NoBom)
}
function Invoke-VibeStep {
    param([string] $Name, [string[]] $Arguments)
    $logPath = Join-Path $script:caseRoot "$Name.log"
    & $script:vibeResolved @Arguments *> $logPath
    $stepExit = $LASTEXITCODE
    if ($stepExit -ne 0) {
        throw "Vibe step '$Name' failed with exit $stepExit; see $logPath"
    }
    return $logPath
}
function Get-BootSources {
    param([string] $ProjectRoot, [string] $PackageSlot)
    $bootRoot = Join-Path $ProjectRoot 'vibevm/vibespecs/boot'
    $indexPath = Join-Path $bootRoot 'INDEX.md'
    Assert-True ([System.IO.File]::Exists($indexPath)) 'Generated boot INDEX.md is missing.'
    $staticPaths = @(
        @((Join-Path $bootRoot 'STATIC.xml'), (Join-Path $bootRoot 'STATIC.md')) |
            Where-Object { [System.IO.File]::Exists($_) }
    )
    Assert-True ($staticPaths.Count -le 1) 'Expected at most one generated STATIC.xml or STATIC.md.'
    $sources = [System.Collections.Generic.List[string]]::new()
    $seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    if ($staticPaths.Count -eq 1) {
        $staticFull = [System.IO.Path]::GetFullPath($staticPaths[0])
        [void] $sources.Add($staticFull); [void] $seen.Add($staticFull)
    }
    $indexText = [System.IO.File]::ReadAllText($indexPath)
    $matches = [regex]::Matches($indexText, '(?m)^\s*(?:path|source|file)\s*=\s*"([^"]+)"\s*$')
    foreach ($match in $matches) {
        $named = $match.Groups[1].Value.Replace('/', [System.IO.Path]::DirectorySeparatorChar)
        $candidates = if ([System.IO.Path]::IsPathFullyQualified($named)) {
            @($named)
        }
        else {
            @(
                (Join-Path $ProjectRoot $named),
                (Join-Path $bootRoot $named),
                (Join-Path $PackageSlot $named)
            )
        }
        $resolved = $null
        foreach ($candidate in $candidates) {
            if ([System.IO.File]::Exists($candidate)) {
                $resolved = [System.IO.Path]::GetFullPath($candidate)
                break
            }
        }
        Assert-True ($null -ne $resolved) "INDEX.md names an unresolved boot source: $named"
        if ($seen.Add($resolved)) { [void] $sources.Add($resolved) }
    }
    return $sources.ToArray()
}
function Assert-BootMarkers {
    param([string[]] $Sources)
    $markers = @('ZAI-GLM-CLAUDE-PARENT-WORKFLOW', 'CODEX-PARENT', 'CLAUDE-PARENT', 'SAME-CWD-CONTINUATION', 'NO-SECRETS-TO-WORKERS')
    foreach ($marker in $markers) {
        $count = 0
        foreach ($source in $Sources) {
            $text = [System.IO.File]::ReadAllText($source)
            $opening = [regex]::Matches($text,
                '<\s*(?:[A-Za-z0-9_.%]+--)?' + [regex]::Escape($marker) + '(?=[\s>])').Count
            if ($opening -gt 0) {
                $count += $opening
            }
            else {
                $count += [regex]::Matches($text, '(?<![A-Za-z0-9_-])' +
                    [regex]::Escape($marker) + '(?![A-Za-z0-9_-])').Count
            }
        }
        Assert-True ($count -eq 1) "Boot marker $marker occurs $count times; expected exactly once."
    }
}
function Assert-PackageArtifacts {
    param([string] $Slot)
    $packageRoot = Join-Path $Slot 'target/vibe-package'
    $items = @(
        [pscustomobject]@{ Name = 'claudez.ps1'; Target = 'claudez-powershell' },
        [pscustomobject]@{ Name = 'claudez.cmd'; Target = 'claudez-command' },
        [pscustomobject]@{ Name = 'claudez'; Target = 'claudez-posix' }
    )
    $actualOutputs = @([IO.Directory]::EnumerateFiles($packageRoot, '*',
        [IO.SearchOption]::AllDirectories) | ForEach-Object { [IO.Path]::GetFullPath($_) })
    Assert-True ($actualOutputs.Count -eq 3) 'Static-file packaging did not produce exactly three files.'
    foreach ($item in $items) {
        $output = [IO.Path]::GetFullPath((Join-Path $packageRoot "$($item.Target)/$($item.Name)"))
        Assert-True ($actualOutputs -ccontains $output) "Missing exact package output $($item.Name)."
        $source = Join-Path (Join-Path $Slot 'launchers') $item.Name
        $sourceBytes = [IO.File]::ReadAllBytes($source)
        $outputBytes = [IO.File]::ReadAllBytes($output)
        Assert-True ($sourceBytes.Length -eq $outputBytes.Length) 'Packaged launcher byte length mismatch.'
        for ($byteIndex = 0; $byteIndex -lt $sourceBytes.Length; $byteIndex++) {
            Assert-True ($sourceBytes[$byteIndex] -eq $outputBytes[$byteIndex]) `
                "Packaged launcher byte mismatch at offset $byteIndex."
        }
        $recordPath = Join-Path $Slot ".vibe/state/artifacts/$($item.Name).json"
        Assert-True ([IO.File]::Exists($recordPath)) "Missing artifact record $($item.Name).json."
        $record = [IO.File]::ReadAllText($recordPath) | ConvertFrom-Json -ErrorAction Stop
        $relative = "target/vibe-package/$($item.Target)/$($item.Name)"
        $digest = (Get-FileHash -LiteralPath $output -Algorithm SHA256).Hash.ToLowerInvariant()
        Assert-True ([string] $record.id -ceq $item.Name) 'Artifact record id mismatch.'
        Assert-True ([string] $record.kind -ceq 'file' -and [string] $record.shape -ceq 'file') `
            'Artifact record kind/shape mismatch.'
        Assert-True ([string] $record.digest.algorithm -ceq 'sha256' -and
            [string] $record.digest.value -ceq $digest) 'Artifact record digest mismatch.'
        Assert-True ([IO.Path]::GetFullPath([string] $record.path_absolute) -ceq $output) `
            'Artifact record absolute path mismatch.'
        Assert-True ([string] $record.path_relative.root -ceq 'project' -and
            ([string] $record.path_relative.path).Replace('\', '/') -ceq $relative) `
            'Artifact record relative path mismatch.'
        Assert-True ([string] $record.producer.target -ceq $item.Target -and
            [string] $record.producer.mechanism -ceq 'package:static-file' -and
            [string] $record.producer.provider.key -ceq 'org.vibevm/vibe#static-file') `
            'Artifact record producer/provider mismatch.'
    }
}
function Assert-Deployments {
    param($Listing, [string] $SettingsRoot)
    $rows = @($Listing.deployments)
    Assert-True ([int] $Listing.count -eq 2 -and $rows.Count -eq 2) `
        'Deployments must contain exactly two rows.'
    $expected = @{
        'claudez-powershell' = 'opt/bin/claudez.ps1'
        'claudez-command' = 'opt/bin/claudez.cmd'
    }
    Assert-True ((@($rows.target | Sort-Object) -join ',') -ceq
        'claudez-command,claudez-powershell') 'Deployment targets differ from the Windows pair.'
    foreach ($row in $rows) {
        Assert-True ([string] $row.profile -ceq 'windows' -and
            [string] $row.provider -ceq 'org.vibevm/vibe#vibe-opt-launcher' -and
            [string] $row.status -ceq 'verified' -and [string] $row.scope -ceq 'user' -and
            [int] $row.resources -eq 1) 'Deployment row identity/status/resource count mismatch.'
    }
    $receiptRoot = Join-Path $SettingsRoot 'state/deployments'
    $receiptFiles = @([IO.Directory]::EnumerateFiles($receiptRoot, 'receipt.json',
        [IO.SearchOption]::AllDirectories))
    Assert-True ($receiptFiles.Count -eq 2) 'Expected exactly two deployment receipt files.'
    $seenTargets = [System.Collections.Generic.HashSet[string]]::new(
        [StringComparer]::Ordinal)
    foreach ($receiptFile in $receiptFiles) {
        $receipt = [IO.File]::ReadAllText($receiptFile) | ConvertFrom-Json -ErrorAction Stop
        $target = [string] $receipt.target
        Assert-True ($expected.ContainsKey($target) -and $seenTargets.Add($target)) `
            'Receipt target is unknown or duplicated.'
        $resources = @($receipt.resources)
        Assert-True ($resources.Count -eq 1) "Receipt $target must own exactly one resource."
        $resource = ([string] $resources[0].resource).Replace('\', '/')
        Assert-True ($resource -ceq $expected[$target]) "Receipt $target owns the wrong resource."
    }
    Assert-True ($seenTargets.Count -eq 2) 'Receipts do not cover both Windows targets.'
}
function Invoke-CapturedProcess {
    param([string] $FileName, [string] $Arguments, [string] $WorkingDirectory,
        [hashtable] $Environment, [int] $TimeoutSeconds = 30)
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $FileName
    $info.Arguments = $Arguments
    $info.WorkingDirectory = $WorkingDirectory
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    foreach ($key in $Environment.Keys) { $info.EnvironmentVariables[$key] = $Environment[$key] }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $info
    Assert-True ($process.Start()) "Could not start fake launcher process $FileName."
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        & (Join-Path $env:SystemRoot 'System32/taskkill.exe') /PID ([string] $process.Id) /T /F *> $null
        throw 'Fake launcher contract timed out.'
    }
    $process.WaitForExit()
    $result = [pscustomobject]@{ ExitCode = $process.ExitCode
        Stdout = $stdoutTask.GetAwaiter().GetResult()
        Stderr = $stderrTask.GetAwaiter().GetResult() }
    $process.Dispose()
    return $result
}
function Assert-ExactVector {
    param($Actual, [string[]] $Expected, [string] $Label)
    $values = @($Actual)
    Assert-True ($values.Count -eq $Expected.Count) "$Label argument count mismatch."
    for ($index = 0; $index -lt $Expected.Count; $index++) {
        Assert-True ([string] $values[$index] -ceq $Expected[$index]) `
            "$Label argument $index mismatch."
    }
}
function Assert-ModuleCacheIsolation {
    param([string] $CaseRoot, [string] $ProfileRoot)
    $probeRoot = Join-Path $CaseRoot 'module-cache-probe-worker'
    $cacheRoot = Join-Path $CaseRoot 'module-cache-probe-state'
    [void] [IO.Directory]::CreateDirectory($probeRoot)
    [void] [IO.Directory]::CreateDirectory($cacheRoot)
    $probeEnvironment = @{
        USERPROFILE = $ProfileRoot
        HOME = $ProfileRoot
        PSModuleAnalysisCachePath = (Join-Path $cacheRoot 'ModuleAnalysisCache')
    }
    $probe = Invoke-CapturedProcess 'powershell.exe' `
        '-NoLogo -NoProfile -NonInteractive -Command "Get-Module -ListAvailable | Out-Null"' `
        $probeRoot $probeEnvironment
    Assert-True ($probe.ExitCode -eq 0) 'PowerShell module-cache isolation probe failed.'
    Assert-True (@([IO.Directory]::EnumerateFileSystemEntries($probeRoot)).Count -eq 0) `
        'PowerShell module analysis cache escaped into the worker cwd.'
}
function Assert-FakeLauncherContract {
    param([string] $CaseRoot, [string] $PsLauncher, [string] $CmdLauncher)
    $sentinel = 'FAKE-ZAI-E2E-SENTINEL-7f8429'
    $tokenPath = Join-Path $CaseRoot 'fake-zai-token.txt'
    Write-Utf8 $tokenPath ($sentinel + "`n")
    $recorder = @'
param([Parameter(ValueFromRemainingArguments = $true)][AllowEmptyString()][string[]] $Forwarded)
$payload = [ordered]@{
    args = @($Forwarded)
    cwd = (Get-Location).ProviderPath
    token_matches = ($env:ANTHROPIC_AUTH_TOKEN -ceq $env:E2E_FAKE_TOKEN_SENTINEL)
}
$json = $payload | ConvertTo-Json -Depth 4
[IO.File]::WriteAllText($env:E2E_FAKE_CLAUDE_RECORD, $json + "`n", [Text.UTF8Encoding]::new($false))
exit 37
'@
    $psBin = Join-Path $CaseRoot 'fake-claude-ps'
    $psCwd = Join-Path $CaseRoot 'fake-ps-cwd'
    [void] [IO.Directory]::CreateDirectory($psBin)
    [void] [IO.Directory]::CreateDirectory($psCwd)
    Write-Utf8 (Join-Path $psBin 'claude.ps1') $recorder
    $psRecord = Join-Path $CaseRoot 'fake-ps-record.json'
    $psResult = Join-Path $CaseRoot 'fake-ps-result.json'
    $psDriver = Join-Path $CaseRoot 'fake-ps-driver.ps1'
    $driver = @'
$tracked = @('ANTHROPIC_AUTH_TOKEN', 'ANTHROPIC_BASE_URL', 'ANTHROPIC_MODEL',
    'ANTHROPIC_DEFAULT_OPUS_MODEL', 'ANTHROPIC_DEFAULT_SONNET_MODEL',
    'ANTHROPIC_DEFAULT_HAIKU_MODEL', 'CLAUDE_CONFIG_DIR', 'MAX_THINKING_TOKENS',
    'API_TIMEOUT_MS', 'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC')
$before = @{}
foreach ($key in $tracked) { $before[$key] = "BEFORE-$key"; [Environment]::SetEnvironmentVariable($key, $before[$key], 'Process') }
$root = $PSScriptRoot
$env:PATH = (Join-Path $root 'fake-claude-ps') + [IO.Path]::PathSeparator + $env:PATH
$env:ZAI_API_TOKEN_FILE = Join-Path $root 'fake-zai-token.txt'
$env:ZAI_GLM_CONFIG_DIR = Join-Path $root 'fake-ps-state'
$env:E2E_FAKE_TOKEN_SENTINEL = 'FAKE-ZAI-E2E-SENTINEL-7f8429'
$env:E2E_FAKE_CLAUDE_RECORD = Join-Path $root 'fake-ps-record.json'
$unicode = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('0K7QvdC40LrQvtC0'))
$vector = @('', 'space value', 'quote"value', 'trail\', $unicode, 'meta&|<>^()%!')
Set-Location -LiteralPath (Join-Path $root 'fake-ps-cwd')
$screen = & (Join-Path $root 'home/.vibe/opt/bin/claudez.ps1') @vector 2>&1
$childExit = $LASTEXITCODE
$restored = $true
foreach ($key in $tracked) { if ([Environment]::GetEnvironmentVariable($key, 'Process') -cne $before[$key]) { $restored = $false } }
$safe = [ordered]@{ child_exit = $childExit; environment_restored = $restored
    screen_chars = ($screen | Out-String).Length
    sentinel_leaked = ($screen | Out-String).Contains($env:E2E_FAKE_TOKEN_SENTINEL) }
[IO.File]::WriteAllText((Join-Path $root 'fake-ps-result.json'),
    (($safe | ConvertTo-Json) + "`n"), [Text.UTF8Encoding]::new($false))
'@
    Write-Utf8 $psDriver $driver
    $psRun = Invoke-CapturedProcess 'powershell.exe' `
        ('-NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + $psDriver + '"') `
        $CaseRoot @{}
    Assert-True ($psRun.ExitCode -eq 0) 'PowerShell launcher contract driver failed.'
    Assert-True (-not $psRun.Stdout.Contains($sentinel) -and -not $psRun.Stderr.Contains($sentinel)) `
        'PowerShell launcher leaked the fake sentinel.'
    $psSafe = [IO.File]::ReadAllText($psResult) | ConvertFrom-Json -ErrorAction Stop
    $psChild = [IO.File]::ReadAllText($psRecord) | ConvertFrom-Json -ErrorAction Stop
    Assert-True ([int] $psSafe.child_exit -eq 37 -and $psSafe.environment_restored -eq $true -and
        $psSafe.sentinel_leaked -eq $false) 'PowerShell launcher exit/restoration/leak contract failed.'
    Assert-True ($psChild.token_matches -eq $true -and
        [IO.Path]::GetFullPath([string] $psChild.cwd) -ceq [IO.Path]::GetFullPath($psCwd)) `
        'PowerShell launcher fake-token/cwd contract failed.'
    $psVector = @('', 'space value', 'quote"value', 'trail\', 'Юникод', 'meta&|<>^()%!')
    Assert-ExactVector $psChild.args $psVector 'PowerShell launcher'

    $cmdBin = Join-Path $CaseRoot 'fake-claude-cmd'
    $cmdCwd = Join-Path $CaseRoot 'fake-cmd-cwd'
    [void] [IO.Directory]::CreateDirectory($cmdBin)
    [void] [IO.Directory]::CreateDirectory($cmdCwd)
    Write-Utf8 (Join-Path $cmdBin 'fake-claude.ps1') $recorder
    Write-Utf8 (Join-Path $cmdBin 'claude.cmd') `
        ("@echo off`r`npowershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File `"%~dp0fake-claude.ps1`" %*`r`nexit /b %ERRORLEVEL%`r`n")
    $cmdRecord = Join-Path $CaseRoot 'fake-cmd-record.json'
    $cmdEnvironment = @{
        PATH = $cmdBin + [IO.Path]::PathSeparator + $env:PATH
        ZAI_API_TOKEN_FILE = $tokenPath
        ZAI_GLM_CONFIG_DIR = (Join-Path $CaseRoot 'fake-cmd-state')
        E2E_FAKE_TOKEN_SENTINEL = $sentinel
        E2E_FAKE_CLAUDE_RECORD = $cmdRecord
    }
    $cmdVector = @('alpha', 'beta-2', 'gamma_3')
    $cmdArguments = '/d /s /c ""' + $CmdLauncher + '" ' + ($cmdVector -join ' ') + '"'
    $cmdRun = Invoke-CapturedProcess 'cmd.exe' $cmdArguments $cmdCwd $cmdEnvironment
    Assert-True ($cmdRun.ExitCode -eq 37) 'CMD launcher did not preserve exit 37.'
    Assert-True (-not $cmdRun.Stdout.Contains($sentinel) -and -not $cmdRun.Stderr.Contains($sentinel)) `
        'CMD launcher leaked the fake sentinel.'
    $cmdChild = [IO.File]::ReadAllText($cmdRecord) | ConvertFrom-Json -ErrorAction Stop
    Assert-True ($cmdChild.token_matches -eq $true -and
        [IO.Path]::GetFullPath([string] $cmdChild.cwd) -ceq [IO.Path]::GetFullPath($cmdCwd)) `
        'CMD launcher fake-token/cwd contract failed.'
    Assert-ExactVector $cmdChild.args $cmdVector 'CMD launcher'
    Assert-True ([IO.File]::ReadAllText($tokenPath).Trim() -ceq $sentinel) `
        'Fake token source changed during launcher contract tests.'
}
function Read-ResultJson {
    param([string] $Text, [int] $ExitCode, [int] $Pass)
    try { $resultJson = $Text | ConvertFrom-Json -ErrorAction Stop }
    catch {
        $diagnostic = [ordered]@{ pass = $Pass; exit_code = $ExitCode
            category = 'invalid-json'; stdout_chars = $Text.Length } | ConvertTo-Json
        Write-Utf8 (Join-Path $script:caseRoot "live-pass-$Pass-diagnostic.json") `
            ($diagnostic + "`n")
        throw "Live GLM pass $Pass did not emit one valid JSON result; no retry."
    }
    [string[]] $modelKeys = @()
    if ($null -ne $resultJson.modelUsage) {
        $modelKeys = [string[]] @($resultJson.modelUsage.PSObject.Properties.Name)
    }
    $safeSummary = [ordered]@{ pass = $Pass; exit_code = $ExitCode
        session_id = [string] $resultJson.session_id; model_usage_keys = $modelKeys
    } | ConvertTo-Json -Depth 3
    Write-Utf8 (Join-Path $script:caseRoot "live-pass-$Pass-summary.json") ($safeSummary + "`n")
    Assert-True ($ExitCode -eq 0) "Live GLM pass $Pass ended with transport exit $ExitCode; no retry."
    Assert-True ($resultJson.type -eq 'result') "Live GLM pass $Pass has no terminal result event."
    Assert-True ($resultJson.subtype -eq 'success') "Live GLM pass $Pass was not terminal success."
    Assert-True ($resultJson.is_error -eq $false) "Live GLM pass $Pass reported an error."
    Assert-True (-not [string]::IsNullOrWhiteSpace([string] $resultJson.session_id)) `
        "Live GLM pass $Pass has no child-generated session id."
    Assert-True ($null -ne $resultJson.modelUsage) "Live GLM pass $Pass has no modelUsage evidence."
    Assert-True ($modelKeys.Count -gt 0) "Live GLM pass $Pass has empty modelUsage evidence."
    foreach ($modelKey in $modelKeys) {
        Assert-True ($modelKey -cmatch '^glm-') "Live GLM pass $Pass used a non-GLM model."
    }
    return [pscustomobject]@{
        TerminalText = ([string] $resultJson.result).Trim()
        ModelKeys = $modelKeys
    }
}
function Test-LiveArtifacts {
    param([string] $WorkerRoot, [string] $Nonce, [string[]] $ModelKeys)
    $names = @([IO.Directory]::EnumerateFileSystemEntries($WorkerRoot) |
        ForEach-Object { [IO.Path]::GetFileName($_) })
    $expectedNames = @('PACKET.md', 'proof.json', 'WORKER-REPORT-E2E.md')
    if ($names.Count -ne 3) { return $false }
    foreach ($expectedName in $expectedNames) {
        if ($names -cnotcontains $expectedName) { return $false }
    }
    $proofPath = Join-Path $WorkerRoot 'proof.json'
    $reportPath = Join-Path $WorkerRoot 'WORKER-REPORT-E2E.md'
    if (-not [System.IO.File]::Exists($proofPath) -or
        -not [System.IO.File]::Exists($reportPath)) { return $false }
    try { $proof = [System.IO.File]::ReadAllText($proofPath) | ConvertFrom-Json -ErrorAction Stop }
    catch { return $false }
    $proofNames = @($proof.PSObject.Properties.Name | Sort-Object)
    if (($proofNames -join ',') -cne 'nonce,reported_model,result') { return $false }
    if ([string] $proof.nonce -cne $Nonce -or
        [string] $proof.result -cne 'GLM_E2E_OK' -or
        [string]::IsNullOrWhiteSpace([string] $proof.reported_model) -or
        -not ($ModelKeys -ccontains [string] $proof.reported_model)) { return $false }
    $report = [System.IO.File]::ReadAllText($reportPath)
    return $report.Contains($Nonce) -and
        $report.Contains('GLM_E2E_OK') -and
        $report.Contains([string] $proof.reported_model)
}
function Invoke-LivePass {
    param([bool] $Continue, [int] $MaxTurns, [int] $Pass)
    $driverPath = Join-Path $script:caseRoot "live-pass-$Pass-driver.ps1"
    $prompt = if ($Continue) {
        'Continue in this exact cwd. Re-read PACKET.md and repair only the two required artifacts. No other reads or writes, shell, Git, or subagents. Return only TASK-DONE.'
    }
    else {
        'Read PACKET.md in this cwd first and follow it exactly. Write only its two required artifacts. No other reads, shell, Git, or subagents. Return only TASK-DONE.'
    }
    $promptBase64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($prompt))
    $continueLine = if ($Continue) { "`$launcherArgs += '-c'" } else { '' }
    $driver = @"
`$ErrorActionPreference = 'Stop'
`$prompt = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('$promptBase64'))
`$launcherArgs = @()
$continueLine
`$launcherArgs += @('-p', `$prompt, '--model', 'sonnet',
    '--permission-mode', 'bypassPermissions', '--tools', 'Read', 'Write',
    '--allowedTools', 'Read', 'Write', '--max-turns', '$MaxTurns',
    '--output-format', 'json')
`$launcher = Join-Path `$PSScriptRoot 'home/.vibe/opt/bin/claudez.ps1'
& `$launcher @launcherArgs
`$childExit = `$LASTEXITCODE
exit `$childExit
"@
    Write-Utf8 $driverPath $driver

    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = 'powershell.exe'
    $startInfo.Arguments = '-NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' +
        $driverPath + '"'
    $startInfo.WorkingDirectory = $script:workerRoot
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    Assert-True ($process.Start()) "Live GLM pass $Pass could not start."
    $childPid = $process.Id
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $completed = $process.WaitForExit($script:LiveTimeoutSeconds * 1000)
    if (-not $completed) {
        $taskkillPath = Join-Path $env:SystemRoot 'System32/taskkill.exe'
        $killOutput = & $taskkillPath /PID ([string] $childPid) /T /F 2>&1
        $killExit = $LASTEXITCODE
        [void] $process.WaitForExit(10000)
        $stdoutChars = 0
        $stderrChars = 0
        if ($process.HasExited) {
            $stdoutChars = $stdoutTask.GetAwaiter().GetResult().Length
            $stderrChars = $stderrTask.GetAwaiter().GetResult().Length
        }
        $timeoutSummary = [ordered]@{ pass = $Pass; pid = $childPid
            timeout_seconds = $script:LiveTimeoutSeconds; taskkill_exit = $killExit
            taskkill_output_chars = ($killOutput | Out-String).Length
            stopped = $process.HasExited; stdout_chars = $stdoutChars; stderr_chars = $stderrChars
        } | ConvertTo-Json
        Write-Utf8 (Join-Path $script:caseRoot "live-pass-$Pass-timeout.json") `
            ($timeoutSummary + "`n")
        $process.Dispose()
        throw 'CLAUDEZ_TRANSPORT_STALL'
    }
    $process.WaitForExit()
    $childExit = $process.ExitCode
    $stdoutText = $stdoutTask.GetAwaiter().GetResult()
    $stderrText = $stderrTask.GetAwaiter().GetResult()
    $processSummary = [ordered]@{ pass = $Pass; pid = $childPid
        timed_out = $false; exit_code = $childExit
        stdout_chars = $stdoutText.Length; stderr_chars = $stderrText.Length } | ConvertTo-Json
    Write-Utf8 (Join-Path $script:caseRoot "live-pass-$Pass-process.json") `
        ($processSummary + "`n")
    $process.Dispose()
    return [pscustomobject]@{ ExitCode = $childExit; Stdout = $stdoutText; Pid = $childPid }
}
$environmentKeys = @('VIBE_SETTINGS', 'USERPROFILE', 'HOME', 'ZAI_GLM_CONFIG_DIR',
    'ZAI_API_TOKEN_FILE', 'ZAI_GLM_MAX_THINKING', 'PSModuleAnalysisCachePath')
$savedEnvironment = @{}
foreach ($key in $environmentKeys) {
    $savedEnvironment[$key] = [System.Environment]::GetEnvironmentVariable($key, 'Process')
}
try {
    $vibeResolved = Resolve-RequiredPath $VibeExe 'VibeExe' $true
    $registryResolved = Resolve-RequiredPath $RegistryRoot 'RegistryRoot' $false
    if ([string]::IsNullOrWhiteSpace($EvidenceRoot)) {
        $evidenceBase = Join-Path ([System.IO.Path]::GetTempPath()) 'zai-glm-claude-e2e'
    }
    else {
        Assert-True ([System.IO.Path]::IsPathFullyQualified($EvidenceRoot)) `
            'EvidenceRoot must be absolute when supplied.'
        $evidenceBase = [System.IO.Path]::GetFullPath($EvidenceRoot)
    }
    [void] [System.IO.Directory]::CreateDirectory($evidenceBase)
    $caseName = 'case-{0}-{1}' -f [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ'), `
        [Guid]::NewGuid().ToString('N')
    $caseRoot = Join-Path $evidenceBase $caseName
    [void] [System.IO.Directory]::CreateDirectory($caseRoot)
    $projectRoot = Join-Path $caseRoot 'project'
    $isolatedProfileRoot = Join-Path $caseRoot 'home'
    $fakeSettings = Join-Path $isolatedProfileRoot '.vibe'
    [void] [System.IO.Directory]::CreateDirectory($fakeSettings)
    $operatorProfile = if (-not [string]::IsNullOrWhiteSpace($env:USERPROFILE)) {
        $env:USERPROFILE
    } else { $env:HOME }
    Assert-True (-not [string]::IsNullOrWhiteSpace($operatorProfile)) 'Cannot resolve operator profile.'
    $realSettings = [System.IO.Path]::GetFullPath((Join-Path $operatorProfile '.vibe'))
    Assert-True (-not [System.StringComparer]::OrdinalIgnoreCase.Equals($fakeSettings, $realSettings)) `
        'The isolated settings root unexpectedly equals the real settings root.'
    [System.Environment]::SetEnvironmentVariable('VIBE_SETTINGS', $fakeSettings, 'Process')
    $null = Invoke-VibeStep '01-init' @(
        'init', '--path', $projectRoot, '--name', 'zai-glm-e2e',
        '--no-registry', '--unattended'
    )
    $null = Invoke-VibeStep '02-install' @(
        'install', 'tool:org.vibevm.world/zai-glm-claude@1.0.0',
        '--path', $projectRoot, '--registry', $registryResolved,
        '--exact', '--offline', '--assume-yes'
    )
    $slot = Join-Path $projectRoot 'vibevm/vibedeps/org.vibevm.world.zai-glm-claude/1.0.0'
    Assert-True ([System.IO.Directory]::Exists($slot)) 'The exact resolved package slot is missing.'
    $bootSources = Get-BootSources $projectRoot $slot
    Assert-BootMarkers $bootSources
    $null = Invoke-VibeStep '03-deploy' @('deploy', '--path', $slot, '--profile', 'windows')
    Assert-PackageArtifacts $slot
    $deployedPs1 = Join-Path $fakeSettings 'opt/bin/claudez.ps1'
    $deployedCmd = Join-Path $fakeSettings 'opt/bin/claudez.cmd'
    foreach ($name in @('claudez.ps1', 'claudez.cmd')) {
        $packaged = Join-Path (Join-Path $slot 'launchers') $name
        $deployed = Join-Path (Join-Path $fakeSettings 'opt/bin') $name
        Assert-True ([System.IO.File]::Exists($packaged)) "Package launcher $name is missing."
        Assert-True ([System.IO.File]::Exists($deployed)) "Deployed launcher $name is missing."
        Assert-True ((Get-FileHash -LiteralPath $packaged -Algorithm SHA256).Hash -ceq
            (Get-FileHash -LiteralPath $deployed -Algorithm SHA256).Hash) `
            "Deployed launcher $name differs from the package bytes."
    }
    Assert-True (-not [System.IO.File]::Exists((Join-Path $fakeSettings 'zai.api.token'))) `
        'Deployment created a token file in isolated settings.'
    $listingPath = Invoke-VibeStep '04-deployments' @('deployments', '--json')
    $listingText = [System.IO.File]::ReadAllText($listingPath)
    $jsonStart = $listingText.IndexOf('{')
    Assert-True ($jsonStart -ge 0) 'Deployments query emitted no JSON document.'
    $listing = $listingText.Substring($jsonStart) | ConvertFrom-Json -ErrorAction Stop
    Assert-True ($listing.ok -eq $true) 'Deployments query did not succeed.'
    Assert-Deployments $listing $fakeSettings
    Assert-FakeLauncherContract $caseRoot $deployedPs1 $deployedCmd
    Assert-ModuleCacheIsolation $caseRoot $isolatedProfileRoot
    Write-Output 'PACKAGE_E2E_OK'
    if (-not $Live) { return }
    $realTokenPath = Join-Path $operatorProfile '.vibe/zai.api.token'
    Assert-True ([System.IO.File]::Exists($realTokenPath)) 'The required real token source is absent.'
    [System.Environment]::SetEnvironmentVariable('USERPROFILE', $isolatedProfileRoot, 'Process')
    [System.Environment]::SetEnvironmentVariable('HOME', $isolatedProfileRoot, 'Process')
    $moduleCacheRoot = Join-Path $caseRoot 'powershell-module-cache'
    [void] [IO.Directory]::CreateDirectory($moduleCacheRoot)
    [System.Environment]::SetEnvironmentVariable('PSModuleAnalysisCachePath',
        (Join-Path $moduleCacheRoot 'ModuleAnalysisCache'), 'Process')
    [System.Environment]::SetEnvironmentVariable(
        'ZAI_GLM_CONFIG_DIR', (Join-Path $caseRoot 'claude-state'), 'Process'
    )
    [System.Environment]::SetEnvironmentVariable('ZAI_API_TOKEN_FILE', $realTokenPath, 'Process')
    [System.Environment]::SetEnvironmentVariable('ZAI_GLM_MAX_THINKING', '4096', 'Process')
    $workerRoot = Join-Path $caseRoot 'worker'
    [void] [System.IO.Directory]::CreateDirectory($workerRoot)
    $nonce = [Guid]::NewGuid().ToString('N')
    $packet = @"
# ZAI GLM live E2E

Read only this PACKET.md. Do not read any other file.
Use only the Write tool. Do not use shell, Git, network, or subagents.
Write exactly two files in this cwd and no others:

1. proof.json: one JSON object with exactly these fields:
   {"nonce":"$nonce","result":"GLM_E2E_OK","reported_model":"<exact model identity>"}
2. WORKER-REPORT-E2E.md containing the same nonce, result, and reported_model.

Какая ты модель? Put the exact model identity you yourself observe in reported_model.
Do not infer it from this packet or launcher text; preserve the complete identifier and suffix.
Your entire terminal response must be exactly TASK-DONE.
"@
    Write-Utf8 (Join-Path $workerRoot 'PACKET.md') $packet
    $passes = 1
    $run = Invoke-LivePass $false 6 $passes
    $terminal = Read-ResultJson $run.Stdout $run.ExitCode $passes
    $artifactsOk = Test-LiveArtifacts $workerRoot $nonce $terminal.ModelKeys
    if (-not $artifactsOk) {
        $passes = 2
        $run = Invoke-LivePass $true 4 $passes
        $terminal = Read-ResultJson $run.Stdout $run.ExitCode $passes
        $artifactsOk = Test-LiveArtifacts $workerRoot $nonce $terminal.ModelKeys
    }
    Assert-True $artifactsOk 'The two live GLM artifacts remain missing or malformed.'
    Assert-True ($terminal.TerminalText -ceq 'TASK-DONE') `
        'The terminal child result was not exactly TASK-DONE.'
    Assert-True (-not [System.IO.File]::Exists((Join-Path $fakeSettings 'zai.api.token'))) `
        'The live launcher created a token file in isolated settings.'
    Write-Output "LIVE_GLM_E2E_OK passes=$passes"
    Write-Output "EVIDENCE_ROOT=$caseRoot"
}
finally {
    foreach ($key in $environmentKeys) {
        [System.Environment]::SetEnvironmentVariable($key, $savedEnvironment[$key], 'Process')
    }
}
