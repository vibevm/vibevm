# `vibe version` — print version information

Prints the binary's version. Both `vibe version` and `vibe --version` work — the former is a subcommand, the latter is a global flag. Output is identical.

## Usage

```
vibe version
vibe --version
vibe -V
```

## Output

A single line:

```
vibe 0.1.0-dev
```

No `--json` / `--quiet` variants — there is no payload here beyond the version string.

## Exit codes

- `0` — always.

## Related

- [`vibe init`](init.md) — `vibe.toml`'s `[meta].generated_by` records the version that initialised the project.
- `vibe.lock`'s `[meta].generated_by` similarly carries the version that produced the latest install.
