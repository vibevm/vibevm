[CmdletBinding()]
param(
    [ValidateNotNullOrEmpty()]
    [string]$InstallBase = (Join-Path $env:USERPROFILE '.vibe'),

    [switch]$All
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$ReleaseVersion = '1.0.0'
$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

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
        throw "Refusing to remove a path outside the install root: $fullPath"
    }
}

function Test-PathUnderRoot {
    param(
        [string]$Path,
        [string]$Root
    )

    $fullPath = [IO.Path]::GetFullPath($Path)
    $fullRoot = [IO.Path]::GetFullPath($Root).TrimEnd('\')
    return $fullPath.StartsWith($fullRoot + '\', [StringComparison]::OrdinalIgnoreCase)
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

function Remove-DirectoryIfEmpty {
    param([string]$Path)

    if ((Test-Path -LiteralPath $Path -PathType Container) -and
        @(Get-ChildItem -LiteralPath $Path -Force).Count -eq 0) {
        Remove-Item -LiteralPath $Path -Force
    }
}

function Normalize-PathEntry {
    param([string]$Entry)

    $value = $Entry.Trim().Trim('"')
    $value = [Environment]::ExpandEnvironmentVariables($value)
    if (-not [IO.Path]::IsPathRooted($value)) {
        $value = [IO.Path]::GetFullPath($value)
    } else {
        $value = [IO.Path]::GetFullPath($value)
    }
    return $value.TrimEnd('\', '/')
}

function Remove-ShimPathEntry {
    param([string]$ShimDirectory)

    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
    if ($null -eq $key) {
        Write-Host 'PATH: HKCU\Environment is absent; nothing to remove.'
        return
    }
    try {
        if (-not ($key.GetValueNames() -contains 'Path')) {
            Write-Host 'PATH: user Path is absent; nothing to remove.'
            return
        }
        $raw = [string]$key.GetValue(
            'Path',
            $null,
            [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
        )
        $kind = $key.GetValueKind('Path')
        if ($kind -ne [Microsoft.Win32.RegistryValueKind]::String -and
            $kind -ne [Microsoft.Win32.RegistryValueKind]::ExpandString) {
            throw "Unsupported HKCU\Environment Path kind: $kind"
        }

        $target = Normalize-PathEntry -Entry $ShimDirectory
        $parts = $raw.Split([char[]]@(';'), [StringSplitOptions]::None)
        $removeAt = -1
        for ($i = 0; $i -lt $parts.Count; $i++) {
            try {
                if ((Normalize-PathEntry -Entry $parts[$i]) -ieq $target) {
                    $removeAt = $i
                    break
                }
            } catch {
                # Preserve malformed or non-path entries byte-for-byte.
            }
        }
        if ($removeAt -lt 0) {
            Write-Host 'PATH: shim entry was not present.'
            return
        }

        $kept = for ($i = 0; $i -lt $parts.Count; $i++) {
            if ($i -ne $removeAt) { $parts[$i] }
        }
        $next = [string]::Join(';', $kept)
        $key.SetValue('Path', $next, $kind)
    } finally {
        $key.Dispose()
    }

    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class VibeDistributionEnvironmentBroadcast {
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr SendMessageTimeout(
        IntPtr hWnd, uint msg, UIntPtr wParam, string lParam,
        uint flags, uint timeout, out UIntPtr result);
}
'@
    [UIntPtr]$result = [UIntPtr]::Zero
    [void][VibeDistributionEnvironmentBroadcast]::SendMessageTimeout(
        [IntPtr]0xffff,
        0x001A,
        [UIntPtr]::Zero,
        'Environment',
        0x0002,
        5000,
        [ref]$result
    )
    Write-Host 'PATH: removed one shim entry (new terminals see the change).'
}

$InstallBase = Resolve-InstallBase -Path $InstallBase
$OptRoot = Join-Path $InstallBase 'opt'
$StoreRoot = Join-Path $OptRoot 'vibevm'
$CurrentPath = Join-Path $StoreRoot 'current'
$ShimDir = Join-Path $OptRoot 'bin'
$ShimCmd = Join-Path $ShimDir 'vibe.cmd'
$ShimPosix = Join-Path $ShimDir 'vibe'
$TagRoot = Join-Path $StoreRoot "versions\tag\$ReleaseVersion"
$LockPath = Join-Path $OptRoot '.distribution-install.lock'

$optExisted = Test-Path -LiteralPath $OptRoot -PathType Container
[IO.Directory]::CreateDirectory($OptRoot) | Out-Null
$lock = $null
$lockAcquired = $false
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

    $env:VIBEVM_INSTALL_ROOT = $InstallBase

    $vibeExe = Join-Path $ScriptRoot 'vibe.exe'
    if (-not (Test-Path -LiteralPath $vibeExe -PathType Leaf)) {
        $vibeExe = $null
        if (Test-Path -LiteralPath $CurrentPath -PathType Leaf) {
            $currentHome = [IO.File]::ReadAllText($CurrentPath).Trim()
            $candidate = Join-Path $currentHome 'vibe.exe'
            if (Test-Path -LiteralPath $candidate -PathType Leaf) {
                $vibeExe = $candidate
            }
        }
    }

    if ($All) {
        if ($null -ne $vibeExe) {
            try {
                Invoke-VibeVisible -Executable $vibeExe -VibeArguments @(
                    'self', 'remove', '--all', '--bin', '--force', '-y'
                )
            } catch {
                Write-Warning "VVM inventory cleanup reported: $($_.Exception.Message)"
            }
        }

        if (Test-Path -LiteralPath $StoreRoot) {
            Assert-PathUnderRoot -Path $StoreRoot -Root $OptRoot
            Remove-Item -LiteralPath $StoreRoot -Recurse -Force
        }
        foreach ($shim in @($ShimCmd, $ShimPosix)) {
            if (Test-Path -LiteralPath $shim -PathType Leaf) {
                Assert-PathUnderRoot -Path $shim -Root $OptRoot
                Remove-Item -LiteralPath $shim -Force
            }
        }
        Remove-ShimPathEntry -ShimDirectory $ShimDir
        Remove-DirectoryIfEmpty -Path $ShimDir
        Write-Host "Removed the complete VVM store and shims under $OptRoot"
    } else {
        $tagInstalled = Test-Path -LiteralPath $TagRoot -PathType Container
        if ($null -ne $vibeExe) {
            try {
                $inventory = (Invoke-VibeCapture -Executable $vibeExe -VibeArguments @(
                    'self', 'ls', '--json'
                )) | ConvertFrom-Json
                $tagInstalled = @($inventory.installs | Where-Object {
                    $_.id -eq "tag:$ReleaseVersion"
                }).Count -gt 0 -or $tagInstalled
            } catch {
                Write-Warning "Could not read VVM inventory; cleaning only tag:${ReleaseVersion}: $($_.Exception.Message)"
            }
        }

        if ($tagInstalled -and $null -ne $vibeExe) {
            Invoke-VibeVisible -Executable $vibeExe -VibeArguments @(
                'self', 'remove', "tag:$ReleaseVersion", '--bin', '--force', '-y'
            )
        }

        if (Test-Path -LiteralPath $CurrentPath -PathType Leaf) {
            $activeHome = [IO.File]::ReadAllText($CurrentPath).Trim()
            if ($activeHome.Length -gt 0 -and (Test-PathUnderRoot -Path $activeHome -Root $TagRoot)) {
                Remove-Item -LiteralPath $CurrentPath -Force
            }
        }
        if (Test-Path -LiteralPath $TagRoot) {
            Assert-PathUnderRoot -Path $TagRoot -Root $OptRoot
            Remove-Item -LiteralPath $TagRoot -Recurse -Force
        }
        Remove-DirectoryIfEmpty -Path (Split-Path -Parent $TagRoot)
        Remove-DirectoryIfEmpty -Path (Split-Path -Parent (Split-Path -Parent $TagRoot))
        Write-Host "Removed tag:$ReleaseVersion from $OptRoot"
        Write-Host 'PATH and settings were not changed.'
    }
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
    if (Test-Path -LiteralPath $OptRoot -PathType Container) {
        Remove-DirectoryIfEmpty -Path $ShimDir
        Remove-DirectoryIfEmpty -Path $OptRoot
    }
    if (-not $optExisted -and (Test-Path -LiteralPath $OptRoot -PathType Container)) {
        if (@(Get-ChildItem -LiteralPath $OptRoot -Force).Count -eq 0) {
            Remove-Item -LiteralPath $OptRoot -Force
        }
    }
}
