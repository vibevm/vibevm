# `vibe mcp status` — preview agent integration state

Read-only counterpart of [`vibe mcp install`](mcp-install.md) + [`vibe mcp upgrade`](mcp-upgrade.md). Walks every supported agent (Claude Code, Claude Desktop, Cursor, OpenCode, Codex) across both project and user scopes, works out what `install` / `upgrade` would do, and reports per-(agent × scope) MCP-config status AND SKILL.md drift status without touching disk. Useful as a CI gate to catch config drift, or as a one-shot probe to see which agents this project + machine combination would integrate with.

Spec: [PROP-004 §5.1](../../spec/research/PROP-004-tessl-comparative-research.md), [`spec/WAL.md`](../../spec/WAL.md) (M1.7 slice 2 + 4).

## Usage

```
vibe mcp status [--path <dir>]
```

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project root with `vibe.toml`. | `.` |
| `--json` (global) | Structured envelope; see [Output (JSON)](#output-json). | off |
| `--quiet` (global) | One-line summary. | off |
| `--invoked-by <agent>` (global) | Stamps `invoked_by` on the JSON envelope. | (env: `VIBE_INVOKED_BY`, else absent) |

## Output

### Human-readable

```
Detected agents: claude, cursor, opencode
would-create  claude  → /home/dev/proj/.mcp.json
would-create  cursor  → /home/dev/proj/.cursor/mcp.json
would-create  opencode  → /home/dev/proj/opencode.json
would-create  claude-desktop  → /home/dev/.config/Claude/claude_desktop_config.json
would-create  codex  → /home/dev/.codex/config.toml
```

### Output (JSON)

```jsonc
{
  "ok": true,
  "command": "mcp:status",
  "project": "/home/dev/proj",
  "detected": ["claude", "cursor", "opencode"],
  "results": [
    { "agent": "claude",         "scope": "project", "config_path": ".../.mcp.json", "status": "would-create", "note": "file does not exist yet" },
    { "agent": "claude",         "scope": "user",    "config_path": "~/.claude.json",   "status": "unchanged",    "note": null },
    { "agent": "claude-desktop", "scope": "user",    "config_path": "~/.config/Claude/...",      "status": "would-create", "note": "file does not exist yet" },
    { "agent": "cursor",         "scope": "project", "config_path": ".../.cursor/mcp.json",      "status": "would-create", "note": "file does not exist yet" },
    { "agent": "opencode",       "scope": "project", "config_path": ".../opencode.json",         "status": "would-create", "note": "file does not exist yet" },
    { "agent": "opencode",       "scope": "user",    "config_path": "~/.config/opencode/...",    "status": "would-update", "note": "mcp/vibevm differs" },
    { "agent": "codex",          "scope": "user",    "config_path": "~/.codex/config.toml",      "status": "would-create", "note": "file does not exist yet" }
  ],
  "skill_results": [
    { "agent": "claude",   "scope": "project", "path": ".../.claude/skills/vibevm/SKILL.md",     "status": "would-create", "note": null },
    { "agent": "claude",   "scope": "user",    "path": "~/.claude/skills/vibevm/SKILL.md",       "status": "unchanged",    "note": null },
    { "agent": "opencode", "scope": "project", "path": ".../.opencode/skills/vibevm/SKILL.md",   "status": "would-create", "note": null },
    { "agent": "opencode", "scope": "user",    "path": "~/.config/opencode/skills/vibevm/SKILL.md", "status": "would-update", "note": null },
    { "agent": "codex",    "scope": "user",    "path": "~/.agents/skills/vibevm/SKILL.md",       "status": "would-create", "note": null }
  ]
}
```

`status` is one of `would-create`, `would-update`, `unchanged` — same vocabulary as `vibe mcp install --dry-run`. Each entry is keyed on `(agent, scope)` — a single agent appears in up to two rows when both project and user scopes have a surface. Cursor and Claude Desktop never appear in `skill_results` (no skill loader); Claude Desktop and Codex never appear in `results` for the `project` scope (no project surface).

## CI usage — drift gate

```bash
# Fail the build if any agent's vibevm block OR SKILL.md has drifted.
vibe --json mcp status \
  | jq -e '[.results[], .skill_results[]
            | select(.status == "would-update" or .status == "would-create")
           ] | length == 0' \
  || { echo "vibevm MCP / skill drift detected; run 'vibe mcp upgrade'"; exit 1; }
```

## Related

- [`vibe mcp install`](mcp-install.md) — write missing configs.
- [`vibe mcp upgrade`](mcp-upgrade.md) — refresh stale ones.
- [`vibe mcp uninstall`](mcp-uninstall.md) — remove existing.
- [`vibe mcp serve`](mcp-serve.md) — the server the configs point at.
