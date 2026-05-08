# `vibe mcp install` — wire vibevm into a coding agent

Detects supported coding agents on this machine + the current project, writes the per-agent MCP server configuration, and (optionally) installs the `vibevm` SKILL.md instructing the agent how to use vibevm. Idempotent — already-correct configs surface as `unchanged`.

Spec: [PROP-004 §5.1](../../spec/research/PROP-004-tessl-comparative-research.md), [`spec/WAL.md`](../../spec/WAL.md) (M1.7 slices 4 + 5).

## Two scopes — project vs user

Every install can land at one of three places:

- **`--scope project`** — files in the project tree (`<proj>/.<agent>/...`), committed to git, every clone gets the same setup. The MCP server entry hardcodes `--path <abs-project>` so the server always serves this project. Strictly requires `vibe.toml` in `--path` — bails out if absent.
- **`--scope user`** — global home / config dirs (`~/.<agent>/...`), machine-local, works in every directory. The MCP server entry omits `--path` so the server resolves CWD per invocation. **Bootstrap-mode** — does NOT require `vibe.toml` in `--path`.
- **`--scope both`** — write to project AND user simultaneously. **Best-effort** for the project leg: when `vibe.toml` is missing in `--path`, the project leg is silently skipped (a `note:` line in text mode flags it) and the user leg runs as normal. Same model as `vibe mcp upgrade` / `vibe mcp uninstall` — designed so first-time-user provisioning scripts can run unattended on a fresh machine before any vibevm project exists. For agents with no project surface (Claude Desktop, Codex), Both collapses to user with a `skipped` row in the project results.

Without `--scope`, the wizard asks. Default in wizard: `project` if `vibe.toml` is present in `--path`, else `user`.

## Two install kinds — MCP and SKILL.md

`--what` chooses what to install:

- **`--what mcp`** — only the MCP server entry (no SKILL.md write).
- **`--what skill`** — only the SKILL.md (no MCP-config touch).
- **`--what both`** (default) — both.

## Supported agents

| Agent | Markers (project) | Project config | User config | Skill loader |
| --- | --- | --- | --- | --- |
| `claude` | `.claude/`, `CLAUDE.md` | `<proj>/.claude/settings.json` | `~/.claude/settings.json` | yes — `<proj>/.claude/skills/` and `~/.claude/skills/` |
| `claude-desktop` | (user-only) `<config-dir>/Claude/` exists | (n/a) | `<config-dir>/Claude/claude_desktop_config.json` | no |
| `cursor` | `.cursor/`, `.cursorrules` | `<proj>/.cursor/mcp.json` | `~/.cursor/mcp.json` | no |
| `opencode` | `.opencode/`, `opencode.json`, `opencode.jsonc`, `AGENTS.md` | `<proj>/opencode.json` | `~/.config/opencode/opencode.json` (XDG path on every OS — see note) | yes — `<proj>/.opencode/skills/` and `~/.config/opencode/skills/` |
| `codex` | (user-only) `~/.codex/` exists | (n/a) | `~/.codex/config.toml` (TOML) | yes — `<proj>/.agents/skills/` and `~/.agents/skills/` |

`<config-dir>` resolves through `dirs::config_dir()` — `%APPDATA%` on Windows, `~/Library/Application Support` on macOS, `~/.config` on Linux. **Used by Claude Desktop only.**

**Note on OpenCode user paths.** OpenCode is documented to read `~/.config/opencode/` on every platform — XDG-style cross-platform, even on Windows where `dirs::config_dir()` would point at `%APPDATA%\opencode\`. vibevm resolves OpenCode user-scope via `dirs::home_dir().join(".config").join("opencode")` to match what opencode actually reads, regardless of OS. (Pre-fix slice-5 versions mistakenly used `%APPDATA%\opencode\` on Windows — that location is silently ignored by opencode. If you ran an earlier slice-5 install, delete `%APPDATA%\Roaming\opencode\` by hand; the config and skill it created there have no effect.)

## Usage

```
vibe mcp install [--path <dir>]
                 [--agent <FILTER> | --auto]
                 [--scope project | user | both]
                 [--what mcp | skill | both]
                 [--dry-run]
                 [--yes]
                 [--force]
