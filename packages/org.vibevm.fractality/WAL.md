# fractality — WAL (project continuation state)

_Updated: 2026-07-10 (Phase 1 EXECUTED — workspace skeleton +
mission-control core live) — the six-crate Cargo workspace exists and the
whole Phase 1 surface is proven on this box: `fractality mc
start|stop|status`, `ps`, `show` over the versioned localhost bus
(lockfile + rotating bearer), append-only JSONL journal with tolerant
replay, pod skeleton with the Job-Object kill guarantee (F5), and the P8
early signal CONFIRMED by killing a real daemon process mid-run — the pod
re-registered with the new generation and the run completed with zero
manual repair. Floor is the full AI-Native panel (D15 rewritten by owner
directive: DEF-9 resolved early) and it is GREEN: fmt · 55 tests (incl.
10 doctests) · clippy -D warnings · conform 6/6 crates gated at an empty
baseline (51 findings drained in one pass) · specmap 31 edges / 0
orphans · test-gate. The workspace is now a real vibe consumer: redbook
^0.2.0 + rust-ai-native ^0.7.0 in vibe.toml, 26 packages materialised
into the workspace-local vibedeps/, the generated boot lane bound as
contract boot step 6 (a standing rule for every future fractality
package). Session rulings now in the contract: the vibevm **pilot
posture** (host defects fixed at once when blocking, else
VIBEVM-BACKLOG.md with per-item non-destructive verification recipes;
working-tree vibe only), the **delegation law** (mandatory GLM routing +
hand-run scoreboard), the **live-observation protocol** (logged
delegates, PROGRESS/TASK-DONE heartbeats, stall alarms, react on first
signal), and the **two context scenarios** (small task = discipline
compiled into the prompt; big task = delegate boots the corpus first).
Delegation scoreboard this session: **delegated 2** (27-site scope!-URI
swap; the 4-enum error-contract drain with self-verify — both
boss-verified, conform-accepted), **kept**: architecture/design,
policy edits, lifecycle doctests (judgment per the matrix), plus two
found-live bugs — F11 lost-wakeup shutdown hang (Notify → watch) and
F12 in-process "crash" that doesn't sever pooled connections (restart
test now kills a real process). Ledger + hashes: plan §14 Phase 1 map.
Next: **Phase 2 — delegate-out** (profiles D6, the D5 env constructor
with the poisoned-parent test, worktree manager D8, the claude-code
backend invocation, the spawn path through pods). Prior status follows._
_Prior: 2026-07-09 (Phase 0 EXECUTED — plan EXECUTING) — spikes all
green, findings F1–F10 folded into the plan (z.ai facts, headless fresh
CLAUDE_CONFIG_DIR, win32job KILL_ON_JOB_CLOSE proven, CC permission
surface incl. native `defer`, rustc 1.93.1 → sysinfo =0.37.2, refs
intake MIT + clean-room, landscape note); interim opencode+GLM paradigm
verified live and recorded in the contract._

## Current state

- **The plan (canonical for all campaign detail):**
  [`fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md`](fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md)
  — status `EXECUTING`, Phases 0–1 landed (ledger §14 carries both
  commit maps + findings F1–F13). Remaining: Phases 2, 3, 4, 4b, 5, 6.
- **Code:** `fractality/v0.1.0/` is its own Cargo workspace — six crates
  (`fractality-core`, `fractality-mission-control`, `fractality-pod`,
  `fractality-mc-client`, `fractality-backend-claude-code`,
  `fractality-cli`), three binaries (`fractality`,
  `fractality-mission-control`, `fractality-pod`), rust-version 1.93,
  `sysinfo =0.37.2` pinned (F9). Floor: `rust-ai-native floor` from the
  workspace root (zero-install recipe in the contract until `vibe bin
  build` lands — backlog item 6).
- **Discipline:** conform gates all six crates (empty baseline);
  specmap namespace `fractality` (31 edges, 0 orphans); pub-doctest
  gate is the named next ratchet.
- **vibe wiring:** workspace vibe.toml requires redbook ^0.2.0 +
  rust-ai-native ^0.7.0; vibedeps/ (26 pkgs) and spec/boot/INDEX.md are
  generated, committed, and bound as boot step 6. Install recipe (the
  working recipe, incl. why `--registry` is exclusive today):
  contract §"Driving vibevm here". Pilot findings + fix-list +
  verification plan: [`VIBEVM-BACKLOG.md`](VIBEVM-BACKLOG.md).
- **Host wiring:** registered in the host `WORKSPACES.md`; two redbook
  members (`atomic-commits`, `sync-from-code` 0.1.0) are vendored into
  the host `packages/org.vibevm/` (tag-pinned mirrors; do not edit).

## Constraints (do not violate without discussion)

- Host Rules 1–4 bind every commit (human-authored surface, Conventional
  Commits, topic-grouped, autonomy on routine only).
- The delegation law + live-observation protocol + two context scenarios
  (contract §"THE DELEGATION LAW") — mandatory, scoreboard in every WAL
  checkpoint.
- Clean-room law for every source in `fractality/v0.1.0/spec/refs/INVENTORY.md`.
- Worker env never inherits `ANTHROPIC_*` / `CLAUDE_*` (I1 — structural
  in the pod, poisoned-parent test pins it).
- Never read/print token file contents; existence checks only.
- MC is the bus; files are the persistence plane, never the medium (I2).
- Publish (any registry) is owner-word-only.
- Floor green at every phase boundary; never wait blind on long runs
  (background + polled verdict markers).

## Next

**Phase 2 — delegate-out** (plan §8): profile loading + validation (D6),
the D5 clean-slate env constructor with the poisoned-parent test at the
backend level, the worktree manager (D8), the claude-code headless
invocation builder, and the spawn path — `POST /v0/runs` provisions the
workspace and launches the pod; `fractality run` drives a GLM worker
end to end with the transcript on disk. Exit: a real GLM worker executes
`spec/examples/hello-glm.toml`; P2 confirmed on live transcripts.
