# z.ai GLM launcher for Claude Code

This package preserves a small cross-platform launcher surface for running
Claude Code against the z.ai Anthropic-compatible API. It ships PowerShell,
CMD, and Bash entry points and deploys them, with receipt ownership, to
`~/.vibe/opt/bin` by default.

## Installation procedure

First declare `tool:org.vibevm.world/zai-glm-claude@1.0.0` in the consuming
project and run that project's ordinary `vibe install`. Materialisation adds
the package and its boot contribution to the project; it does not write a
launcher into user state.

Then explicitly deploy the resolved package slot with its Windows profile:

```text
vibe deploy --profile windows --path vibevm/vibedeps/org.vibevm.world.zai-glm-claude/1.0.0
```

On Linux or macOS, deploy the POSIX profile instead:

```text
vibe deploy --profile posix --path <resolved-package-slot>
```

There is intentionally no default deploy profile until first-class target OS
applicability exists; omitting `--profile` must not silently choose Windows on
a POSIX host or vice versa.

Run that command from the consumer root. If the resolver selected a different
slot location, pass that resolved package-slot path instead. This second phase
publishes `claudez.ps1` and `claudez.cmd` to the configured settings root's
`opt/bin` directory; with the default settings root, that is
`~/.vibe/opt/bin`. The POSIX profile publishes executable
`<settings-root>/opt/bin/claudez` on Linux and macOS, likewise under
`~/.vibe/opt/bin` with default settings.

Claude Code's `claude` command must already be installed and discoverable on
`PATH`.

The complete parent/worker contract is
`vibevm/vibespecs/flows/zai-glm-claude/ZAI-GLM-CLAUDE-PROTOCOL.xml`; its sibling
`packet-template.xml` is the copy-ready task/report handoff. The boot snippet
loads the standing laws for either a Claude or Codex parent and points to both
files before the first GLM worker is commissioned.

The Windows install/deploy path and one real child run were exercised on
2026-09-04. Terminal metadata and the worker's independent self-report both
identified `glm-5.3[1m]`; the Linux/macOS launcher has syntax and fake-child
contract coverage, while its real-host executable-mode manual test remains
explicitly open.

## Credentials and configuration

By default the launcher reads the z.ai bearer from
`~/.vibe/zai.api.token`. Set `ZAI_API_TOKEN_FILE` to use another file. The file
must exist and contain a nonempty value. Keep its contents out of prompts,
packets, reports, logs, and source control.

The launcher configures these defaults:

- API base: `https://api.z.ai/api/anthropic`
- big model: `glm-5.3[1m]`
- small model: `glm-5-turbo`
- API timeout: `3000000` milliseconds
- maximum thinking budget: `32000`
- persistent Claude state: `~/.claude-glm`

Use `ZAI_GLM_BASE_URL`, `ZAI_GLM_BIG_MODEL`, `ZAI_GLM_SMALL_MODEL`,
`ZAI_GLM_API_TIMEOUT_MS`, `ZAI_GLM_MAX_THINKING`, and `ZAI_GLM_CONFIG_DIR` to
override those values. `CLAUDEZ_MAX_THINKING` and `CLAUDEZ_CONFIG_DIR` remain
supported as lower-precedence legacy overrides.

An endpoint override is trusted operator configuration, but the launcher still
requires an absolute HTTPS URL with a nonempty host and no userinfo or fragment;
it validates this before opening the token file.

The configured model names are requested provider identifiers. They are not
proof of the model ultimately resolved by the gateway or runtime. Treat the
resolved model recorded by a worker run as the execution evidence.

## Worker use

Start a fresh bounded unattended task with
`claudez -p <prompt-or-packet-pointer> --permission-mode bypassPermissions` and
pass any other Claude Code arguments normally. A useful packet names the goal,
allowed files, prohibitions, acceptance checks, and a durable report path.
Review that report, the resulting diff, and the verification evidence before
accepting the work.

On Windows, invoke `claudez.ps1` directly when exact preservation of empty or
pathologically quoted/backslash arguments matters. `claudez.cmd` is a convenient
bare-command shim with the normal reparsing behavior of `cmd.exe`.

For one correction, wait for the prior process to end, return to the exact same
dedicated working directory, and run
`claudez -c -p <correction> --permission-mode bypassPermissions`. The launcher
deliberately keeps the caller's current directory unchanged and stores
GLM-backed Claude state separately from the ordinary Claude state, so both the
working directory and the configured state directory matter when continuing.
