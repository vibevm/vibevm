# DEF-C2 slice — STARTED plan dashboard

_2026-07-10 21:11. Plan identifier: `defc2slice`. Commissioned by the
owner post-close, verbatim: «…после чего сохрани сессию (вероятно
нужно будет вначале доделать DEF-C2-1…4)». Source of truth: the
CLOSED campaign plan's §15 deferrals (DEF-C2-1…4) — this slice
executes them as a direct-order follow-up, not a new campaign (one
session, ~4 commits, work pre-specified in §15); the wind-down
follows immediately after._

## Checklist

- [x] Owner rulings recorded first (`7a49159`): MT-C2-01…04 signed,
      RP2 ON, RP3 settings.local.json
- [ ] **DEF-C2-2a** — trial runner: restore MSVC auto-detect under
      `env -i` (whitelist the ProgramFiles family); verified by a
      before/after repro build in scratch
- [ ] **DEF-C2-3** — cold-start board: when all-time runs = 0, the
      global block leads with the route verb + spawn pointer instead
      of zero counters (D7-factual; engine tests pin the text)
- [ ] **DEF-C2-1** — mid-work nudge: `decide_midwork_nudge` in the
      engine + PostToolUse `additionalContext` emission in the hook
      (same threshold, same shared cooldown anchor, distinct journal
      reason, `midwork_nudges` config knob + kill switch); D5
      rewritten in place; staged scratch smoke
- [ ] **DEF-C2-2b (thin slice)** — worker-credibility facts the bus
      already proves (completed/failed counts) surfaced at the
      decision point; the full "cargo-test proven" form explicitly
      remains next-campaign scope (needs acceptance-schema work)
- [ ] **DEF-C2-4** — MT-C2-05: the re-run protocol, pre-registered
      and UNFIRED (multi-prompt design + the mid-work channel +
      fatigue counters; paid runs gated on a new RP5, OPEN)
- [ ] floor green; commits grouped by meaning; slice report
- [ ] wind-down: WAL / CONTINUE / WORKSPACES + mirror push

## Key decisions going in

- The mid-work nudge reuses the session-level cooldown anchor
  (`last_nudge_ts_ms`), so the fatigue bound stays "one nudge per
  cooldown window per session" across ALL channels — no new state.
- Midwork fires on the slate threshold only; parked questions keep
  their two existing channels (prompt + stop).
- The statusline strip keeps its zero form (F20: invisible in `-p`;
  ambient zeros in interactive sessions are honest and tiny).

## Risks / uncertainties

- The env-whitelist fix is verified against THIS box's VS layout;
  other boxes may need more vars (recorded in the runner comment).
- Mid-work nudge adds one MC metrics round-trip per work-tool call;
  P4 headroom is 2× — re-bench after the change (revisit trigger:
  P95 > 100 ms).
- Fatigue for the new channel is unmeasured by design — that is
  MT-C2-05's job (RP5-gated), not this slice's.
