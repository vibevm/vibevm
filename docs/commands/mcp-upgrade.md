# `vibe mcp upgrade` — refresh stale vibevm integrations

Scans known per-agent MCP-config files and SKILL.md paths, compares the on-disk shape to what the current `vibe` binary would write, and rewrites only the diverged ones. **Does not create new installations** — that's [`vibe mcp install`](mcp-install.md)'s job.

Use after upgrading vibevm itself (`cargo install --path crates/vibe-cli`, new release, etc.) to pull the new SKILL.md body or wire-shape into agents that already had vibevm wired.

Spec: [PROP-004 §5.1](../../spec/research/PROP-004-tessl-comparative-research.md), [`spec/WAL.md`](../../spec/WAL.md) (M1.7 slice 5).

## Usage

```
vibe mcp upgrade [--path <dir>]
                 [--scope project | user | both]
                 [--agent <FILTER>]
                 [--config-only | --skill-only]
                 [--dry-run]
                 [--yes]
```

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project root for project-scope walks. Project-scope walks silently skip when `vibe.toml` is absent. | `.` |
| `--scope project|user|both` | Which scopes to scan. `project` requires `vibe.toml`; `user` works anywhere; `both` scans everything that exists. | `both` |
| `--agent <FILTER>` | One of `all`, `claude`, `claude-desktop`, `cursor`, `opencode`, `codex`. | `all` |
| `--config-only` | Refresh only MCP-config files, skip SKILL.md. Conflicts with `--skill-only`. | off |
| `--skill-only` | Refresh only SKILL.md files, skip MCP-config. Conflicts with `--config-only`. | off |
| `--dry-run` | Print refresh plan without writing. | off |
| `--yes` | Skip the apply confirm prompt on a TTY. Aliased to `--assume-yes`. The global `--unattended` flag (or `VIBE_UNATTENDED`) has the same effect. In non-TTY runs no prompt is shown regardless. | off |
| `--json` (global) | Structured envelope. | off |
| `--quiet` (global) | One-line summary. | off |

## Status vocabulary

- **`unchanged`** — file exists, vibevm-block matches the current shape, no write needed.
- **`updated`** — file exists, vibevm-block diverged, rewrote to match.
- **`would-update`** — same under `--dry-run`.
- **`not-installed`** — file or vibevm-block absent. Upgrade does NOT create new installations; the row is a hint that `vibe mcp install` is needed for this combination.
- **`skipped`** — agent has no surface for this scope (Cursor / Claude Desktop have no skill loader; Claude Desktop / Codex have no project surface).

## Examples

```bash
# Refresh everything that's installed (default both-scope scan).
vibe mcp upgrade

# Preview refresh plan without writing.
vibe mcp upgrade --dry-run

# Refresh only user-level installs (e.g. after a `cargo install`
# upgrade and you don't want to touch project-level pins).
vibe mcp upgrade --scope user

# Refresh only SKILL.md files (e.g. when only the template body
# changed in the new vibe version).
vibe mcp upgrade --skill-only

# Refresh one specific agent.
vibe mcp upgrade --agent opencode --skill-only

# CI-friendly: pre-flight gate that fails on drift.
vibe --json mcp upgrade --dry-run | jq -e '
  [.results[], .skill_results[]
   | select(.status == "would-update")
  ] | length == 0
'
```

## Output

### Human-readable

```
✓ mcp     claude (user) → /home/dev/.claude/settings.json
would mcp     opencode (user) → /home/dev/.config/opencode/opencode.json (mcp/vibevm differs)
· mcp     codex (user) → /home/dev/.codex/config.toml (config file does not exist; use `vibe mcp install` to create)
✓ skill   claude (user) → /home/dev/.claude/skills/vibevm/SKILL.md
would skill   opencode (user) → /home/dev/.config/opencode/skills/vibevm/SKILL.md
```

Sigil legend:

- `✓` — unchanged.
- `would` / `updated` — drift; `would` under `--dry-run`, `updated` after apply.
- `·` — `not-installed` (use `vibe mcp install` to provision).
- `skipped` — no surface for this (agent, scope) pair.

### Output (JSON)

```jsonc
{
  "ok": true,
  "command": "mcp:upgrade",
  "project": "/home/dev/proj",
  "scope": "both",
  "what": "both",
  "results": [
    { "agent": "claude",   "scope": "user",    "config_path": "~/.claude/settings.json", "status": "unchanged", "note": null },
    { "agent": "opencode", "scope": "user",    "config_path": "~/.config/opencode/opencode.json", "status": "would-update", "note": "mcp/vibevm differs" },
    { "agent": "codex",    "scope": "user",    "config_path": "~/.codex/config.toml", "status": "not-installed", "note": "config file does not exist; use `vibe mcp install` to create" }
  ],
  "skill_results": [
    { "agent": "claude",   "scope": "user",    "path": "~/.claude/skills/vibevm/SKILL.md", "status": "unchanged", "note": null },
    { "agent": "opencode", "scope": "user",    "path": "~/.config/opencode/skills/vibevm/SKILL.md", "status": "would-update", "note": null }
  ],
  "dry_run": true,
  "invoked_by": "opencode"
}
```

## Implementation notes

- For each (agent × scope) walk, upgrade does a two-step probe:
  1. File missing → `not-installed`.
  2. File exists but no `vibevm` key in the section → `not-installed` (the file is somebody else's; we don't add ourselves).
  3. `vibevm` key present → fall through to the install-time decide-then-apply path (`unchanged` / `updated`).

- For SKILL.md, missing-file → `not-installed`; existing file → reuse install_skill (its decide-then-apply already returns `unchanged` / `updated` for existing paths).

- Refresh overwrites stale on-disk content with the binary's current `SKILL_TEMPLATE`. If you've manually edited a SKILL.md and want to keep your edits, do NOT run `mcp upgrade` against it — uninstall + reinstall + re-edit is the safe path.

## Related

- [`vibe mcp install`](mcp-install.md) — create new installations.
- [`vibe mcp uninstall`](mcp-uninstall.md) — remove existing installations.
- [`vibe mcp status`](mcp-status.md) — read-only drift report (read-only counterpart of `mcp upgrade --dry-run`).
