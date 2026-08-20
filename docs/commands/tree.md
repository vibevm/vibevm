# `vibe tree`

Analyze the resolved spec and dependency tree. The command shows boot-load type, transitive and conditional edges, the static/dynamic boot lanes, and in-place `@spec` markers without mutating the project.

## Usage

```text
vibe tree [OPTIONS]
```

## Options

| Option | Effect |
| --- | --- |
| `--path <PATH>` | Select the project root; default `.`. |
| `--plain` | Force a static ASCII tree, including on a TTY. |
| `-c`, `--console` | Open the in-terminal console TUI; mutually exclusive with `--terminal`. |
| `-t`, `--terminal` | Open the vibeterm desktop terminal; mutually exclusive with `--console`. |
| `--json` | Emit the machine model. |

The common `--quiet`, `--invoked-by`, `--unattended`, and `--offline` options are also available. A non-TTY renders the plain tree automatically; `--plain` wins over both interactive selectors.

## Examples

```bash
vibe tree --plain
vibe tree --json --path ./my-project
vibe tree --console
```

## Related

- [`vibe list`](list.md)
- [`vibe show`](show.md)
- [`vibe check`](check.md)

