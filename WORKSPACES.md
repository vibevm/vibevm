# Workspaces

Sub-projects hosted in this repository that are worked on as
independent projects, each with its own boot contract, WAL, and
cold-resume file. Canon: `flow:org.vibevm/wal-workspaces`
(authored in `packages/org.vibevm/wal-workspaces/`); the local
grammar lives in `CLAUDE.md` §Workspaces.

| name | root | wal | continue | status |
|---|---|---|---|---|
| fractality | packages/org.vibevm.fractality/ | WAL.md | CONTINUE.md | 2026-07-10 — IGNITION **EXECUTING**: Phases 0–1 executed; Phase 2 (delegate-out) CODE LANDED, floor all-green — profiles → clean-slate env → invocation → pod run-spec → MC spawn → `fractality run`; open exit criterion: the live GLM E2E (hello-glm), queued as the next session's opening step per CONTINUE.md; plan: fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md |
