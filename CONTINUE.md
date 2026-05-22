# CONTINUE.md — cold-resume checkpoint

_Written 2026-05-22 at session end (`СОХРАНИ СЕССИЮ`). `main` is at
`639b959`, even with `origin/main`, working tree clean._

> **`spec/WAL.md` is the canonical living state.** If this snapshot and the
> WAL ever disagree, the WAL wins — it is refreshed every session; this file
> is a point-in-time cold-start aid.

---

## TL;DR

This session shipped **PROP-008 Phase 7** — the group-native package
index. `vibe-index` is now the index analogue of what Phase 4 did to the
registry: package identity inside the index is `(group, name)`, `kind` is
pure metadata. It landed on `main` as three commits — `59355d3`
(`feat(vibe-index)`, 32 files), `569d1b0` (`docs(spec)`), `639b959`
(`docs(wal)`) — all pushed to `origin/main`. `bash tools/self-check.sh`
is green on all four steps.

**Why Phase 7 and not Phase 5.** The session opened on "do the next
phase". A design pass found Phase 5 (index-backed short-name resolution)
cannot be built until the index carries `group` and re-keys `by-name`
by bare name — that *is* Phase 7. So the dependency-correct order is
**7 → 5 → 6**; Phase 7 was done first.

**The change.** The index entry (`VersionEntry`) gained the mandatory
`group` and the optional `workspace_origin`. `PackageEntry` re-keyed
from `kind` to `group`; the in-RAM `PkgKey` is `(Group, name)`. The
`by-name/` layer moved from `by-name/<kind>/<name>.json` to the
candidate-set file **`by-name/<name>.json`** — a new `NameEntry` holding
every `(group, *)` package that shares one bare name, so PROP-008 §2.6
short-name resolution becomes one GET per registry. `NamingConvention`
gained `Fqdn` (the new default) + `KindSlashName`; HTTP routes, the
`vibe-index` CLI, and the `vibe.lock` reader followed. The two
cross-crate consumers were realigned in the same cut:
`vibe-registry::IndexClient::list_versions` reads the candidate set and
selects by `group`; `vibe-publish::post_hook` adds `group` to the entry
it POSTs.

**No blocker.** PROP-008 Phases 5, 6, 8 remain. The next session picks
one — Phase 5 is now fully unblocked.

---

## Where work stands

- **Branch `main`:** at `639b959`, even with `origin/main`, working tree
  clean. Gate green — `bash tools/self-check.sh` passes all four steps
  (`cargo fmt --all --check`, `cargo test --workspace`, `cargo clippy
  --workspace --all-targets -- -D warnings`, `vibe check --path .`
  reports 0/0/0).
- **Branch `m1.17-workspace`:** still retained on origin (merged long ago,
  never deleted) — harmless, ignorable.
- `cargo test --workspace` — 0 failures. `vibe-index` is **92 lib tests**
  plus its integration suites (`cli_lifecycle`, `cli_read`, `cli_write`,
  `scanner_e2e`, `server_e2e`, `server_writes`, `from_github_e2e`,
  `rate_limit_e2e`, `content_hash_parity`, `help_smoke`), all green.

## Active blocker

None. PROP-008 Phases 1–4 + 7 are shipped and green. Phases 5, 6, 8 are
fresh units of work, not blockers.

**Owner-only outward-facing work** (deferred, blocks nothing in-repo):
rename / re-publish the live `vibespecs` GitHub package repos and
re-lay-out the `vibespecstest1/2/3` test orgs into the `naming = "fqdn"`
shape (`org.vibevm.wal`, …). Every hermetic test is self-contained and
green, so this gates nothing.

---

## Next steps — PROP-008 Phases 5, 6, 8

The identity core (Phases 1–4) and the group-native index (Phase 7) are
done. Remaining, from PROP-008 §6:

