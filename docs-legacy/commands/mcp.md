# `vibe mcp`

Serve project state over the Model Context Protocol and manage vibevm integration with supported coding agents.

## Usage

```text
vibe mcp [OPTIONS] <COMMAND>
```

## Subcommands

| Command | Purpose |
| --- | --- |
| `vibe mcp serve` | Run the JSON-RPC 2.0 MCP server over stdio until the client disconnects. |
| `vibe mcp install` | Detect supported agents and write MCP configuration plus an optional `vibevm` skill. |
| `vibe mcp status` | Preview the install/configuration diff without writing. |
| `vibe mcp upgrade` | Refresh existing integrations without creating new ones. |
| `vibe mcp uninstall` | Remove selected vibevm MCP and skill integration while preserving foreign config. |

Supported install targets are Claude Code, Claude Desktop, Cursor, OpenCode, and Codex. Installation is idempotent; already-correct files are reported as unchanged.

The common `--json`, `--quiet`, `--invoked-by`, `--unattended`, and `--offline` options are available. Run `vibe mcp <command> --help` for scope, agent, and install-kind selectors.

## Examples

```bash
vibe mcp status
vibe mcp install --help
vibe mcp upgrade --help
vibe mcp serve --path ./my-project
```

## Related

- [`vibe mcp serve`](mcp-serve.md)
- [`vibe mcp install`](mcp-install.md)
- [`vibe mcp status`](mcp-status.md)
- [`vibe mcp upgrade`](mcp-upgrade.md)
- [`vibe mcp uninstall`](mcp-uninstall.md)
