# Morning routine — the human side of WAL {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** The five-minute human ritual at the start
of each day: read the WAL, reconcile it with your own memory, and only
then let an agent session begin. @impl/done

##WAL-IS-A-TWO-SIDED-PROTOCOL The WAL is a two-sided protocol — the
agent writes it at the end of each session, the human reads it at the
start of each day. @impl/done

##WITHOUT-THE-MORNING-READ-THE-WAL-ROTS Without the morning read, the WAL rots: the agent's
claim that "all tests pass" drifts from your memory that the timeout
test was flaky yesterday. @spec/done

## Read before coding {#read}

##before-coding-lead Before opening any code, before starting any agent session: @impl/done

1. ##STEP-OPEN-THE-WAL Open `spec/WAL.md`. @impl/done
2. ##STEP-READ-IT-END-TO-END Read it end to end. @impl/done
3. ##STEP-COMPARE-AGAINST-WHAT-YOU-REMEMBER Compare what it says against what you remember. @impl/done

## Head wins {#head-wins}

##HEAD-WINS-EDIT-THE-WAL-TO-MATCH-MEMORY If the WAL and your memory disagree, **head wins** — edit the WAL to
match what you remember, *then* start the session. @impl/done

##why-head-wins Head wins because persistent human memory is the one authoritative
channel the agent cannot reach directly. @spec/done

##a-flaky-test-example If the WAL says "all tests
pass" and you remember the timeout test is flaky, the WAL is wrong:
fix it before the next agent session reads and trusts it. @impl/done

##top-of-the-conflict-hierarchy (This is
the top of the conflict hierarchy — Human > Spec > Tests > Code >
WAL.) @impl/done

## Fast skim: wal-status {#skim}

##the-daily-read-has-a-fast-form The daily read has a fast form. @impl/done

##FLOW-SHIPS-A-WAL-STATUS-SKILL This flow ships a `wal-status` skill:
install it into your agent and ask for a WAL status at session start. @impl/done

##SKILL-ANSWERS-IN-AT-MOST-TEN-LINES It reads `spec/WAL.md` end to end and answers in at most ten lines —
one line of phase and status, up to three attention bullets, one line
of next step — warning first when the WAL is older than 24 hours. @impl/done

##equivalent-prompt-lead For an agent without skill support, paste the equivalent prompt into
your project's agent instructions: @impl/done

```markdown
## /wal-status
Read spec/WAL.md end to end. Emit in ≤ 10 lines:
- one line: current phase and status
- up to three bullets: what needs attention (blockers, risks)
- one line: next priority step
If the WAL `_Updated:` line is older than 24 hours, warn first.
```

##the-summary-is-a-quick-daily-read The ten-line summary is a quick daily read. @impl/done

##THE-SKIM-DOES-NOT-REPLACE-THE-WEEKLY-RE-READ It does not replace the
end-of-week full re-read — the same way a `top` command does not
replace looking at dashboards. @impl/done

## Cold starts {#cold}

##cold-start-situations Coming back after a machine switch or a long gap, or arriving at a
repository that is not yours? @impl/done

##READ-CONTINUE-FIRST-THEN-THE-WAL Read `CONTINUE.md` at the repository
root first — it carries the tour, the commands, the map — then
`spec/WAL.md`, which is canonical wherever the two diverge. @impl/done

##cold-resume-contract-pointer The full
contract is [`cold-resume.md`](cold-resume.md). @impl/done

## Weekly re-read {#weekly}

##RE-READ-KEY-SPEC-DOCUMENTS-WEEKLY Once a week, re-read the key spec documents end-to-end. Watch for: @impl/done

- ##WATCH-FOR-INTERNAL-CONTRADICTIONS Internal contradictions (§2 says one thing, §5 says another). @impl/done
- ##WATCH-FOR-STALE-REVIEW-MARKERS REVIEW markers older than your configured threshold. @impl/done
- ##WATCH-FOR-ORPHAN-ANCHORS Orphan anchors (`{#something}` that nothing references). @impl/done

##these-drift-silently These drift silently. @spec/done

##A-SCHEDULED-RE-READ-IS-THE-GARBAGE-COLLECTOR A scheduled re-read is the garbage collector for
the spec corpus. @spec/done

## If the WAL is clearly stale {#stale}

##A-STALE-DATE-LINE-MEANS-THE-LAST-SESSION-DID-NOT-CLOSE If you come back after a week and the WAL's `_Updated:` line is from
last Tuesday, the previous session clearly did not close cleanly. Your
move: @spec/done

1. ##STALE-STEP-DO-NOT-START-A-SESSION-YET Do NOT start a new agent session yet. @impl/done
2. ##STALE-STEP-RECONSTRUCT-FROM-GIT Look at `git log` and `git diff` for the interval. Reconstruct what
   actually happened. @impl/done
3. ##STALE-STEP-REWRITE-THE-WAL-BY-HAND Rewrite the WAL yourself to reflect the true current state. @impl/done
4. ##STALE-STEP-THEN-START-THE-NEXT-SESSION *Now* start the next session. @impl/done

##THE-HUMAN-IS-THE-BACKUP-FOR-THE-WAL The human is the backup for the WAL. @impl/done

##where-the-human-is-irreplaceable This is one of the situations
where the human is irreplaceable in the system, and where ten minutes
of manual reconciliation prevents hours of agent confusion. @spec/done

## Never {#never}

- ##NEVER-START-ON-A-WAL-YOU-HAVE-NOT-READ-TODAY Never start an agent session on a WAL you have not read today. @impl/done
- ##NEVER-FIX-A-STALE-WAL-FROM-MEMORY-ALONE Never "fix" a stale WAL from memory alone — reconstruct from
  `git log` and `git diff` first; memory is reconstructive, not
  archival. @impl/done
- ##NEVER-LET-THE-SKIM-REPLACE-THE-WEEKLY-RE-READ Never let the skim replace the weekly full re-read. @impl/done
- ##NEVER-LEAVE-A-KNOWN-DIVERGENCE-FOR-LATER Never leave a known divergence in place "for later" — the very next
  session will read and trust it. @impl/done

## Summary {#summary}

- ##SUM-READ-THE-WAL-EVERY-MORNING Read the WAL every morning, end to end, before any session. @impl/done
- ##SUM-HEAD-WINS Head wins: your memory corrects the WAL, never the other way around. @impl/done
- ##SUM-SKIM-AND-WEEKLY-RE-READ `wal-status` is the fast skim; the weekly full re-read still happens. @impl/done
- ##SUM-COLD-START-ORDER Cold start? `CONTINUE.md` first for the tour, the WAL for the truth. @impl/done
- ##SUM-STALE-WAL-PROCEDURE Stale WAL: reconstruct from git, rewrite by hand, then start. @impl/done