- **Phase 5 — index-backed short-name resolution.** A CLI-boundary
  lookup: `vibe install wal` resolves the bare `wal` → `org.vibevm/wal`
  via the package index, then writes the qualified form into `[requires]`
  (PROP-008 §2.6). Manifests are always qualified; the short form is
  CLI-only sugar. **Now fully unblocked** — Phase 7 shipped exactly what
  it consumes: one GET of `by-name/<name>.json` per registry yields the
  whole candidate set (a `NameEntry` with one `PackageEntry` per group).
  The resolver enumerates `(*, name)` by reading that file across every
  configured registry. Wants a short design pass on where the
  CLI-boundary lookup sits relative to `MultiRegistryResolver`.
- **Phase 6 — collision detection + exit code `7`.** When a short name
  matches two packages with different `group` — i.e. the `NameEntry`
  read in Phase 5 has `packages.len() > 1` — fail and list the
  alternatives; new exit code `7` ("ambiguous package"), distinct from
  `3` ("package conflict"). PROP-008 §2.7. Naturally pairs with Phase 5.
- **Phase 8 — milestone close-out.** Migrate the three canonical
  packages (`flow-wal`, `flow-sync-from-code`, `flow-atomic-commits`) to
  `group = "org.vibevm"`; edit `VIBEVM-SPEC.md §7.1` (owner sanction
  already recorded in the PROP-008 header — name-uniqueness moves from
  "within a kind" to "within a group", the identity tuple + pkgref
  grammar update); update `CHANGELOG.md` and `ROADMAP.md` (neither
  records PROP-008 yet); docs sweep.

**Lightest starting point:** Phase 8's docs half — `CHANGELOG.md` /
`ROADMAP.md` / `VIBEVM-SPEC.md §7.1` for everything already shipped
(Phases 1–4 + 7) — closes the milestone's paper trail and needs no
design work. **Phase 5 is the next real code unit** and may want a short
design pass first; Phase 6 then folds in beside it.

Recipe for whoever picks up cold:

1. Run the boot sequence (`CLAUDE.md` → `spec/boot/` → `spec/WAL.md`),
   then read PROP-008
   (`spec/modules/vibe-registry/PROP-008-qualified-naming.md`) and
   PROP-005 (`spec/modules/vibe-index/PROP-005-package-index.md`).
2. Confirm green: `bash tools/self-check.sh`.
3. For Phase 5: read `crates/vibe-registry/src/index_client.rs`
   (`list_versions` already reads `by-name/<name>.json`) and the
   `MultiRegistryResolver`; decide where the bare-name → qualified
   lookup sits. Proceed under MFBT.

---

## Non-obvious findings (this session)

- **Phase 7 was the dependency-correct "next phase", not Phase 5.**
  PROP-008's CONTINUE list ordered the phases 5–8, but Phase 5
  (short-name resolution) consumes a `by-name/<name>.json` candidate set
  that did not exist until Phase 7. The real order is 7 → 5 → 6; the
  CONTINUE numbering was a listing order, not an implementation order.
- **Phase 4's `IndexClient` had guessed the wrong by-name shape.** When
  `vibe-registry` went group-native in Phase 4, `IndexClient::list_versions`
  was written to fetch `by-name/<group>/<name>.json` — a one-for-one
  `kind`→`group` swap of the old layout. But PROP-008 §2.8 actually
  specifies `by-name/<name>.json`, the bare-name **candidate set**.
  Phase 7 reconciled the client to the spec: it now fetches
  `by-name/<name>.json` and selects the candidate whose `group` matches.
- **`vibe-publish::post_hook` would have been rejected by a Phase-7
  index.** `build_payload` POSTs a JSON entry to `/v1/packages`; it did
  not include `group`. After Phase 7 the server deserialises the body as
  `VersionEntry`, where `group` is mandatory — the publish hook would
  400 on every fire. Phase 7 added `group` (+ `workspace_origin`) to the
  payload. The lesson: an index schema change ripples to every producer
  and consumer of index data; the cut must be atomic across crates.
- **`by-name/<name>.json` is a candidate set, not one package.** The
  file is a `NameEntry { name, indexed_at, packages: [PackageEntry] }`.
  `packages.len() > 1` is, by construction, a short-name collision
  (PROP-008 §2.7) — Phase 6 reads exactly this.
