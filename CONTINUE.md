# CONTINUE.md — cold-resume checkpoint

_Written 2026-05-23 at session end (`СОХРАНИ СЕССИЮ`). `main` is at
`b39b0a2`, even with `origin/main`, working tree clean._

> **`spec/WAL.md` is the canonical living state.** If this snapshot and
> the WAL ever disagree, the WAL wins — it is refreshed every session;
> this file is a point-in-time cold-start aid.

---

## TL;DR

This session closed **M1.19 — qualified package naming (PROP-008) end
to end**. Phase 8 (docs/spec close-out), Phase 5 (index-backed
short-name resolution), and Phase 6 (collision detection + exit code
`7`) all landed on top of the identity core (Phases 1–4 + 7) shipped
earlier the same day — twelve commits since `2e85032`, all on
`origin/main`, gate green.

On top of the milestone, two things happened that the next session
should know about:

- **The canonical GitHub `vibespecs` registry org was migrated** to the
  `fqdn` shape. Three new repos published (`org.vibevm.{wal,
  sync-from-code, atomic-commits}` at `v0.1.0`) via `vibe registry
  publish`; three legacy `flow-*` repos archived read-only (reversible
  — not deleted). The live install smoke against the migrated registry
  caught a latent **`vibe init` defect** — every fresh project was
  being scaffolded with `naming = "kind-name"`, so a freshly-initialised
  project could not resolve any qualified pkgref — and a `cli_init`
  test asserted the broken value as correct. Fixed in `cc32d7e`.

- **PROP-013 — the periodic health audit — was authored as a process
  PROP** ([`spec/common/PROP-013`](spec/common/PROP-013-periodic-health-audit.md)).
  The `vibe init` defect was the motivating case: it survived eight
  PROP-008 phases green, ~800 hermetic tests passing, gate clean on
  every commit. Only the live smoke caught it. The audit is the
  deliberate breadth-first sweep that complements the per-commit gate
  — finding what regression-detection is structurally blind to:
  uncovered code, code outside `cargo test --workspace`, drift, slow
  debt. Inventory lives in [`AUDIT.md`](AUDIT.md); cadence floor is
  once per milestone. The seed run (2026-05-23) catalogued **13
  findings** — 2 fixed, 1 filed, 1 accepted, 9 open — headlined by
  **P1 `2026-05-23-01`**: the production git-registry + naming path is
  under-tested.

The next session's strongest move is **the first full PROP-013 audit
run + closing the P1 test gap** (a hermetic harness driving
`GitPackageRegistry` against real `file://` git repositories, plus a
default-path `vibe init` → `vibe install` e2e) before, or in parallel
with, PROP-010 (M1.20 — local package cache, DRAFT, needs an owner
design session for its 5 §5 open questions).

---

## Where work stands

- **Branch `main`:** at `b39b0a2`, even with `origin/main`, working
  tree clean. Gate green at the latest full run — `bash
  tools/self-check.sh` passes all four steps (`cargo fmt --all
  --check`, `cargo test --workspace`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `vibe check --path .` 0/0/0). The
  five doc/audit commits that closed the session are all
  no-code-impact.
- **GitHub `vibespecs` registry org:** migrated — `org.vibevm.wal`,
  `org.vibevm.sync-from-code`, `org.vibevm.atomic-commits` active
  (fqdn, tag `v0.1.0`); the legacy `flow-wal`, `flow-sync-from-code`,
  `flow-atomic-commits` archived (read-only, reversible — not
  deleted).
- **Branch `m1.17-workspace`:** still retained on origin (merged long
  ago, never deleted) — harmless, ignorable.

## Active blocker

None for in-repo code work. The remaining migration work — the
GitVerse side (`vibespecs-gitverse`, `vibespecstest3`) and the GitHub
test orgs `vibespecstest1/2` — is owner-only outward work that gates
nothing in-repo (every hermetic test is self-contained; the live
`cli_live_e2e.rs` tests are `#[ignore]`d).

