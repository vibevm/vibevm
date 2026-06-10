# DEBT — human view of `debt.json`

Generated from [`debt.json`](debt.json) (the machine-readable record —
edit that file, then regenerate this view). Phase −1 seed:
2026-06-10, tree `ccbc3d9`, branch `new`. BROWNFIELD-PROTOCOL §3;
severity scale PROP-013. AUDIT.md remains the social inventory this
registry mechanizes; the two reconcile at the first full audit run
(INT-0001).

**18 entries** — 1 P1 · 7 P2 · 10 P3. Dispositions: 1 filed ·
1 accepted · 16 open. Kinds: 5 disputed-spec · 4 stale-doc ·
3 unimplemented-req · 3 external-drift · 1 coverage-gap ·
1 failing-test · 1 orphan-code.

## P1 — blockers (resolve before the next milestone ships)

| id | kind | title | disposition |
|---|---|---|---|
| DBT-0001 | coverage-gap | Production git-registry + naming path is under-tested (the M1.19-defect gap) | **filed** → INT-0002 |

## P2 — debt (scheduled)

| id | kind | title | disposition |
|---|---|---|---|
| DBT-0002 | failing-test | `cli_live_e2e` is `#[ignore]`d and red (3 tests; → tests-baseline) | open |
| DBT-0004 | external-drift | GitVerse registry side un-migrated to fqdn (owner-only) | open |
| DBT-0005 | external-drift | GitHub test orgs `vibespecstest1/2` un-migrated (coupled to DBT-0002) | open |
| DBT-0012 | disputed-spec | PROP-002 §2.5 vs PROP-008 §2.5 — naming default + value set | open · **owner adjudication** |
| DBT-0013 | disputed-spec | boot/00-core.md vs boot/90-user.md — registry host (frozen surfaces) | open · **owner adjudication** |
| DBT-0014 | disputed-spec | boot/90-user.md `<kind>-<name>` line vs PROP-008 / live org (frozen surface) | open · **owner adjudication** |
| DBT-0015 | disputed-spec | PROP-003 duplicate `{#phases}` anchor — URI ambiguity in the Phase 1 pilot PROP | open · **owner adjudication** |

## P3 — notes (recorded; re-judged each audit)

| id | kind | title | disposition |
|---|---|---|---|
| DBT-0003 | orphan-code | `fixtures/manual-test-packages/` carries retired schema | open |
| DBT-0006 | external-drift | Legacy `vibespecs/flow-*` archived, not deleted | accepted |
| DBT-0007 | stale-doc | PROP-005 references a non-existent `schemas/` directory | open |
| DBT-0008 | stale-doc | Peripheral command docs still use pre-PROP-008 example forms | open |
| DBT-0009 | unimplemented-req | PROP-011 deferrals: hash spot-check; true incremental re-resolve | open |
| DBT-0010 | unimplemented-req | Parked: workspace version inheritance; publish-signalling polish | open |
| DBT-0011 | unimplemented-req | `NaiveDepSolver` is the only depsolver (SAT solver unbuilt) | open |
| DBT-0016 | disputed-spec | PLAYBOOK vs BROWNFIELD — REVIEW/TODO marker homing (package-internal) | open |
| DBT-0017 | stale-doc | ROADMAP internal staleness (TASKS.md pointer, unticked boxes, …) | open |
| DBT-0018 | stale-doc | `vibe init` hint leads with the kind-qualified pkgref shape | open |

## Marker sweep — result and skip rule

The `<!-- REVIEW -->` / TODO / FIXME / HACK sweep over `crates/**`,
`xtask/**`, `spec/**` found **zero load-bearing instances**. Skip rule
used (recorded per playbook Phase −1): hits that are *references to the
convention itself* (docs, doc-comments, the `vibe check` implementation,
PROP texts describing markers) and *template literals that emit
placeholder text* (`init.rs`'s generated `_TODO: …_` README line) are
not debt. An `#[allow(…)]` sweep is deliberately left to the first full
PROP-013 audit (category D3) — it is not in the playbook's Phase −1
debt-source list.
