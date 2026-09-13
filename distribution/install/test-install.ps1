$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$env:VIBEVM_INSTALL_TEST_MODE = '1'
. (Join-Path $PSScriptRoot 'install.ps1')

$testDirectory = Join-Path ([IO.Path]::GetTempPath()) ("vibevm-install-test-" + [Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($testDirectory) | Out-Null
try {
    $manifestPath = Join-Path $testDirectory 'DISTRIBUTIONS.json'
    @'
{
  "schema_version": 1,
  "product": "vibevm",
  "repository": "vibevm/vibevm",
  "version": "1.2.3",
  "tag": "v1.2.3",
  "source_commit": "0123456789012345678901234567890123456789",
  "platforms": [
    {
      "schema_version": 1,
      "product": "vibevm",
      "repository": "vibevm/vibevm",
      "version": "1.2.3",
      "tag": "v1.2.3",
      "source_commit": "0123456789012345678901234567890123456789",
      "target": "x86_64-pc-windows-msvc",
      "asset": {
        "name": "vibevm-1.2.3-x86_64-pc-windows-msvc.zip",
        "size": 10,
        "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "bootstrap": {
        "name": "vibe-bootstrap-x86_64-pc-windows-msvc.exe",
        "size": 123,
        "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
      },
      "bundle": {}
    }
  ]
}
'@ | Set-Content -LiteralPath $manifestPath -Encoding UTF8

    if (-not (Test-VibeSemVer '1.2.3-rc.1+build.7')) {
        throw 'SemVer validation rejected a valid version'
    }
    # A real probe of this machine: the harness runs on Windows x64, the only
    # target the release ships, and the probe must not touch any .NET member
    # that Windows PowerShell 5.1 lacks (it once read OSArchitecture and died).
    if ((Get-VibeTarget) -cne 'x86_64-pc-windows-msvc') {
        throw 'target detection did not name the Windows x64 target on a Windows x64 machine'
    }
    $refreshUri = Add-VibeRefreshNonce `
        -Uri 'https://example.invalid/DISTRIBUTIONS.json' `
        -Nonce 'acceptance-nonce'
    if ($refreshUri -cne 'https://example.invalid/DISTRIBUTIONS.json?vvm_refresh=acceptance-nonce') {
        throw 'refresh URI construction is not compatible with Windows PowerShell StrictMode'
    }
    $release = Read-VibeReleaseManifest `
        -Path $manifestPath `
        -Target 'x86_64-pc-windows-msvc' `
        -RequestedVersion '1.2.3'
    if ($release.Version -ne '1.2.3' -or
        $release.Name -ne 'vibe-bootstrap-x86_64-pc-windows-msvc.exe' -or
        $release.Size -ne 123 -or
        $release.Digest -ne 'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb') {
        throw 'manifest parser returned the wrong bootstrap metadata'
    }

    $oversized = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    $oversized.platforms[0].bootstrap.size = $script:VibeBootstrapMaxBytes + 1
    $oversized | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $manifestPath -Encoding UTF8
    $rejected = $false
    try {
        Read-VibeReleaseManifest `
            -Path $manifestPath `
            -Target 'x86_64-pc-windows-msvc' `
            -RequestedVersion '1.2.3' | Out-Null
    }
    catch {
        $rejected = $_.Exception.Message -like '*policy limit*'
    }
    if (-not $rejected) {
        throw 'oversized bootstrap was not rejected'
    }

    Write-Host 'install.ps1 offline tests passed'
}
finally {
    Remove-Item -LiteralPath $testDirectory -Recurse -Force
    Remove-Item Env:VIBEVM_INSTALL_TEST_MODE -ErrorAction SilentlyContinue
}
