# fractality — WAL (project continuation state)

_Updated: 2026-07-09 (ignition; same day — owner amendments accepted) —
the IGNITION plan is **PLANNED · ACCEPTED with owner amendments**: the
supervision topology is now MC → **pod** → worker (D3; `fractality-pod`
is the sixth crate — pods own stdio/job-objects/watchdogs, talk HTTP to
MC, survive MC restarts, and are the future federation seam), a non-yolo
interaction layer exists as **Phase 4b** (profile allowlists + pod
permission broker + `ask_boss` + `waiting_on_boss` + `fractality
questions/answer`, D18), the CLI obeys a UNIX-ergonomics law (D17: `ps`,
`wait`, `logs -f`, semantic exit codes, `--json` everywhere). RP1
RESOLVED (dogfood = EULA→UPL-1.0 relicensing with minimal acceptance),
RP2 RESOLVED (wal-workspaces joins redbook — DEF-11, host-side). The
**interim opencode+GLM delegation paradigm** is live, verified
(`opencode run -m zai-coding-plan/glm-5.2 …` → OK) and recorded in the
contract. No code yet by design: Phase 0 (spikes, no commits) is the next
work; its findings rewrite Decisions in place before Phase 1 lands
anything._

## Current state

- **The plan (canonical for all campaign detail):**
  [`fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md`](fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md)
  — status `PLANNED`. Covers Phases 0–6: spikes → workspace + mission-control
  core → delegate-out → collect-back → swarm → policy layer (delegation
  rules + model playbooks) → boss integration + stats. Campaigns 2
  (initiative system) and 3 (RLM) are seeded in its deferrals ledger.
- **Foundation:**
  [`fractality/v0.1.0/spec/PROP-001-foundation.md`](fractality/v0.1.0/spec/PROP-001-foundation.md)
  — vision (agent OS), system model and glossary, architecture, invariants
  I1–I6, usage & ToS posture, naming, evolution horizons.
- **Tree:** spec corpus only. The Cargo workspace and crates are created by
  Phase 1, not before.
- **Refs intake:** not started. Sources and clean-room rules:
  [`fractality/v0.1.0/spec/refs/INVENTORY.md`](fractality/v0.1.0/spec/refs/INVENTORY.md).
  Clones land under the host `/refs/src/` (gitignored); only notes and the
  inventory are committed.
- **Host wiring:** registered in the host `WORKSPACES.md`; the workspace
  grammar (`восстанови/заверши сессию fractality`) is live in the host
  contracts; `flow:org.vibevm/wal-workspaces` 0.1.0 canonizes it.

## Constraints (do not violate without discussion)

- Do not scan or load the host vibevm project context; host facts live in
  the plan §5. (Owner directive, 2026-07-09.)
- Clean-room law binds all reference sources — inspiration-only, study-note
  → implement, no line porting. (Host directive 2026-07-07, extended to
  this workspace 2026-07-09.)
- Worker env never inherits `ANTHROPIC_*` / `CLAUDE_*` (invariant I1).
- Never read/print token file contents; existence checks only.
- All boss↔worker data exchange goes through files on disk (owner ruling
  2026-07-09) — no result passing through process stdout contracts beyond
  the recorded transcript, no shared memory, no sockets for *content*.
- Publish (any registry) is owner-word-only.

## Next

Execute plan **Phase 0** (spikes s1–s9: provider facts, nested-spawn spike,
stream-json fixtures, kill-tree probe, refs intake + licenses, landscape
one-pager, crate pins, host-gate probe). Then: rewrite affected Decisions in
place, flip the plan status to `EXECUTING` on the amendment commit, proceed
to Phase 1.
