# INTENT — human view of `intent.json`

Generated from [`intent.json`](intent.json) (edit the JSON, regenerate
this view). Phase −1 harvest: 2026-06-10, tree `ccbc3d9`. Sources:
`spec/WAL.md` (Next / Known issues / pending decisions), `CONTINUE.md`
next-steps, `ROADMAP.md` open milestones + side quests. `TASKS.md`
does not exist (its dangling ROADMAP pointer → DBT-0017). WAL
"Known issues" items are deliberately *not* duplicated here — they are
debt, single-homed in `debt.json`.

**31 entries** — 30 open · 1 done. Carry-over guarantee
(BROWNFIELD §8): at beta exit every entry is
`done | rescoped | rejected`; unaccounted = 0 is a hard exit criterion.

## Near-term (WAL / CONTINUE-rooted)

| id | intent | links |
|---|---|---|
| INT-0001 | First full PROP-013 audit run (fresh sweep; re-judge 10 carried findings; reconcile AUDIT ↔ debt.json) | PROP-013, AUDIT.md |
| INT-0002 | Close the P1 coverage gap: hermetic `GitPackageRegistry` harness + default-path init→install e2e | DBT-0001 |
| INT-0003 | PROP-010 local package cache (M1.20): owner design session (5 OQs), then implement | PROP-010 |
| INT-0004 | M1.5 Generation (deferred behind base-machinery-first; 5 sub-slices + acceptance) | ROADMAP M1.5 |

## M1-era residue (ROADMAP)

| id | intent |
|---|---|
| INT-0005 | Docs completeness: every command documented; close the M1.5-gate checklist |
| INT-0006 | `vibe check` deferred checks #2/#3/#4/#10 + `--fix` (check #4 catches the DBT-0015 class) |
| INT-0007 | `vibe show graph/node/plan` (rides M1.5) |
| INT-0008 | M1.6 residue: resolver perf; attestation (optional); more publish adapters on demand |
| INT-0009 | vibe-mcp follow-ups: plan-preview+confirm; discovery tools; Gemini/Copilot; M1.7 smoke recipe; toml_edit |
| INT-0010 | `vibe review` static quality scoring (M1.8) |
| INT-0011 | `describes` PURL linkage (M1.9) |
| INT-0012 | `vibe outdated` (M1.10) |
| INT-0013 | SAT depsolver (resolvo) + pin-preferences + true incremental re-resolve |

## M2 / M3 (ROADMAP)

| id | intent |
|---|---|
| INT-0014 | M2.1 LLM-based install review |
| INT-0015 | M2.2 plugin contribution model v2 (graph nodes) |
| INT-0016 | M2.3 private registries — *likely subsumed by shipped M1.14; verify at reconciliation* |
| INT-0017 | M2.4 cross-platform CI + pre-built binaries |
| INT-0018 | M2.5 error polish + `vibe doctor` |
| INT-0019 | M2.6 opt-in telemetry |
| INT-0020 | M2.7 `vibe review --optimize` + multi-model |
| INT-0021 | M2.8 lazy-push / lazy-pull runtime via vibe-mcp |
| INT-0022 | M2.9 scenario generation from real commits |
| INT-0023 | M3+ speculative: interpret mode; multi-stack; skill layer; hosted registry; registry explorer |
| INT-0024 | M3.1 security threat model (research-only, parked) |

## Side quests (ROADMAP)

| id | intent | state |
|---|---|---|
| INT-0025 | `.gitattributes` `eol=lf` — *actively relevant: content-hash drift risk on Windows* | open |
| INT-0026 | `git config gc.auto 0` + manual gc procedure | open |
| INT-0027 | Workspace README.md | open |
| INT-0028 | CHANGELOG.md | **done** (exists, maintained) |
| INT-0029 | Clippy lint promotion — partially overtaken (`-D warnings` already gates); residue: pedantic set, CI | open |
| INT-0030 | `cargo deny` in CI | open |
| INT-0031 | Docs site (mdBook/Zola) | open |

---

## Reconciliation — 2026-06-10 (terraform Phase 6)

Every entry now carries a `resolution` in `intent.json` — the canonical
status holder; this file keeps the prose inventory. Outcome: **3 done**
(INT-0012 `vibe outdated`, INT-0016 private registries in substance,
INT-0028 CHANGELOG), **1 rejected** (INT-0017 CI matrix — the no-CI
posture is a standing Rule-4 owner decision), **27 rescoped** to their
durable homes (ROADMAP milestones, AUDIT carries, DBT ids, PROP open
sections). Unaccounted: **0** — the Phase 6 beta-exit requirement.
