<#
.SYNOPSIS
Installs the current VibeVM release for native Windows x64.

.EXAMPLE
irm https://vibevm.org/install.ps1 | iex

.EXAMPLE
& ([scriptblock]::Create((irm https://vibevm.org/install.ps1))) -Version 1.0.0

.EXAMPLE
& ([scriptblock]::Create((irm https://vibevm.org/install.ps1))) -Force
#>
[CmdletBinding()]
param(
    [string]$Version,
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:VibeReleaseRepository = 'vibevm/vibevm'
$script:VibeReleaseOrigin = "https://github.com/$($script:VibeReleaseRepository)/releases"
$script:VibeDistributionsAsset = 'DISTRIBUTIONS.json'
$script:VibeManifestMaxBytes = [UInt64](4 * 1024 * 1024)
$script:VibeBootstrapMaxBytes = [UInt64](512 * 1024 * 1024)
$script:VibeDownloadTimeout = [TimeSpan]::FromMinutes(30)
$script:VibeReadTimeout = [TimeSpan]::FromSeconds(60)

function Throw-VibeInstallError {
    param([Parameter(Mandatory)][string]$Message)
    throw "vibevm installer: $Message"
}

function Test-VibeSemVer {
    param([Parameter(Mandatory)][string]$Value)
    return $Value -match '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'
}

function Get-VibeTarget {
    if (-not [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::Windows
    )) {
        Throw-VibeInstallError 'install.ps1 supports native Windows only; use install.sh on Linux, macOS, or WSL'
    }

    $architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    if ($architecture -ne [System.Runtime.InteropServices.Architecture]::X64) {
        Throw-VibeInstallError "unsupported Windows architecture '$architecture'; this release supports x64 only"
    }
    return 'x86_64-pc-windows-msvc'
}

function Invoke-VibeDownload {
    param(
        [Parameter(Mandatory)][uri]$Uri,
        [Parameter(Mandatory)][string]$Destination,
        [Parameter(Mandatory)][UInt64]$MaximumBytes
    )

    $client = $null
    $request = $null
    $response = $null
    $inputStream = $null
    $outputStream = $null
    $failure = $null
    try {
        Add-Type -AssemblyName System.Net.Http | Out-Null
        $client = [System.Net.Http.HttpClient]::new()
        $client.Timeout = $script:VibeDownloadTimeout
        $request = [System.Net.Http.HttpRequestMessage]::new([System.Net.Http.HttpMethod]::Get, $Uri)
        $request.Headers.CacheControl = [System.Net.Http.Headers.CacheControlHeaderValue]::new()
        $request.Headers.CacheControl.NoCache = $true
        $response = $client.SendAsync(
            $request,
            [System.Net.Http.HttpCompletionOption]::ResponseHeadersRead
        ).GetAwaiter().GetResult()
        $response.EnsureSuccessStatusCode() | Out-Null
        $contentLength = $response.Content.Headers.ContentLength
        if ($null -ne $contentLength -and [UInt64]$contentLength -gt $MaximumBytes) {
            throw [IO.InvalidDataException]::new("response exceeds the $MaximumBytes-byte policy limit")
        }

        $inputStream = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
        $outputStream = [IO.File]::Create($Destination)
        $buffer = [byte[]]::new(65536)
        $elapsed = [Diagnostics.Stopwatch]::StartNew()
        [UInt64]$total = 0
        while ($true) {
            if ($elapsed.Elapsed -gt $script:VibeDownloadTimeout) {
                throw [TimeoutException]::new('download exceeded the 30-minute transfer limit')
            }
            $readDeadline = [Threading.CancellationTokenSource]::new($script:VibeReadTimeout)
            try {
                $read = $inputStream.ReadAsync(
                    $buffer,
                    0,
                    $buffer.Length,
                    $readDeadline.Token
                ).GetAwaiter().GetResult()
            }
            finally {
                $readDeadline.Dispose()
            }
            if ($read -eq 0) { break }
            if ([UInt64]$read -gt $MaximumBytes - $total) {
                throw [IO.InvalidDataException]::new("response exceeds the $MaximumBytes-byte policy limit")
            }
            $outputStream.Write($buffer, 0, $read)
            $total += [UInt64]$read
        }
        $elapsed.Stop()
    }
    catch {
        $failure = $_.Exception.Message
    }
    finally {
        if ($null -ne $outputStream) { $outputStream.Dispose() }
        if ($null -ne $inputStream) { $inputStream.Dispose() }
        if ($null -ne $response) { $response.Dispose() }
        if ($null -ne $request) { $request.Dispose() }
        if ($null -ne $client) { $client.Dispose() }
    }
    if ($null -ne $failure) {
        if (Test-Path -LiteralPath $Destination -PathType Leaf) {
            Remove-Item -LiteralPath $Destination -Force
        }
        Throw-VibeInstallError "download failed: $Uri ($failure)"
    }
}

function Read-VibeReleaseManifest {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Target,
        [string]$RequestedVersion
    )

    try {
        $manifest = Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
    }
    catch {
        Throw-VibeInstallError 'release manifest is not valid JSON'
    }

    if ($manifest.schema_version -ne 1 -or
        $manifest.product -ne 'vibevm' -or
        $manifest.repository -ne $script:VibeReleaseRepository) {
        Throw-VibeInstallError 'release manifest identity is invalid'
    }
    if (-not (Test-VibeSemVer ([string]$manifest.version))) {
        Throw-VibeInstallError 'release manifest does not contain a valid version'
    }
    if ($RequestedVersion -and $manifest.version -ne $RequestedVersion) {
        Throw-VibeInstallError "release manifest version '$($manifest.version)' does not match requested '$RequestedVersion'"
    }
    if ($manifest.tag -ne "v$($manifest.version)") {
        Throw-VibeInstallError 'release manifest tag does not match its version'
    }

    $platforms = @($manifest.platforms | Where-Object { $_.target -eq $Target })
    if ($platforms.Count -ne 1) {
        Throw-VibeInstallError "release manifest must contain exactly one platform entry for $Target"
    }
    $platform = $platforms[0]
    if ($platform.version -ne $manifest.version -or
        $platform.tag -ne $manifest.tag -or
        $platform.product -ne $manifest.product -or
        $platform.repository -ne $manifest.repository) {
        Throw-VibeInstallError "platform metadata for $Target does not match the aggregate manifest"
    }

    $expectedName = "vibe-bootstrap-$Target.exe"
    $bootstrapName = [string]$platform.bootstrap.name
    if ($bootstrapName -notmatch '^[A-Za-z0-9._-]+$' -or $bootstrapName -ne $expectedName) {
        Throw-VibeInstallError "release manifest names an unsafe or unexpected bootstrap for $Target"
    }
    try {
        $bootstrapSize = [UInt64]$platform.bootstrap.size
    }
    catch {
        Throw-VibeInstallError 'release manifest contains an invalid bootstrap size'
    }
    if ($bootstrapSize -eq 0) {
        Throw-VibeInstallError 'release manifest contains an invalid bootstrap size'
    }
    if ($bootstrapSize -gt $script:VibeBootstrapMaxBytes) {
        Throw-VibeInstallError "release bootstrap exceeds the $($script:VibeBootstrapMaxBytes)-byte policy limit"
    }
    $bootstrapDigest = [string]$platform.bootstrap.digest
    if ($bootstrapDigest -cnotmatch '^sha256:[0-9a-f]{64}$') {
        Throw-VibeInstallError 'release manifest contains an invalid bootstrap SHA-256 digest'
    }

    return [pscustomobject]@{
        Version = [string]$manifest.version
        Name = $bootstrapName
        Size = $bootstrapSize
        Digest = $bootstrapDigest
    }
}

function Invoke-VibeInstall {
    param(
        [string]$RequestedVersion,
        [switch]$ForceInstall
    )

    if ($RequestedVersion -and -not (Test-VibeSemVer $RequestedVersion)) {
        Throw-VibeInstallError "invalid SemVer '$RequestedVersion'"
    }

    $target = Get-VibeTarget
    $tempDirectory = Join-Path ([IO.Path]::GetTempPath()) ("vibevm-install-" + [Guid]::NewGuid().ToString('N'))
    [IO.Directory]::CreateDirectory($tempDirectory) | Out-Null

    try {
        $cacheNonce = [Guid]::NewGuid().ToString('N')
        $manifestPath = Join-Path $tempDirectory $script:VibeDistributionsAsset
        if ($RequestedVersion) {
            $manifestUri = "$($script:VibeReleaseOrigin)/download/v$RequestedVersion/$($script:VibeDistributionsAsset)"
        }
        else {
            $manifestUri = "$($script:VibeReleaseOrigin)/latest/download/$($script:VibeDistributionsAsset)"
        }

        Write-Host "Fetching VibeVM release metadata for $target..."
        Invoke-VibeDownload `
            -Uri "$manifestUri?vvm_refresh=$cacheNonce" `
            -Destination $manifestPath `
            -MaximumBytes $script:VibeManifestMaxBytes
        $release = Read-VibeReleaseManifest -Path $manifestPath -Target $target -RequestedVersion $RequestedVersion

        $releaseBase = "$($script:VibeReleaseOrigin)/download/v$($release.Version)"
        $bootstrapPath = Join-Path $tempDirectory $release.Name
        Write-Host "Downloading VibeVM $($release.Version) bootstrap..."
        Invoke-VibeDownload `
            -Uri "$releaseBase/$($release.Name)?vvm_refresh=$cacheNonce" `
            -Destination $bootstrapPath `
            -MaximumBytes $release.Size

        $actualSize = [UInt64](Get-Item -LiteralPath $bootstrapPath).Length
        if ($actualSize -ne $release.Size) {
            Throw-VibeInstallError "bootstrap size mismatch (expected $($release.Size) bytes, received $actualSize)"
        }
        $expectedHash = $release.Digest.Substring('sha256:'.Length)
        $actualHash = (Get-FileHash -LiteralPath $bootstrapPath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actualHash -cne $expectedHash) {
            Throw-VibeInstallError 'bootstrap SHA-256 mismatch; the downloaded file was not executed'
        }
        Unblock-File -LiteralPath $bootstrapPath -ErrorAction SilentlyContinue

        Write-Host 'Installing vibe, vibe-index, and the matching VibeVM sources...'
        $bootstrapArguments = @(
            'self', 'bootstrap',
            '--manifest', $manifestPath,
            '--version', $release.Version,
            '--release-base', $releaseBase
        )
        if ($ForceInstall) {
            $bootstrapArguments += '--force'
        }
        & $bootstrapPath @bootstrapArguments
        if ($LASTEXITCODE -ne 0) {
            Throw-VibeInstallError "verified bootstrap exited with status $LASTEXITCODE"
        }

        Write-Host 'VibeVM is installed; the active selector is shown above.'
        Write-Host 'Follow the PATH guidance above, then run `vibe self current` to inspect it.'
    }
    finally {
        if (Test-Path -LiteralPath $tempDirectory -PathType Container) {
            Remove-Item -LiteralPath $tempDirectory -Recurse -Force
        }
    }
}

if ($env:VIBEVM_INSTALL_TEST_MODE -ne '1') {
    Invoke-VibeInstall -RequestedVersion $Version -ForceInstall:$Force
}
