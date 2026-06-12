# AUDIT.md — project health inventory

The recurring defect / rot / drift inventory defined by
[PROP-013](spec/common/PROP-013-periodic-health-audit.md). Each audit
run appends a dated section; findings carry forward until they are
dispositioned. This file is committed to git — its history is the
project's health trend.

**Severity** — `P1` blocker (resolve before the next milestone ships) ·
`P2` debt (scheduled) · `P3` note (recorded, low cost of leaving).
**Disposition** — `fixed` (resolved in-run, with the commit hash) ·
`filed` (became tracked work — WAL / `TASKS.md` / a PROP) · `accepted`
(deliberate no-action, with the reason) · `open` (carries to the next
run). **Categories** are PROP-013 §2.2: **A** test integrity, **B** rot
outside the gate, **C** drift, **D** debt, **E** discipline depth
(added 2026-06-12).

---

## Audit run — 2026-05-23 (seed)

The **seed run**. It records the findings already in hand at the close
of the M1.19 session — it is **not** a fresh full sweep of the PROP-013
§2.2 checklist. The first full sweep is the next invocation; this seed
gives that run a populated inventory to carry forward rather than a
blank page. Findings came from the M1.19 work itself and from the
WAL's standing Known-issues list.

**13 findings** — 2 P1, 4 P2, 7 P3. Disposition: 2 fixed, 1 filed,
1 accepted, 9 open. **10 carry forward** to the next run.

### 2026-05-23-01 · A1 · P1 · filed

**Production git-registry + naming path is under-tested.** The install
e2e suite drives `LocalRegistry` (the `--registry <dir>` path) almost
exclusively; that path bypasses `GitPackageRegistry` and
`NamingConvention` — the code a real registry install actually runs.
The `vibe init` default-config path (no `--registry`) has no e2e at
all. This is the gap that let finding `-02` ship green through eight
milestone phases. **Filed:** the test-hardening work — a hermetic
harness driving `GitPackageRegistry` against real `file://` git
repositories named per the `fqdn` convention, plus a default-path
`vibe init` → `vibe install` e2e. Large enough for its own milestone
task or a PROP.

### 2026-05-23-02 · A1 · P1 · fixed (`cc32d7e`)

**`vibe init` / `vibe registry add` scaffolded `kind-name`.** PROP-008
made `fqdn` the default `NamingConvention`, but `vibe init` hardcoded
`naming = "kind-name"` into every scaffolded `[[registry]]` block, and
`vibe registry add`'s `--naming` parser rejected `fqdn` outright. A
freshly-initialised project could not resolve a qualified pkgref.
Surfaced by the live registry-migration smoke during the M1.19
session. **Fixed** in `cc32d7e`.

### 2026-05-23-03 · A3 · P2 · fixed (`cc32d7e`)

**A test encoded the `vibe init` bug as correct.**
`crates/vibe-cli/tests/cli_init.rs::init_writes_default_registry`
asserted `primary.naming == NamingConvention::KindName` — so it stayed
green while the behavior (`-02`) was wrong. The instance is **fixed**
in `cc32d7e`; recorded here as the concrete case behind category A3 —
when a phase changes a default, updating the test that guards it must
be part of that same phase.

### 2026-05-23-04 · A2 · P2 · open

**`cli_live_e2e.rs` is `#[ignore]`d and red.** Live e2e tests against
real GitHub / GitVerse exist but are quarantined — not in the gate —
and currently red against the partially-migrated orgs. A
quarantined-and-red test is neither a safety net nor a signal.
**Open:** make them green and run them on a cadence (pre-release /
per-milestone), or consciously retire whatever is obsolete. Coupled to
`-07`.

### 2026-05-23-05 · B1 · P3 · open

**`fixtures/manual-test-packages/` carries retired schema.**
`flow-vibevm-github-smoke` — and likely `flow-vibevm-direct-push-smoke`
— still use `[writes]` and `[boot_snippet].filename` and carry no
`[package].group`; all retired by M1.18 and PROP-008. No hermetic test
parses these fixtures, so the gate stays green while they rot.
**Open:** a small de-rot pass, or delete them if the manual-test
recipes no longer reference them.

### 2026-05-23-06 · C4 · P2 · open

**GitVerse registry side un-migrated.** `vibespecs-gitverse` and the
`vibespecstest3` GitVerse org still predate PROP-008. The GitHub
publish token does not apply to GitVerse, and GitVerse exposes no API
DELETE, so this is owner web-UI / owner-token work. **Open** —
owner-only; gates nothing in-repo.