---

## Next steps — first full PROP-013 audit + the P1 test-hardening

The session-end finding map (the M1.19 `vibe init` defect, the
`LocalRegistry` shadowing) crystallised a structural test gap: most
install e2e drive `LocalRegistry` and never touch the
`GitPackageRegistry` + `NamingConvention` path that real installs run.
The seed audit captured it as **P1 `2026-05-23-01`** and filed it for
the test-hardening work.

The recommended next session, in order:

1. **Run the first full PROP-013 audit** per [PROP-013](spec/common/PROP-013-periodic-health-audit.md)
   §4 — walk the §2.2 checklist breadth-first (a fresh sweep, not the
   seed), record findings in `AUDIT.md`, carry forward the seed's 10
   open findings, re-judge each.
2. **Close finding `2026-05-23-01` (P1).** Build a hermetic test
   harness that drives `GitPackageRegistry` against real `file://` git
   repositories — `git init`-shaped repos with `v0.1.0` tags, named
   per the `fqdn` convention (`org.vibevm.wal.git`). The existing
   `install_from_git_registry` test (gated on `git_available()`,
   `crates/vibe-cli/tests/cli_e2e.rs`) is the partial precedent; do it
   systematically. Add a default-path e2e — `vibe init` (no
   `--registry`) → `vibe install <pkgref>` against such a file-served
   fqdn registry — to guard the exact path the M1.19 defect fell
   through.
3. **PROP-010 — M1.20, local package cache.** Owner design session to
   close its 5 §5 open questions (cache layout, command namespace,
   staleness signalling, eviction, scaffolding UX) before
   implementation. See [PROP-010](spec/modules/vibe-registry/PROP-010-local-package-cache.md).
4. **Then M1.5 (Generation).** Deferred behind base-machinery-first.

Recipe for whoever picks up cold:

1. Run the boot sequence (`CLAUDE.md` → `spec/boot/INDEX.md` → the
   files it names → `spec/WAL.md`).
2. Read [PROP-013](spec/common/PROP-013-periodic-health-audit.md) and
   [`AUDIT.md`](AUDIT.md) for the audit process and the seed inventory.
3. Confirm green: `bash tools/self-check.sh`.
4. For the P1 (`2026-05-23-01`) test-hardening: read
   `crates/vibe-registry/src/git_package_registry.rs`,
   `crates/vibe-registry/src/multi_registry_resolver.rs`,
   `crates/vibe-registry/src/lib.rs` (`LocalRegistry`), and the
   existing `install_from_git_registry` test. Proceed under MFBT.

---

## Non-obvious findings (this session)

- **M1.19 shipped on a broken `vibe init`.** Eight PROP-008 phases
  (1–4 + 7 → 8 → 5 → 6) landed green, ~800 hermetic tests passing,
  the per-commit gate clean throughout — and a fresh `vibe init`
  produced projects that could not install any qualified pkgref. The
  live install smoke against the migrated registry caught it; the
  gate did not. This is the headline case study in PROP-013 §1.
- **A test asserted the bug as correct.**
  `cli_init.rs::init_writes_default_registry` literally asserted
  `primary.naming == NamingConvention::KindName`. It stayed green
  while the behaviour was wrong. When a milestone changes a default,
  the test that guards it must update in the same phase; PROP-013
  category A3 records this class as a permanent audit line.
- **`LocalRegistry` shadows the production git path.** Most install
  e2e use `--registry <dir>` → the `LocalRegistry` directory layout,
  which bypasses `GitPackageRegistry::resolve` (clone / ls-remote /
  archive / tags) and `NamingConvention::repo_name` entirely. The
  naming convention is only meaningfully exercised on the
  `GitPackageRegistry` path; most tests skip it. This is the
  structural gap the P1 finding files for repair.
