---
name: vibevm
description: Use whenever the workspace contains a `vibe.toml` file. Triggers when the user mentions vibevm, vibe install/update/check/show/outdated/registry/search, packages, subskills, lockfile, spec corpus, or anything in spec/ or packages/. The MCP server exposes lockfile and subskill content; query it before guessing about installed packages, features, or describe-PURL bindings.
---

# vibevm

This workspace is managed by **vibevm** — a CLI software project manager
for spec-driven AI-assisted development. Treat the rules below as
non-negotiable when you operate in this repository.

## Bootstrap protocol (do this first, every session)

1. Read `CLAUDE.md` (or its identical copy `AGENTS.md`) at session start.
   The four non-negotiable rules there govern every action you take in
   this repository.
2. Read every file under `spec/boot/` in filename order, then `spec/WAL.md`
   for the current project state.
3. Consult relevant `PROP-*` / `FEAT-*` documents under `spec/common/`
   and `spec/modules/` for the task at hand.

Do not skip steps 1–3. The repository is spec-driven; ignoring the spec
is how regressions land.

## Use the MCP server, do not guess

The vibevm MCP server is registered in your MCP config as `vibevm`.
Tools it exposes:

- **`query_package(name)`** — returns the lockfile entry for an
  installed package (kind, name, version, content_hash, registry,
  source_url, source_ref, resolved_commit, files_written, features,
  subskills_active with delivery + describes, top-level describes PURL,
  language). Call this **before** asking the user about installed
  package versions, features, or PURL bindings.
- **`read_subskill(package, subskill_path)`** — returns the textual
  content of an active subskill's files. For `lazy-pull` deliveries
  the bytes come from the package cache without writing to the project
  tree.
- **`materialise_subskill(package, subskill_path, force?)`** — promotes
  a `lazy-pull` subskill into the project tree on demand. Refuses to
  overwrite existing files unless `force=true`.

Do not guess versions, features, or `describes` from filenames or commit
history. The lockfile is canonical. If you need information about an
installed package, the MCP server has it.

## Always pass `--invoked-by`

When you invoke any `vibe` CLI subcommand, **you must pass
`--invoked-by <YOUR-AGENT>`** so vibevm can attribute the work in its
JSON envelopes and structured logs. Pick the value that matches your
identity:

- `--invoked-by claude-code`
- `--invoked-by claude-desktop`
- `--invoked-by cursor`
- `--invoked-by opencode`
- `--invoked-by codex`

Alternatively set `VIBE_INVOKED_BY=<your-agent>` once for the session —
the CLI reads it whenever the flag is absent. The flag wins on
conflict.

This is not optional. Logs and envelopes without `invoked_by` are
ambiguous, and the operator relies on the field to track which agent
proposed which change.

## Read `--help` before suggesting commands

`vibe --help` is **long** and the subcommands have many flags that
matter. Before suggesting any vibe command to the user, run
`vibe <subcommand> --help` and read the actual current surface. Do not
invent flags from memory; vibevm evolves and the CLI changes between
sessions.

The most relevant subcommands you will likely need:

- `vibe install <pkgref>` — flags: `--features`, `--language`,
  `--no-default-features`, `--all-features`, `--registry`,
  `--assume-yes`. Per `--help`.
- `vibe update <pkgref>` / `vibe update --all` — version-bump within
  the lockfile's root constraints. Refuses on user-edited files.
- `vibe outdated [--json]` — read-only preview of upstream-newer
  versions for installed packages.
- `vibe check [--quiet] [--json] [--wal-max-age-hours]
  [--review-max-age-days]` — spec-consistency linter. Run before any
  PR.
- `vibe show effective` / `vibe show config` / `vibe show features` /
  `vibe show subskills` / `vibe show purls` — pure inspection.
- `vibe list [--kind <flow|feat|stack|tool>] [--verbose]`
- `vibe search <query>` — flags: `--purl`, `--kind`, `--registry`,
  `--full-scan`, `--no-cache`, `--cache-ttl`.
- `vibe registry list` / `add` / `remove` / `set-mirror` / `vendor` /
  `sync` / `publish`.
- `vibe mcp serve --path <project>` — already wired into your MCP
  config; do not invoke manually unless debugging.
- `vibe mcp install` / `vibe mcp status` — manage agent integrations.

For every flag you intend to use, verify it still exists by reading
`vibe <subcommand> --help`. The CLI's truth is its own help text, not
training data.

## Ask before destructive action

The four non-negotiable rules in `CLAUDE.md` / `AGENTS.md` require you
to stop and confirm with the user before:

- rewriting published history (rebase of pushed commits, amending
  pushed work),
- force-pushing or `--force-with-lease`,
- adding large binary blobs,
- changing CI / signing / secrets configuration,
- any operation whose reversal costs work.

Honour those rules. They apply to every action you take in this
repository, regardless of whether you reach the binary through the
MCP server, the shell, or your own tool layer.

## When in doubt

- Re-read the relevant section of `VIBEVM-SPEC.md` and `spec/`.
- If still unclear, mark the decision with `<!-- REVIEW: YYYY-MM-DD … -->`,
  pick the conservative interpretation, proceed, and surface the
  decision in your end-of-session report.
- Never silently invent semantic behaviour.
