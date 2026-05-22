# CONTINUE — cold-resume checkpoint

_Written 2026-05-22. Snapshot for resuming from a fresh context (new
machine, new session, post-compaction)._

> **The WAL is the canonical living state.** `spec/WAL.md` is
> authoritative; if this file and the WAL disagree, the WAL wins. Read
> `CLAUDE.md`, then `spec/boot/*`, then `spec/WAL.md`, then the relevant
> PROP docs, before working.

---

## TL;DR

**M1.18 — the PROP-009 loading model — is SHIPPED and merged to
`main`.** This session finished M1.18 Phase 7 (migration + docs), added
**PROP-012** (the managed `<vibevm>` block), tied off three follow-ups,
and merged the whole of M1.17 + M1.18 into `main` via the `--no-ff`
merge commit **`ffd5e1c`**.

`main` is the active branch, pushed to `origin/main`, working tree
clean, the full gate green. **No blocker.** The next unit of work is
the owner's choice — see "What's next".

---

## Where work stands

- **Branch:** `main`, at merge commit `ffd5e1c`, in sync with
  `origin/main`. Working tree clean (`.claude/settings.local.json` is
  git-ignored).
- **`m1.17-workspace`** — the feature branch that carried M1.17 + M1.18.
  Merged into `main` (`--no-ff`, merge commit retained); the branch
  itself is kept, not deleted.
- **Gate green:** `cargo test --workspace`, `cargo clippy --workspace
  --all-targets -- -D warnings`, and `vibe check --path .` all pass.
- M1.18 (PROP-009 loading model + PROP-012 managed block) is shipped;
  M1.17 (Workspace) shipped earlier and is now also on `main`.

---

## What's next — the owner's choice

No blocker; M1.18 is closed. Candidate next units:

1. **PROP-010 / PROP-011 design sessions.** Both are DRAFT proposals —
   `spec/modules/vibe-registry/PROP-010-local-package-cache.md` and
   `spec/modules/vibe-workspace/PROP-011-incremental-install.md` — each
   with a small §5 open-question set. An owner design session closes
   them, then either can be scheduled. PROP-011 (incremental install)
   has **no dependency beyond shipped PROP-009** — it can go first.
   PROP-010 (the local cache) is keyed by PROP-008 identity.
2. **PROP-005 (the package index) → M1.19 / PROP-008 (qualified
   naming).** PROP-008 needs PROP-005 for short-name resolution.
3. **M1.5 — Generation.** The LLM `vibe build` pipeline — the milestone
   that makes vibevm produce code, not just manage specs.
4. **The PROP-009 §2.3 `when` contract gap** — a small design decision
   (see Known issues, below).

---

## The loading model in one breath

Two physically separate trees: authored `spec/` (only the human writes
it) and a committed `vibedeps/` (only `vibe` writes it — one slot
`vibedeps/<kind>-<name>/<version>/` per resolved package, the package's
published tree verbatim). The boot sequence is **computed** per node
from the unified resolution: inherited foundation + the node's own
authored boot + dependency boot + user overrides. `vibe install`
generates, per entry-point node, `spec/boot/INLINE.md` (verbatim
concatenation of `inline`-linked contributions — read first, the
priority lane) and `spec/boot/INDEX.md` (a TOML manifest of `static`
paths + `dynamic` INCLUDE pointers). Three inclusion types — `inline` /
`static` / `dynamic` — set per dependency in `vibe.toml` via `link`
(default `static`). The `NN-` filename prefix and `[writes]` are
retired; `vibe` owns ordering by `[boot_snippet].category` band
(`foundation` → `flow` → `stack` → `user-override`). `CLAUDE.md` /
`AGENTS.md` / `GEMINI.md` are co-tenant files — vibevm owns only a
delimited `<vibevm>` … `</vibevm>` block inside each (PROP-012), never
the whole file. `vibe reinstall [<path>] [--force]` regenerates
without re-resolving. A single-package project is a degenerate
one-node workspace.

---

## Non-obvious findings from this session

- **The redirect write was destructive.** Before PROP-012,
  `vibe-workspace::boot_artifacts::write_boot_artifacts` did an
  unconditional whole-file `fs::write` on `CLAUDE.md` / `AGENTS.md` /
  `GEMINI.md`. Any `vibe install` / `reinstall` on a project with a
  hand-authored instruction file destroyed it. PROP-012's managed
  block is the fix.
