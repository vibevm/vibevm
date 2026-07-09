# fractality — WAL (project continuation state)

_Updated: 2026-07-09 (Phase 0 EXECUTED — plan now EXECUTING) — the
IGNITION plan's Phase 0 spikes all landed GREEN with no code committed
(spikes commit nothing; findings F1–F10 fold into the plan). Highlights:
nested spawn works with a clean-slate env (P1 ✅; Windows needs APPDATA/
LOCALAPPDATA in the whitelist — D5 updated); z.ai facts resolved (base
URL `https://api.z.ai/api/anthropic`, model mapping via
`ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU}_MODEL`, big=`glm-5.2[1m]`,
small=`glm-5-turbo`; quota is tier-scoped — the "4000 MCP" figure is the
Max tier — D6/D12 rewritten); GLM smoke ran headless first try on a fresh
CLAUDE_CONFIG_DIR (P2 ✅, R5 resolved), stream-json fixture captured with
usage fields; the pod kill-tree mechanism is proven (`win32job`
KILL_ON_JOB_CLOSE reaps the worker even when the pod exits without killing
— a pod crash leaks nothing; D3/D11 fixed); CC permission surface
confirmed incl. the `defer` native park-and-resume that maps onto
`waiting_on_boss` (D18 strengthened). MSRV finding: this box is rustc
1.93.1, so `sysinfo` pins to `=0.37.2` (Phase 1 sets a rust-version floor
or the owner bumps the toolchain). Refs intake done — all three studied
repos MIT, clean-room intact; codex-first study note + landscape note
written. Two grunt tasks were delegated to GLM-5.2 via opencode and
boss-verified (first Phase-5 field data). Next: the single Phase-0
amendment commit, then **Phase 1** (Cargo workspace + core + MC +
pod skeleton). Prior status follows._
_Prior: 2026-07-09 (ignition; same day — owner amendments accepted) —
the IGNITION plan is **PLANNED · ACCEPTED with owner amendments**: the
supervision topology is now MC → **pod** → worker (D3; `fractality-pod`
is the sixth crate — pods own stdio/job-objects/watchdogs, talk HTTP to
MC, survive MC restarts, and are the future federation seam), a non-yolo
interaction layer exists as **Phase 4b** (profile allowlists + pod
permission broker + `ask_boss` + `waiting_on_boss` + `fractality
questions/answer`, D18), the CLI obeys a UNIX-ergonomics law (D17: `ps`,
`wait`, `logs -f`, semantic exit codes, `--json` everywhere). RP1
RESOLVED (dogfood = EULA→UPL-1.0 relicensing with minimal acceptance),
RP2 RESOLVED (wal-workspaces joins redbook — DEF-11, host-side), RP4
RESOLVED (no yolo in v0.1 — the D18 stack is the way of life; if yolo
ever returns, worktree-restricted profiles only); the future
checkpoints layer (à la Entire.io Checkpoints) is recorded as DEF-12 /
PROP-001 §7 / inventory S8. I2 re-scoped by the owner's same-day
refinement: mission-control is the command bus for ALL boss↔worker
interface; files are the guaranteed persistence plane (NFS/Ceph-able in
the federation era), never the medium — D4/D10/D18 aligned. D19 added
same day: bulk data rides the bus as claim-check FileRefs
(scope-relative path + range, incl. a head/tail form for growing
files); filesystem scopes prove identity by a rendezvous beacon (mount
metadata + node identity as corroboration; `FRACTALITY_NODE_ID` +
`fractality node` expose where an agent runs); dereference locally only
on proven scope match, else the bus serves the bytes. The
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
