# Morning routine — the human side of WAL {#root}

The WAL is a two-sided protocol. The AI writes it at the end of each
session; the human reads it at the start of each day. Without the
morning read, the WAL rots: the AI's claim that "all tests pass" drifts
from your memory that the timeout test was flaky yesterday.

This is the five-minute ritual that keeps the two sides in sync.

## Read before coding {#read}

Before opening any code, before starting any agent session:

1. Open `spec/WAL.md`.
2. Read it end to end.
3. Compare what it says against what you remember. If the two disagree,
   **head wins** — edit the WAL to match what you remember, *then* start
   the session.

Head wins because persistent human memory is the one authoritative
channel the AI cannot reach directly. If the WAL says "all tests pass"
and you remember the timeout test is flaky, the WAL is wrong: fix it
before the next AI session reads and trusts it.

## Fast skim {#skim}

A useful shortcut when the WAL has grown: drop the following line into
your project `CLAUDE.md` (or equivalent), then ask the agent for a
`/wal-status` read.

```markdown
## /wal-status
Read spec/WAL.md end to end. Emit in ≤ 10 lines:
- one line: current phase and status
- up to three bullets: what needs attention (blockers, risks)
- one line: next priority step
If the WAL `_Updated:` line is older than 24 hours, warn first.
```

The agent's 10-line summary is a quick daily read. It does not replace
the end-of-week full re-read — the same way a `top` command does not
replace looking at dashboards.

## Weekly re-read {#weekly}

Once a week, re-read the key PROP/FEAT documents end-to-end. Watch for:

- Internal contradictions (§2 says one thing, §5 says another).
- REVIEW markers older than your configured threshold.
- Orphan anchors (`{#something}` that nothing references).

These drift silently. A scheduled re-read is the garbage collector for
the spec corpus.

## If the WAL is clearly stale {#stale}

If you come back after a week and the WAL's `_Updated:` line is from
last Tuesday, the AI's previous session clearly did not close cleanly.
Your move:

1. Do NOT start a new agent session yet.
2. Look at `git log` and `git diff` for the interval. Reconstruct what
   actually happened.
3. Rewrite the WAL yourself to reflect the true current state.
4. *Now* start the next session.

The human is the backup for the WAL. This is one of the situations
where the human is irreplaceable in the system, and where 10 minutes of
manual reconciliation prevents hours of agent confusion.
