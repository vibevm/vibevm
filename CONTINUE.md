# CONTINUE — cold-resume checkpoint

_Written 2026-05-21. Snapshot for resuming from a fresh context (new
machine, new session, post-compaction)._

> **The WAL is the canonical living state.** `spec/WAL.md` is authoritative;
> if this file and the WAL ever disagree, the WAL wins. This document is a
> convenience snapshot — read `CLAUDE.md`, then `spec/boot/*` in filename
> order, then `spec/WAL.md`, then the relevant PROP docs, before working.

---

## TL;DR

**M1.18 — the PROP-009 loading model — is well advanced.** Phases 1–5
(schema, the `vibedeps/` materialisation tree, the computed-view engine,
boot-artifact generation, and the `vibe install` switch-over) are landed,
**and all five Phase-5 follow-ups are landed too.** The whole workspace
builds and tests green.

**Nothing is blocked.** The next unit of work is **M1.18 Phase 6 —
`vibe reinstall` and published-copy regeneration** (PROP-009 §7 phase 6).
Pick that up first.

Branch `m1.17-workspace`, pushed to `origin`, working tree clean, in sync.
Not merged to `main` — that is the owner's call.

---

## Where work stands

- **Branch:** `m1.17-workspace`. **Pushed**, `origin/m1.17-workspace` in
  sync (0 ahead / 0 behind). Working tree **clean**.