- **The managed-block engine** (`boot_artifacts.rs`): `locate_block`
  classifies a file as Absent / WellFormed / Malformed by a plain
  line scan; `write_managed_block` splices the body between markers,
  appends a block after co-tenant content, migrates a file that is
  wholly the old generated redirect, or creates a missing file. A
  malformed file (not exactly one ordered marker pair) is a hard error
  validated at plan time — before any materialisation.
- **`vibe init` now generates the boot artifacts.** It calls
  `Workspace::load` + `install::regenerate_boot` after scaffolding, so
  a freshly-`init`ed project has `spec/boot/INDEX.md` and the
  `<vibevm>` blocks — bootable at once. `vibe install` with no
  `[requires]` still bails before `apply_resolution`; `vibe reinstall`
  is the depless-regeneration path.
- **`vibe check` had two loading-model-stale checks**, both reworked
  this session: `boot_directory` (was enforcing the retired `NN-`
  pattern) and `lockfile_files` / check 8 (was verifying `files_written`
  and `spec/flows` orphans — meaningless under the loading model; now
  verifies `vibedeps/` slot consistency). New `CheckId::RedirectBlock`
  reports a malformed `<vibevm>` block. `vibe-check` stays
  dependency-light — the marker scan is duplicated rather than pulling
  in a `vibe-workspace` dependency (the `numeric_prefix` precedent).
- **The merge:** `m1.17-workspace` was 64 commits ahead of `main`;
  `git merge --no-ff` produced merge commit `ffd5e1c`.

---

## Repository map

```
vibevm/                     Rust workspace — the `vibe` CLI (12 crates)
├── CLAUDE.md / AGENTS.md / GEMINI.md   hand-authored rules + a managed
│                                       <vibevm> boot block (PROP-012)
├── MEMORY.md               pointer to spec/boot/90-user.md
├── CONTINUE.md             this file — cold-resume snapshot
├── ROADMAP.md              milestone roadmap (M0 … M1.22, M1.5, M2, M3+)
├── CHANGELOG.md            shipped-change log (M1.18 entry is current)
├── VIBEVM-SPEC.md          product spec (owner-frozen; §6 = the loading model)
├── Cargo.toml              workspace manifest
├── crates/
│   ├── vibe-cli/           the `vibe` binary — install / uninstall /
│   │                       update / reinstall / workspace / registry / …
│   ├── vibe-core/          manifest + lockfile + package types (schema)
│   ├── vibe-graph/         dependency-graph types
│   ├── vibe-registry/      registry resolution, fetch, the registry cache
│   ├── vibe-resolver/      the depsolver (NaiveDepSolver, ResolvedGraph)
│   ├── vibe-llm/           LLM-provider integration (M1.5 scope)
│   ├── vibe-mcp/           the MCP server
│   ├── vibe-check/         `vibe check` — project/manifest validation
│   ├── vibe-publish/       publishing + post-hook index submission
│   ├── vibe-wire/          generated wire/JSON types
│   └── vibe-workspace/     workspace discovery + THE LOADING MODEL:
│       └── src/
│           ├── lib.rs           Workspace::discover / load
│           ├── vibedeps.rs      the vibedeps/ materialisation layout
│           ├── boot.rs          compute_effective_boot — the view engine
│           ├── boot_artifacts.rs  INDEX.md / INLINE.md + the <vibevm> block
│           ├── install.rs       apply_resolution / regenerate_boot
│           └── publish.rs       select / topo_order / stage_node
├── vibedeps/               materialised dependencies (committed) — absent
│                           in this repo: vibevm has no [requires]
├── docs/                   user docs — incl. docs/loading-model.md and
│                           docs/commands/reinstall.md (new this session)
├── fixtures/registry/      test-fixture packages
├── services/vibe-index/    the index service — OUTSIDE the cargo workspace
└── spec/
    ├── boot/               00-core.md, 90-user.md (authored) + the
    │                       generated INDEX.md
    ├── common/             PROP-000 (process), PROP-004 / PROP-006, …
    ├── modules/            per-crate PROPs — see spec/modules/README.md:
    │   ├── vibe-registry/  PROP-001, PROP-002, PROP-008, PROP-010
    │   ├── vibe-resolver/  PROP-003
    │   ├── vibe-index/     PROP-005
    │   └── vibe-workspace/ PROP-007, PROP-009, PROP-011, PROP-012
    ├── design/             non-normative rationale
    └── WAL.md              ← canonical living state. Read this.
```

---

## Architectural / policy decisions in force

