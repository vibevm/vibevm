# Revisit triggers {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** Why a decision without a revisit
condition rots into a sacred cow, what a measurable trigger is made
of, good and bad trigger shapes, the periodic sweep that actually
fires triggers, and the procedure when one fires. @status:impl/done

@fact:record-shape-pointer The record shape
itself lives in [`record-template.md`](record-template.md). @status:impl/done

## Why decisions rot without triggers {#sacred-cows}

@fact:EVERY-DECISION-IS-CORRECT-RELATIVE-TO-DATA Every decision is correct *relative to data*: 600 s is right while
15 % of users sit behind slow corporate VPNs. @status:spec/done

@fact:data-changes-and-the-decision-outlives-its-reason Then the data changes —
networks get faster, the library ships the missing feature, the
compliance regime shifts — and the decision quietly outlives its
reason. @status:spec/done

@fact:two-failures-lead Without a recorded revisit condition, one of two failures follows: @status:spec/done

- @fact:FAILURE-THE-SACRED-COW **The sacred cow.** The value is written down, so nobody dares
  question it. The reason expired long ago; the decision survives on
  the authority of being recorded. Recording — the thing meant to
  preserve reasoning — now preserves a fossil. @status:spec/done
- @fact:FAILURE-THE-PERMANENT-RE-LITIGATION **The permanent re-litigation.** The opposite failure: with no
  stated condition for reopening, every reader feels free to reopen
  at any time, and the record's immunity is worth nothing. @status:spec/done

@fact:TRIGGER-FIXES-BOTH-AT-ONCE The trigger fixes both at once. @status:spec/done

@fact:TRIGGER-SAYS-THIS-DECISION-STANDS-UNTIL-X It tells every future reader *this
decision stands until X* — which both forbids re-litigation before X
and mandates it after. @status:impl/done

@fact:NO-CONDITION-MEANS-A-SACRED-COW A decision without a revisit condition
becomes a sacred cow; a decision with one stays alive. @status:spec/done

## Anatomy of a measurable trigger {#anatomy}

@fact:A-TRIGGER-HAS-THREE-PARTS A trigger has three parts: @status:impl/done

| Part | Question it answers | Example |
|------|---------------------|---------|
| @fact:ROW-PART-METRIC **Metric** @status:impl/done | What signal is watched? @status:impl/done | p99 delivery latency @status:impl/done |
| @fact:ROW-PART-THRESHOLD **Threshold** @status:impl/done | What value crossing counts? @status:impl/done | below 100 s @status:impl/done |
| @fact:ROW-PART-OBSERVATION-POINT **Observation point** @status:impl/done | Where would one look? @status:impl/done | the network monitoring dashboard @status:impl/done |

@fact:ALL-THREE-OR-IT-IS-NOT-A-TRIGGER All three, or it is not a trigger. @status:impl/done

@fact:why-each-of-the-three-parts-is-needed A metric without a threshold
cannot fire; a threshold without an observation point cannot be
checked; an observation point nobody has is a wish. @status:spec/done

@fact:EVENT-TRIGGERS-ARE-THE-SANCTIONED-NON-NUMERIC-VARIANT **Event triggers** are the sanctioned variant for non-numeric
conditions: an external event that is unambiguous when it happens. @status:impl/done

@fact:event-trigger-examples "A compliance requirement mandates a NIST-approved hash"; "upstream
ships no release for 24 months"; "the vendor removes the v1 API". @status:spec/done

@fact:EVENT-TRIGGERS-TAKE-THE-SAME-TEST The test is the same — a stranger could answer yes-or-no today. @status:impl/done

@fact:UNOBSERVABLE-TRIGGERS-ARE-AS-BAD-AS-NONE **Unobservable triggers** are as bad as none. @status:impl/done

@fact:COLLECT-THE-SIGNAL-OR-REWRITE-THE-TRIGGER If the trigger names a
metric the project does not collect, either start collecting it or
rewrite the trigger against a signal that exists. @status:impl/done

@fact:sweep-reports-unobservable-triggers The periodic sweep
below reports these explicitly. @status:impl/done

## Good and bad triggers {#good-bad}

| Trigger | Verdict | Why |
|---------|---------|-----|
| @fact:ROW-TRIGGER-P99-LATENCY "p99 delivery latency drops below 100 s, per the monitoring dashboard" @status:impl/done | Good @status:impl/done | Metric, threshold, observation point. @status:impl/done |
| @fact:ROW-TRIGGER-HOT-PATH-CPU "hot-path CPU exceeds 80 % in the weekly perf run" @status:impl/done | Good @status:impl/done | Fires from a run that already happens. @status:impl/done |
| @fact:ROW-TRIGGER-UPSTREAM-SILENT "upstream ships no release for 24 months" @status:impl/done | Good @status:impl/done | Event; checkable from the repository today. @status:impl/done |
| @fact:ROW-TRIGGER-COMPLIANCE-MANDATE "a compliance mandate requires a NIST-approved hash" @status:impl/done | Good @status:impl/done | Unambiguous external event. @status:impl/done |
| @fact:ROW-TRIGGER-WHEN-IT-BREAKS "when it breaks" @status:impl/done | Bad @status:impl/done | Breakage is undefined and arrives too late to be a review. @status:impl/done |
| @fact:ROW-TRIGGER-LATER "later" / "at some point" @status:impl/done | Bad @status:impl/done | Never fires. This is "revisit: never" in costume. @status:impl/done |
| @fact:ROW-TRIGGER-WHEN-WE-REFACTOR "when we refactor" @status:impl/done | Bad @status:impl/done | Names an unrelated activity, not a signal about *this* decision. @status:impl/done |
| @fact:ROW-TRIGGER-IF-IT-BECOMES-A-PROBLEM "if it becomes a problem" @status:impl/done | Bad @status:impl/done | No metric, no threshold, no observer — pure vibes. @status:impl/done |