- **The owner's standing token-discipline rule was lifted once,
  deliberately.** `spec/boot/90-user.md` makes the publish token at
  `~/.vibevm/<host>.publish.token` never-echoed and not-`cat`-able by
  tools — sessions are video-recorded; one echo is a leak. For this
  session the owner explicitly authorised raw token use ("конкретно
  для текущего случая, сейчас записи экрана нет") to enable `curl`
  API calls (rename / archive / list against GitHub) for the
  registry migration. This was case-specific; the standing rule
  remains default-on for all other sessions and operations.
  `vibe registry publish` is **token-safe** by construction (reads +
  injects the token internally, never echoes it) and was used for
  the new-repo publishes; only the archive calls needed the raw
  token.
- **The migration is half-done by design.** GitHub `vibespecs` is
  fully migrated. The GitVerse side and the GitHub test orgs are
  deferred — the owner explicitly said skip GitVerse for now, and the
  test orgs are coupled to the `#[ignore]`d `cli_live_e2e` tests
  (re-laying the fixtures means updating what those tests expect, a
  unit of work best done together). Neither gates anything in-repo.
- **PROP-005's `schemas/` directory is fictional.** PROP-005 §2.6 /
  §3.1 reference `crates/vibe-index/schemas/index-entry.jtd.json`,
  but no such directory exists; the index wire types are hand-rolled
  serde structs. Recorded as audit finding `2026-05-23-09` (C2,
  drift). Pre-existing; left for a later pass.
- **`fixtures/manual-test-packages/` has rotted across two
  milestones.** Manifests still use `[writes]`, `[boot_snippet].
  filename`, and lack `[package].group` — all retired by M1.18 and
  PROP-008. No hermetic test parses them, so the gate stayed green.
  Audit finding `2026-05-23-05` (B1).
- **`vibe registry publish` correctly defaults to `fqdn` now.** With
  PROP-008 Phase 4 making `NamingConvention::Fqdn` the default and
  `cc32d7e` fixing `vibe init` to scaffold `naming = "fqdn"`, the
  publish path is end-to-end consistent: a fresh project → `vibe
  registry publish fixtures/registry/org.vibevm/<name>/v0.1.0
  --registry vibespecs` → creates `vibespecs/org.vibevm.<name>` on
  GitHub. Verified live.

---

## Repository map

```
vibevm/
├── CLAUDE.md / AGENTS.md / GEMINI.md   the four rules + boot directive (identical)
├── VIBEVM-SPEC.md                      owner-frozen implementation spec
├── ROADMAP.md  CHANGELOG.md  CONTINUE.md
├── AUDIT.md                            PROP-013 health-audit inventory (new this session)
├── .claude/settings.json               project Claude Code settings — bypassPermissions
├── Cargo.toml                          workspace root — members, shared deps, profiles
├── crates/
│   ├── vibe-core        core types: PackageRef/PackageKind/Group/CapabilityRef,
│   │                    the unified Manifest, Lockfile (schema v5), Purl, i18n
│   ├── vibe-cli         the `vibe` binary — every subcommand. New this session:
│   │                    commands/short_name.rs (CLI-boundary short-name resolver)
│   ├── vibe-registry    git-backed registry, multi-registry resolver,
│   │                    IndexClient, compute_content_hash. New this session:
│   │                    LocalRegistry::candidate_groups, IndexClient::name_candidates,
│   │                    MultiRegistryResolver::resolve_name_candidates (PROP-008 §2.6)
│   ├── vibe-resolver    dependency resolution — NaiveDepSolver, features, activation
│   ├── vibe-workspace   workspace discovery, loading model, install orchestrator,
│   │                    vibedeps, freshness
│   ├── vibe-publish     publishing to GitHub / GitVerse, post-publish index hook
│   ├── vibe-check       the spec linter (`vibe check`)
│   ├── vibe-index       the package index utility — server + CLI; group-native
│   ├── vibe-mcp         MCP server
│   ├── vibe-graph       task graph
│   ├── vibe-llm         LLM provider integration (M1.5 — deferred)
│   └── vibe-wire        JTD-generated wire types (src/generated/)
├── xtask/               build / maintenance tasks
├── spec/
│   ├── boot/            00-core.md, 90-user.md (authored) + generated INDEX.md
│   ├── common/          PROP-000 (process), PROP-006 (operating modes),
│   │                    PROP-013 (periodic health audit — new this session)
│   ├── modules/         per-crate PROPs — PROP-008 (qualified naming, ✅ shipped)
│   │                    under modules/vibe-registry/; PROP-010 (local cache, DRAFT)
│   ├── design/          workspace-and-qualified-naming.md — the PROP-007/008 lore
│   ├── research/
│   └── WAL.md           the canonical living checkpoint
├── docs/                user-facing docs (commands/, loading-model.md, …)
├── fixtures/registry/   hermetic test-fixture packages — laid out
│                        org.vibevm/<name>/v<version>/ (group-native); also the
│                        publish source for the GitHub `vibespecs` org
├── manual-tests/        operator smoke recipes
├── tools/               self-check.sh, jtd-codegen
└── refs/                the owner's book + reference sources (read-only)
```

---

## Architectural / policy decisions in force

- **The four rules** (`CLAUDE.md`, authoritative `PROP-000 §12`): keep
  the repo human-authored (no AI attribution anywhere); Conventional
  Commits with a *why*-explaining body; group commits by meaning;
  autonomy on routine work only — stop and ask for history rewrites,
  force-push, large blobs, CI / signing / secrets, anything costly to
  reverse.
- **PROP-013 — periodic health audit** (new this session, in force):
  a recurring breadth-first sweep complementary to the per-commit
  gate. Catches what regression-detection cannot — uncovered code,
  rot outside the gate, drift, slow debt. Inventory in `AUDIT.md`
  (append-only, dated runs, severity / disposition, carry-forward);
  cadence floor once per milestone; mechanical categories migrate
  into `vibe check` over time.
- **`.claude/settings.json` runs Claude Code in `bypassPermissions`
  mode** for this project — versioned, team-visible.
- **MFBT operating mode** (PROP-006 §2): when the owner says "move
  fast and break things", the agent works heads-down through testable
  phases with no mid-work confirmations; the four rules and the
  red-line escape hatch survive. This session's PROP-008 Phase 8 / 5
  / 6 work ran under MFBT.
- **Token discipline default** (`spec/boot/90-user.md`): the publish
  token at `~/.vibevm/<host>.publish.token` is never echoed in stdout
  / stderr / chat / logs; file is editor-only, not `cat`/`Read`/`echo`.
  Sessions are video-recorded; one echo = a leak. `vibe registry
  publish` is token-safe by construction. The owner may lift this
  once, case-specifically, for a particular operation (as happened
  this session for the GitHub API rename / archive calls); the
  default remains on.
- **Language Rust, manifests TOML.** One `vibe.toml` per node, role
  set by section (`[project]` ⊕ `[package]`, optional `[workspace]`).
  Lockfile `vibe.lock`, **schema v5** (PROP-008 Phase 3 bump). Four
  installable kinds — `flow` / `feat` / `stack` / `tool` — but `kind`
  is **metadata only**, not identity.
- **PROP-008 — qualified naming (M1.19): ✅ SHIPPED 2026-05-22.** All
  eight phases on `main`. Identity is
  `(group, name, version, content_hash)`; reverse-FQDN `group`
  qualifier; pkgref grammar `[kind:][group/]name[@version]`;
  manifests store the kindless `org.vibevm/<name>`; registry and
  package index both group-native; `NamingConvention::Fqdn` the
  default. Short names are CLI sugar resolved at the CLI input
  boundary via the index (Phase 5); collisions get exit code `7`
  (Phase 6). `VIBEVM-SPEC.md` §7 / §8 / §9.4 reconciled under owner
  sanction (Phase 8).
- **The canonical GitHub `vibespecs` registry org** hosts
  `org.vibevm.{wal, sync-from-code, atomic-commits}` (fqdn, active).
  The legacy `flow-*` repos are archived read-only (reversible).
- **The package index (PROP-005).** Opt-in; a derived hot cache —
  package repos stay authoritative, `content_hash` verified at fetch
  time. Group-native (PROP-008 Phase 7); entry carries `group` +
  `workspace_origin`; `by-name/<name>.json` is the candidate-set file
  (one GET per registry yields every group sharing a bare name —
  Phase 5 consumes it).
- **Loading model (PROP-009, M1.18).** Two physically separate trees
  — authored `spec/` and committed `vibedeps/`. Boot computed per
  node and projected into `spec/boot/INLINE.md` + `INDEX.md`. `vibe`
  owns one `<vibevm>` block inside each shared agent instruction file
  (PROP-012). `vibedeps/<kind>-<name>/<version>/` slot layout still
  carries `kind` — a PROP-009 schema, untouched by PROP-008.
- **Decentralised registry (PROP-002).** Git-as-registry; content-
  hash identity; `[[registry]]` / `[[mirror]]` / `[[override]]`;
  redirect stubs.
- **Incremental install (PROP-011, M1.21).** `vibe install` is
  lockfile-respecting — skips the depsolver when `vibe.lock` is
  fresh, materialises only the changed `vibedeps/` slots.
- **Split-host posture.** vibevm source on GitVerse
  (`git@gitverse.ru:anarchic/vibevm.git`); the package registry org
  on GitHub (`github.com/vibespecs`).
- **M1.5 (LLM generation) is deferred.** Base-machinery-first:
  stabilise the package machinery before layering any generation on
  top. PROP-013's recurring audit is the *measurement* of that
  stability.

---

## Recent commit chain (newest first)

```
b39b0a2 docs(wal): register PROP-013, the periodic health audit
e3410d2 docs(audit): 2026-05-23 seed inventory
4848304 docs(spec): PROP-013 — periodic health audit process
dc62acb docs(wal): record the vibespecs registry-org migration
cc32d7e fix(cli): vibe init and registry add default to fqdn naming
2139c10 docs(wal): checkpoint M1.19 shipped
56c574e docs(changelog,roadmap): close M1.19 — qualified naming shipped
cee8c4a feat(cli): collision detection + exit code 7 (PROP-008 Phase 6)
f4e8ee2 feat(cli): index-backed short-name resolution (PROP-008 Phase 5)
1d66822 docs(changelog,roadmap): record M1.19 qualified naming
503f912 docs: reconcile user docs with qualified naming
a54fbea docs(spec): VIBEVM-SPEC §7–§8 — group-qualified identity
2e85032 docs(continue): cold-resume checkpoint
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
```

This session's twelve commits run `a54fbea` through `b39b0a2` —
M1.19's closing phases (Phase 8 docs/spec, Phase 5 short-name
resolution, Phase 6 collision detection), the registry-migration
record + the `vibe init` fix surfaced by the live smoke, and
PROP-013 + `AUDIT.md` born on top.

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

# Routine push (GitVerse SSH key picked up automatically in Git Bash).
git push origin main

# Live install smoke against the migrated GitHub registry.
# (Optional — exercises the production path; requires network.)
mkdir tmp-smoke && cd tmp-smoke
cargo run -p vibe-cli --manifest-path /path/to/vibevm/Cargo.toml -- init --path .
cargo run -p vibe-cli --manifest-path /path/to/vibevm/Cargo.toml -- install org.vibevm/wal --path . --assume-yes
```

To run an audit: follow [PROP-013](spec/common/PROP-013-periodic-health-audit.md)
§4 — open a new dated section in `AUDIT.md`, walk the §2.2 checklist,
record findings, carry forward what is still open.

---

## Pointer

`spec/WAL.md` is the canonical living state and supersedes this
snapshot if they diverge. The WAL's "Current phase" block carries the
full M1.19 + migration + PROP-013 record; its "Known issues" lists
the active items, with `AUDIT.md` named as the canonical durable
inventory they mirror.
