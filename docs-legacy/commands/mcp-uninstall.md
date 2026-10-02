# `vibe mcp uninstall` — remove managed MCP registrations

Mirror of [`vibe mcp install`](mcp-install.md): same scope axis (project / user / both), same agent filter, same `--config-only` / `--skill-only` toggle. Without a package selector, the legacy product-MCP path drops the `vibevm` key from each agent's MCP config (foreign keys preserved) and deletes the SKILL.md file (and parent `vibevm/` skill subdir if it becomes empty).

With `mcp:<group>/<name>`, removes only registrations owned by that installed package; `--server <name>` narrows the removal to one declared server. It preserves unrelated and externally edited entries. This unregisters the server from agents; to remove a globally installed package itself, use [`vibe uninstall -g`](uninstall.md).

Spec: [PROP-004 §5.1](../../legacy-spec/research/PROP-004-tessl-comparative-research.md), [`spec/WAL.md`](../../spec/WAL.md) (M1.7 slice 5).

## Usage

```
vibe mcp uninstall [mcp:<group>/<name>] [--server <name>]
                   [--path <dir>]
                   [--scope project | user | both]
                   [--agent <FILTER>]
                   [--config-only | --skill-only]
                   [--dry-run]
                   [--yes]
```

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project root for project-scope walks. Required when scope is `project` or `both`. | `.` |
| `mcp:<group>/<name>` | Optional installed MCP package whose owned registrations should be removed. | legacy product MCP and package walk |
| `--server <name>` | Limit removal to one server in the selected package. | all servers in selected package |
| `--scope project|user|both` | Where to remove from. | `both` |
| `--agent <FILTER>` | One of `all`, `claude`, `claude-desktop`, `cursor`, `opencode`, `codex`, `qwen-code`. | `all` |
| `--config-only` | Drop only MCP-config blocks (keep SKILL.md). Conflicts with `--skill-only`. | off |
| `--skill-only` | Delete only SKILL.md (keep MCP-config block). Conflicts with `--config-only`. | off |
| `--dry-run` | Print removal plan without writing. | off |
| `--yes` | Skip the apply confirm prompt on a TTY. Aliased to `--assume-yes`. The global `--unattended` flag (or `VIBE_UNATTENDED`) has the same effect. In non-TTY runs no prompt is shown regardless. | off |
| `--json` (global) | Structured envelope. | off |
| `--quiet` (global) | One-line summary. | off |

## Removal contract

The following file and skill behavior describes removal of vibevm's own product MCP integration. A selected package removal targets only that package's managed server entries.

**Removed:**

- The `vibevm` key from `mcpServers` / `mcp` / `mcp_servers` in the agent's config file.
- The SKILL.md file at `<scope>/<skill-dir>/vibevm/SKILL.md`.
- The parent `vibevm/` skill subdir, but only if it becomes empty after deleting SKILL.md (best-effort `rmdir`).

**NOT removed:**

- Foreign keys in JSON / TOML — `provider`, `model`, other `mcpServers.<other>`, top-level scalars all survive.
- The containing section — even if `mcpServers: {}` or `[mcp_servers]` becomes empty after dropping vibevm.
- The agent's config file itself — never deleted.
- `vibe.toml`, `vibe.lock`, `spec/`, `packages/`, anything else in the project tree besides the project-scope SKILL.md.
- User config dirs (`~/.claude/`, `~/.config/opencode/`, `~/.codex/`) — these are the agent's property, not ours.

## Status vocabulary

- **`removed`** — vibevm-block / SKILL.md was present, deleted.
- **`would-remove`** — same under `--dry-run`.
- **`not-installed`** — file or block absent; nothing to remove.
- **`skipped`** — agent has no surface for this scope (Cursor/Claude Desktop have no skill loader; Claude Desktop has no project surface).

## Examples

```bash
# Wizard mode — pick scope + agents.
vibe mcp uninstall

# Wipe everything (CI / "I want vibevm gone").
vibe mcp uninstall --auto-equivalent: --agent all --scope both --yes

# Remove only project-scope vibevm — keep user-level installs.
vibe mcp uninstall --scope project

# Remove only the SKILL.md (keep MCP server connection).
vibe mcp uninstall --skill-only

# Remove vibevm from one agent only.
vibe mcp uninstall --agent opencode --scope both

# Remove one package's user registration while keeping its materialised package.
vibe mcp uninstall mcp:ai.lev/fpf-mcp --agent qwen-code --scope user --yes

# Pre-flight diff before applying.
vibe mcp uninstall --dry-run
```

## Output

### Human-readable

```
would mcp     claude (project) → /home/dev/proj/.mcp.json (drop `vibevm` from mcpServers)
would skill   claude (project) → /home/dev/proj/.claude/skills/vibevm/SKILL.md (delete SKILL.md and parent vibevm/ dir if empty)
· mcp     cursor (project) → /home/dev/proj/.cursor/mcp.json (no `vibevm` entry in mcpServers)
```

### Output (JSON)

```jsonc
{
  "ok": true,
  "command": "mcp:uninstall",
  "project": "/home/dev/proj",
  "scope": "project",
  "what": "both",
  "results": [
    { "agent": "claude", "scope": "project", "config_path": ".../.mcp.json", "status": "removed", "note": "dropped `vibevm` from mcpServers" },
    { "agent": "cursor", "scope": "project", "config_path": ".../.cursor/mcp.json", "status": "not-installed", "note": "no `vibevm` entry in mcpServers" }
  ],
  "skill_results": [
    { "agent": "claude", "scope": "project", "path": ".../.claude/skills/vibevm/SKILL.md", "status": "removed", "note": null }
  ],
  "dry_run": false,
  "invoked_by": "opencode"
}
```

## Edge cases

- **Empty config file after removal.** The MCP config file remains; only the `vibevm` key is dropped. If `mcpServers` had only vibevm, you'll see `mcpServers: {}` left behind. Manual cleanup is your decision — vibevm doesn't trim other people's containers.
- **SKILL.md edits.** Slice-5 uninstall deletes the SKILL.md without checking for hand edits. If you customised your SKILL.md, back it up first.
- **Stragglers in `vibevm/` skill dir.** If you put extra notes inside `<skills>/vibevm/`, the parent-dir cleanup is best-effort — a non-empty dir survives the uninstall silently.
- **`--scope project` without `vibe.toml`.** Hard error with a hint pointing at `--scope user`.

## Related

- [`vibe mcp install`](mcp-install.md) — install vibevm into agents.
- [`vibe mcp upgrade`](mcp-upgrade.md) — refresh existing installs to current shape.
- [`vibe mcp status`](mcp-status.md) — read-only "what's installed where" probe.
