# `vibe init` — scaffold a new vibevm project

Creates the standard project tree spelled out in [`VIBEVM-SPEC.md` §4.2](../../VIBEVM-SPEC.md): the authored boot files the AI agent reads at session start, a generated `spec/boot/INDEX.md`, an empty `spec/` content tree, a `vibe.toml` project manifest pointing at the default public registry, an empty `vibe.lock`, the per-project cache directory, and a managed `<vibevm>` block in each agent instruction file.

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
├── CLAUDE.md          # Agent instruction file — carries a managed <vibevm> block.
├── AGENTS.md          # Agent instruction file — carries a managed <vibevm> block.
├── GEMINI.md          # Agent instruction file — carries a managed <vibevm> block.
├── spec/
│   ├── boot/
│   │   ├── 00-core.md   # Authored, user-owned. The "first thing every session reads."
│   │   ├── 90-user.md   # Authored, user-owned overrides.
│   │   └── INDEX.md     # Generated boot manifest — do not edit.
│   ├── flows/           # Empty — for project-authored flow content.
│   ├── feats/           # Empty — for project-authored feat content.
│   ├── stacks/          # Empty — for project-authored stack content.
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

`CLAUDE.md`, `AGENTS.md`, and `GEMINI.md` are **shared** agent instruction files — `vibe init` writes a managed `<vibevm>` block into each (the boot redirect; per [PROP-012](../../spec/modules/vibe-workspace/PROP-012-managed-redirect-block.md)) and leaves the rest of the file to you. If the file does not exist, `vibe init` creates it containing just the block; if it already has hand-authored content, the block is appended at the end and you may move it wherever you like.

`spec/boot/00-core.md`, `spec/boot/90-user.md`, and `spec/WAL.md` are **authored, user-owned** — `vibe install`, `vibe reinstall`, and `vibe uninstall` never modify them. Edit freely. `spec/boot/INDEX.md` is a **generated** boot manifest ([the loading model](../loading-model.md)); `vibe` rewrites it and `INLINE.md` (when there are inline contributions) — do not hand-edit them. A dependency's content is never written into `spec/`: `vibe install` materialises it into a separate `vibedeps/` tree at the workspace root.

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

- [`vibe install`](install.md) — resolve packages and materialise them into `vibedeps/`.
- [The loading model](../loading-model.md) — the boot artifacts and the `<vibevm>` block `vibe init` scaffolds.
- [`vibe.toml` schema](../../VIBEVM-SPEC.md) §7.5.
- [`PROP-002` §2.2](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#registry-model) — the registry model (`[[registry]]` array, naming convention, mirror layer).
