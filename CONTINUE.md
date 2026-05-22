# CONTINUE.md — cold-resume checkpoint

_Written 2026-05-22 at session end (`ЗАВЕРШИ СЕССИЮ`). `main` even with
`origin/main`; the PROP-008 Phase 2 work-in-progress lives on the pushed
branch `prop-008-phase2`._

> **`spec/WAL.md` is the canonical living state.** If this snapshot and the
> WAL ever disagree, the WAL wins — it is refreshed every session; this file
> is a point-in-time cold-start aid.

---

## TL;DR

This session did three things:

1. **Confirmed `bypassPermissions`.** The owner asked to record that Claude
   Code should always run in bypass-permissions mode for this project. It was
   already set: `.claude/settings.json` (the versioned project settings)
   carries `permissions.defaultMode = "bypassPermissions"`, committed back on
   2026-04-22 (`9756fa5`). Nothing to change.

2. **Closed a carried item.** The owner deleted the stale
   `gitverse.ru/vibespecs/vibevm-direct-push-smoke` repository, so that
   owner-only item is cleared from the WAL.

3. **Started PROP-008 — qualified package naming (M1.19) — under MFBT.**
   This is the headline work. PROP-008 replaces the flat `<kind>:<name>`
   namespace with Maven-style reverse-FQDN `group` qualification, across an
   8-phase internal plan. **Phase 1 SHIPPED** (`feat(core)` `9b662c5`): the
   `Group` type and the mandatory `[package].group` field — green, committed,
   pushed. **Phase 2 — the `PackageRef` identity refactor — is IN PROGRESS:**
   the entire `vibe-core` library is migrated and compiles; that WIP is
   preserved on the **pushed branch `prop-008-phase2`** (commit `a7d2238`,
   intentionally non-green).

**No active blocker.** `main` is green. Phase 2 just needs a fresh focused
run to finish.

---

## Where work stands

