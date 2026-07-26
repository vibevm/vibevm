# Revisit triggers {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** Why a decision without a revisit
condition rots into a sacred cow, what a measurable trigger is made
of, good and bad trigger shapes, the periodic sweep that actually
fires triggers, and the procedure when one fires. @impl/done

##record-shape-pointer The record shape
itself lives in [`record-template.md`](record-template.md). @impl/done

## Why decisions rot without triggers {#sacred-cows}

##EVERY-DECISION-IS-CORRECT-RELATIVE-TO-DATA Every decision is correct *relative to data*: 600 s is right while
15 % of users sit behind slow corporate VPNs. @spec/done

##data-changes-and-the-decision-outlives-its-reason Then the data changes —
networks get faster, the library ships the missing feature, the
compliance regime shifts — and the decision quietly outlives its
reason. @spec/done

##two-failures-lead Without a recorded revisit condition, one of two failures follows: @spec/done

- ##FAILURE-THE-SACRED-COW **The sacred cow.** The value is written down, so nobody dares
  question it. The reason expired long ago; the decision survives on
  the authority of being recorded. Recording — the thing meant to
  preserve reasoning — now preserves a fossil. @spec/done
- ##FAILURE-THE-PERMANENT-RE-LITIGATION **The permanent re-litigation.** The opposite failure: with no
  stated condition for reopening, every reader feels free to reopen
  at any time, and the record's immunity is worth nothing. @spec/done

##TRIGGER-FIXES-BOTH-AT-ONCE The trigger fixes both at once. @spec/done

##TRIGGER-SAYS-THIS-DECISION-STANDS-UNTIL-X It tells every future reader *this
decision stands until X* — which both forbids re-litigation before X
and mandates it after. @impl/done

##NO-CONDITION-MEANS-A-SACRED-COW A decision without a revisit condition
becomes a sacred cow; a decision with one stays alive. @spec/done

## Anatomy of a measurable trigger {#anatomy}

##A-TRIGGER-HAS-THREE-PARTS A trigger has three parts: @impl/done

| Part | Question it answers | Example |
|------|---------------------|---------|
| ##ROW-PART-METRIC **Metric** @impl/done | What signal is watched? @impl/done | p99 delivery latency @impl/done |
| ##ROW-PART-THRESHOLD **Threshold** @impl/done | What value crossing counts? @impl/done | below 100 s @impl/done |
| ##ROW-PART-OBSERVATION-POINT **Observation point** @impl/done | Where would one look? @impl/done | the network monitoring dashboard @impl/done |

##ALL-THREE-OR-IT-IS-NOT-A-TRIGGER All three, or it is not a trigger. @impl/done

##why-each-of-the-three-parts-is-needed A metric without a threshold
cannot fire; a threshold without an observation point cannot be
checked; an observation point nobody has is a wish. @spec/done

##EVENT-TRIGGERS-ARE-THE-SANCTIONED-NON-NUMERIC-VARIANT **Event triggers** are the sanctioned variant for non-numeric
conditions: an external event that is unambiguous when it happens. @impl/done

##event-trigger-examples "A compliance requirement mandates a NIST-approved hash"; "upstream
ships no release for 24 months"; "the vendor removes the v1 API". @spec/done

##EVENT-TRIGGERS-TAKE-THE-SAME-TEST The test is the same — a stranger could answer yes-or-no today. @impl/done

##UNOBSERVABLE-TRIGGERS-ARE-AS-BAD-AS-NONE **Unobservable triggers** are as bad as none. @impl/done

##COLLECT-THE-SIGNAL-OR-REWRITE-THE-TRIGGER If the trigger names a
metric the project does not collect, either start collecting it or
rewrite the trigger against a signal that exists. @impl/done

##sweep-reports-unobservable-triggers The periodic sweep
below reports these explicitly. @impl/done

## Good and bad triggers {#good-bad}

| Trigger | Verdict | Why |
|---------|---------|-----|
| ##ROW-TRIGGER-P99-LATENCY "p99 delivery latency drops below 100 s, per the monitoring dashboard" @impl/done | Good @impl/done | Metric, threshold, observation point. @impl/done |
| ##ROW-TRIGGER-HOT-PATH-CPU "hot-path CPU exceeds 80 % in the weekly perf run" @impl/done | Good @impl/done | Fires from a run that already happens. @impl/done |
| ##ROW-TRIGGER-UPSTREAM-SILENT "upstream ships no release for 24 months" @impl/done | Good @impl/done | Event; checkable from the repository today. @impl/done |
| ##ROW-TRIGGER-COMPLIANCE-MANDATE "a compliance mandate requires a NIST-approved hash" @impl/done | Good @impl/done | Unambiguous external event. @impl/done |
| ##ROW-TRIGGER-WHEN-IT-BREAKS "when it breaks" @impl/done | Bad @impl/done | Breakage is undefined and arrives too late to be a review. @impl/done |
| ##ROW-TRIGGER-LATER "later" / "at some point" @impl/done | Bad @impl/done | Never fires. This is "revisit: never" in costume. @impl/done |
| ##ROW-TRIGGER-WHEN-WE-REFACTOR "when we refactor" @impl/done | Bad @impl/done | Names an unrelated activity, not a signal about *this* decision. @impl/done |
| ##ROW-TRIGGER-IF-IT-BECOMES-A-PROBLEM "if it becomes a problem" @impl/done | Bad @impl/done | No metric, no threshold, no observer — pure vibes. @impl/done |

