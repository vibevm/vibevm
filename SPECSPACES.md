# Specspaces

Sub-projects hosted in this repository that are worked on as
independent projects, each with its own boot contract, WAL, and
cold-resume file. Canon: `flow:org.vibevm.world/wal-specspaces`
(installed; authored in `packages/org.vibevm.world/wal-specspaces/`).
The scoped grammar and target resolution come from its boot snippet
(slot 11, read at boot); this file is the registry.

`default:` sets what a **bare** session phrase (`RESUME SESSION` /
`ВОССТАНОВИ СЕССИЮ` with no name) targets. `default: host` — the value
here — means a bare phrase resumes or winds down the **host** project
(this repository's own `spec/WAL.md` + `CONTINUE.md`), never a
specspace by accident. Target a specspace by naming it
(`RESUME SESSION fractality`); an explicit name or directory always
overrides this default.

default: host

| name | root | wal | continue | status |
|---|---|---|---|---|
| fractality | packages/org.vibevm.fractality/ | WAL.md | CONTINUE.md | 2026-07-12 — **five-task goal COMPLETE (5/5)** (~28 commits, both remotes). ✅1 branch cleanup · ✅3 PP-004 (caps + `decisions` verb + schema/Silo menu tasks; gated re-run MT-C3-03 fired+scored: P-C3-a CONFIRMED as a hard count 80%, P-C3-b SUPPORTED, P-C3-d CONFIRMED — the two Ф6-inconclusive predictions moved) · ✅4 PP-001 (MT-C2-05 initiative re-run fired+scored: A′=11.1% B′=0% — repairs did NOT move cold delegation, REPLICATES Ф6; the RLM gate 44.4% is the lever, initiative hooks are not — validates the C3 pivot) · ✅5 PP-002 (`worker_credibility` query + the cold-board credibility line, D7, answers the F24 keep-reason). **Remaining: 2 validated Stage C** — `advise` verb + advisor ladder + help/hurt MT-C3-02 pre-reg + menu/preambles landed; harness+hidden-tests+scorer building (GLM), then fire alone×3/advised×3 + the C-3 uncertainty-trigger doc. **NEW binding rule:** paid-run evidence → committed `reports/trial-results/` (dated groups, per-meaning READMEs; `save-results.sh`). Prior: Stage B COMPLETE (the RLM runs). |
