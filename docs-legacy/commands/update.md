# `vibe update`

Re-fetch and apply changes for selected installed packages, or for the whole lockfile.

## Usage

```text
vibe update [OPTIONS] [PACKAGES]...
vibe update --all [OPTIONS]
```

Named package references use `<kind>:<name>` and must already be installed. Named packages and `--all` are mutually exclusive.

## Options

| Option | Effect |
| --- | --- |
| `--all` | Update every package in `vibe.lock`. |
| `-g`, `--global` | Update one installed user MCP package (or one global application) without selecting a project. Global MCP updates require one unversioned `mcp:<group>/<name>` coordinate. |
| `--path <PATH>` | Select the project directory; default `.`. |
| `--assume-yes` | Skip the confirmation prompt. |
| `--exact` | Tighten updated root constraints in `vibe.toml` to `=<resolved-version>`. |
| `--auth-required` | Fail on 401/403 from a public registry instead of walking to another source. |
| `--offline` | Forbid network access; resolution and fetch must be satisfied locally. |

The common `--json`, `--quiet`, `--invoked-by`, and `--unattended` options are also available.

## Examples

```bash
vibe update flow:wal
vibe update --all --assume-yes
vibe update --all --offline --path ./my-project
vibe update -g mcp:ai.lev/fpf-mcp
```

For global MCP packages, the command uses the dedicated user MCP project, updates the selected root to an exact version, and refreshes its owned agent registrations. Other global MCP roots retain their versions. Global MCP updates require `mcp:` plus the full group/name, reject `--all` and `--path`, and do not accept an `@version` suffix. A remote URL package has no binary to rebuild; its published version pins the URL declaration, not the live remote implementation. Project `vibe update` updates package materialisation; use [`vibe mcp install`](mcp-install.md) to refresh selected project agent registrations after a package change.

If an alpha format break prevents reconciliation, follow the re-fetch recipe in [ALPHA-NOTES.md](../ALPHA-NOTES.md).

## Related

- [`vibe install`](install.md)
- [`vibe cache`](cache.md)
- [`vibe check`](check.md)