##THE-MECHANICAL-TEST The mechanical test: *"when it breaks" is not a trigger; a trigger
is a measurable signal.* @impl/done

##THE-FIVE-MINUTE-STRANGER-TEST If a stranger with access to the
observation point could not answer "has it fired?" in five minutes,
rewrite it. @impl/done

## The periodic sweep {#periodic-sweep}

##TRIGGERS-DO-NOT-FIRE-THEMSELVES Triggers do not fire themselves. @spec/done

##nothing-pages-anyone-for-a-decision-grade-signal Nothing pages anyone when p99
crosses 100 s — unless the project wires an alert, and most
decision-grade signals never earn one. @spec/done

##RE-READING-IS-WHAT-FIRES-TRIGGERS The mechanism that actually
fires triggers is **re-reading**, on a rhythm: @impl/done

- ##RHYTHM-OPPORTUNISTIC **Opportunistic:** whenever a session touches a document, glance
  at the triggers of the records in it. Cost: seconds. @impl/done
- ##RHYTHM-PERIODIC **Periodic:** weekly, or at each milestone close — whichever
  rhythm the project already keeps — sweep all records and check
  every trigger against current data. Delegate the sweep: @impl/done

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

##SWEEP-OUTPUT-IS-A-REVIEW-QUEUE-NOT-AN-EDIT The sweep's output is a review queue, not an edit. @impl/done

##FIRING-A-TRIGGER-IS-A-HUMAN-DECISION-POINT Firing a trigger
is a human decision-point, because what fires is a *question*, not
an answer. @impl/done

## When a trigger fires {#when-fired}

##RE-OPEN-THE-RECORD-NEVER-SILENTLY-EDIT-THE-VALUE Re-open the record — do not silently edit the value. @impl/done

##the-failure-to-avoid The failure to
avoid: the constant changes in code, the record keeps the old why,
and the spec now testifies to a reason that no longer produced the
value. @spec/done

##drift-manufactured-where-it-was-to-be-prevented That is drift manufactured at the exact point built to
prevent it. @spec/done

##procedure-lead Procedure: @impl/done

1. ##STEP-NAME-THE-FIRED-STATE **Name the fired state.** "The trigger fired: p99 has been at
   82 s for three consecutive weeks, per the dashboard." This
   sentence opens the re-litigation legitimately — and it is the
   only thing that does. @impl/done
2. ##STEP-RE-RUN-THE-DECISION-WITH-CURRENT-DATA **Re-run the decision with current data.** The old rejected
   alternatives get first look: their rejection reasons may have
   expired along with the trigger. @impl/done
3. ##STEP-LAND-ONE-OF-TWO-OUTCOMES **Land one of two outcomes:** @impl/done
   - ##OUTCOME-REAFFIRMED **Reaffirmed.** The decision stands on new data. Refresh the
     why with the new evidence and set a *new* trigger — a fired
     trigger is spent. @impl/done
   - ##OUTCOME-CHANGED **Changed.** Rewrite the record in place — new decision, new
     why, new rejections (the old winner joins them, with its
     reason), new trigger. Add a dated line to the document's
     version history. Git keeps the old text. @impl/done
4. ##STEP-COMMIT-CITING-THE-ANCHOR **Commit citing the anchor.** The commit body names the trigger
   state that opened the record and cites the record's anchor; the
   spec carries the reasoning, the commit points at it. @impl/done

##NEVER-DELETE-A-RECORD-REWRITE-IT Never delete a record when a decision changes — rewrite it. @impl/done

##why-rewriting-keeps-citations-live The
anchor stays stable, every citation into it stays live, and the
superseded reasoning remains one `git log -p` away. @spec/done

## Summary {#summary}

- ##SUM-NO-CONDITION-MEANS-A-SACRED-COW A decision without a revisit condition becomes a sacred cow — or a
  permanent re-litigation target. The trigger prevents both. @spec/done
- ##SUM-WHAT-A-TRIGGER-IS-MADE-OF A trigger is metric + threshold + observation point, or an
  unambiguous external event. "Later" and "when it breaks" are not
  triggers. @impl/done
- ##SUM-UNOBSERVABLE-TRIGGERS-ARE-AS-BAD-AS-NONE Unobservable triggers are as bad as none: collect the signal or
  rewrite the trigger. @impl/done
- ##SUM-TRIGGERS-FIRE-BY-BEING-RE-READ Triggers fire by being re-read: opportunistically on touch,
  periodically by sweep. Delegate the sweep; keep the reopening
  decision human. @impl/done
- ##SUM-A-FIRED-TRIGGER-RE-OPENS-THE-RECORD A fired trigger re-opens the record, never silently edits the
  value. Reaffirm with a fresh why and a new trigger, or rewrite in
  place with a changelog line. Git is the history. @impl/done
