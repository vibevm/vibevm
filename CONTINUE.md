# CONTINUE.md — cold-resume checkpoint

_Written 2026-05-22 at session end. `main` @ `b84e61a`, even with `origin/main`._

> **`spec/WAL.md` is the canonical living state.** If this snapshot and the
> WAL ever disagree, the WAL wins — it is refreshed every session; this file
> is a point-in-time cold-start aid.

---

## TL;DR

This session closed **PROP-005 — the package index** end to end.

A state review found PROP-005 was *not* pending work: the `vibe-index`
server + CLI (slices 1–8), the publisher hook and consumer fast path
(slices 9–10), and M2.10 `vibe search` had all shipped in earlier
sessions. But `vibe-index` lived in its own Cargo workspace under
`services/`, outside the routine `cargo test --workspace` gate, and it had
**silently rotted** against the M1.17 / M1.18 manifest-schema churn — its
hand-duplicated `vibe.toml` parser could no longer parse a current
manifest, and its test suite was red.

Three pieces of work, all on `main`, all pushed:

1. **De-rot** — realigned the scanner with the current `vibe.toml` schema,
   refreshed the golden fixture + parity hash, hardened the gate, retired
   dead slice-1 scaffolding.
2. **The fold** (owner's call) — moved `services/vibe-index/` →
   `crates/vibe-index/` as a workspace member; rewrote the scanner to parse
   through `vibe-core`'s own `Manifest` / `SubskillManifest`. The duplicated
   parser is **deleted** — the index schema can no longer drift.
3. **Formatting** — `cargo fmt --all` across the whole workspace (69 files
   of accumulated drift) and a new `cargo fmt --check` step in
   `tools/self-check.sh`.

**No active blocker.** Everything is committed, pushed, and green.

**Next milestone:** PROP-008 (M1.19 — qualified naming). Its PROP-005
dependency is now satisfied.

---

## Where work stands

- **Branch:** `main`, at `b84e61a`, **even with `origin/main`** (0 ahead,
  0 behind). Working tree clean.
- **Gate:** `bash tools/self-check.sh` is green on all four steps —
  `cargo fmt --all --check`, `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `vibe check --path .` (0 errors / 0 warnings / 0 info).
- **This session's 11 commits:** `c1f0a26` … `b84e61a` (see the commit
  chain below).

---

## Active blocker

None. PROP-005 is complete; the tree is clean and green; nothing awaits a
human action.

(Carried, non-blocking, owner-only: delete the stale
`https://gitverse.ru/vibespecs/vibevm-direct-push-smoke` repo via the
GitVerse web UI — no API DELETE endpoint exists.)

---

## Next steps — exact recipe

The base-machinery-first plan (WAL "Next") puts **PROP-008 — qualified
naming (M1.19)** next. PROP-005 is done, so its dependency
(index-backed short-name resolution) is satisfied.

To pick it up cold:

1. Run the boot sequence (`CLAUDE.md` → `spec/boot/` in filename order →
   `spec/WAL.md`).
2. Read `spec/modules/vibe-registry/PROP-008-qualified-naming.md` (the
   design lock — requirements locked 2026-05-20, implementation pending)
   and `spec/design/workspace-and-qualified-naming.md` (the design lore).
3. Scope per `ROADMAP.md` "M1.19": mandatory `[package].group`
   (reverse-FQDN); identity tuple becomes `(group, name, version,
   content_hash)`, `kind` leaves identity; pkgref grammar
   `[kind:][group/]name[@version]`; `naming = "fqdn"` repo names;
   index-backed short-name resolution; collision detection with new exit
   code `7`; migration of the three canonical packages to
   `group = "org.vibevm"`.
4. PROP-008 is a large milestone — a fresh session is the clean way in.

**Smaller well-scoped alternative** (tracked, not blocking): lower
`compute_content_hash` out of `vibe-registry` into `vibe-core`. That would
let `crates/vibe-index/` drop its last real duplicate
(`src/content_hash.rs` + `tests/content_hash_parity.rs`) and would unblock
PROP-011's deferred `slot_integrity = verify` content-hash spot-check.

---

## Non-obvious findings (this session)

- **A separate Cargo workspace escapes `cargo test --workspace`.**
  `services/vibe-index/` had its own `[workspace]` table, so the routine
  gate never touched it and it rotted unnoticed for a full milestone. The
  fold fixed this at the root — it is now a `crates/` member.