### 2026-05-23-07 · C4 · P2 · open

**GitHub test orgs `vibespecstest1/2` un-migrated.** They still carry
`kind-name`-shaped fixture repos. Re-laying them out is coupled to the
`#[ignore]`d `cli_live_e2e` tests (`-04`) — the fixtures and the
tests' expectations move together. **Open** — best done as one unit
with `-04`.

### 2026-05-23-08 · C4 · P3 · accepted

**Legacy `vibespecs/flow-*` repos archived, not deleted.** The M1.19
migration archived `flow-wal` / `flow-sync-from-code` /
`flow-atomic-commits` (read-only, reversible) rather than deleting
them. **Accepted:** archive is the reversible tidy; the owner can
delete them outright if a fully-clean org is wanted. Re-judge next run.

### 2026-05-23-09 · C2 · P3 · open

**PROP-005 references a non-existent `schemas/` directory.** PROP-005
§2.6 / §3.1 cite `crates/vibe-index/schemas/index-entry.jtd.json`, but
no `schemas/` directory exists under `crates/vibe-index/` — the index
wire types are hand-rolled serde structs. Spec-versus-reality drift.
**Open:** reconcile PROP-005 to the implementation, or add the JTD
schemas it describes.

### 2026-05-23-10 · C1 · P3 · open

**Residual doc requalification deferred from PROP-008 Phase 8.** Phase
8 reconciled the identity-defining docs (glossary, lockfile-format, the
install / version-syntax / git-source references) but deliberately
left a cosmetic sweep — requalifying `(kind, name)` / `<kind>:<name>`
example forms across the ~15 remaining peripheral command docs.
**Open:** low-priority doc tidy.

### 2026-05-23-11 · D1 · P3 · open

