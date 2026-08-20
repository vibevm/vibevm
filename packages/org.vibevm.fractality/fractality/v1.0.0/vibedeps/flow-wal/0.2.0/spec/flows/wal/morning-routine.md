# Morning routine — the human side of WAL {#root}

**Scope of this document.** The five-minute human ritual at the start
of each day: read the WAL, reconcile it with your own memory, and only
then let an agent session begin. The WAL is a two-sided protocol — the
agent writes it at the end of each session, the human reads it at the
start of each day. Without the morning read, the WAL rots: the agent's
claim that "all tests pass" drifts from your memory that the timeout
test was flaky yesterday.

## Read before coding {#read}

Before opening any code, before starting any agent session:

1. Open `spec/WAL.md`.
2. Read it end to end.
3. Compare what it says against what you remember.

## Head wins {#head-wins}

If the WAL and your memory disagree, **head wins** — edit the WAL to
match what you remember, *then* start the session.

Head wins because persistent human memory is the one authoritative
channel the agent cannot reach directly. If the WAL says "all tests
pass" and you remember the timeout test is flaky, the WAL is wrong:
fix it before the next agent session reads and trusts it. (This is
the top of the conflict hierarchy — Human > Spec > Tests > Code >
WAL.)

## Fast skim: wal-status {#skim}

The daily read has a fast form. This flow ships a `wal-status` skill:
install it into your agent and ask for a WAL status at session start.
It reads `spec/WAL.md` end to end and answers in at most ten lines —
one line of phase and status, up to three attention bullets, one line
of next step — warning first when the WAL is older than 24 hours.

For an agent without skill support, paste the equivalent prompt into
your project's agent instructions:

```markdown
## /wal-status
Read spec/WAL.md end to end. Emit in ≤ 10 lines:
- one line: current phase and status
- up to three bullets: what needs attention (blockers, risks)
- one line: next priority step
If the WAL `_Updated:` line is older than 24 hours, warn first.
```

The ten-line summary is a quick daily read. It does not replace the
end-of-week full re-read — the same way a `top` command does not
replace looking at dashboards.

## Cold starts {#cold}

Coming back after a machine switch or a long gap, or arriving at a
repository that is not yours? Read `CONTINUE.md` at the repository
root first — it carries the tour, the commands, the map — then
`spec/WAL.md`, which is canonical wherever the two diverge. The full
contract is [`cold-resume.md`](cold-resume.md).

## Weekly re-read {#weekly}

Once a week, re-read the key spec documents end-to-end. Watch for:

- Internal contradictions (§2 says one thing, §5 says another).
- REVIEW markers older than your configured threshold.
- Orphan anchors (`{#something}` that nothing references).

These drift silently. A scheduled re-read is the garbage collector for
the spec corpus.

## If the WAL is clearly stale {#stale}

If you come back after a week and the WAL's `_Updated:` line is from
last Tuesday, the previous session clearly did not close cleanly. Your
move:

1. Do NOT start a new agent session yet.
2. Look at `git log` and `git diff` for the interval. Reconstruct what
   actually happened.
3. Rewrite the WAL yourself to reflect the true current state.
4. *Now* start the next session.

The human is the backup for the WAL. This is one of the situations
where the human is irreplaceable in the system, and where ten minutes
of manual reconciliation prevents hours of agent confusion.

## Never {#never}

- Never start an agent session on a WAL you have not read today.
- Never "fix" a stale WAL from memory alone — reconstruct from
  `git log` and `git diff` first; memory is reconstructive, not
  archival.
- Never let the skim replace the weekly full re-read.
- Never leave a known divergence in place "for later" — the very next
  session will read and trust it.

## Summary {#summary}

- Read the WAL every morning, end to end, before any session.
- Head wins: your memory corrects the WAL, never the other way around.
- `wal-status` is the fast skim; the weekly full re-read still happens.
- Cold start? `CONTINUE.md` first for the tour, the WAL for the truth.
- Stale WAL: reconstruct from git, rewrite by hand, then start.