```

Without flags, drops into a 3-question wizard (TTY required): pick scope, pick what, pick agents (multi-select with detected ones preselected). For CI / scripts use `--auto` (install everywhere, all detected agents, scope auto-resolves) or any combination of explicit flags.

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project root for project-scope walks. Strictly required only when scope is `project`. `--scope both` is best-effort — if `vibe.toml` is missing, the project leg is silently skipped and only the user leg runs. `--scope user` ignores `--path`. | `.` |
| `--agent <FILTER>` | One of `all`, `claude`, `claude-desktop`, `cursor`, `opencode`, `codex`. Conflicts with `--auto`. | (interactive) |
| `--auto` | Detect every supported agent and install in all of them. No prompts (except apply confirm — pass `--yes` to skip). Conflicts with `--agent`. Auto-resolves: scope = `project` if `vibe.toml` in `--path`, else `user`; what = `both`. | off |
| `--scope project|user|both` | See [Two scopes](#two-scopes--project-vs-user). | (interactive / auto-resolved under `--auto`) |
| `--what mcp|skill|both` | See [Two install kinds](#two-install-kinds--mcp-and-skillmd). | `both` |
| `--dry-run` | Print what would be written without touching disk. | off |
| `--yes` | Skip the apply confirm prompt on a TTY. Aliased to `--assume-yes` for symmetry with `vibe install` / `uninstall` / `update`. The global `--unattended` flag (or `VIBE_UNATTENDED` env-var) has the same effect. In non-TTY runs (CI / opencode harness), no prompt is shown regardless — pre-this-version behaviour preserved for scripts that never passed `--yes`. | off |
| `--force` | Provision the agent's config even when no presence marker is detected. | off |
| `--json` (global) | Emit a structured envelope. | off |
| `--quiet` (global) | One-line summary. | off |
| `--invoked-by <agent>` (global) | Stamps `invoked_by` on the JSON envelope. | (env: `VIBE_INVOKED_BY`, else absent) |
| `--unattended` (global) | Implies `--yes` and refuses to open any interactive wizard — if `--scope` / `--what` / `--agent` are missing under `--unattended`, the command bails with a hint instead of prompting. Stamps `unattended: true` on the JSON envelope. Falls back to `VIBE_UNATTENDED` env-var (truthy: `1`, `true`, `yes`, `on`). | (env: `VIBE_UNATTENDED`, else off) |

## Examples

### Bootstrap (first install, no project yet)

```bash
# Wizard mode — pick scope + what + agents.
vibe mcp install

# Or fully scripted: install everywhere, MCP + skill, user-level.
vibe mcp install --auto --scope user

# Or explicit single-agent bootstrap.
vibe mcp install --agent opencode --scope user --what both
```

### Project-level (already inside a vibevm project)

```bash
# Wizard — defaults to project-scope when vibe.toml is present.
vibe mcp install

# Scripted, all agents detected in the project tree.
vibe mcp install --auto

# One-off: just refresh the OpenCode SKILL.md, nothing else.
vibe mcp install --agent opencode --scope project --what skill
```

### Both — project pinned + user fallback

```bash
# Write project config (always serves this project) AND user config
# (works when opencode starts outside the project too).
vibe mcp install --scope both --auto
```

### Provisioning a fresh user account (no project yet)

A setup script that runs once per user — before any vibevm project
exists on the machine — wires up MCP + SKILL.md at the user level
for a specific agent. Use the global `--unattended` flag instead of
`--yes` — it's the human-readable shape for "I'm in a script, no
prompts, no wizard":

```bash
vibe --unattended mcp install \
    --agent opencode \
    --scope both \
    --what both
