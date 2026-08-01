# CONTINUE — cold-resume snapshot (2026-08-01, session end)

**Do not quote the numbers in this file. Measure them:**
`python campaigns/packages-2026-09/tasks/drift-registry.py` ·
`python campaigns/packages-2026-09/tasks/summary.py`.
`spec/WAL.md` was rewritten this session (anchored, whys cited) and supersedes
this snapshot wherever they diverge.

**TL;DR.** PROP-043 wave 2, Phase D, near its exit. This session ran waves 8–10:
re-verified the release route (16/40 fell), executed THE PUBLICATION (local, no
bumps — `vibe reinstall --force`; the address family closed whole, `../flows/`
in STATIC.md 69→0), landed six owner rulings (group A=(2), B+A′, F-220(b),
B-004(i), marker fork (а)+B-011, «Проведи всё это» for health-audit), applied
the twelve spec/common decision records, партия 1a, and the health-audit
adoption. Corpus **11 070 / 232 / 44 — 97.6 %** (session start: 96.2 % / 391);
registry **108 obligations / 232 drifts** (start: 165/391). Everything committed
and fanned out to both mirrors (`cargo xtask mirror` — the B-009-fixed standard
rollout).

## THE ONE OPEN TAIL (first action next session)

`vibe progress seal --campaign campaigns/packages-2026-09 <4 files>` **refused
4 paths** after the decision-record insertions (among PROP-000, PROP-018,
PROP-024, spec/design/README.md, PROP-014 — read the refusal output). Nothing
was written — refusal is the tool's safe mode. The D13 merges (30+10 confirmed,
0 merge refusals) stand in `run/cache.json` unsealed. **Fix:** run
`vibe progress mirror --campaign campaigns/packages-2026-09`, re-run the seal
on those files, READ THE REASONS (likely: the new `##record` anchors changed
the anchor sets), then `drift-registry.py --write`.

## Where work stands

- Branch `main`, synced to both mirrors (GitVerse=origin + GitHub); tree clean
  at session end except scratch.
- `cargo install cargo-audit cargo-outdated --locked` was running in background
  at wind-down — verify `cargo audit --version`; re-run if absent.
- Health-audit skill installed (5 projections, incl. `.agents/skills/`).
- The волна-10/D13 §7 LOG entry is NOT yet written — write it from this file +
  `git log` before other work.

## Owner's queue (nothing here proceeds without him)

1. **Sync group B, партии 1b–1d** — 1a applied; remaining: F-146×5 + F-206×2
   (ENGINE-CONFORM), F-159×5 (LEDGER-INTENT), F-207, F-263. Prepared texts in
   `harvest/d7a-core-sync-reverify.md`; present per document, full texts in
   chat (owner's format ruling (ii), 2026-08-01).
2. **B-004 carve-out choice** (d12-adr §3.13: `##SPLIT-HOST-POSTURE` — the
   claim-vs-section unit question; changes the census 35→36).
3. **Build-or-demote tail** (~17/21) and the rest of the owed set — registry.
4. **Phase D exit gate** when the queues drain: CONVERGENCE owed→0-or-ruled,
   `progress check` green over both corpora, `baseline.json` (A6), and the
   260-vs-259 file-count discrepancy reconciled (plan §11's own flag).

## Rules that bit this session (law: batch plan §3.6/§3.7/§3.8/§6.1 + WAL #constraints)

verdict-first for false confirms · a strike-by-ruling checks each anchor's own
cache reason, never the row's · bulk re-judges go through the instrument's join
(a naive filter matched 104 vs the true 46) · `progress check --exhaustive
--campaign` WRITES zone state (B-010); never point anything at
`campaigns/progress-2026-08` · briefs cite durable files only (never this file
or the WAL) · merge-verdicts and seal never chained · §3.8 audiences (Go/TS =
package-own bench only) · `legacy-spec/**` excluded · every number names its
command; git figures name their HEAD.

## Map

`campaigns/packages-2026-09/`: PHASE-D-BATCH-PLAN.md (the law), the executed
PHASE-D-PUBLICATION-RUNBOOK.md (fork ruled (а); B-011 = the `#use spec://… as
SOMETHING` aliasing design), the three queues, `harvest/d7a…d13-*`,
`tasks/*.py`. `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` §7 LOG
— read from the end. `BACKLOG.md` B-001…B-012 (B-012 = PROP-014's ten unbuilt
mechanisms, «провести исследование, можно ли реализовать»). `spec/WAL.md` —
current and anchored.

## Quick-start

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/drift-registry.py
bash tools/self-check.sh   # last known green pre-D13; re-run
```
