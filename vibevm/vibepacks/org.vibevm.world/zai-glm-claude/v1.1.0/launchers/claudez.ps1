# vibe:zai-glm-claude launcher epoch=1 owner=org.vibevm.world/zai-glm-claude
# Launch Claude Code against the z.ai Anthropic-compatible API while preserving
# the caller's working directory and forwarding every command-line argument.
$ErrorActionPreference = 'Stop'

# Prove the executable is available before consulting credential storage.
$claudeCommand = Get-Command -Name 'claude' -CommandType Application, ExternalScript -ErrorAction SilentlyContinue |
    Select-Object -First 1
if ($null -eq $claudeCommand) {
    [Console]::Error.WriteLine('claudez: claude command was not found on PATH')
    exit 127
}

$baseUrl = if ($env:ZAI_GLM_BASE_URL) {
    $env:ZAI_GLM_BASE_URL
} else {
    'https://api.z.ai/api/anthropic'
}
[Uri]$endpointUri = $null
$endpointValid = [Uri]::TryCreate($baseUrl, [UriKind]::Absolute, [ref]$endpointUri) -and
    $endpointUri.Scheme -eq 'https' -and
    -not [string]::IsNullOrWhiteSpace($endpointUri.Host) -and
    [string]::IsNullOrEmpty($endpointUri.UserInfo) -and
    [string]::IsNullOrEmpty($endpointUri.Fragment)
if (-not $endpointValid) {
    [Console]::Error.WriteLine('claudez: invalid z.ai endpoint: ZAI_GLM_BASE_URL must be an absolute HTTPS URI with a host and no userinfo or fragment')
    exit 1
}

$tokenFile = if ($env:ZAI_API_TOKEN_FILE) {
    $env:ZAI_API_TOKEN_FILE
} else {
    Join-Path $env:USERPROFILE '.vibe\zai.api.token'
}

if (-not (Test-Path -LiteralPath $tokenFile -PathType Leaf)) {
    [Console]::Error.WriteLine("claudez: token file not found: $tokenFile")
    exit 1
}

try {
    $token = Get-Content -Raw -LiteralPath $tokenFile
} catch {
    [Console]::Error.WriteLine("claudez: unable to read token file: $tokenFile")
    exit 1
}

if ([string]::IsNullOrWhiteSpace($token)) {
    [Console]::Error.WriteLine("claudez: token file is empty: $tokenFile")
    exit 1
}

$managedEnvironment = @(
    'ANTHROPIC_BASE_URL',
    'ANTHROPIC_AUTH_TOKEN',
    'ANTHROPIC_DEFAULT_OPUS_MODEL',
    'ANTHROPIC_DEFAULT_SONNET_MODEL',
    'ANTHROPIC_DEFAULT_HAIKU_MODEL',
    'API_TIMEOUT_MS',
    'MAX_THINKING_TOKENS',
    'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC',
    'CLAUDE_CONFIG_DIR'
)
$previousEnvironment = @{}
foreach ($name in $managedEnvironment) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}
$childExit = 1

try {
$bigModel = if ($env:ZAI_GLM_BIG_MODEL) { $env:ZAI_GLM_BIG_MODEL } else { 'glm-5.3[1m]' }
$smallModel = if ($env:ZAI_GLM_SMALL_MODEL) { $env:ZAI_GLM_SMALL_MODEL } else { 'glm-5-turbo' }

$env:ANTHROPIC_BASE_URL = $baseUrl
$env:ANTHROPIC_AUTH_TOKEN = $token.Trim()
$env:ANTHROPIC_DEFAULT_OPUS_MODEL = $bigModel
$env:ANTHROPIC_DEFAULT_SONNET_MODEL = $bigModel
$env:ANTHROPIC_DEFAULT_HAIKU_MODEL = $smallModel
$env:API_TIMEOUT_MS = if ($env:ZAI_GLM_API_TIMEOUT_MS) {
    $env:ZAI_GLM_API_TIMEOUT_MS
} else {
    '3000000'
}
$env:MAX_THINKING_TOKENS = if ($env:ZAI_GLM_MAX_THINKING) {
    $env:ZAI_GLM_MAX_THINKING
} elseif ($env:CLAUDEZ_MAX_THINKING) {
    $env:CLAUDEZ_MAX_THINKING
} else {
    '32000'
}
$env:CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = '1'
$env:CLAUDE_CONFIG_DIR = if ($env:ZAI_GLM_CONFIG_DIR) {
    $env:ZAI_GLM_CONFIG_DIR
} elseif ($env:CLAUDEZ_CONFIG_DIR) {
    $env:CLAUDEZ_CONFIG_DIR
} else {
    Join-Path $env:USERPROFILE '.claude-glm'
}

Remove-Variable -Name token

& $claudeCommand.Source @args
$childExit = $LASTEXITCODE
}
finally {
    Remove-Variable -Name token -ErrorAction SilentlyContinue
    foreach ($name in $managedEnvironment) {
        [Environment]::SetEnvironmentVariable($name, $previousEnvironment[$name], 'Process')
    }
}

exit $childExit
