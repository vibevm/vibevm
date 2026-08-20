[CmdletBinding()]
param(
    [ValidateNotNullOrEmpty()]
    [string]$InstallBase = (Join-Path $env:USERPROFILE '.vibe'),

    [switch]$NoPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$ReleaseVersion = '1.0.0'
$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$PayloadPath = Join-Path $ScriptRoot 'vibe.exe'
$ManifestPath = Join-Path $ScriptRoot 'SHA256SUMS.txt'

function Resolve-InstallBase {
    param([string]$Path)

    $expanded = [Environment]::ExpandEnvironmentVariables($Path)
    $full = [IO.Path]::GetFullPath($expanded)
    $volumeRoot = [IO.Path]::GetPathRoot($full)
    if ($full.TrimEnd('\') -ieq $volumeRoot.TrimEnd('\')) {
        throw "InstallBase must not be a filesystem root: $full"
    }
    return $full.TrimEnd('\')
}

function Assert-PathUnderRoot {
    param(
        [string]$Path,
        [string]$Root
    )

    $fullPath = [IO.Path]::GetFullPath($Path)
    $fullRoot = [IO.Path]::GetFullPath($Root).TrimEnd('\')
    if (-not $fullPath.StartsWith($fullRoot + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to modify a path outside the install root: $fullPath"
    }
}

function Invoke-VibeCapture {
    param(
        [string]$Executable,
        [string[]]$VibeArguments
    )

    $lines = @(& $Executable @VibeArguments 2>&1 | ForEach-Object { $_.ToString() })
    $exitCode = $LASTEXITCODE
    $text = [string]::Join([Environment]::NewLine, $lines)
    if ($exitCode -ne 0) {
        throw "vibe exited with code ${exitCode}: $text"
    }
    return $text
}

function Invoke-VibeVisible {
    param(
        [string]$Executable,
        [string[]]$VibeArguments
    )

    $text = Invoke-VibeCapture -Executable $Executable -VibeArguments $VibeArguments
    if ($text.Length -gt 0) {
        Write-Host $text
    }
}

function Get-ExpectedPayloadHash {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Checksum manifest is missing beside install.ps1: $Path"
    }
    $matches = @(Get-Content -LiteralPath $Path | Where-Object {
        $_ -match '^([0-9A-Fa-f]{64})  vibe\.exe$'
    })
    if ($matches.Count -ne 1) {
        throw 'SHA256SUMS.txt must contain exactly one checksum for vibe.exe'
    }
    [void]($matches[0] -match '^([0-9A-Fa-f]{64})  vibe\.exe$')
    return $Matches[1].ToLowerInvariant()
}

function Restore-CurrentPointer {
    param(
        [string]$CurrentPath,
        [bool]$Existed,
        [string]$PreviousValue
    )

    if ($Existed) {
        $parent = Split-Path -Parent $CurrentPath
        [IO.Directory]::CreateDirectory($parent) | Out-Null
        $utf8 = New-Object Text.UTF8Encoding($false)
        [IO.File]::WriteAllText(
            $CurrentPath,
            $PreviousValue.TrimEnd("`r", "`n") + [Environment]::NewLine,
            $utf8
        )
    } elseif (Test-Path -LiteralPath $CurrentPath -PathType Leaf) {
        Remove-Item -LiteralPath $CurrentPath -Force
    }
}

if (-not [Environment]::Is64BitOperatingSystem) {
    throw 'vibe 1.0.0 for Windows requires an x86_64 (64-bit) Windows installation'
}
if (-not (Test-Path -LiteralPath $PayloadPath -PathType Leaf)) {
    throw "vibe.exe must be beside install.ps1: $PayloadPath"
}

$payloadVersion = (Invoke-VibeCapture -Executable $PayloadPath -VibeArguments @('--version')).Trim()
if ($payloadVersion -ne "vibe $ReleaseVersion") {
    throw "Unexpected payload version '$payloadVersion'; expected 'vibe $ReleaseVersion'"
}

$expectedHash = Get-ExpectedPayloadHash -Path $ManifestPath
$actualHash = (Get-FileHash -LiteralPath $PayloadPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualHash -ne $expectedHash) {
    throw "SHA256 mismatch for vibe.exe (expected $expectedHash, got $actualHash)"
}

$InstallBase = Resolve-InstallBase -Path $InstallBase
$OptRoot = Join-Path $InstallBase 'opt'
$StoreRoot = Join-Path $OptRoot 'vibevm'
$CurrentPath = Join-Path $StoreRoot 'current'
$ShimDir = Join-Path $OptRoot 'bin'
$ShimCmd = Join-Path $ShimDir 'vibe.cmd'
$ShimPosix = Join-Path $ShimDir 'vibe'
$LockPath = Join-Path $OptRoot '.distribution-install.lock'

$optExisted = Test-Path -LiteralPath $OptRoot -PathType Container
[IO.Directory]::CreateDirectory($OptRoot) | Out-Null

$lock = $null
$lockAcquired = $false
$importAttempted = $false
$tagExisted = $false
$currentExisted = $false
$previousCurrent = ''
$shimCmdExisted = $false
$shimPosixExisted = $false
$hadInstallRoot = Test-Path Env:VIBEVM_INSTALL_ROOT
$previousInstallRoot = if ($hadInstallRoot) { $env:VIBEVM_INSTALL_ROOT } else { $null }

try {
    try {
        $lock = [IO.File]::Open(
            $LockPath,
            [IO.FileMode]::OpenOrCreate,
            [IO.FileAccess]::ReadWrite,
            [IO.FileShare]::None
        )
        $lockAcquired = $true
    } catch {
        throw "Another vibe distribution install/uninstall is using $OptRoot"
    }

    $currentExisted = Test-Path -LiteralPath $CurrentPath -PathType Leaf
    $previousCurrent = if ($currentExisted) { [IO.File]::ReadAllText($CurrentPath) } else { '' }
    $shimCmdExisted = Test-Path -LiteralPath $ShimCmd -PathType Leaf
    $shimPosixExisted = Test-Path -LiteralPath $ShimPosix -PathType Leaf

    $env:VIBEVM_INSTALL_ROOT = $InstallBase
    $beforeJson = Invoke-VibeCapture -Executable $PayloadPath -VibeArguments @('self', 'ls', '--json')
    $before = $beforeJson | ConvertFrom-Json
    $tagExisted = @($before.installs | Where-Object {
        $_.id -eq "tag:$ReleaseVersion"
    }).Count -gt 0

    Write-Host "Installing vibe $ReleaseVersion into $OptRoot"
    $importAttempted = $true
    Invoke-VibeVisible -Executable $PayloadPath -VibeArguments @(
        'self', 'import', $PayloadPath, '--tag', $ReleaseVersion, '--use'
    )

    if (-not (Test-Path -LiteralPath $ShimCmd -PathType Leaf)) {
        throw "Installation did not create the Windows shim: $ShimCmd"
    }
    $installedVersion = (Invoke-VibeCapture -Executable $ShimCmd -VibeArguments @('--version')).Trim()
    if ($installedVersion -ne "vibe $ReleaseVersion") {
        throw "Shim verification returned '$installedVersion'; expected 'vibe $ReleaseVersion'"
    }
    $active = (Invoke-VibeCapture -Executable $ShimCmd -VibeArguments @('self', 'current')).Trim()
    if ($active -notmatch ('^tag:' + [regex]::Escape($ReleaseVersion) + '(?: #[0-9]+)?$')) {
        throw "Active version is '$active'; expected tag:$ReleaseVersion"
    }

    if ($NoPath) {
        Write-Host 'PATH update skipped (-NoPath).'
    } else {
        Invoke-VibeVisible -Executable $ShimCmd -VibeArguments @(
            'self', 'doctor', '--fix', '-y'
        )
    }

    Write-Host ''
    Write-Host "vibe $ReleaseVersion installed successfully."
    Write-Host "Install root: $OptRoot"
    Write-Host "Shim:         $ShimCmd"
    if ($NoPath) {
        Write-Host 'PATH:         unchanged'
    } else {
        Write-Host 'PATH:         shim directory ensured for new terminals'
    }
} catch {
    $failure = $_
    Write-Warning "Install failed; rolling back: $($failure.Exception.Message)"

    if (-not $lockAcquired) {
        throw $failure
    }

    if ($importAttempted -and -not $tagExisted) {
        try {
            Invoke-VibeVisible -Executable $PayloadPath -VibeArguments @(
                'self', 'remove', "tag:$ReleaseVersion", '--bin', '--force', '-y'
            )
        } catch {
            Write-Warning "VVM rollback command failed: $($_.Exception.Message)"
        }
        $tagRoot = Join-Path $StoreRoot "versions\tag\$ReleaseVersion"
        if (Test-Path -LiteralPath $tagRoot) {
            Assert-PathUnderRoot -Path $tagRoot -Root $OptRoot
            Remove-Item -LiteralPath $tagRoot -Recurse -Force
        }
    }

    try {
        Restore-CurrentPointer -CurrentPath $CurrentPath -Existed $currentExisted -PreviousValue $previousCurrent
        if (-not $shimCmdExisted -and (Test-Path -LiteralPath $ShimCmd -PathType Leaf)) {
            Remove-Item -LiteralPath $ShimCmd -Force
        }
        if (-not $shimPosixExisted -and (Test-Path -LiteralPath $ShimPosix -PathType Leaf)) {
            Remove-Item -LiteralPath $ShimPosix -Force
        }
    } catch {
        Write-Warning "Pointer/shim rollback failed: $($_.Exception.Message)"
    }

    throw $failure
} finally {
    if ($lockAcquired -and $null -ne $lock) {
        $lock.Dispose()
    }
    if ($lockAcquired -and (Test-Path -LiteralPath $LockPath -PathType Leaf)) {
        Remove-Item -LiteralPath $LockPath -Force -ErrorAction SilentlyContinue
    }
    if ($hadInstallRoot) {
        $env:VIBEVM_INSTALL_ROOT = $previousInstallRoot
    } else {
        Remove-Item Env:VIBEVM_INSTALL_ROOT -ErrorAction SilentlyContinue
    }
    if (-not $optExisted -and (Test-Path -LiteralPath $OptRoot -PathType Container)) {
        if (@(Get-ChildItem -LiteralPath $OptRoot -Force).Count -eq 0) {
            Remove-Item -LiteralPath $OptRoot -Force
        }
    }
}
