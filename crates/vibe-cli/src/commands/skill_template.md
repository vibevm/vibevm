---
name: vibevm
description: Use whenever the user mentions vibevm, vibe install/update/check/show/outdated/registry/search/init/list/uninstall/mcp, packages, subskills, or the lockfile. Also when the user asks to create a new vibevm-managed project. The MCP server exposes lockfile-aware queries; query it before guessing about installed packages.
---

# vibevm

This skill teaches you how to use **vibevm** — a CLI package manager
for spec-driven AI-assisted development. It covers only the package
manager's CLI surface and the MCP-server queries it exposes. Any
project-specific conventions (custom commit rules, write-ahead log
protocols, custom spec corpus, etc.) live elsewhere — see the last
section.

## First step — detect the situation

Check whether `vibe.toml` exists in the working directory.

- **Exists** → the directory is a vibevm-managed project. You can
  call `vibe install`, `vibe update`, MCP tools, etc. against it.
  See [Section B](#section-b--inside-a-vibevm-project).
- **Absent** → not a vibevm project. If the user wants to create
  one, see [Section A](#section-a--no-vibevm-project-here-yet).
  If they just want ad-hoc CLI access (`vibe search`, `vibe show
  config`, etc.), most read-only commands work without a project.

If the user explicitly says "create a vibevm project" / "set up
vibevm" / "I have a vibevm project" — trust them, skip the probe.

## Section A — no vibevm project here yet

To create one:

1. **`vibe init`** — scaffolds `vibe.toml` (project manifest with
   default registries) + an empty `vibe.lock`. It also writes a
   handful of starter files (`spec/boot/00-core.md`,
   `spec/boot/90-user.md`, `spec/WAL.md`, `CLAUDE.md`, `AGENTS.md`,
   `GEMINI.md`). These starter files are **project conventions**,
   not load-bearing for vibevm itself — they're a template the
   project's owner can keep, edit, or delete. vibevm commands work
   the same way whether they're present or not.

2. **(Optional) `vibe install <pkgref>`** — install one or more
   packages from the configured registries (default: `vibespecs`
   on GitHub + GitVerse). Use `vibe search <query>` to discover
   what's available. There is no required first package.

3. The project is now operational. Switch to Section B for further
   work.

## Section B — inside a vibevm project

### Use the MCP server before guessing

The vibevm MCP server is registered as `vibevm` in your MCP config.
The lockfile is canonical — query the server rather than inferring
package state from filenames or commit history.

Tools:

- **`query_package(name)`** — returns the lockfile entry for an
  installed package: kind, name, version, content_hash, registry,
  source_url, source_ref, resolved_commit, files_written, features,
  subskills_active (with delivery + describes), top-level describes
  PURL, language.
- **`read_subskill(package, subskill_path)`** — returns the textual
  content of an active subskill's files. For `lazy-pull` deliveries,
  reads from the package cache; for `eager` / `lazy-push`, reads
  from the project tree.
- **`materialise_subskill(package, subskill_path, force?)`** —
  promotes a `lazy-pull` subskill into the project tree on demand.
  Refuses to overwrite existing files unless `force=true`.

If the user asks about installed packages — what's installed, what
version, what features are active, what files a package contributed
— call `query_package` first. Don't infer.

### Project conventions are out of scope here

A vibevm project may carry its own additional disciplines (write-
ahead-log protocols, commit-message rules, spec corpus,
PROP/FEAT documents, etc.). Those are NOT part of this skill — they
live in:

- The project's own `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` (if
  present — read those before acting; they're how the project's
  owner expresses local rules).
- Additional skills installed in the project's `.<agent>/skills/`
  or your user-level `~/.<agent>/skills/` dirs.
- Specific packages installed via `vibe install` (for example,
  `flow:wal` ships a write-ahead-log protocol; a project that
  installed it has opted into that protocol; one that didn't,
  hasn't).

If you see custom conventions in the project (new files in `spec/`
you don't recognise, references to a protocol in CLAUDE.md, etc.),
follow what's documented there. This skill doesn't impose any
particular workflow on the project.

## Common — applies to both sections

### `vibe --help` is the source of truth for the CLI

The vibevm CLI evolves between releases. Before suggesting **any**
vibe command, run `vibe <subcommand> --help` and read the actual
current surface. Do not invent flags from training data — they
will be wrong.

Frequently relevant subcommands (all of them have flags worth
checking via `--help`):

- `vibe init` — scaffold a new project (Section A entry point).
- `vibe install <pkgref>` — install one or more packages.
- `vibe update <pkgref>` / `vibe update --all` — version-bump within
  the lockfile's root constraints.
- `vibe outdated [--json]` — preview upstream-newer versions.
- `vibe check [--quiet] [--json]` — spec-consistency linter.
- `vibe show effective | config | features | subskills | purls`
  — pure inspection.
- `vibe list [--kind] [--verbose]` — what's in the lockfile.
- `vibe search <query>` — query registries.
- `vibe registry list | add | remove | set-mirror | vendor | sync
  | publish` — registry CRUD.
- `vibe mcp install | upgrade | uninstall | status | serve` —
  agent integration. Use `--scope project|user|both` and
  `--what mcp|skill|both` to control where and what.

### Pass `--invoked-by <agent>` (recommended)

vibevm stamps a top-level `invoked_by` field on every JSON envelope
for log / telemetry attribution. Pass `--invoked-by <YOUR-AGENT>`
on every CLI invocation:

- `--invoked-by claude-code`
- `--invoked-by claude-desktop`
- `--invoked-by cursor`
- `--invoked-by opencode`
- `--invoked-by codex`

Or set `VIBE_INVOKED_BY=<your-agent>` once for the session — the
CLI reads it whenever the flag is absent. The flag wins on
conflict.

This is **not** required for the commands to function — without
it, envelopes simply have no `invoked_by` field. But the operator
relies on attribution to track which agent proposed which change,
so pass it whenever you can.