- **`cargo fmt` was never gated.** `tools/self-check.sh` checked test /
  clippy / `vibe check` but not formatting, so rustfmt drift accumulated
  workspace-wide (69 files). Now `cargo fmt --all --check` is `self-check`
  step 1.
- **`.gitattributes` pins `* text=auto eol=lf`.** Every text file is LF in
  every checkout on every OS — content hashes are cross-platform stable.
  Matters whenever you touch `fixtures/` or `compute_content_hash`.
- **`vibe-core` ref types canonicalise versions.** `CapabilityRef` /
  `PackageRef` read a bare `@0.3.0` as caret `@^0.3.0` (Cargo semantics).
  The index now records canonicalised capability / pkgref strings — the
  `scanner_e2e` regression test was updated to expect this.
- **`vibe-core::PackageKind` lacks `Ord` + `clap::ValueEnum`.** That is why
  `crates/vibe-index/src/types/kinds.rs` keeps its *own* `PackageKind` /
  `NamingConvention` (the index needs them as sort keys and CLI value
  enums). The scanner converts `vibe-core`'s kind to the index's with a
  total `match`. This duplication is deliberate (PROP-005 §3.2) — the four
  kinds are frozen by `VIBEVM-SPEC.md` §4.
- **`compute_content_hash` stays duplicated** (`crates/vibe-index/src/content_hash.rs`
  ↔ `vibe-registry`), guarded by `tests/content_hash_parity.rs`. The
  parity `GOLDEN` constant is `sha256:f2af92b8…f119e7` for
  `fixtures/registry/flow/wal/v0.1.0/`.
- **This machine (Windows):** the PowerShell `Remove-Item` is sandbox-
  blocked on some repo paths — use the Bash tool's `rm`, or copy files
  individually.

---

## Repository map

```
vibevm/
├── CLAUDE.md / AGENTS.md / GEMINI.md   the four rules + boot directive (identical)
├── VIBEVM-SPEC.md                      owner-frozen implementation spec
├── ROADMAP.md  CHANGELOG.md  CONTINUE.md
├── Cargo.toml                          workspace root — members, shared deps, profiles
├── crates/
│   ├── vibe-core        core types: PackageRef/PackageKind/CapabilityRef,
│   │                    the unified `Manifest`, Lockfile, Purl, i18n
│   ├── vibe-cli         the `vibe` binary — every subcommand
│   ├── vibe-registry    git-backed registry, multi-registry resolver,
│   │                    `IndexClient` (consumer index fast path), compute_content_hash
│   ├── vibe-resolver    dependency resolution — depsolver, features, activation
│   ├── vibe-workspace   workspace discovery, the loading model (boot /
│   │                    boot_artifacts), the install orchestrator, vibedeps, freshness
│   ├── vibe-publish     publishing to GitHub / GitVerse, the post-publish index hook
│   ├── vibe-check       the spec linter (`vibe check`)
│   ├── vibe-index       the package index utility — server + CLI.
│   │                    NEW workspace member this session (was services/vibe-index/)
│   ├── vibe-mcp         MCP server
│   ├── vibe-graph       task graph
│   ├── vibe-llm         LLM provider integration (M1.5 — deferred)
│   └── vibe-wire        JTD-generated wire types (src/generated/)
├── xtask/               build / maintenance tasks
├── spec/
│   ├── boot/            00-core.md, 90-user.md (authored) + generated INDEX.md
│   ├── common/          PROP-000 (process), PROP-004 (research), PROP-006 (modes)
│   ├── modules/         per-crate PROPs (vibe-registry / vibe-resolver /
│   │                    vibe-index / vibe-workspace)
│   ├── research/  design/
│   └── WAL.md           the canonical living checkpoint
├── docs/                user-facing docs (commands/, loading-model.md, …)
├── fixtures/registry/   hermetic test-fixture packages
├── manual-tests/        operator smoke recipes
├── tools/               self-check.sh, jtd-codegen
└── refs/                the owner's book + reference sources (read-only)
```

---

## Architectural / policy decisions in force

- **The four rules** (`CLAUDE.md`, authoritative `PROP-000 §12`): keep the
  repo human-authored (no AI attribution anywhere); Conventional Commits
  with a *why*-explaining body; group commits by meaning; autonomy on
  routine work only — stop and ask for history rewrites, force-push, large
  blobs, CI / signing / secrets, anything costly to reverse.
