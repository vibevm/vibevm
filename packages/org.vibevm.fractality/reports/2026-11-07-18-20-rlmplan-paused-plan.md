# rlmplan — paused plan (2026-07-11 18:20)

**Plan:** `fractality/v0.1.0/spec/plans/FRACTALITY-RLM-PLAN-v0.1.md` —
Campaign 3 · Stage B, Option B. **Paused at the 70%-context boundary**
per the owner's standing rule (checkpoint + ask for a restart, not a
blocker). All work floor-green and committed; the next session resumes
cold from the WAL + this file + the state-plan tracker.

## Done this session ✅

- [x] RP-C3-1 ruled → **Option B** (plan §1, §8); status COMMISSIONED
- [x] Option C (advisor) postponed → **PP-003**
- [x] Dashboards: started-plan + live state-plan tracker (carries the
      seam reconnaissance — the next session need not re-read the crates)
- [x] **Ф0 spikes CLOSED** — all four seams green (s1 ran: jsonschema
      0.47.0 on rustc 1.93.1; s2/s3/s4 by inspection/design). Report
      `2026-11-07-18-12-campaign3-f0-spikes.md`
- [x] **Ф1.1** `ContextSpec.context_from` access-list (D-C3-2) — landed
      `35a378c`, floor green (165 tests / conform 0 / specmap 170/63/63/0)

## Exact stop point

**Execution stopped cleanly after committing Ф1.1.** The next item is
**Ф1.2** — `OutputSpec.output_schema` + validation at the pod/collect
seam + one retry-on-violation. Nothing is half-done; the working tree is
clean, `main` is ahead of origin by the session's commits (pushed at this
checkpoint).

## Remaining checklist

- [ ] **Ф1.2** output_schema field (core, raw JSON string) + pod/collect
      validation (jsonschema 0.47.0) + one retry (pod re-invoke — read
      pod/supervise first; scope the retry carefully)
- [ ] **Ф1.3** BudgetSpec six axes + wall-clock (RD-4)
- [ ] **Ф1.4** D-C3-3 boundary behaviors per verb (MC + profiles)
- [ ] **Ф2** need-gate + delegation-rules (own Cargo workspace) (D-C3-1, D-C3-10)
- [ ] **Ф3** descent verbs — await any/all/named + merge node (D-C3-4, D-C3-5)
- [ ] **Ф4** escalation channel (D-C3-6) — the s4 design is in the Ф0 report
- [ ] **Ф5** acceptance / PP-002 fold-in (RD-11, FD-9) + journal schema (D-C3-8)
- [ ] **Ф6** trial (D-C3-9) — **HARD STOP at RP-C3-2** before paid arms
- [ ] **Ф7** close — verdicts, deferrals, reports, WAL, WORKSPACES.md

## Key decisions / findings carried forward

- **jsonschema 0.47.0** is the output_schema validator (default-features
  off); violation shape `at <JSON-Pointer>: <message>`.
- **Delegation reality:** opencode/GLM stalled twice on this box today —
  floor/test runs go through backgrounded cargo; discipline-bound code
  stays boss-side. (Phase-5 datum, in the WAL scoreboard.)
- **Seam map** (in the state-plan tracker): context_from→ContextSpec,
  output_schema→OutputSpec, six axes→BudgetSpec, escalated→new terminal
  RunState + climbs `parent` edges. No external struct literals of these
  types exist, so extensions touch only `impl Default` + the golden snap.

## Risks / problems / uncertainties

- **Ф1.2 retry-on-violation** is the first non-trivial control-flow
  change (pod re-invokes a worker once). Read `pod/supervise.rs` +
  `pod/main.rs` before coding; keep the retry pod-local (no MC round-trip)
  if the lifecycle allows, else scope it down and note the deferral.
- **Context budget** is the pacing constraint, not correctness — each
  slice is independently floor-green and committed, so a mid-plan stop is
  always safe (this pause proves it).
- The Ф6 trial is paid and gated (RP-C3-2) — do not let an autonomous run
  drift into firing arms without the owner's verbatim word.

## Source of truth

Spec tree wins on divergence: plan (decisions + §10), the two syntheses
(RD-n / FD-n), `WAL.md` (living state). This dashboard is the
owner-facing snapshot; the WAL supersedes it if they diverge.