- **The index's `repo_name` is infallible; the registry's is not.**
  `vibe-core::NamingConvention::repo_name` returns `Result` because a
  kindless pkgref cannot feed a legacy `kind-*` convention. The index
  always has a concrete `kind` on every entry, so
  `vibe-index`'s mirror of `repo_name` takes `kind: PackageKind` (not
  `Option`) and never fails.
- **Incremental reindex reverse-derives a clone's directory name.** The
  `--incremental` retention path computes `naming.repo_name(kind, group,
  name)` and matches it against the scanned clone directory names. With
  `Fqdn` now the default, the `scanner_e2e` fixtures had to be renamed
  from the `KindName` shape (`flow-wal`) to the `fqdn` shape
  (`org.vibevm.wal`) to stay representative. Latent fragility worth a
  note: an operator whose `--from-clones` directory layout does not
  match the registry's `naming` will see incremental retention silently
  drop entries — pre-existing, out of Phase 7 scope, not redesigned.
- **`Group` is re-exported from `vibe_index::types`.** Integration tests
  in `crates/vibe-index/tests/` cannot `use vibe_core::…` (vibe-core is a
  regular dependency, not a dev-dependency). `vibe_index::types` now
  re-exports `vibe_core::Group` so the test crates — and any external
  consumer of `vibe_index::types` — reach it without a second dep.
- **No JTD schema directory exists for `vibe-index`.** PROP-005 §2.6 /
  §3.1 reference `crates/vibe-index/schemas/index-entry.jtd.json`, but no
  `schemas/` directory is on disk — the index wire types are hand-rolled
  serde structs. Pre-existing doc-rot, left untouched (out of Phase 7
  scope); a candidate for a future de-rot pass.

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
│   │                    the unified Manifest, Lockfile (schema v5), Purl, i18n
│   ├── vibe-cli         the `vibe` binary — every subcommand
│   ├── vibe-registry    git-backed registry, multi-registry resolver,
│   │                    IndexClient (group-native), compute_content_hash
│   ├── vibe-resolver    dependency resolution — depsolver, features, activation
│   ├── vibe-workspace   workspace discovery, the loading model, the install
│   │                    orchestrator, vibedeps, freshness
│   ├── vibe-publish     publishing to GitHub / GitVerse, the post-publish index hook
│   ├── vibe-check       the spec linter (`vibe check`)
│   ├── vibe-index       the package index utility — server + CLI; group-native
│   │                    as of PROP-008 Phase 7 (entry identity = (group, name))
│   ├── vibe-mcp         MCP server
│   ├── vibe-graph       task graph
│   ├── vibe-llm         LLM provider integration (M1.5 — deferred)
│   └── vibe-wire        JTD-generated wire types (src/generated/)
├── xtask/               build / maintenance tasks
├── spec/
│   ├── boot/            00-core.md, 90-user.md (authored) + generated INDEX.md
│   ├── common/          PROP-000 (process), PROP-004 (research), PROP-006 (modes)
│   ├── modules/         per-crate PROPs — PROP-008 (qualified naming) under
│   │                    modules/vibe-registry/, PROP-005 (index) under
│   │                    modules/vibe-index/
│   ├── design/          workspace-and-qualified-naming.md — the PROP-007/008 lore
│   ├── research/
│   └── WAL.md           the canonical living checkpoint
├── docs/                user-facing docs (commands/, loading-model.md, …)
├── fixtures/registry/   hermetic test-fixture packages — laid out
│                        org.vibevm/<name>/v<version>/ (group-native)
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
  blobs, CI/signing/secrets, anything costly to reverse.
- **`.claude/settings.json` runs Claude Code in `bypassPermissions` mode**
  for this project — versioned, team-visible.
- **MFBT operating mode** (PROP-006 §2): when the owner says "move fast and
  break things", the agent works heads-down through testable phases with no
  mid-work confirmations; the four rules and the red-line escape hatch
  survive. This session's PROP-008 Phase 7 work ran under MFBT.