- **Language Rust, manifests TOML.** One `vibe.toml` per node; the role is
  set by section (`[project]` ⊕ `[package]`, optional `[workspace]`).
  Lockfile `vibe.lock`. Four installable kinds only: `flow` / `feat` /
  `stack` / `tool`.
- **Loading model (PROP-009, M1.18).** Two physically separate trees —
  authored `spec/` and a committed `vibedeps/`. The boot sequence is
  *computed* per node and projected into `spec/boot/INLINE.md` +
  `INDEX.md`. `vibe` owns one `<vibevm>` block inside `CLAUDE.md` /
  `AGENTS.md` / `GEMINI.md` (PROP-012).
- **Decentralised registry (PROP-002).** Git-as-registry; content-hash
  identity; `[[registry]]` / `[[mirror]]` / `[[override]]`; redirect stubs.
- **Incremental install (PROP-011, M1.21).** `vibe install` is
  lockfile-respecting — skips the depsolver when `vibe.lock` is fresh,
  materialises only the changed `vibedeps/` slots.
- **The package index (PROP-005).** Opt-in everywhere; the index is a
  *derived hot cache* — package repos stay authoritative, reality wins
  over the index, `content_hash` is verified at fetch time regardless.
- **`vibe-index` is a `crates/` workspace member** (this session). This
  *reverses* PROP-005 §6's original standalone-workspace decision — see
  PROP-005 §3.2 / §6 / §9 item 11 for the full rationale.
- **Split-host posture.** vibevm source on GitVerse
  (`git@gitverse.ru:anarchic/vibevm.git`); the package registry org on
  GitHub (`github.com/vibespecs`). See `spec/boot/90-user.md`.
- **M1.5 (LLM generation) is deferred.** Base-machinery-first: stabilise
  the package machinery — tests, refactor-readiness — before layering any
  generation on top.

---

## Recent commit chain (newest first)

```
b84e61a build(self-check): gate cargo fmt --check
8cdbb65 style: apply rustfmt across the workspace
bbfc89d docs(wal): checkpoint the vibe-index fold
28172c5 docs(spec): reconcile PROP-005 and docs with the vibe-index fold
ea7e4d8 refactor(vibe-index): fold the crate into the workspace
ac5ce1d docs(changelog): record the PROP-005 package index milestone
5c4cc66 docs(wal): checkpoint the PROP-005 de-rot
40c9e0f docs(spec): reconcile PROP-005 and ROADMAP with the shipped index
9e3ee85 style(vibe-index): apply rustfmt across the standalone workspace
455795d refactor(vibe-index): retire the slice-1 skeleton scaffolding
c1f0a26 fix(vibe-index): realign the scanner with the current schema
f6e47bf docs: record the M1.5 deferral — stabilise the base first
8295333 docs(wal): checkpoint — M1.21 PROP-011 shipped
3f95333 docs: register M1.21 — incremental install
577e11d docs(spec): VIBEVM-SPEC §9.1 + PROP-011 — the shipped install contract
f22f629 feat(install): hold lockfile pins when re-resolving
2b1b6cc feat(install): materialise only the changed vibedeps/ slots
d6c4248 feat(install): skip resolution when vibe.lock is fresh
00bdd48 docs(spec): PROP-011 — close the §5 design questions
0164a20 docs(wal): checkpoint — the `when` gate shipped
6ef1258 docs: register the `when` gate as shipped
a557669 docs(spec): PROP-009 — pin the `when` declaration site
4e3223f feat(workspace): render `when` on dynamic boot entries
fef37e5 feat(core): WhenCondition — the OS gate for boot snippets
c74b2a5 docs(wal): session-end checkpoint — M1.18 shipped
```

The 11 commits `c1f0a26` … `b84e61a` are this session (PROP-005 de-rot +
CHANGELOG + fold + workspace formatting).

---

## Quick-start commands

```sh
# The full gate — formatting, tests, clippy, spec linter.
bash tools/self-check.sh

# Individual invariants.
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p vibe-cli -- check --path .

# The package index utility (now a workspace member).
cargo build -p vibe-index
cargo run  -p vibe-index -- --help

# Routine push (GitVerse SSH key picked up automatically in Git Bash).
git push origin main
```
