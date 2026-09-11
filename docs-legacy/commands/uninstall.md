# `vibe uninstall`

Remove one installed package from a project.

## Usage

```text
vibe uninstall [OPTIONS] <PACKAGE>
```

`<PACKAGE>` is `<kind>:<name>`; a supplied version is ignored.

## Options

| Option | Effect |
| --- | --- |
| `--path <PATH>` | Select the project directory; default `.`. |
| `--assume-yes` | Skip the confirmation prompt. |
| `--offline` | Forbid network access for the invocation. |

The common `--json`, `--quiet`, `--invoked-by`, and `--unattended` options are also available.

## Example

```bash
vibe uninstall flow:wal --path ./my-project
```

The command removes the project's declaration/materialisation for the package. It does not mean “purge this identity from the machine store”; use [`vibe cache clean`](cache.md) for explicit store cleanup.

## Related

- [`vibe install`](install.md)
- [`vibe list`](list.md)
- [`vibe cache`](cache.md)

