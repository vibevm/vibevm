# fractality — workspace contract (read this first)

**Project:** fractality — an agent operating system in its earliest form: a
mission-control scheduler plus a delegation toolchain that lets an expensive
"boss" agent hand tasks to swarms of cheap worker agents running in isolated
Claude Code processes under other model providers, exchanging everything
through files on disk.
**Binaries (planned):** `fractality` (CLI), `fractality-mission-control` (the scheduler daemon).
**Status:** pre-code. The IGNITION campaign plan is authored; Phase 0 (spikes) is next.

This is a **workspace** inside the vibevm repository (host registry:
`WORKSPACES.md` at the repo root; canon: `flow:org.vibevm/wal-workspaces`) —
but an independent product. It does not depend on vibevm; vibevm does not
depend on it. The repository is only its incubator.

## Session boot sequence

1. The host's Rules 1–4 (root `CLAUDE.md`) are repo-wide and bind every
   commit made here: human-authored surface, Conventional Commits, commits
   grouped by meaning, autonomy on routine work only.
2. This file, end to end.
3. `WAL.md` (this directory) — the living project state. Canonical.
4. `CONTINUE.md` (this directory) — the cold-resume snapshot; the WAL wins
   wherever they diverge.
5. The active plan the WAL names. Today:
   [`fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md`](fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md).

Do **not** load the host's `spec/boot/`, `spec/WAL.md`, or host specs, and do
not scan the host tree — every host fact this project needs is recorded in
the plan's §5 (current-state facts) or here. If a task genuinely crosses into
the host project, say so before touching host files.

## Hard conventions

- **Language:** Rust. Each code-bearing package version dir is its **own
  Cargo workspace** (the host root workspace excludes `packages/`), starting
  with `fractality/v0.1.0/`. No Python, no Node; shell only as thin
  launchers when unavoidable.
- **Artifacts in English** (code, specs, docs, commit messages); chat with
  the owner in Russian.
- **Commit scope:** `fractality` (e.g. `feat(fractality): …`), regardless of
  which crate inside the workspace changed. The workspace is one subsystem
  from the host's point of view.
- **Floor (gate panel), from Phase 1 on:** run inside `fractality/v0.1.0/`:
  `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
  Green at every phase boundary (safe-stop law). Until crates exist, the
  floor is "host `bash tools/self-check.sh` stays green".
- **Secrets:** never read, print, or log token files
  (`~/.vibevm/zai.api.token` and siblings). Code reads them at spawn time
  only and never echoes them; tests use fakes. One accidental echo is a leak
  (sessions are screen-recorded).
- **Clean-room law:** every reference source in
  [`fractality/v0.1.0/spec/refs/INVENTORY.md`](fractality/v0.1.0/spec/refs/INVENTORY.md)
  is inspiration-only. Study → write a study note (what it achieves, which
  decisions we take) → implement from the note. Never port lines, never
  adapt code file-by-file. This binds the whole workspace.
- **Worker-env invariant (security):** a spawned worker's environment is
  constructed from a whitelist plus its profile — it never inherits
  `ANTHROPIC_*` / `CLAUDE_*` from the parent. Tests enforce this; treat any
  weakening as a review point for the owner.
- **Machine quirks (this box, inherited from the host):** edits via editor
  tools only (PowerShell 5.1 corrupts UTF-8-no-BOM round-trips); commits via
  `git commit -F - <<'MSG'` heredoc only; bash through Git Bash, not WSL;
  never name a test binary `*install*` (Windows UAC blocks it).

## End of session

Rewrite `WAL.md` to the current state (checkpoint, not journal). On a
wind-down phrase naming this workspace (`заверши сессию fractality` /
`END SESSION fractality`): also overwrite `CONTINUE.md` wholesale and refresh
the fractality status line in the host `WORKSPACES.md`. Commit per host
rules; push via `cargo xtask mirror` from the host root (routine per Rule 4).
Resume (`восстанови сессию fractality`) is report-then-wait: restore, verify
empirically, report, stop.
