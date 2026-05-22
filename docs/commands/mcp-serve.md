# `vibe mcp serve` — Model Context Protocol server

Runs the JSON-RPC 2.0 server for the [Model Context Protocol](https://modelcontextprotocol.io) over stdio. Coding agents (Claude Code, Claude Desktop, Cursor, OpenCode, Codex) pick it up via their MCP config — see [`vibe mcp install`](mcp-install.md) — and can then query the project's lockfile and active subskill content live, without restarting the agent on every `vibe install`.

Spec: [PROP-004 §5.1](../../spec/research/PROP-004-tessl-comparative-research.md), [`spec/WAL.md`](../../spec/WAL.md) (M1.7 slices 1–3).

## Usage

```
vibe mcp serve [--path <dir>]
```

You normally don't run this manually — agents launch it themselves through the MCP config block written by `vibe mcp install`. Reach for it directly only when debugging the wire shape (e.g. piping a JSON-RPC `initialize` request through stdin).

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project root with `vibe.toml`. The server reloads the lockfile fresh on every tool call so a concurrent `vibe install` becomes visible without a restart. | `.` |

## Tools exposed

| Tool | Arguments | Returns |
| --- | --- | --- |
| `query_package` | `name: string` | The lockfile entry for an installed package: kind, name, version, content_hash, registry, source_url, source_ref, resolved_commit, features, subskills_active, top-level describes PURL, language. (The legacy `files_written` field is retained in the lockfile schema but left empty under the loading model — see [`docs/lockfile-format.md`](../lockfile-format.md).) |
| `read_subskill` | `package: string`, `subskill_path: string` | Concatenated text of an active subskill's content files. For `lazy-pull` deliveries the bytes come from the package cache without writing to the project tree. |
| `materialise_subskill` | `package: string`, `subskill_path: string`, `force?: bool` | Promotes a `lazy-pull` subskill into the project tree on demand. Refuses to overwrite existing files unless `force = true`. Returns the list of paths it wrote. |

## Wire form

The server speaks line-delimited JSON-RPC 2.0 on stdio (the canonical MCP shape for stdio servers). Protocol version reported during the `initialize` handshake: `2024-11-05`.

Trace example — handshake + one tool call:

```jsonc
// → stdin
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}
// ← stdout
{"jsonrpc": "2.0", "id": 1, "result": {
   "protocolVersion": "2024-11-05",
   "serverInfo": { "name": "vibe-mcp", "version": "0.1.0-dev" },
   "capabilities": { "tools": { "listChanged": false } }
}}
// → stdin
{"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}
// ← stdout (abbreviated)
{"jsonrpc": "2.0", "id": 2, "result": {
   "tools": [
     { "name": "query_package", ... },
     { "name": "read_subskill", ... },
     { "name": "materialise_subskill", ... }
   ]
}}
// → stdin
{"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
   "name": "query_package",
   "arguments": { "name": "wal" }
}}
// ← stdout — see vibe show subskills / vibe show features for the
//          field set; the payload is the lockfile entry verbatim
```

## Edge cases

- **Empty lockfile.** `query_package` returns a tool-level error with `isError: true` and a clear message rather than crashing — fresh projects without `vibe install` runs are legal callers.
- **Lockfile reload.** Every `tools/call` reloads `vibe.lock` fresh, so a parallel `vibe install` run becomes visible to the next call without restarting the server.
- **Lazy-pull subskills.** `read_subskill` transparently resolves `lazy-pull` deliveries from the package cache rather than the project tree. The agent need not know whether a subskill was eagerly materialised.
- **Cold subskill.** `materialise_subskill` writes the cache content to the project tree and refuses to overwrite existing files unless `force = true` — preserves operator edits in the same way `vibe update`'s `UserEditedFile` gate does.

## Related

- [`vibe mcp install`](mcp-install.md) — write the per-agent MCP config.
- [`vibe mcp status`](mcp-status.md) — preview without writing.
- [`vibe show subskills`](show.md) — inspect the same data the MCP server exposes, from the CLI.
