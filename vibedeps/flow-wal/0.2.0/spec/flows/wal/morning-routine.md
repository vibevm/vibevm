# Morning routine — the human side of WAL {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** The five-minute human ritual at the start
of each day: read the WAL, reconcile it with your own memory, and only
then let an agent session begin. @status:impl/done

@fact:WAL-IS-A-TWO-SIDED-PROTOCOL The WAL is a two-sided protocol — the
agent writes it at the end of each session, the human reads it at the
start of each day. @status:impl/done

@fact:WITHOUT-THE-MORNING-READ-THE-WAL-ROTS Without the morning read, the WAL rots: the agent's
claim that "all tests pass" drifts from your memory that the timeout
test was flaky yesterday. @status:spec/done

## Read before coding {#read}

@fact:before-coding-lead Before opening any code, before starting any agent session: @status:impl/done

1. @fact:STEP-OPEN-THE-WAL Open `spec/WAL.md`. @status:impl/done
2. @fact:STEP-READ-IT-END-TO-END Read it end to end. @status:impl/done
3. @fact:STEP-COMPARE-AGAINST-WHAT-YOU-REMEMBER Compare what it says against what you remember. @status:impl/done

## Head wins {#head-wins}

@fact:HEAD-WINS-EDIT-THE-WAL-TO-MATCH-MEMORY If the WAL and your memory disagree, **head wins** — edit the WAL to
match what you remember, *then* start the session. @status:impl/done

@fact:why-head-wins Head wins because persistent human memory is the one authoritative
channel the agent cannot reach directly. @status:spec/done

@fact:a-flaky-test-example If the WAL says "all tests
pass" and you remember the timeout test is flaky, the WAL is wrong:
fix it before the next agent session reads and trusts it. @status:impl/done

@fact:top-of-the-conflict-hierarchy (This is
the top of the conflict hierarchy — Human > Spec > Tests > Code >
WAL.) @status:impl/done

## Fast skim: wal-status {#skim}

@fact:the-daily-read-has-a-fast-form The daily read has a fast form. @status:impl/done

@fact:FLOW-SHIPS-A-WAL-STATUS-SKILL This flow ships a `wal-status` skill:
install it into your agent and ask for a WAL status at session start. @status:impl/done

@fact:SKILL-ANSWERS-IN-AT-MOST-TEN-LINES It reads `spec/WAL.md` end to end and answers in at most ten lines —
one line of phase and status, up to three attention bullets, one line
of next step — warning first when the WAL is older than 24 hours. @status:impl/done

@fact:equivalent-prompt-lead For an agent without skill support, paste the equivalent prompt into
your project's agent instructions: @status:impl/done

```markdown
## /wal-status
Read spec/WAL.md end to end. Emit in ≤ 10 lines:
- one line: current phase and status
- up to three bullets: what needs attention (blockers, risks)
- one line: next priority step
If the WAL `_Updated:` line is older than 24 hours, warn first.
```

@fact:the-summary-is-a-quick-daily-read The ten-line summary is a quick daily read. @status:impl/done

@fact:THE-SKIM-DOES-NOT-REPLACE-THE-WEEKLY-RE-READ It does not replace the
end-of-week full re-read — the same way a `top` command does not
replace looking at dashboards. @status:impl/done

## Cold starts {#cold}

@fact:cold-start-situations Coming back after a machine switch or a long gap, or arriving at a
repository that is not yours? @status:impl/done

@fact:READ-CONTINUE-FIRST-THEN-THE-WAL Read `CONTINUE.md` at the repository
root first — it carries the tour, the commands, the map — then
`spec/WAL.md`, which is canonical wherever the two diverge. @status:impl/done

@fact:cold-resume-contract-pointer The full
contract is [`cold-resume.md`](cold-resume.md). @status:impl/done

## Weekly re-read {#weekly}

@fact:RE-READ-KEY-SPEC-DOCUMENTS-WEEKLY Once a week, re-read the key spec documents end-to-end. Watch for: @status:impl/done

- @fact:WATCH-FOR-INTERNAL-CONTRADICTIONS Internal contradictions (§2 says one thing, §5 says another). @status:impl/done
- @fact:WATCH-FOR-STALE-REVIEW-MARKERS REVIEW markers older than your configured threshold. @status:impl/done
- @fact:WATCH-FOR-ORPHAN-ANCHORS Orphan anchors (`{#something}` that nothing references). @status:impl/done

@fact:these-drift-silently These drift silently. @status:spec/done

@fact:A-SCHEDULED-RE-READ-IS-THE-GARBAGE-COLLECTOR A scheduled re-read is the garbage collector for
the spec corpus. @status:spec/done

## If the WAL is clearly stale {#stale}

@fact:A-STALE-DATE-LINE-MEANS-THE-LAST-SESSION-DID-NOT-CLOSE If you come back after a week and the WAL's `_Updated:` line is from
last Tuesday, the previous session clearly did not close cleanly. Your
move: @status:spec/done

1. @fact:STALE-STEP-DO-NOT-START-A-SESSION-YET Do NOT start a new agent session yet. @status:impl/done
2. @fact:STALE-STEP-RECONSTRUCT-FROM-GIT Look at `git log` and `git diff` for the interval. Reconstruct what
   actually happened. @status:impl/done
3. @fact:STALE-STEP-REWRITE-THE-WAL-BY-HAND Rewrite the WAL yourself to reflect the true current state. @status:impl/done
4. @fact:STALE-STEP-THEN-START-THE-NEXT-SESSION *Now* start the next session. @status:impl/done

@fact:THE-HUMAN-IS-THE-BACKUP-FOR-THE-WAL The human is the backup for the WAL. @status:impl/done

@fact:where-the-human-is-irreplaceable This is one of the situations
where the human is irreplaceable in the system, and where ten minutes
of manual reconciliation prevents hours of agent confusion. @status:spec/done

## Never {#never}

- @fact:NEVER-START-ON-A-WAL-YOU-HAVE-NOT-READ-TODAY Never start an agent session on a WAL you have not read today. @status:impl/done
- @fact:NEVER-FIX-A-STALE-WAL-FROM-MEMORY-ALONE Never "fix" a stale WAL from memory alone — reconstruct from
  `git log` and `git diff` first; memory is reconstructive, not
  archival. @status:impl/done
- @fact:NEVER-LET-THE-SKIM-REPLACE-THE-WEEKLY-RE-READ Never let the skim replace the weekly full re-read. @status:impl/done
- @fact:NEVER-LEAVE-A-KNOWN-DIVERGENCE-FOR-LATER Never leave a known divergence in place "for later" — the very next
  session will read and trust it. @status:impl/done

## Summary {#summary}

- @fact:SUM-READ-THE-WAL-EVERY-MORNING Read the WAL every morning, end to end, before any session. @status:impl/done
- @fact:SUM-HEAD-WINS Head wins: your memory corrects the WAL, never the other way around. @status:impl/done
- @fact:SUM-SKIM-AND-WEEKLY-RE-READ `wal-status` is the fast skim; the weekly full re-read still happens. @status:impl/done
- @fact:SUM-COLD-START-ORDER Cold start? `CONTINUE.md` first for the tour, the WAL for the truth. @status:impl/done
- @fact:SUM-STALE-WAL-PROCEDURE Stale WAL: reconstruct from git, rewrite by hand, then start. @status:impl/done
