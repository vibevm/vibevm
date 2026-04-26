# `vibe init` — scaffold a new vibevm project

Creates the standard project tree spelled out in [`VIBEVM-SPEC.md` §4.2](../../VIBEVM-SPEC.md): boot snippets the AI agent reads at session start, an empty `spec/` content tree, a `vibe.toml` project manifest pointing at the default public registry, an empty `vibe.lock`, and the per-project cache directory.

`init` is **idempotent**. Running it twice in the same directory does not destroy user-modified files — every existing file is reported as `kept`, and only missing pieces are created.

## Usage

```
vibe init [--path <dir>] [--name <project-name>] [--stack <stack-name>]
          [--registry-url <url> | --registry-ref <ref> | --no-registry]
          [--json | --quiet]
```

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Directory to initialise. Created if it does not exist. | `.` (current directory) |
| `--name <name>` | Project name written to `[project].name` in `vibe.toml`. | basename of `--path` |
| `--stack <name>` | Pre-populate `[active].stack` so `vibe build` later picks the right target. The stack package is **not** installed by `init`; install separately with `vibe install stack:<name>`. | unset |
| `--registry-url <url>` | URL written into the default `[[registry]]` entry. Conflicts with `--no-registry`. | the public GitVerse registry default |
| `--registry-ref <ref>` | Ref written into the default `[[registry]]` entry. Conflicts with `--no-registry`. | `main` |
| `--no-registry` | Do not write a `[[registry]]` section. The project then requires `--registry <path>` on every `vibe install`, or a manual edit to `vibe.toml`. | off |
| `--json` | Emit a structured report instead of human-readable output. Schema: [`schemas/init_report.jtd.json`](../../schemas/init_report.jtd.json). | off |
| `--quiet` | Single-line summary `vibe init: <N> created, <K> kept in <path>`. Conflicts with `--json`. | off |

## What gets created

After a fresh `vibe init`:

```
<project>/
├── CLAUDE.md          # AI-agent redirect
├── AGENTS.md          # AI-agent redirect (byte-identical to CLAUDE.md)
├── GEMINI.md          # AI-agent redirect (byte-identical)
├── spec/
│   ├── boot/
│   │   ├── 00-core.md   # User-owned. The "first thing every session reads."
│   │   └── 90-user.md   # User-owned overrides.
│   ├── flows/           # Empty — populated by `vibe install flow:…`.
│   ├── feats/           # Empty — populated by `vibe install feat:…`.
│   ├── stacks/          # Empty — populated by `vibe install stack:…`.
│   ├── common/          # Empty — for project-specific PROP / FEAT docs.
│   ├── modules/         # Empty — for module-specific docs.
│   └── WAL.md           # User-owned project state checkpoint.
├── vibe.toml          # Project manifest — `[project]`, `[[registry]]`.
├── vibe.lock          # Empty lockfile (no packages installed yet).
├── .vibe/
│   ├── cache/         # Per-project package cache (gitignored).
│   └── .gitignore     # Excludes the entire cache from git.
└── .gitignore         # Sensible defaults for vibevm projects.
```

`spec/boot/00-core.md`, `spec/boot/90-user.md`, and `spec/WAL.md` are **user-owned** — `vibe install` and `vibe uninstall` never modify them. Edit freely.

## Examples

Initialise the current directory with the default registry:

```bash
vibe init
```

Create a new project in a fresh directory, pre-set its stack:

```bash
vibe init --path my-tg-bot --name "tg-bot" --stack rust-cli
vibe install stack:rust-cli --path my-tg-bot
vibe install feat:welcome-page --path my-tg-bot
```

Scaffold without a registry section — useful for offline development:

```bash
vibe init --no-registry
# Subsequent installs need an explicit --registry path.
vibe install flow:wal --registry /local/registry --path .
```

Pin a corporate registry instead of the default public one:

```bash
vibe init \
    --registry-url "git@gitverse.internal:vibe-packages" \
    --registry-ref main
```

## Exit codes

- `0` — success (idempotent re-run also returns `0`).
- `1` — generic error (target path is not a directory, write failure, etc.).

## Related

- [`vibe install`](install.md) — populate the `spec/` tree from a registry.
- [`vibe.toml` schema](../../VIBEVM-SPEC.md) §7.5.
- [`PROP-002` §2.2](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#registry-model) — the registry model (`[[registry]]` array, naming convention, mirror layer).
