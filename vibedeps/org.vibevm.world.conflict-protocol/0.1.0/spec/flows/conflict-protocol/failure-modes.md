# Failure Modes and Recovery {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** The conflict protocol fails in three
recurring, recognizable ways. @status:impl/done

@fact:this-file-names-explains-and-drills-each-failure This file names each failure, explains
the mechanics of why it happens — none of them requires a badly
behaved agent, only defaults plus entropy — and gives the recovery
drill for each. @status:impl/done

@fact:READ-IT-WHEN-SOMETHING-FEELS-WRONG Read it when something feels wrong; re-read it
occasionally so the detection signals stay loaded. @status:impl/done

| # | Failure | Detection signal | First move |
|---|---------|------------------|------------|
| @fact:ROW-FAILURE-SILENT-SPEC-EDIT 1 @status:impl/done | Spec changed without a REVIEW @status:impl/done | A normative value differs in the diff; no marker, no report line @status:impl/done | Revert the spec file; keep the code @status:impl/done |
| @fact:ROW-FAILURE-STALE-STATE-FILE 2 @status:impl/done | Stale state file after a crash @status:impl/done | The session died before the end-of-session rewrite; the state file promises work git does not show @status:impl/done | Reconstruct from `git log` / `git diff` *before* any new session @status:impl/done |
| @fact:ROW-FAILURE-SELF-CONTRADICTING-SPEC 3 @status:impl/done | Spec contradicts itself @status:impl/done | Two sections answer one question differently @status:impl/done | Full re-read; the human picks the winner; fix every echo @status:impl/done |

## Failure 1 — the spec changed without a REVIEW {#silent-spec-edit}

@fact:F1-THE-DIFF-REPLACES-A-NORMATIVE-VALUE **What you see.** The diff replaces "600 seconds" with "300 seconds
with exponential backoff" inside a spec document. @status:impl/done

@fact:F1-NO-MARKER-AND-NO-REPORT-LINE There is no
`REVIEW:` marker near the change and no line in the session report
announcing it. @status:impl/done

@fact:f1-the-edit-rides-in-a-long-diff Typically the edit rides in a long diff that is
otherwise exactly what you asked for. @status:spec/done

@fact:f1-nothing-physical-prevents-it **Why it happens.** Nothing physical prevents it: the spec is a file,
the agent has a file editor, and the agent's local reasoning ("300 is
more robust") feels to it like a favor, not a violation. @status:spec/done

@fact:f1-attention-is-the-only-tripwire The human's
attention is the only tripwire, and attention is exactly what long
diffs exhaust. @status:spec/done

@fact:F1-IS-THE-DEFAULT-STATE-OF-THE-WORLD This failure is the default state of the world — the
protocol is what suppresses it, so any gap in the protocol lets it
back in. @status:spec/done

@fact:f1-recovery-drill-lead **Recovery drill.** @status:impl/done

1. @fact:F1-DRILL-REVERT-THE-SPEC-FILE-ONLY Revert the spec file — and only it — to the last human-approved
   state (`git restore <spec-file>`, or check out that one file from
   the last good commit). Keep the session's code if it is otherwise
   correct. @status:impl/done
2. @fact:F1-DRILL-HANDLE-THE-CODE-SIDE-DIVERGENCE If the code also carries the unauthorized value, you now hold an
   ordinary Spec > Code divergence: schedule the code fix, or let the
   agent dispute the restored value properly — with a marker. @status:impl/done
3. @fact:F1-DRILL-STATE-THE-CORRECTION-NEXT-SESSION Open the next session by stating the correction in plain text:
   "You changed a spec value without a REVIEW marker. I reverted it.
   If you still believe backoff is better, add a REVIEW with the
   reason and we will discuss it." @status:impl/done
4. @fact:F1-DRILL-ADD-THE-STANDING-RULE-TO-THE-BOOT-FILE Add the standing rule to the boot file: "Never modify a normative
   spec value without a REVIEW marker." A correction spoken in chat
   evaporates with the session; a correction in the boot file is
   re-read at every future session start and compounds. @status:impl/done

@fact:F1-STEP-4-IS-THE-ONE-THAT-MATTERS Step 4 is the one that matters. @status:impl/done

@fact:f1-steps-1-3-fix-the-incident-step-4-lowers-the-rate Steps 1–3 fix the incident; step 4
lowers the rate. @status:impl/done

## Failure 2 — stale state file after a crashed session {#stale-state}

@fact:F2-THE-CRASH-SKIPPED-THE-END-OF-SESSION-REWRITE **What you see.** The previous session ended in a crash — context
overflow, out-of-memory, a killed process — and the end-of-session
rewrite of the project state file (WAL or equivalent) never ran. @status:impl/done

@fact:F2-THE-FILE-DESCRIBES-A-STATE-THAT-NO-LONGER-EXISTS The
file now describes a state that no longer exists: it promises tests
that were later broken, or names an in-progress refactor that was
abandoned halfway. @status:impl/done

@fact:f2-the-state-file-is-volatile-by-design **Why it happens.** The state file is volatile *by design*: rewritten
at session end, trusted at session start. @status:spec/done

@fact:f2-a-crash-deletes-exactly-the-rewrite-step A crash deletes exactly the
rewrite step and nothing else. @status:spec/done