```

Or via env-var (handier inside cloud-init / Dockerfile / Ansible):

```bash
VIBE_UNATTENDED=1 vibe mcp install --agent opencode --scope both --what both
```

The project leg is silently skipped (no `vibe.toml` here yet); the
user leg writes once and applies in every directory the operator
opens later. Re-runs are idempotent — entries become `unchanged`.

`--unattended` also refuses to open any wizard — if you forget one
of `--agent` / `--scope` / `--what`, the command bails with a hint
rather than blocking on a hidden prompt. CI-safe by construction.

### Pre-flight diff before applying

```bash
vibe mcp install --auto --dry-run
```

## Output

### Human-readable

```
→ created mcp     claude (project) → /home/dev/proj/.claude/settings.json
→ created skill   claude (project) → /home/dev/proj/.claude/skills/vibevm/SKILL.md
→ unchanged mcp   opencode (project) → /home/dev/proj/opencode.json
→ skipped skill   cursor (project) → (no skill loader) (agent `cursor` does not load filesystem skills)
```

### Output (JSON)

```jsonc
{
  "ok": true,
  "command": "mcp:install",
  "project": "/home/dev/proj",
  "scope": "project",
  "what": "both",
  "detected": ["claude", "cursor", "opencode"],
  "targeted": ["claude", "cursor", "opencode"],
  "results": [
    { "agent": "claude",   "scope": "project", "config_path": ".../.claude/settings.json", "status": "created", "note": "file does not exist yet" },
    { "agent": "cursor",   "scope": "project", "config_path": ".../.cursor/mcp.json",      "status": "unchanged", "note": null },
    { "agent": "opencode", "scope": "project", "config_path": ".../opencode.json",         "status": "created", "note": "file does not exist yet" }
  ],
  "skill_results": [
    { "agent": "claude",   "scope": "project", "path": ".../.claude/skills/vibevm/SKILL.md", "status": "created", "note": null },
    { "agent": "cursor",   "scope": "project", "path": null, "status": "skipped", "note": "agent `cursor` does not load filesystem skills" },
    { "agent": "opencode", "scope": "project", "path": ".../.opencode/skills/vibevm/SKILL.md", "status": "created", "note": null }
  ],
  "mode": "auto",
  "dry_run": false,
  "invoked_by": "opencode"
}
```

`status` vocabulary:

- `created` — file did not exist; we wrote it.
- `updated` — file existed but differed; we rewrote it. Foreign keys outside the `mcpServers` / `mcp` / `mcp_servers` block are preserved.
- `unchanged` — byte-identical block already on disk.
- `would-create` / `would-update` — `--dry-run` previews.
- `skipped` — agent has no surface for the requested action (skill writes for Cursor/Claude Desktop; project-scope MCP for Claude Desktop/Codex when `--scope both` or `--scope project --force`).

`mode`:

- `auto` — `--auto` was used.
- `flags` — explicit `--scope` / `--what` / `--agent` mix.
- `interactive` — wizard ran.

## What gets written

### Claude Code / Claude Desktop / Cursor (JSON, `mcpServers`)

Project scope:
```jsonc
{
  "mcpServers": {
    "vibevm": {
      "command": "vibe",
      "args": ["mcp", "serve", "--path", "/home/dev/proj"]
    }
  }
}
```

User scope (no `--path`):
```jsonc
{
  "mcpServers": {
    "vibevm": {
      "command": "vibe",
      "args": ["mcp", "serve"]
    }
  }
}
```

### OpenCode (JSON, `mcp`, command-array shape)

Project scope:
```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "vibevm": {
      "type": "local",
      "command": ["vibe", "mcp", "serve", "--path", "/home/dev/proj"],
      "enabled": true
    }
  }
}
```

User scope:
```jsonc
{
  "mcp": {
    "vibevm": {
      "type": "local",
      "command": ["vibe", "mcp", "serve"],
      "enabled": true
    }
  }
}
```

### Codex (TOML, `mcp_servers`)

```toml
[mcp_servers.vibevm]
command = "vibe"
args = ["mcp", "serve"]   # or ["mcp", "serve", "--path", "/home/dev/proj"] for project scope
```

### SKILL.md (Claude Code, OpenCode, Codex)

Two-state document with three sections:

1. **Detect** — first step is to check whether `vibe.toml` exists in CWD.
2. **Section A — bootstrap** — what to do when no `vibe.toml` is here (`vibe init`, install starter packages, optionally land project skill, transition to Section B).
3. **Section B — inside an existing project** — bootstrap protocol (`CLAUDE.md` → `spec/boot` → `spec/WAL.md` → relevant PROPs), MCP-tool contracts, "lockfile is canonical".
4. **Common section** (applies to both) — required `--invoked-by` / `VIBE_INVOKED_BY`, `vibe <subcmd> --help` discipline (with the full subcommand list including `mcp install/upgrade/uninstall/status/serve`), the four non-negotiable rules.

The exact body is `crates/vibe-cli/src/commands/skill_template.md`, vendored at compile time so it ships byte-identical inside the `vibe` binary. Two-state structure means a global / user-scope skill works whether the agent is bootstrapping or inside a project.

## Edge cases

- **No agents detected, no `--force`.** Empty `targeted` list; the run succeeds with a "no supported agents detected" summary.
- **Non-TTY without `--auto` / `--agent` / `--scope`.** The wizard refuses with a hint pointing at `--scope`/`--auto` rather than panicking inside dialoguer.
- **`--scope project` without `vibe.toml`.** Hard error with a hint pointing at `--scope user` for bootstrap-mode.
- **`--scope both` without `vibe.toml`.** Soft skip: the project leg is silently dropped, the user leg runs as normal, and a `note:` line in text mode flags what happened. Exit 0. Use this from first-time-user provisioning scripts that run before any vibevm project exists.
- **`--unattended` with missing wizard dimensions.** Hard error with a concrete hint listing which of `--scope` / `--what` / `--agent` is missing. The bail is deliberate — `--unattended` promises no wizard will open, so silently picking a default would be the wrong contract. Pass the missing flag(s) or use `--auto` to detect every dimension automatically.
- **Stale skill content.** `install --what skill` overwrites stale on-disk SKILL.md with the current template — the contract is set by the binary. For "refresh after vibe upgrade" use [`vibe mcp upgrade`](mcp-upgrade.md).
- **Foreign keys.** The JSON / TOML mergers preserve every key outside the `mcpServers` / `mcp` / `mcp_servers` block.
- **User-level `--scope user` writes touch `~/`.** Claude Desktop and Codex configs live in the operator's home / config dir, not the project tree. `--auto` will mutate them when their parent dir exists. `--dry-run` is the safe preview path.

## Related

- [`vibe mcp upgrade`](mcp-upgrade.md) — refresh stale installs to the current shape, no new installations.
- [`vibe mcp uninstall`](mcp-uninstall.md) — remove vibevm from one or more agents.
- [`vibe mcp status`](mcp-status.md) — read-only drift report (MCP + skill).
- [`vibe mcp serve`](mcp-serve.md) — the MCP server invoked from the configs `mcp install` writes.
- [`vibe show config`](show.md) — surfaces the resolved `--invoked-by` value with provenance.
- [Quickstart guide](../guides/agent-mcp-quickstart-opencode.md) — end-to-end walkthrough for opencode.