- **Not merged to `main`.** The owner decides when M1.17 + M1.18 merge.
- **Gate is green:**
  - `cargo test --workspace` — all green, **no `--exclude` needed** (the
    old `os error 740` problem is gone — see Findings).
  - `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- Owner authorised committing **and** pushing freely from this branch;
  routine commits land without asking (CLAUDE.md Rule 4). Non-routine red
  lines (force-push, history rewrite, large blobs, CI/signing/secrets,
  irreversible ops) still need explicit owner confirmation.

## Active blocker

**None.** Phase 6 can start immediately.

Owner-facing items that are *not* blockers for Phase 6 but need attention
later:

1. **`VIBEVM-SPEC.md` sanction** — Phase 7 edits `VIBEVM-SPEC.md`
   (migration + docs). That file may not be edited without explicit owner
   sanction. Must be granted *before Phase 7* (not Phase 6).
2. **The `when` contract gap** — PROP-009 §2.3 shows a `when` field on a
   `dynamic` `INDEX.md` entry, but §2.6 pins no manifest field that
   declares it. The renderer (`boot_artifacts.rs`) is `when`-ready but
   always leaves it `None`. Needs an owner decision on where `when` is
   declared. Tracked as "Owner attention §5" in the WAL.
3. **Branch merge** — `m1.17-workspace` carries M1.17 + all of M1.18 so
   far; merging to `main` is the owner's decision.

## Next steps — M1.18 Phase 6 (recipe for a cold start)

**Goal:** `vibe reinstall [<path>] [--force]` and published-copy boot
regeneration. Spec: `spec/modules/vibe-workspace/PROP-009-loading-model.md`
§7 phase 6, §2.10, §2.11.

1. Read `PROP-009` §2.10 (`vibe reinstall`) and §2.11 (published-copy
   regeneration) in full.
2. **`vibe reinstall`** — a new CLI command (`crates/vibe-cli/src/commands/`,
   wire it in `cli.rs` + `commands/mod.rs` + the dispatch in `main.rs`):
   - Without `--force`: recompute the materialised state + boot artifacts
     from the existing `vibe.lock` + the local cache. The boot half is
     already done — reuse `vibe_workspace::install::regenerate_boot(&ws)`
     (it reads every `vibedeps/` slot off disk and regenerates). For the
     materialise half, re-materialise each locked package from its cache
     dir (or just regenerate boot if the slots are already present).
   - With `--force`: re-fetch the subtree from source, then apply.
   - Look at `crates/vibe-cli/src/commands/update.rs` — the scoped-update
     flow (FU3) is the closest existing pattern: discover workspace,
     build the resolver, fetch, `vibedeps::materialise`, `regenerate_boot`.
3. **Published-copy regeneration (§2.11)** — `vibe workspace publish`
   must regenerate each staged member copy's boot artifacts for the
   published, registry-pinned shape. Find the publish staging code
   (`crates/vibe-publish/`, and the `vibe workspace publish` command) and
   call the boot-artifact generation on each staged copy.
4. Gate: `cargo test --workspace` + `cargo clippy --workspace
   --all-targets -- -D warnings` must stay green. Add e2e coverage in
   `crates/vibe-cli/tests/cli_e2e.rs`.
5. Checkpoint the WAL, commit topic-grouped, push.

After Phase 6: **Phase 7** (migration tooling + docs + `VIBEVM-SPEC.md`
edits — needs the owner sanction above), then **Phase 8** (the
effective-spec view — v1.5 scope).

## The loading model in one breath

Two physically separate trees: authored `spec/` (only the human writes it)
and a committed `vibedeps/` (only `vibe` writes it — one slot
`vibedeps/<kind>-<name>/<version>/` per resolved package, the package's
published tree verbatim). The boot sequence is **computed** per node from
the unified resolution: inherited foundation + the node's own boot +
dependency boot + user overrides. `vibe install` generates, per
entry-point node, `spec/boot/INLINE.md` (verbatim concatenation of
`inline`-linked contributions — read first, the priority lane) and
`spec/boot/INDEX.md` (a TOML manifest of `static` paths + `dynamic`
INCLUDE pointers), plus thin `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`
redirects. Three inclusion types — `inline` / `static` / `dynamic` — set
per dependency in `vibe.toml` (`link = …`, default `static`). The `NN-`
filename prefix is retired; `vibe` owns ordering by `[boot_snippet].category`
band (`foundation` → `flow` → `stack` → `user-override`). `[writes]` is
retired. A single-package project is a degenerate one-node workspace.

## Non-obvious findings discovered this session

- **`os error 740` is solved for good.** `cargo test` built a test-harness
  binary named `vibe_install-<hash>.exe`; Windows UAC installer-detection
  refuses to launch any unsigned executable whose name contains `install`.
  It was never Windows Defender (an earlier misdiagnosis). FU5 removed the
  `vibe-install` crate outright, so the binary no longer exists.
  `cargo test --workspace` runs clean with no exclude. If a future crate
  is ever named `*install*`, add `[lib]\ntest = false` or rename it.
- **The manifest `[requires]` is the authority for an update constraint,
  not the lockfile.** `vibe update <pkgref>` re-resolves against the
  constraint in `vibe.toml` `[requires]` (the operator edits it to widen a
  pin); `vibe.lock`'s `meta.root_dependencies` is only a stale mirror /
  fallback. Reading the lockfile first is a bug (it ignores a widened pin).
- **Boot must be regenerated *after* `[requires]` is updated, not before.**
  `apply_resolution` composes each node's dependency boot from its
  `[requires]` closure. `install::run` therefore merges the CLI pkgref
  into `[requires]`, writes the manifest, and re-discovers the workspace
  **before** calling `apply_resolution` — otherwise a fresh `vibe install
  flow:wal` produces an `INDEX.md` missing the new package's own boot.
  (Fixed in `a6e20db`; regression-guarded by `full_install_cycle`.)
- **`vibedeps::materialise` clears the version slot before copying.** Never
  pass a `content_dir` that *is* the slot being materialised — it would
  delete-then-copy-from-nothing. `vibe update`'s scoped path materialises
  from the fresh cache dir, never from an existing slot.
- **Additive → switch → retire.** The whole of M1.18 used this: add the
  new schema/path beside the old, switch consumers over, then delete the
  old. `[writes]` survived additively through Phases 1–4 and was deleted
  only at the Phase-5 switch-over and FU1.
- **`MEMORY.md` at repo root** points at `spec/boot/90-user.md` — the
  user-owned boot snippet. Do not put project facts in a harness's global
  user-memory; they live in the repo (`CLAUDE.md`, `spec/**`).
- **User-owned files vibevm must not edit casually:** `spec/boot/00-core.md`,
  `spec/boot/90-user.md` (owner-owned), `VIBEVM-SPEC.md` (needs sanction).
  `spec/WAL.md` is updated only as a checkpoint.

## Repository map

```
vibevm/                     Rust workspace — the `vibe` CLI
├── CLAUDE.md / AGENTS.md / GEMINI.md   session-boot rules (kept identical)
├── MEMORY.md               pointer to spec/boot/90-user.md
├── CONTINUE.md             this file — cold-resume snapshot
├── VIBEVM-SPEC.md          product spec (owner-sanctioned edits only)
├── Cargo.toml              workspace manifest (12 crates after FU5)
├── crates/
│   ├── vibe-cli/           the `vibe` binary — commands, CLI, dispatch
│   ├── vibe-core/          manifest + lockfile + package types (the schema)
│   ├── vibe-graph/         dependency-graph types
│   ├── vibe-registry/      registry resolution, fetch, CachedPackage,
│   │                       MultiRegistryResolver, mirror dispatch
│   ├── vibe-resolver/      the depsolver (NaiveDepSolver, ResolvedGraph)
│   ├── vibe-llm/           LLM-provider integration
│   ├── vibe-mcp/           the MCP server
│   ├── vibe-check/         `vibe check` — project/manifest validation
│   ├── vibe-publish/       publishing + post-hook index submission
│   ├── vibe-wire/          generated wire/JSON types
│   └── vibe-workspace/     workspace discovery + THE LOADING MODEL:
│       └── src/
│           ├── lib.rs           Workspace::discover / load, node iteration
│           ├── vibedeps.rs      the vibedeps/ materialisation layout
│           ├── boot.rs          compute_effective_boot — the view engine
│           ├── boot_artifacts.rs  render INDEX.md / INLINE.md / redirects
│           └── install.rs       apply_resolution / regenerate_boot /
│                                regenerate_boot_from / prune_stale_slots
├── fixtures/registry/      test-fixture packages (PROP-009 manifest shape)
├── services/vibe-index/    the index service — OUTSIDE the cargo workspace
└── spec/
    ├── boot/               session-boot files (00-core.md … 90-user.md)
    ├── common/             PROP-000 (process), shared FEAT/PROP docs
    ├── modules/            PROP-007 (workspace), PROP-009 (loading model)
    ├── design/             non-normative rationale (loading-and-boot-model)
    └── WAL.md              ← canonical living state. Read this.
```

(`vibe-install` was a crate; FU5 folded its one remaining type,
`InstallError`, into `vibe-cli/src/exit_code.rs` and removed the crate.)

## Architectural / policy decisions in force

- **The four CLAUDE.md rules are non-negotiable every session.** (1) Never
  attribute authorship to any AI/machine system anywhere — commits,
  trailers, branches, code, docs. (2) Conventional Commits. (3) Group
  commits by meaning, one logical unit each. (4) Autonomy on routine
  changes; stop and ask for non-routine (history rewrite, force-push,
  large blobs, CI/signing/secrets, irreversible ops).
- **`~/.vibevm/github.publish.token` is a surface-secret** — never printed
  to stdout/stderr/chat/logs/commits.
- **Project facts live in the repo**, never in a harness's global
  user-memory (a teammate cloning the repo never sees that).
- **PROP-009 loading model** (the spine of M1.18): two trees,
  computed-per-node boot, `inline`/`static`/`dynamic` link types,
  `category` ordering bands, `vibedeps/` slots. `[writes]` and the `NN-`
  prefix are retired.
- **`vibe.lock` schema** stays v4 — the loading model did not bump it.
  `LockedPackage.boot_snippet` (a `String`) and `files_written` (a `Vec`)
  still exist in the schema but are always `None` / empty now.
- **Milestone numbering:** PROP-009 took M1.18; PROP-008 (qualified
  naming) shifted to M1.19.
- **MFBT** ("move fast and break things", PROP-006 §2) — a codeword the
  owner uses to pre-authorise heads-down execution with no mid-work
  confirmations; the four rules still hold, only Rule 4's "ask before
  routine large changes" is suspended.

## Recent commit chain (newest first)

```
1af02b1 docs(wal): checkpoint — Phase 5 follow-ups landed
85dbc9a feat(cli): scope vibe update to the named packages          (FU3)
b313829 refactor(cli): fold the vibe-install crate into vibe-cli    (FU5)
6ec47d2 feat(workspace): prune stale vibedeps/ slots on apply       (FU4)
1a55409 feat(cli): unified resolution across all workspace members  (FU2)
2f42776 refactor(core): retire [writes] and [boot_snippet].filename (FU1)
b4ebd08 docs(wal): checkpoint — M1.18 Phase 5 complete
682e06d test(cli): rewrite the install e2e suite for vibedeps
72b87b9 build(install): disable the vibe-install test harness
a6e20db fix(cli): merge [requires] before regenerating boot
f208050 chore(repo): untrack .claude/settings.local.json
7347208 refactor(install): delete the [writes] mirror-layout path
93fd043 feat(cli): PROP-009 Phase 5 — rework uninstall and update
440a88c feat(cli): PROP-009 Phase 5 — vibe install onto vibedeps
830e8c1 docs(wal): checkpoint — M1.18 Phase 5 underway
f4d45a4 feat(workspace): PROP-009 Phase 5 — install orchestrator
7519d2c docs(wal): checkpoint — M1.18 Phase 4 (boot artifacts)
e06a5ff feat(workspace): PROP-009 Phase 4 — boot artifacts
0c274d4 docs(wal): checkpoint — M1.18 Phase 3 (boot engine)
15dbefe feat(workspace): PROP-009 Phase 3 — boot engine
4e488e1 fix(core): store an explicit link, including "static"
d9f2576 docs(wal): checkpoint — M1.18 Phase 2 (vibedeps/ tree)
e0a8d75 feat(workspace): PROP-009 Phase 2 — the vibedeps/ tree
d9ff8bf docs(wal): checkpoint — M1.18 Phase 1 (schema) landed
ce14877 feat(core): PROP-009 schema — link, boot category, [boot]
6556d24 docs(continue): cold-resume checkpoint — PROP-009/M1.18
```

## Quick-start commands

```sh
# Build / gate the whole workspace
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Focused
cargo test -p vibe-workspace            # the loading-model engine
cargo test -p vibe-cli --test cli_e2e   # the install/uninstall/update e2e
cargo build -p vibe-cli                 # the `vibe` binary

# Git
git status
git log --oneline -25
```

Platform note: Windows / PowerShell. `cargo test --workspace` runs clean
(the `os error 740` issue is resolved — see Findings).