- **Language Rust, manifests TOML.** One `vibe.toml` per node; role set by
  section (`[project]` ⊕ `[package]`, optional `[workspace]`). Lockfile
  `vibe.lock`, **schema v5**. Four installable kinds — `flow` / `feat` /
  `stack` / `tool` — but `kind` is **metadata only**, not identity.
- **PROP-008 — qualified naming (M1.19): Phases 1–4 + 7 SHIPPED.**
  Identity is `(group, name, version, content_hash)`; reverse-FQDN
  `group` qualifier; pkgref grammar `[kind:][group/]name[@version]`;
  manifests store the kindless `org.vibevm/<name>`; the registry **and**
  the package index are both group-native, `NamingConvention::Fqdn` the
  default. The index entry carries `group` + `workspace_origin`; the
  `by-name/` layer is the candidate-set file `by-name/<name>.json`.
  Phases 5 (short-name resolution), 6 (collision detection + exit code
  7), 8 (canonical-package migration + `VIBEVM-SPEC.md §7.1` + docs)
  remain.
- **The package index (PROP-005).** Opt-in; a derived hot cache —
  package repos stay authoritative, `content_hash` verified at fetch
  time. `vibe-index` lives at `crates/vibe-index/`, a workspace member,
  parsing manifests through `vibe-core::Manifest`. Group-native as of
  PROP-008 Phase 7.
- **Loading model (PROP-009, M1.18).** Two physically separate trees —
  authored `spec/` and committed `vibedeps/`. The boot sequence is
  computed per node and projected into `spec/boot/INLINE.md` + `INDEX.md`.
  `vibe` owns one `<vibevm>` block inside `CLAUDE.md` / `AGENTS.md` /
  `GEMINI.md` (PROP-012). The `vibedeps/<kind>-<name>/<version>/` slot
  layout still carries `kind` — a PROP-009 schema, untouched by PROP-008.
- **Decentralised registry (PROP-002).** Git-as-registry; content-hash
  identity; `[[registry]]` / `[[mirror]]` / `[[override]]`; redirect stubs.
- **Incremental install (PROP-011, M1.21).** `vibe install` is
  lockfile-respecting — skips the depsolver when `vibe.lock` is fresh,
  materialises only the changed `vibedeps/` slots.
- **Split-host posture.** vibevm source on GitVerse
  (`git@gitverse.ru:anarchic/vibevm.git`); the package registry org on
  GitHub (`github.com/vibespecs`).
- **M1.5 (LLM generation) is deferred.** Base-machinery-first: stabilise
  the package machinery before layering any generation on top.

---

## Recent commit chain (newest first)

```
639b959 docs(wal): checkpoint PROP-008 Phase 7
569d1b0 docs(spec): reconcile PROP-005/008 — group-native index
59355d3 feat(vibe-index): group-native index (PROP-008 Phase 7)
d69ff04 docs(continue): cold-resume checkpoint
e83c398 docs(wal): checkpoint PROP-008 Phases 1-4 shipped
c5c4fe6 feat(core): group-qualified package identity (PROP-008 Phase 2)
1ebd279 docs(wal): session-end checkpoint
744afa7 docs(continue): cold-resume checkpoint
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
```

The PROP-008 Phase 7 work this session is `59355d3` (the atomic
`feat(vibe-index)` cut — 32 files), `569d1b0` (the PROP-005/PROP-008
reconciliation), and `639b959` (the WAL checkpoint).

---

## Quick-start commands

```sh
# The full gate — must be green before any commit lands.
bash tools/self-check.sh

# Individual invariants.
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p vibe-cli -- check --path .

# Just the index crate (the Phase 7 surface).
cargo test -p vibe-index

# Routine push (GitVerse SSH key picked up automatically in Git Bash).
git push origin main
```

---

## Pointer

`spec/WAL.md` is the canonical living state and supersedes this snapshot
if they diverge. The WAL's "Current phase" block carries the full
PROP-008 status — Phases 1–4 + 7 shipped, the Phase-7 detail, the two
cross-crate findings, and the Phase 5 / 6 / 8 plan.
