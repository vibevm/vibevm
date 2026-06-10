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
outside the gate, **C** drift, **D** debt.

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
