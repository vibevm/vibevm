param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Options
)

Set-PSDebug -Trace 0
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$upload = $false
$checks = $false
$tests = $false
$selfCheck = $false
foreach ($option in $Options) {
    switch -CaseSensitive ($option) {
        '--upload' { $upload = $true }
        '--checks' { $checks = $true }
        '--tests' { $tests = $true }
        '--self-check' { $selfCheck = $true }
        '-h' {
            Write-Host 'usage: build-windows-x86_64.ps1 [--upload] [--checks] [--tests] [--self-check]'
            exit 0
        }
        '--help' {
            Write-Host 'usage: build-windows-x86_64.ps1 [--upload] [--checks] [--tests] [--self-check]'
            exit 0
        }
        default { throw "Unknown argument '$option'. Expected --upload, --checks, --tests, or --self-check." }
    }
}

$os = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows
)
$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
if (-not $os -or $arch -ne [System.Runtime.InteropServices.Architecture]::X64) {
    throw "This wrapper requires Windows x86_64; detected OS=$([System.Runtime.InteropServices.RuntimeInformation]::OSDescription), architecture=$arch."
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'cargo was not found on PATH; install the repository Rust toolchain before building the distribution.'
}

$target = 'x86_64-pc-windows-msvc'
$distArguments = @('dist', 'build', '--target', $target)
if ($checks) {
    $distArguments += '--checks'
}
if ($tests) {
    $distArguments += '--tests'
}
if ($selfCheck) {
    $distArguments += '--self-check'
}

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$xtaskTargetDir = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    Join-Path $repositoryRoot 'target'
} elseif ([IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
    [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
} else {
    [IO.Path]::GetFullPath((Join-Path $repositoryRoot $env:CARGO_TARGET_DIR))
}
$xtaskBinary = Join-Path $xtaskTargetDir 'debug\xtask.exe'
$credentialValues = @{}
$credentialEntries = @(Get-ChildItem Env: | Where-Object {
    $_.Name -eq 'VIBEVM_PUBLISH_TOKEN' -or
    $_.Name -like 'VIBEVM_PUBLISH_TOKEN_*' -or
    $_.Name -in @(
        'GITHUB_TOKEN', 'GH_TOKEN', 'GITHUB_ENTERPRISE_TOKEN', 'GH_ENTERPRISE_TOKEN',
        'GITHUB_PAT', 'ACTIONS_ID_TOKEN_REQUEST_TOKEN', 'ACTIONS_RUNTIME_TOKEN',
        'GIT_ASKPASS', 'SSH_ASKPASS', 'SSH_AUTH_SOCK', 'SSH_AGENT_PID', 'GIT_SSH',
        'GIT_SSH_COMMAND'
    ) -or
    $_.Name -like 'GIT_CONFIG_*'
})
$credentialEntries | ForEach-Object {
    $credentialValues[$_.Name] = $_.Value
    Remove-Item -LiteralPath "Env:$($_.Name)"
}
$manifestDirWasSet = Test-Path Env:CARGO_MANIFEST_DIR
$previousManifestDir = $env:CARGO_MANIFEST_DIR
$exitCode = 0
Push-Location -LiteralPath $repositoryRoot
try {
    & cargo build --locked --package xtask --target-dir $xtaskTargetDir
    $exitCode = $LASTEXITCODE
    if ($exitCode -eq 0) {
        if (-not (Test-Path -LiteralPath $xtaskBinary -PathType Leaf)) {
            throw "Compiled xtask was not found at '$xtaskBinary'."
        }
        $env:CARGO_MANIFEST_DIR = Join-Path $repositoryRoot 'xtask'
        & $xtaskBinary @distArguments
        $exitCode = $LASTEXITCODE
        if ($exitCode -eq 0 -and $upload) {
            foreach ($name in @('VIBEVM_PUBLISH_TOKEN_GITHUB', 'VIBEVM_PUBLISH_TOKEN')) {
                if ($credentialValues.ContainsKey($name)) {
                    Set-Item -LiteralPath "Env:$name" -Value $credentialValues[$name]
                }
            }
            & $xtaskBinary dist upload-built --target $target
            $exitCode = $LASTEXITCODE
        }
    }
} finally {
    $restoredPublishEntries = @(Get-ChildItem Env: | Where-Object {
        $_.Name -eq 'VIBEVM_PUBLISH_TOKEN' -or $_.Name -like 'VIBEVM_PUBLISH_TOKEN_*'
    })
    $restoredPublishEntries | ForEach-Object { Remove-Item -LiteralPath "Env:$($_.Name)" }
    foreach ($entry in $credentialValues.GetEnumerator()) {
        Set-Item -LiteralPath "Env:$($entry.Key)" -Value $entry.Value
    }
    if ($manifestDirWasSet) {
        $env:CARGO_MANIFEST_DIR = $previousManifestDir
    } else {
        Remove-Item Env:CARGO_MANIFEST_DIR -ErrorAction SilentlyContinue
    }
    Pop-Location
}
if ($exitCode -ne 0) {
    [Console]::Error.WriteLine("distribution build failed with exit code $exitCode")
    exit $exitCode
}