- **Branch `main`:** even with `origin/main`, working tree clean. Gate green —
  `bash tools/self-check.sh` passes all four steps (`cargo fmt --all --check`,
  `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `vibe check --path .`).
- **Branch `prop-008-phase2`:** `a7d2238`, pushed to origin. Holds the
  `vibe-core` Phase-2 migration (`package_ref.rs` + `manifest/package.rs` +
  `manifest/lockfile.rs`). **Intentionally non-green** — `cargo build -p
  vibe-core --lib` passes, but `cargo build --workspace` does not (the
  downstream crates are not yet migrated). This branch is scaffolding for the
  next session, to be superseded by the final green `feat(core)` commit on
  `main` and then deleted.

---

## Active blocker

None. Phase 2 is mid-refactor — an expected in-progress state, not a blocker.
`main` is green; the WIP is safely on a pushed branch.

Outward-facing PROP-008 §3 work is deferred to the owner (not blocking):
renaming and re-publishing the live `vibespecs` GitHub package repositories,
and re-laying-out the `vibespecstest1/2/3` test orgs, into the
`naming = "fqdn"` shape (`org.vibevm.wal`, …). All *in-repo* work proceeds
autonomously; only those live-infrastructure operations need the owner.

---

## Next steps — exact recipe (resume PROP-008 Phase 2)

The next session finishes Phase 2 — the atomic `PackageRef` identity
refactor. The full design spec is in `spec/WAL.md` (the "Phase 2 execution
spec" block); the contract is `spec/modules/vibe-registry/PROP-008-qualified-naming.md`.

1. Run the boot sequence (`CLAUDE.md` → `spec/boot/` → `spec/WAL.md`), then
   read PROP-008.

2. **Recover the WIP.** `git checkout prop-008-phase2` — the `vibe-core`
   library is already migrated and compiles (`cargo build -p vibe-core --lib`
   is green). Continue on that branch.

3. **`vibe-core` test module.** `cargo build -p vibe-core --tests` → fix the
   test module of `crates/vibe-core/src/manifest/package.rs` (~50 sites):
   requalify embedded `[packages]` keys to `org.vibevm/<name>`; `link_for` /
   `declared_link` calls now take `(&group, name)`; `qualified_name()`
   assertions become `org.vibevm/<name>`; `GitPackageDep` / `PathPackageDep` /
   `VarRegistryDep` `.kind` is now `Option<PackageKind>`. Add an `org()`
   helper as already done in `lockfile.rs`'s test module.

4. **Downstream.** `cargo build --workspace` enumerates ~99 call sites across
   `vibe-cli` / `vibe-registry` / `vibe-resolver` / `vibe-workspace` /
   `vibe-publish` / `vibe-check` / `vibe-index` / `vibe-mcp`. The compiler is
   the worklist — fix each.

5. **Embedded manifests + assertions.** `cargo test --workspace` → every
   embedded test manifest's `[requires.packages]` key and every CLI-test
   `vibe install <pkgref>` invocation must become group-qualified
   (`org.vibevm/wal`). This is scriptable — Phase 1 used an idempotent Python
   script (`target/_p1_migrate.py`, gitignored, still on disk); the same
   pattern requalifies `[requires]` keys. Update every pkgref-string
   assertion.

6. **Gate.** `cargo fmt --all` (a bulk script can push lines past the
   rustfmt width), then `bash tools/self-check.sh`.

7. **Land it.** One green commit on `main` —
   `feat(core): qualified PackageRef identity`. Then
   `git push origin --delete prop-008-phase2` and drop the local branch; it
   was scaffolding.

8. **Phases 3–8.** Phase 3 (lockfile — schema v5, `(group, name)` identity)
   is already folded into the Phase-2 commit. Then: `naming = "fqdn"` repo
   names; index-backed short-name resolution; collision detection + exit code
   `7`; the `vibe-index` entry extension (`group` + `workspace_origin`);
   migration of the canonical packages to `group = "org.vibevm"` +
   `VIBEVM-SPEC.md §7.1` (owner sanction recorded in the PROP-008 header) +
   docs. See the WAL phase plan.

---

## Non-obvious findings (this session)

- **`.claude/settings.json` already carried `bypassPermissions`.** The
  owner's request to "record bypass mode in the project properties" was
  already satisfied — `permissions.defaultMode = "bypassPermissions"` was
  committed in `9756fa5` (2026-04-22). `.claude/settings.json` is the
  *versioned* project settings; only `.claude/settings.local.json` is
  gitignored.

- **Lockfile schema → v5, not v4.** PROP-008 §3 says the lockfile "schema
  bumps to v4 (shared bump with PROP-007)". But PROP-007 already shipped v4.
  So Phase 2 bumps to **v5** — a deliberate deviation from the PROP's literal
  text, flagged here and in the WAL.

- **`kind` is `Option` on `PackageRef`, and that is the common case.**
  Manifests store the kindless qualified form `org.vibevm/wal` (PROP-008
  §2.6), so a parsed-from-manifest `PackageRef` has `kind = None`. This is why
  the downstream cascade is large: every `.kind` access must handle the
  `Option`.

- **The Phase-2 WIP commit (`a7d2238` on `prop-008-phase2`) is intentionally
  non-green.** `vibe-core` lib compiles; the `vibe-core` test build and the
  downstream workspace do not. It is a scaffolding commit on a non-`main`
  branch — `main` itself stays green.

- **Embedded-manifest migration is scriptable.** Phase 1 inserted `group`
  into 76 embedded `[package]` blocks across 23 test files via an idempotent
  Python script. The rule "insert after the `[package]` header" provably
  never matches `[[package]]` (lockfile — followed by `]`) or doc comments
  (followed by a backtick). The same approach requalifies Phase 2's
  `[requires]` keys.

- **Two stale fixtures.** `fixtures/manual-test-packages/flow-vibevm-{direct-push,github}-smoke/vibe.toml`
  still use the pre-M1.18 schema (`[writes]`, `[boot_snippet].filename`).
  They are not exercised by `cargo test`, so Phase 1 left them untouched.
  They are dead weight — candidates for deletion.

- **`.gitattributes` pins `* text=auto eol=lf`** — every text file is LF in
  every checkout, so content hashes are cross-platform stable. The vibe-index
  golden-hash parity constant was re-derived this session for the
  `group`-bearing `wal` fixture: `sha256:9c934642…ca412e`.

---

## Repository map

```
vibevm/
├── CLAUDE.md / AGENTS.md / GEMINI.md   the four rules + boot directive (identical)
├── VIBEVM-SPEC.md                      owner-frozen implementation spec
├── ROADMAP.md  CHANGELOG.md  CONTINUE.md
├── .claude/settings.json               project Claude Code settings — bypassPermissions
├── Cargo.toml                          workspace root — members, shared deps, profiles
├── crates/
│   ├── vibe-core        core types: PackageRef/PackageKind/Group/CapabilityRef,
│   │                    the unified Manifest, Lockfile, Purl, i18n
│   ├── vibe-cli         the `vibe` binary — every subcommand
│   ├── vibe-registry    git-backed registry, multi-registry resolver,
│   │                    IndexClient (consumer index fast path), compute_content_hash
│   ├── vibe-resolver    dependency resolution — depsolver, features, activation
│   ├── vibe-workspace   workspace discovery, the loading model, the install
│   │                    orchestrator, vibedeps, freshness
│   ├── vibe-publish     publishing to GitHub / GitVerse, the post-publish index hook
│   ├── vibe-check       the spec linter (`vibe check`)
│   ├── vibe-index       the package index utility — server + CLI (a crates/ member)
│   ├── vibe-mcp         MCP server
│   ├── vibe-graph       task graph
│   ├── vibe-llm         LLM provider integration (M1.5 — deferred)
│   └── vibe-wire        JTD-generated wire types (src/generated/)
├── xtask/               build / maintenance tasks
├── spec/
│   ├── boot/            00-core.md, 90-user.md (authored) + generated INDEX.md
│   ├── common/          PROP-000 (process), PROP-004 (research), PROP-006 (modes)
│   ├── modules/         per-crate PROPs — PROP-008 (qualified naming) under
│   │                    modules/vibe-registry/
│   ├── design/          workspace-and-qualified-naming.md — the PROP-007/008 lore
│   ├── research/
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
  repo human-authored (no AI attribution anywhere); Conventional Commits with
  a *why*-explaining body; group commits by meaning; autonomy on routine work
  only — stop and ask for history rewrites, force-push, large blobs,
  CI/signing/secrets, anything costly to reverse.
