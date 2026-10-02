# `vibe uninstall`

Remove one installed package from a project, or a user-scoped MCP package with `-g`.

## Usage

```text
vibe uninstall [OPTIONS] <PACKAGE>
```

`<PACKAGE>` is `<kind>:<name>`; a supplied version is ignored.

## Options

| Option | Effect |
| --- | --- |
| `--path <PATH>` | Select the project directory; default `.`. |
| `-g`, `--global` | Remove one user-scoped MCP package (or global application) without selecting a project. For MCP, also remove only its owned user-scope agent registrations. |
| `--assume-yes` | Skip the confirmation prompt. |
| `--offline` | Forbid network access for the invocation. |

The common `--json`, `--quiet`, `--invoked-by`, and `--unattended` options are also available.

## Example

```bash
vibe uninstall flow:wal --path ./my-project
vibe uninstall -g mcp:ai.lev/fpf-mcp
```

The command removes the project's declaration/materialisation for the package. It does not mean “purge this identity from the machine store”; use [`vibe cache clean`](cache.md) for explicit store cleanup.

Global MCP removal requires an unversioned, fully qualified `mcp:<group>/<name>` coordinate; omit `--path`. It uses the dedicated user MCP project and removes registrations owned by that package while preserving other packages and agent entries. In a regular project, package removal and agent unregistration are separate actions: run [`vibe mcp uninstall mcp:<group>/<name>`](mcp-uninstall.md) for its managed agent entries.

## Related

- [`vibe install`](install.md)
- [`vibe list`](list.md)
- [`vibe cache`](cache.md)