@fact:F2-THE-NEXT-AGENT-BOOTS-FROM-THE-STALE-FILE The next agent has no memory except
files, so it boots from the stale file and trusts it completely —
executing yesterday's abandoned intent against today's tree. @status:spec/done

@fact:f2-recovery-drill-lead **Recovery drill.** @status:impl/done

1. @fact:F2-DRILL-DO-NOT-START-A-NEW-SESSION-YET **Do not start a new session yet.** The next agent's first act is
   reading the state file; feed it a lie and it returns work built on
   the lie. @status:impl/done
2. @fact:F2-DRILL-RECONSTRUCT-FROM-THE-DURABLE-RECORD Reconstruct reality from the durable record: `git log` and
   `git diff` over the crashed session's window, plus the test suite
   if it is fast. Git survived the crash; the state file did not. @status:impl/done
3. @fact:F2-DRILL-REWRITE-THE-STATE-FILE-BY-HAND Rewrite the state file by hand — a full rewrite, not an appended
   correction. Append-mode fixes leave the stale text in place for
   the next reader to trip over. @status:impl/done
4. @fact:F2-DRILL-ONLY-THEN-START-THE-SESSION Only then start the session. @status:impl/done

@fact:F2-THE-HUMAN-IS-THE-LIVE-BACKUP The human is the live backup for the state file. @status:impl/done

@fact:heads-persist-across-crashes-volatile-files-do-not Heads persist across
crashes; volatile files do not. @status:spec/done

@fact:F2-THIS-RECOVERY-CANNOT-BE-DELEGATED This recovery cannot be delegated:
an agent booted on a stale file cannot tell which parts are stale,
which is precisely the problem. @status:impl/done

## Failure 3 — the spec contradicts itself {#self-contradiction}

@fact:F3-TWO-SECTIONS-ANSWER-ONE-QUESTION-DIFFERENTLY **What you see.** After twenty iterations, §2 answers a question one
way and §5 another. @status:impl/done

@fact:f3-each-session-behaved-correctly-by-its-own-lights Every session that read only one of the two
sections behaved correctly by its own lights; the document as a whole
no longer has a single answer. @status:spec/done

@fact:f3-edits-are-local **Why it happens.** Edits are local. @status:spec/done

@fact:f3-the-habit-that-keeps-sessions-cheap-lets-sections-drift Each session touches the section
it was pointed at and re-reads little else — pointing sessions at
narrow, addressable targets is the efficient habit, so the same habit
that keeps sessions cheap lets distant sections drift apart. @status:spec/done

@fact:f3-long-files-make-it-worse Long
files make it worse: readers, human and agent alike, attend to the
beginning and the end and skim the middle, so contradictions
accumulate precisely where nobody looks. @status:spec/done

@fact:f3-recovery-drill-lead **Recovery drill.** @status:impl/done

1. @fact:F3-DRILL-SCHEDULE-A-FULL-RE-READ Put a full re-read of the key specs on a schedule — weekly on an
   active project. A calendar item, not an aspiration. @status:impl/done
2. @fact:F3-DRILL-NOTE-EVERY-NORMATIVE-VALUE-STATED Read end to end, noting every place a normative value or rule is
   stated. Duplicates are pre-contradictions. @status:impl/done
3. @fact:F3-DRILL-THE-HUMAN-PICKS-THE-WINNER When two sections disagree, the human picks the winner. This is a
   Human > Spec ruling; no automatic rule can make it, because both
   sections carry equal formal authority. @status:impl/done
4. @fact:F3-DRILL-FIX-EVERY-ECHO-AND-NOTE-THE-RESOLUTION Fix every echo of the losing version in the same change, and note
   the resolution in the spec's changelog so the next full re-read
   has an anchor. @status:impl/done

@fact:PREVENTION-BEATS-RECOVERY-ONE-HOME-PER-VALUE Prevention beats recovery here: give every normative value exactly
one home and cite it from everywhere else. @status:impl/done

@fact:A-VALUE-STATED-TWICE-IS-A-CONTRADICTION-ON-A-DELAY-TIMER A value stated twice is a
contradiction on a delay timer. @status:spec/done

@fact:garbage-collection-for-shared-state This is garbage collection for shared state, and the human is the
collector. @status:spec/done

@fact:garbage-collection-does-not-delegate It is boring, it is unskippable, and it does not delegate:
the agent inside a session is the process *generating* the garbage;
only the reader who spans sessions can sweep it. @status:spec/done

## Summary {#summary}

- @fact:SUM-FAILURE-1-SILENT-SPEC-EDIT Failure 1, silent spec edit: revert the file, keep the code, state
  the rule next session, and write the rule into the boot file so the
  correction compounds instead of evaporating. @status:impl/done
- @fact:SUM-FAILURE-2-STALE-STATE-FILE Failure 2, stale state file: the human is the live backup —
  reconstruct from git, rewrite the file wholesale, and only then let
  a new session boot. @status:impl/done
- @fact:SUM-FAILURE-3-SELF-CONTRADICTING-SPEC Failure 3, self-contradicting spec: weekly full re-read, the human
  picks the winner, fix every echo in one change, keep each value in
  exactly one home. @status:impl/done
- @fact:SUM-ALL-THREE-SHARE-ONE-MECHANIC All three share one mechanic: state that nobody re-reads drifts.
  Every drill is re-reading with authority attached. @status:impl/done