- **The four CLAUDE.md rules are non-negotiable every session.** (1)
  Never attribute authorship to any AI/machine system anywhere. (2)
  Conventional Commits. (3) Group commits by meaning, one logical unit
  each. (4) Autonomy on routine changes; stop and ask for non-routine
  (history rewrite, force-push, large blobs, CI/signing/secrets,
  irreversible ops).
- **`~/.vibevm/github.publish.token` is a surface-secret** — never
  printed to stdout/stderr/chat/logs/commits.
- **Project facts live in the repo**, never in a harness's global
  user-memory.
- **PROP-009 loading model** (shipped): two trees, computed-per-node
  boot, `inline`/`static`/`dynamic` link types, `category` ordering
  bands, `vibedeps/` slots. `[writes]` and the `NN-` prefix are retired.
- **PROP-012 managed block** (shipped): `CLAUDE.md` / `AGENTS.md` /
  `GEMINI.md` are co-tenant files; vibevm owns only the `<vibevm>`
  block. Exactly one block per file; a malformed file is a hard error
  validated before any mutation; the block's position is the user's.
- **`vibe install` is whole-workspace, location-independent**;
  resolution is unified (one `vibe.lock`, one version per package).
- **`vibe.lock` schema stays v4** — `LockedPackage.boot_snippet` and
  `files_written` still exist in the schema but are unused (empty /
  `None`) under the loading model.
- **PROP-010 (DRAFT)** — the local package cache: machine-global,
  accretive, identity-keyed; an `--offline` flag; a user-level default
  registry config. Five §5 open questions.
- **PROP-011 (DRAFT)** — incremental install: skip the depsolver when
  `vibe.lock` is fresh; materialise only changed slots. Three §5 open
  questions.
- **Milestone numbering:** M1.18 = PROP-009 (shipped), M1.19 = PROP-008
  (qualified naming, needs PROP-005), M1.20 = PROP-010, M1.21 =
  PROP-011, M1.22 = PROP-012 (shipped within M1.18; the number is
  nominal).
- **MFBT** ("move fast and break things", PROP-006 §2) — a codeword the
  owner uses to pre-authorise heads-down execution; the four rules
  still hold.

---

## Recent commit chain (newest first)

```
ffd5e1c Merge branch 'm1.17-workspace' — M1.17 Workspace + M1.18 Loading model
56d7a5f docs(boot): update 00-core.md for the loading model
f35c557 fix(check): align vibe check with the loading model
09592af docs: PROP-009 loading-model docs sweep
2028699 docs: register M1.18 as shipped
bcb09fe docs(spec): VIBEVM-SPEC.md — the PROP-009 loading model
2981970 chore(boot): migrate vibevm to the PROP-009 loading model
55f24cd fix(workspace): PROP-012 — the managed <vibevm> block
ee117f4 fix(check): retire the NN- boot-directory enforcement
651a57d docs(spec): PROP-012 — settle the marker syntax
78d9613 docs(spec): PROP-012 — the managed redirect block
ace6a69 docs(wal): session-end checkpoint
58d43f7 docs(continue): cold-resume checkpoint — M1.18 Phase 6 done
987e4d4 docs: register PROP-010 / PROP-011 and align milestone numbers
040c8c3 docs(spec): PROP-011 — incremental install
9069f13 docs(spec): PROP-010 — the local package cache
95a0498 docs(wal): checkpoint — M1.18 Phase 6 complete
0706ae2 feat(workspace): regenerate published-copy boot artifacts
4606132 feat(cli): vibe reinstall — regenerate the loading model
50c2a43 docs(continue): cold-resume checkpoint — M1.18 Phase 5 done
1af02b1 docs(wal): checkpoint — Phase 5 follow-ups landed
85dbc9a feat(cli): scope vibe update to the named packages
b313829 refactor(cli): fold the vibe-install crate into vibe-cli
6ec47d2 feat(workspace): prune stale vibedeps/ slots on apply
1a55409 feat(cli): unified resolution across all workspace members
```

The session-end checkpoint commits (this `CONTINUE.md` and the
`spec/WAL.md` refresh) land on top of `ffd5e1c`.

---

## Quick-start commands

```sh
# Build / gate the whole workspace
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Focused
cargo test -p vibe-workspace            # the loading-model engine
cargo test -p vibe-cli --test cli_e2e   # install/uninstall/update/reinstall e2e
cargo run -q -p vibe-cli -- check --path .   # lint this repo
cargo run -q -p vibe-cli -- reinstall --assume-yes   # regenerate boot artifacts

# Git
git status
git log --oneline -25
```

Platform note: Windows / PowerShell. `cargo test --workspace` runs
clean — no `--exclude` needed.
