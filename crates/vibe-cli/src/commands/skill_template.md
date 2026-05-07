---
name: vibevm
description: Use whenever the user mentions vibevm, vibe install/update/check/show/outdated/registry/search/init, packages, subskills, lockfile, spec corpus, or anything in spec/ or packages/. ALSO when the user asks to create a new vibevm-managed project — this skill covers both bootstrapping new projects and working inside existing ones. The MCP server exposes lockfile and subskill content; query it before guessing about installed packages, features, or describe-PURL bindings.
---

# vibevm

This skill activates whenever vibevm is in scope. Behaviour depends
on whether you are inside a vibevm-managed project or not — detect
first, then follow the matching section.

## First step: detect the situation

Before anything else, check whether `vibe.toml` exists in the working
directory. The MCP server exposes this via `query_package` failures
on a fresh project, but the cheapest probe is a simple file check
(`ls vibe.toml`, `Test-Path vibe.toml`, or whatever your tooling
provides).

- **`vibe.toml` exists** → you are inside an existing vibevm project.
  Jump to **Section B**.
- **`vibe.toml` is absent** → you are bootstrapping. Jump to
  **Section A**.

If the user tells you what they want explicitly ("create a new vibevm
project", "set up vibevm here", "I have a vibevm project") — trust
the user and skip the probe.

## Section A — bootstrap mode (no `vibe.toml` here)

The user wants to start a new vibevm-managed project from scratch.
Recommended flow:

1. **Run `vibe init`** in the target directory. This creates:
   - `vibe.toml` — project manifest with `[[registry]]` blocks
     pointing at GitHub `vibespecs` (primary) and GitVerse
     `vibespecs` (secondary).
   - `vibe.lock` — empty lockfile.
   - `spec/boot/00-core.md` + `spec/boot/90-user.md` — boot snippets
     read at every session start.
   - `spec/WAL.md` — project-state checkpoint (write-ahead log).
   - `CLAUDE.md`, `AGENTS.md`, `GEMINI.md` — cross-agent rule files
     (identical copies of the four non-negotiable rules).

2. **Choose what to install.** Common starter packages from the
   `vibespecs` registry (`https://github.com/vibespecs`):
   - `flow:wal` — write-ahead-log discipline. Canonical first install.
   - `flow:sync-from-code` — protocol for syncing code-side changes
     back to the spec.
   - `flow:atomic-commits` — atomic commit discipline.
   - Discover more via `vibe search <query>` once a registry is
     configured.

3. **Run `vibe install <pkgref>`** for each chosen package. The
   lockfile gets populated; subskills materialise into
   `spec/flows/...`, `spec/feats/...`, `spec/stacks/...` per the
   package manifest. Pass `--invoked-by <YOUR-AGENT>` (see Common
   below) on every invocation.

4. **Optionally land a project-scope vibevm skill** so future sessions
   in the same repo get the contract committed to git rather than
   relying on user-level config:
   ```
   vibe mcp install --scope project --agent <YOUR-AGENT> --what skill --invoked-by <YOUR-AGENT>
   ```

5. **Switch to Section B** — the project now exists; from here, treat
   the rules in Section B as binding.

## Section B — inside an existing vibevm project

The moment you confirm `vibe.toml` is present in CWD, the rules below
are non-negotiable.

### Bootstrap protocol (do this first, every session)

1. Read `CLAUDE.md` (or its identical copy `AGENTS.md`) at session
   start. The four non-negotiable rules there govern every action
   you take in this repository.
2. Read every file under `spec/boot/` in filename order, then
   `spec/WAL.md` for the current project state.
3. Consult relevant `PROP-*` / `FEAT-*` documents under
   `spec/common/` and `spec/modules/` for the task at hand.

Do not skip steps 1–3. The repository is spec-driven; ignoring the
spec is how regressions land.

### Use the MCP server, do not guess

The vibevm MCP server is registered in your MCP config as `vibevm`.
Tools it exposes:

- **`query_package(name)`** — returns the lockfile entry for an
  installed package: kind, name, version, content_hash, registry,
  source_url, source_ref, resolved_commit, files_written, features,
  subskills_active with delivery + describes, top-level describes
  PURL, language. Call this **before** asking the user about
  installed package versions, features, or PURL bindings.
- **`read_subskill(package, subskill_path)`** — returns the textual
  content of an active subskill's files. For `lazy-pull` deliveries
  the bytes come from the package cache without writing to the
  project tree.
- **`materialise_subskill(package, subskill_path, force?)`** —
  promotes a `lazy-pull` subskill into the project tree on demand.
  Refuses to overwrite existing files unless `force=true`.

Do not guess versions, features, or `describes` from filenames or
commit history. The lockfile is canonical.

## Common — applies to both sections

### Always pass `--invoked-by`

When you invoke any `vibe` CLI subcommand, **you must pass
`--invoked-by <YOUR-AGENT>`** so vibevm can attribute the work in
its JSON envelopes and structured logs. Pick the value matching your
identity:

- `--invoked-by claude-code`
- `--invoked-by claude-desktop`
- `--invoked-by cursor`
- `--invoked-by opencode`
- `--invoked-by codex`

Alternatively set `VIBE_INVOKED_BY=<your-agent>` once for the
session — the CLI reads it whenever the flag is absent. The flag
wins on conflict.

This is not optional. Logs and envelopes without `invoked_by` are
ambiguous, and the operator relies on the field to track which agent
proposed which change.

### Read `--help` before suggesting commands

`vibe --help` is **long** and the subcommands have many flags that
matter. **Before suggesting any vibe command to the user**, run
`vibe <subcommand> --help` and read the actual current surface. Do
not invent flags from memory; vibevm evolves and the CLI changes
between sessions.

The most relevant subcommands you will likely need:

- `vibe init` — scaffold a new vibevm project (Section A entry point).
- `vibe install <pkgref>` — flags: `--features`, `--language`,
  `--no-default-features`, `--all-features`, `--registry`,
  `--assume-yes`.
- `vibe update <pkgref>` / `vibe update --all` — version-bump within
  the lockfile's root constraints. Refuses on user-edited files.
- `vibe outdated [--json]` — read-only preview of upstream-newer
  versions for installed packages.
- `vibe check [--quiet] [--json] [--wal-max-age-hours]
  [--review-max-age-days]` — spec-consistency linter. Run before
  any PR.
- `vibe show effective` / `vibe show config` / `vibe show features`
  / `vibe show subskills` / `vibe show purls` — pure inspection.
- `vibe list [--kind <flow|feat|stack|tool>] [--verbose]`.
- `vibe search <query>` — flags: `--purl`, `--kind`, `--registry`,
  `--full-scan`, `--no-cache`, `--cache-ttl`.
- `vibe registry list` / `add` / `remove` / `set-mirror` / `vendor`
  / `sync` / `publish`.
- `vibe mcp install` / `upgrade` / `uninstall` / `status` / `serve`
  — manage agent integrations. Pass `--scope project|user|both` and
  `--what mcp|skill|both` to control where and what.

For every flag you intend to use, verify it still exists by reading
`vibe <subcommand> --help`. The CLI's truth is its own help text,
not training data.

### Ask before destructive action

The four non-negotiable rules in `CLAUDE.md` / `AGENTS.md` require
you to stop and confirm with the user before:

- rewriting published history (rebase of pushed commits, amending
  pushed work),
- force-pushing or `--force-with-lease`,
- adding large binary blobs,
- changing CI / signing / secrets configuration,
- any operation whose reversal costs work.

Honour those rules regardless of which section you are operating in.
They apply equally during bootstrap (Section A) and inside an
existing project (Section B).

### When in doubt

- Re-read the relevant section of `VIBEVM-SPEC.md` and `spec/`.
- If still unclear inside an existing project, mark the decision with
  `<!-- REVIEW: YYYY-MM-DD … -->`, pick the conservative
  interpretation, proceed, and surface the decision in your end-of-
  session report.
- During bootstrap, when no project exists yet, ask the user
  explicitly rather than guess project-shape decisions.
- Never silently invent semantic behaviour.