**Deferred PROP-011 refinements.** PROP-011 §5 / §8 record two: a
`content_hash` slot spot-check for `slot_integrity = verify` (needs
`compute_content_hash` lowered out of `vibe-registry`), and true
incremental re-resolution that skips the registry walk for an
unchanged subtree (needs PROP-003's SAT `pin_preferences`). **Open** —
both gate on other work; recorded so they are not forgotten.

### 2026-05-23-12 · D1 · P3 · open

**Parked backlog.** `version = { workspace = true }` member-version
inheritance (PROP-007 §6) and the publish-signalling polish
(`--archive`, `has_issues`) were parked behind larger milestones.
**Open:** re-judge whether either is still wanted.

### 2026-05-23-13 · D1 · P3 · open

**`NaiveDepSolver` is still the only depsolver.** PROP-003's SAT
solver (resolvo / libsolv) is unimplemented; `NaiveDepSolver` (DFS, no
backtracking) handles the current scale. Several deferred items
(`-11`) gate on the SAT solver. **Open** — architectural; not urgent
at current package counts, but a known ceiling.

---

## Audit run — 2026-06-10 (terraform close-out, instrumented category C)

Run during the terraform Phase 6 close-out, scoped to what the new
machinery can feed the audit automatically — category **C (drift)**
plus the gate panel. It is **not** the full §2.2 breadth sweep
(INT-0001 stays rescoped to the next audit window); its value is that
category C is now machine-fed, which PROP-013 never had before.

**Specmap panel** (`cargo xtask specmap --check`): 489 spec units, 170
tagged code items, 177 edges (79 item-grain — the pilot's 19 plus the
Phase 2/3/4 affirmations — and 98 module-grain scope markers),
**0 suspects**, 0 dangling
edges, the six known `pin-into-unmarked-unit` warnings (specmark usage
tests, retire with PROP-014 unit-ification).

**Orphan ratchet**: 0 gated orphans across the ten gated crates; 6
dispositioned under DBT-0019 (vibe-core error/timestamp/values — no
scannable home until `VIBEVM-SPEC.md` is unit-ified); 8 crates exempt,
each with its reason recorded in `specmap-ratchet.json`.

**Disputes**: DBT-0016 (PLAYBOOK vs BROWNFIELD marker homing) remains
the one open dispute, by design — it feeds the discipline package v0.2.

**Conform panel** (`cargo xtask conform check`): 6 findings
workspace-wide, all `unsafe-gate`, all frozen in
`conform-baseline.json` (4× vibe-cli output/main, 1× more in output,
1× vibe-index stop); scope `crates/vibe-resolver`: 0. New findings: 0.

| id | cat | sev | finding | disposition |
|---|---|---|---|---|
| AUD-0014 | C | P3 | `expand_features` doc-string says cycles are "rejected"; the seen-set silently terminates them (test `cycles_terminate` pins the actual behaviour) | open — one-line doc fix, flagged in the Phase 2 proposals note |
| AUD-0015 | C | P3 | `ResolvedNode` doc-comment cites "PROP-008 §2.3" where the identity tuple is §2.2 (#identity); §2.3 is #kind | open — same family as AUD-0014 |
| AUD-0016 | C | P3 | six `unsafe` blocks live outside any designated audit crate; frozen in the conform baseline, no audit-crate list exists yet | filed — the audit-crate designation is an owner decision; baseline may only shrink |
| AUD-0017 | D | P3 | vibe-core leaf trio without scannable spec home | filed — DBT-0019 |

---

## Audit run — 2026-06-12 (discipline depth — the full AI-Native sweep)

Owner-requested («полный аудит кода — насколько хорошо он соответствует
идеалам AI-Native Rust»): the INT-0001 audit window, run under the new
category **E (discipline depth)** this run adds to PROP-013 §2.2.
Method: the measuring stick is the installed Discipline corpus
(`GUIDE-AI-NATIVE-RUST` + the nine scaffold cards); mechanical censuses
over `specmap.json`, the conform rule sources, and the full tree; three
structural deep-reads (vibe-cli; vibe-registry; vibe-index/check/core);
one empirical gate probe on a clean tree.

**12 findings** — 1 P1 (fixed in-run), 7 P2 (filed), 4 P3. **Headline:
the adoption is real but approximately one crate deep.** vibe-resolver
carries nearly all of the discipline's mass (80 of 198 edges, 42 of 50
`#[verifies]`, all 4 `#[cell]` manifests, the only differential
oracle); the rest of the workspace is anchored, exempted, or gated by
rules weaker than their cards — and the panel's first gate was silently
red on `main` (-01).

**Instrumented panel for this run** (after the -01 fix, on the live
tree): `specmap --check` green — 352 units / 190 items / 198 edges /
0 suspects / 6 known warnings; `conform check` green — 8 frozen (all
`unsafe-gate`), 0 new; `test-gate` green (xfail-strict). `fast-loop`
budget figures inherited from 2026-06-11 (no code changed this run;
note the inherited panel predates the history rewrite, see -01).

### 2026-06-12-01 · E4/B2 · P1 · fixed (`9f06fbf`)

**`cargo xtask specmap --check` was red on a clean checkout of `main` —
the committed index had lost every content hash.** All 352
`content_hash` fields in `specmap.json` were empty while the scanner
emits real hashes (`specmap-core/src/mdspec.rs:273`); gate #1 of the
merge panel failed on an untouched tree ("out of date relative to the
tree" + five unbumped-hash drift lines), and the cross-session
editorial-drift audit had no stored baseline — the unbumped-hash
detector was structurally blind. Trail: the post-session **history
rewrite** of 2026-06-11 (every adoption-day commit re-hashed —
`1792c14`→`3ab0986`, `09d0da5`→`f244a7a`, …; pre-rewrite objects gone)
re-serialized `specmap.json` with hashes emptied; the close-out panel's
green specmap verdict certified the *pre-rewrite* tree. Empirical
probe: editing one revisioned unit fired drift on all five — the
stored side was uniformly empty. **Fixed in-run** (`9f06fbf`,
352-line hash-only diff); specmap/conform/test-gate re-run green on
the actual tree. **Open rider (owner):** what produced the rewrite? A
scrub/filter that re-serializes committed derived artifacts must
regenerate them or leave them alone.

### 2026-06-12-02 · E1 · P2 · filed

**The spec tree is anchored, not typed: 347 of 352 units carry no
kind/revision/status.** The entire formal REQ fabric is PROP-003's
pilot five (4 `req` + 1 `design`). Untyped units cannot participate in
revision discipline — asymmetric invalidation and the unbumped-hash
audit (`specmap-core/src/index.rs:211` deliberately skips revisionless
units) are dormant for 98.6 % of the spec. **Filed:** the unit-typing
program — type the implemented modules' PROPs first
(PROP-002/005/007/008/012), REQ grain, revision lines on.

### 2026-06-12-03 · E1 · P2 · filed (DBT-0019, escalated P3→P2)

**`VIBEVM-SPEC.md` (1190 lines) has zero units — and it is the only
spec home for ~24 kLOC.** Chain: vibe-cli (21.4 kLOC), vibe-mcp,
vibe-wire, xtask have no taggable spec → 8 crates ratchet-exempt → the
depth program cannot start for half the workspace. Was AUD-0017 / P3;
escalated: it now gates the remediation of every other E finding in
those crates.

### 2026-06-12-04 · E2 · P2 · filed

**Edge coverage is resolver-shaped.** 198 edges: vibe-resolver 80,
vibe-index 54 (all module-grain `scope!`), every other crate ≤ 13;
57 / 352 units (16 %) have any inbound edge. Implemented-but-unmapped:
**PROP-012** (15 units, 0 edges — yet shipped as
`vibe-core::manifest::redirect`, `vibe-check::check_redirect_blocks`,
the CLI `registry redirect*` commands, and the `<vibevm>` block in this
repo's own CLAUDE.md); **PROP-007** (24 units / 3 edged, vibe-workspace
4.7 kLOC); **PROP-005** (44 / 8, vibe-index 9.8 kLOC). PROP-010's 18/0
is honest (DRAFT — design session pending). **Filed:** affirmation
sweeps in the Phase-2 recipe; PROP-012 first (cheapest, fully shipped).

### 2026-06-12-05 · E2 · P2 · filed

**`#[verifies]` exists only around the resolver: 42 of 50 attributes
repo-wide.** vibe-cli 269 tests, vibe-core 180, vibe-index 137,
vibe-registry 123, vibe-workspace 103 — zero `#[verifies]` among them;
"what verifies this requirement?" is machine-answerable only inside
vibe-resolver. **Filed:** rides -02/-04 — once units are typed, tag the
strongest *existing* tests; no new tests needed for the first pass.

### 2026-06-12-06 · E3 · P2 · filed

**Cells exist at exactly one seam pair.** 4 `#[cell]` manifests
repo-wide (DepSolver naive/sat, DepProvider local/multi — all
vibe-resolver). The workspace has 8 seam traits; uncelled: `Registry`
(**3 production impls — a validated seam**: `LocalRegistry` lib.rs:574,
`GitRegistry` git_registry.rs:172, `GitPackageRegistry`
git_package_registry.rs:1275), `GitBackend` (1 impl — speculative until
a second backend), `RepoCreator`, `Transport`, `Frontend`, `Rule`. The
R-001 registry covers solver/provider flags only; `cell-has-oracle`
self-scopes to `#[cell]` crates → gates only vibe-resolver. **Filed:**
cell-ify `Registry` variants first — the seam is already proven.

### 2026-06-12-07 · E3 · P2 · filed

**God-files (R3-013) at the centers of gravity** — 23 src files over
600 lines. Worst: `vibe-cli/src/commands/registry.rs` 3245 (14 handlers
≈ 4 natural cells: sync / config / publish / redirect),
`vibe-registry/src/multi_registry_resolver.rs` 2870 (≥ 5
responsibilities), `vibe-registry/src/git_package_registry.rs` 2539
(≥ 6), `vibe-cli/src/commands/mcp.rs` 2460 (MCP server + agent-config
installer tangled), `vibe-check/src/lib.rs` 1913 (whole crate one file:
11 checks, hardcoded dispatch, no `Check` seam),
`vibe-core/src/manifest/package.rs` 1628 (19 types, wire conversions
inline), `conform-core/src/lib.rs` 1486 (the discipline's own engine in
one file), `xtask/src/main.rs` 1118; test-side `cli_e2e.rs` 5673 lines
/ 109 flat tests. **Filed:** the decomposition backlog — CLI
registry.rs and the two vibe-registry files first.

### 2026-06-12-08 · E4 · P2 · filed

**Two shipped conform rules are weaker than their cards; two
guide-mandated checkers don't exist.** (a) `seam-has-doctest` audits
`src/lib.rs` only (`conform-core/src/lib.rs:694`) — pub seams in
submodules ungated (the `GitBackend` trait, the ~47 pub methods of the
two registry god-files); (b) `error-enum-cites-req` checks for a
`#[spec]` attribute on the enum (`:880`), not the Class-F *message*
grammar — no product error Display text carries «violates REQ … fix
surface …» (vibe-registry's three enums confirmed message-bare; only
conform's own diagnostics speak the grammar); (c) guide §2 "position
is a resource" mandates a file-length warn — no such rule exists (see
-07); (d) guide §6's unwrap/expect-in-domain ban has no checker
(src-side upper bounds incl. inline test mods: vibe-registry 406,
vibe-workspace 257, vibe-index 222, vibe-core 218 — unmeasured, not
adjudicated). By the discipline's own law these are WISHes. **Filed:**
the conform rule backlog — widen (a) beyond lib.rs, grow (b) toward
message grammar, add (c) and (d, with cfg(test) exclusion); each lands
ratcheted (frozen baseline, shrink-only).

### 2026-06-12-09 · E2/E3 · P3 · filed

**vibe-index is structurally outside the discipline:** zero seam traits
across 9.8 kLOC (scanner trio, rate limiter, persistence all concrete),
zero item-grain tags (54 module `scope!` markers only), zero doctests,
not in the doctest/error gates; all tests integration-grain (2.9 kLOC
in tests/, none in-module). Natural first seam: `PackageScanner` over
from_clones / from_github. vibe-mcp: same family (exempt, untagged,
`Transport` seam bare). **Filed.**

### 2026-06-12-10 · E3 · P3 · accepted

**The fast-loop "cell" is the crate, not the discipline's module-grain
cell** — the 18 budget cells are the 17 crates + xtask; only resolver
modules carry true manifests. **Accepted** while every crate fits the
60 s budget; revisit at the first breach or when vibe-cli decomposes.

### 2026-06-12-11 · D · P3 · open

Hygiene census for the record: `#[ignore]` 5 (vibe-cli live quartet +
1 specmap-core); `#[allow]` 28 src-side (19 in vibe-cli); `anyhow`
outside the binary edge: conform-core 2 / specmap-core 6
(internal-tooling crates — borderline-legal, noted); TODO-family ≈ 17
raw, of which 14 are vibe-check's own detector pattern strings (false
positives). Nothing actionable beyond carried -04 (cli_live_e2e).

### 2026-06-12-12 · C3 · P3 · fixed (this run's WAL/AUDIT commits)

**WAL and CONTINUE cited commit hashes that no longer exist**
(`e3f06ec` … `1792c14` — the pre-rewrite chain). Same root event as
-01. Fixed: this run's WAL checkpoint records the live chain;
CONTINUE.md is rewritten at the next session-end per protocol. Rider
to -01 stands: owner to confirm the rewrite was intentional.

### Carry-forward (2026-05-23 series + 2026-06-10), re-judged

- **2026-05-23-01** (A1, git-registry path under-tested) — **reduced**:
  the Phase-3 hermetic differential oracle drives both provider cells
  over real bare `file://` git repos (fqdn-named), and cli_e2e carries
  git-registry + redirect e2e; the `vibe init` default-path e2e remains
  unverified this run. Re-judged P2 → P3, open.
- **-04** (quarantined live e2e red) — open, unchanged (4 `#[ignore]`
  sites in vibe-cli).
- **-05** (manual-test fixture rot) — open. **-06 / -07** (registry-side
  migrations) — open, owner-court. **-08** (archived legacy repos) —
  accepted, stands. **-09** (PROP-005 `schemas/` dir) — open. **-10**
  (doc requalification sweep) — open. **-11** (PROP-011 refinements) —
  open, both still gated. **-12** (parked backlog) — open.
- **-13** (NaiveDepSolver the only solver) — **superseded in
  substance**: the `Sat` cell landed 2026-06-11 (DBT-0011 closed);
  what remains is the production *selection* decision via the R-001
  registry — owner-gated. Re-pointed, P3.
- **AUD-0014 / AUD-0015** (doc-string one-liners) — open; cheap, fix on
  next resolver touch. **AUD-0016** (no designated unsafe-audit crates;
  now 8 frozen) — filed, owner decision → **fixed** (SHRINK-PLAN v0.2,
  same day — see the second same-day update below). **AUD-0017** —
  folded into **2026-06-12-03** (DBT-0019, escalated).

### Same-day disposition update — the depth program executed

The owner directed the filed program to completion the same day
(«вся программа глубины должна быть выполнена до конца»); all seven
filed P2s closed in one commit series (hashes in the WAL checkpoint):

- **-02 · fixed.** 67 kind/revision lines typed the implemented PROPs'
  decision units; the formal REQ fabric grew 5 → 72 typed units
  (59 `req` + 13 `design`).
- **-03 · fixed (DBT-0019 closed).** The scanner reads
  `VIBEVM-SPEC.md` as a root spec doc; 90 anchors landed additively;
  the vibe-core trio carries scope! edges; vibe-cli left the ratchet
  exemption (21 module markers); the six dispositions retired. The MCP
  surface got the honest treatment instead of a wrong edge: DBT-0020
  + 10 dispositions.
- **-04 · fixed.** Edges 198 → 347, tagged items 190 → 337; PROP-012
  went 0-edged → implemented+verified (block engine, vibe-check rule,
  plan-time validation all tagged); item-grain landed in vibe-index
  (54 → 76 items), vibe-workspace (8 → 31), vibe-core (13 → 41).
- **-05 · fixed.** `#[verifies]` 40 → 104; the strongest suites of
  vibe-cli (17), vibe-core (16), vibe-index (9), vibe-registry (3+3
  oracle), vibe-workspace (10), vibe-check (oracle) now machine-link
  to their REQ units, r-pinned.
- **-06 · fixed.** `#[cell]` manifests 4 → 18: the `Registry` seam's
  three production variants (local / git-monorepo / git-per-package,
  each with a cell-has-oracle reference) and vibe-check's new `Check`
  seam (11 check cells behind one `all_checks()` registration point).
  Residual, recorded: Registry-cell *selection* is config-driven, not
  yet R-001-flag-driven — the frozen
  `R-001|commands/install.rs|LocalRegistry` finding is its tracker.
- **-07 · fixed.** All six named cuts executed: CLI
  `commands/registry.rs` 3245 → 6 modules; `multi_registry_resolver`
  2870 → 5; `git_package_registry` 2539 → 4; `vibe-check` 2010 → root
  + 11 cell files (every file ≤ 600); `manifest/package.rs` 1755 →
  597-line hub + 4; `conform-core` 1811 → 7; `cli_e2e.rs` 5673 → 4
  feature binaries + common (109/109 tests green). Residual 28
  over-budget files frozen under `file-length`, shrink-only.
- **-08 · fixed.** Three rules + one widening shipped and frozen via
  the new `cargo xtask conform freeze`: `error-message-cites-req`
  (68 frozen), `file-length` 600 (28), `no-unwrap-in-domain` (24 —
  the honest domain count once cfg(test) scoping is real),
  `seam-has-doctest` beyond lib.rs (1 new: `GitBackend`). Baseline
  130 entries total, shrink-only from here.
- **-09 · reduced, open.** vibe-index gained item-grain tags and 9
  verifies edges; the `PackageScanner` seam (zero traits in 9.8 kLOC)
  remains the open structural item.
- **-10 / -11** unchanged (accepted / open).
- One forced deviation recorded en route: the e2e install cluster
  lives in `tests/cli_pkg_cycle.rs` — Windows UAC installer detection
  (os error 740) refuses unelevated exes whose names contain
  "install"/"update"/"setup"; the PROP-007 §9.5 lesson, met again.

### Second same-day disposition update — SHRINK-PLAN v0.2

The owner directed the three moves v0.1 §8 had reserved
(«execute all the spec/terraforms/SHRINK-PLAN-v0.2.md»):

- **AUD-0016 · fixed** (`be4aaef` and the two commits before it).
  The unsafe-gate posture, redesigned: **`env-audit`** is the
  designated audit crate — a process-global serialized, restoring
  `EnvGuard` behind a safe API replaces the three hand-rolled test
  guards (whose own SAFETY comment admitted a transient-observation
  race; the mutex closes it). The two production boundaries that
  cannot move — vibe-cli's startup env promotion, vibe-index's
  `libc::kill` FFI — testify at fn grain via
  `#[spec(deviates = ENGINE-CONFORM-v0.1#rules, reason)]`, which
  frontend v5 now extracts (`UnsafeUse.in_test` / `.in_deviation`)
  and the rule honors per ENGINE-CONFORM §4. Test-context unsafe is
  deliberately NOT exempt (unsoundness in tests is still
  unsoundness). Baseline 10 → 2: every unsafe-gate fingerprint left
  by drain, none by freeze-widening; the residual 2 is the DBT-0020
  MCP pair, untouched by owner instruction.