@fact:THE-MECHANICAL-TEST The mechanical test: *"when it breaks" is not a trigger; a trigger
is a measurable signal.* @status:impl/done

@fact:THE-FIVE-MINUTE-STRANGER-TEST If a stranger with access to the
observation point could not answer "has it fired?" in five minutes,
rewrite it. @status:impl/done

## The periodic sweep {#periodic-sweep}

@fact:TRIGGERS-DO-NOT-FIRE-THEMSELVES Triggers do not fire themselves. @status:spec/done

@fact:nothing-pages-anyone-for-a-decision-grade-signal Nothing pages anyone when p99
crosses 100 s — unless the project wires an alert, and most
decision-grade signals never earn one. @status:spec/done

@fact:RE-READING-IS-WHAT-FIRES-TRIGGERS The mechanism that actually
fires triggers is **re-reading**, on a rhythm: @status:impl/done

- @fact:RHYTHM-OPPORTUNISTIC **Opportunistic:** whenever a session touches a document, glance
  at the triggers of the records in it. Cost: seconds. @status:impl/done
- @fact:RHYTHM-PERIODIC **Periodic:** weekly, or at each milestone close — whichever
  rhythm the project already keeps — sweep all records and check
  every trigger against current data. Delegate the sweep: @status:impl/done

```
Sweep the spec tree for decision records — sections carrying the
Decision / Why / Considered and rejected / When to revisit fields.
For each record:
1. Extract the revisit trigger.
2. Classify it against current data: fired / not fired /
   unobservable. Name the data source you checked, or the one you
   failed to find.
3. Edit nothing.
Report a table: anchor, trigger, state, evidence. End with the list
of unobservable triggers — each needs either a data source or a
rewritten trigger. I will decide which records to reopen.
```

@fact:SWEEP-OUTPUT-IS-A-REVIEW-QUEUE-NOT-AN-EDIT The sweep's output is a review queue, not an edit. @status:impl/done

@fact:FIRING-A-TRIGGER-IS-A-HUMAN-DECISION-POINT Firing a trigger
is a human decision-point, because what fires is a *question*, not
an answer. @status:impl/done

## When a trigger fires {#when-fired}

@fact:RE-OPEN-THE-RECORD-NEVER-SILENTLY-EDIT-THE-VALUE Re-open the record — do not silently edit the value. @status:impl/done

@fact:the-failure-to-avoid The failure to
avoid: the constant changes in code, the record keeps the old why,
and the spec now testifies to a reason that no longer produced the
value. @status:spec/done

@fact:drift-manufactured-where-it-was-to-be-prevented That is drift manufactured at the exact point built to
prevent it. @status:spec/done

@fact:procedure-lead Procedure: @status:impl/done

1. @fact:STEP-NAME-THE-FIRED-STATE **Name the fired state.** "The trigger fired: p99 has been at
   82 s for three consecutive weeks, per the dashboard." This
   sentence opens the re-litigation legitimately — and it is the
   only thing that does. @status:impl/done
2. @fact:STEP-RE-RUN-THE-DECISION-WITH-CURRENT-DATA **Re-run the decision with current data.** The old rejected
   alternatives get first look: their rejection reasons may have
   expired along with the trigger. @status:impl/done
3. @fact:STEP-LAND-ONE-OF-TWO-OUTCOMES **Land one of two outcomes:** @status:impl/done
   - @fact:OUTCOME-REAFFIRMED **Reaffirmed.** The decision stands on new data. Refresh the
     why with the new evidence and set a *new* trigger — a fired
     trigger is spent. @status:impl/done
   - @fact:OUTCOME-CHANGED **Changed.** Rewrite the record in place — new decision, new
     why, new rejections (the old winner joins them, with its
     reason), new trigger. Add a dated line to the document's
     version history. Git keeps the old text. @status:impl/done
4. @fact:STEP-COMMIT-CITING-THE-ANCHOR **Commit citing the anchor.** The commit body names the trigger
   state that opened the record and cites the record's anchor; the
   spec carries the reasoning, the commit points at it. @status:impl/done

@fact:NEVER-DELETE-A-RECORD-REWRITE-IT Never delete a record when a decision changes — rewrite it. @status:impl/done

@fact:why-rewriting-keeps-citations-live The
anchor stays stable, every citation into it stays live, and the
superseded reasoning remains one `git log -p` away. @status:spec/done

## Summary {#summary}

- @fact:SUM-NO-CONDITION-MEANS-A-SACRED-COW A decision without a revisit condition becomes a sacred cow — or a
  permanent re-litigation target. The trigger prevents both. @status:spec/done
- @fact:SUM-WHAT-A-TRIGGER-IS-MADE-OF A trigger is metric + threshold + observation point, or an
  unambiguous external event. "Later" and "when it breaks" are not
  triggers. @status:impl/done
- @fact:SUM-UNOBSERVABLE-TRIGGERS-ARE-AS-BAD-AS-NONE Unobservable triggers are as bad as none: collect the signal or
  rewrite the trigger. @status:impl/done
- @fact:SUM-TRIGGERS-FIRE-BY-BEING-RE-READ Triggers fire by being re-read: opportunistically on touch,
  periodically by sweep. Delegate the sweep; keep the reopening
  decision human. @status:impl/done
- @fact:SUM-A-FIRED-TRIGGER-RE-OPENS-THE-RECORD A fired trigger re-opens the record, never silently edits the
  value. Reaffirm with a fresh why and a new trigger, or rewrite in
  place with a changelog line. Git is the history. @status:impl/done