- **`.claude/settings.json` runs Claude Code in `bypassPermissions` mode**
  for this project — versioned, team-visible, committed in `9756fa5`.
- **MFBT operating mode** (PROP-006 §2): when the owner says "move fast and
  break things", the agent works heads-down through testable phases with no
  mid-work confirmations; the four rules and the red-line escape hatch
  survive. This session's PROP-008 work ran under MFBT.
- **Language Rust, manifests TOML.** One `vibe.toml` per node; role set by
  section (`[project]` ⊕ `[package]`, optional `[workspace]`). Lockfile
  `vibe.lock`. Four installable kinds: `flow` / `feat` / `stack` / `tool`.
- **PROP-008 — qualified naming (M1.19), IN PROGRESS.** Reverse-FQDN `group`
  qualifier; identity becomes `(group, name, version, content_hash)`; `kind`
  leaves identity and stays metadata; pkgref grammar
  `[kind:][group/]name[@version]`; `naming = "fqdn"` repo names; index-backed
  short-name resolution; collision detection with exit code `7`. Phase 1
  shipped; Phase 2 in progress.
- **Loading model (PROP-009, M1.18).** Two physically separate trees —
  authored `spec/` and committed `vibedeps/`. The boot sequence is computed
  per node and projected into `spec/boot/INLINE.md` + `INDEX.md`. `vibe` owns
  one `<vibevm>` block inside `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`
  (PROP-012).
- **Decentralised registry (PROP-002).** Git-as-registry; content-hash
  identity; `[[registry]]` / `[[mirror]]` / `[[override]]`; redirect stubs.
- **Incremental install (PROP-011, M1.21).** `vibe install` is
  lockfile-respecting — skips the depsolver when `vibe.lock` is fresh,
  materialises only the changed `vibedeps/` slots.
- **The package index (PROP-005).** Opt-in; a derived hot cache — package
  repos stay authoritative, `content_hash` verified at fetch time regardless.
- **Split-host posture.** vibevm source on GitVerse
  (`git@gitverse.ru:anarchic/vibevm.git`); the package registry org on GitHub
  (`github.com/vibespecs`).
- **M1.5 (LLM generation) is deferred.** Base-machinery-first: stabilise the
  package machinery before layering any generation on top.

---

## Recent commit chain (newest first)

```
cce7014 docs(wal): checkpoint PROP-008 Phase 2 — vibe-core migrated
8b8c4c6 docs(wal): record PROP-008 Phase 2 design + stashed WIP
73a5092 docs(wal): checkpoint PROP-008 Phase 1
9b662c5 feat(core): add the mandatory [package].group field
e167107 docs(continue): cold-resume checkpoint
7c1c090 docs(wal): session-end checkpoint
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
```

The session-end `docs(continue)` and `docs(wal)` checkpoint commits sit on
top of these. The PROP-008 Phase-2 WIP commit `a7d2238` is on the
`prop-008-phase2` branch, not on `main`.

---

## Quick-start commands

```sh
# The full gate.
bash tools/self-check.sh

# Individual invariants.
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p vibe-cli -- check --path .

# Resume PROP-008 Phase 2.
git checkout prop-008-phase2      # the WIP branch — vibe-core already migrated
cargo build -p vibe-core --lib    # green — confirms the WIP base compiles
cargo build -p vibe-core --tests  # the worklist for package.rs's test module

# Routine push (GitVerse SSH key picked up automatically in Git Bash).
git push origin main
```

---

## Pointer

`spec/WAL.md` is the canonical living state and supersedes this snapshot if
they diverge. The full PROP-008 Phase 2 execution spec — the exact
`PackageRef` shape, the `Requires` cascade, the lockfile-v5 bump, and the
downstream order of work — is in the WAL's "Phase 2 execution spec" block.
