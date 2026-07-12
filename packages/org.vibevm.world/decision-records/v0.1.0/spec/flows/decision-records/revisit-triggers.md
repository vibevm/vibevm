# Revisit triggers {#root}

**Scope of this document.** Why a decision without a revisit
condition rots into a sacred cow, what a measurable trigger is made
of, good and bad trigger shapes, the periodic sweep that actually
fires triggers, and the procedure when one fires. The record shape
itself lives in [`record-template.md`](record-template.md).

## Why decisions rot without triggers {#sacred-cows}

Every decision is correct *relative to data*: 600 s is right while
15 % of users sit behind slow corporate VPNs. Then the data changes —
networks get faster, the library ships the missing feature, the
compliance regime shifts — and the decision quietly outlives its
reason.

Without a recorded revisit condition, one of two failures follows:

- **The sacred cow.** The value is written down, so nobody dares
  question it. The reason expired long ago; the decision survives on
  the authority of being recorded. Recording — the thing meant to
  preserve reasoning — now preserves a fossil.
- **The permanent re-litigation.** The opposite failure: with no
  stated condition for reopening, every reader feels free to reopen
  at any time, and the record's immunity is worth nothing.

The trigger fixes both at once. It tells every future reader *this
decision stands until X* — which both forbids re-litigation before X
and mandates it after. A decision without a revisit condition
becomes a sacred cow; a decision with one stays alive.

## Anatomy of a measurable trigger {#anatomy}

A trigger has three parts:

| Part | Question it answers | Example |
|------|---------------------|---------|
| **Metric** | What signal is watched? | p99 delivery latency |
| **Threshold** | What value crossing counts? | below 100 s |
| **Observation point** | Where would one look? | the network monitoring dashboard |

All three, or it is not a trigger. A metric without a threshold
cannot fire; a threshold without an observation point cannot be
checked; an observation point nobody has is a wish.

**Event triggers** are the sanctioned variant for non-numeric
conditions: an external event that is unambiguous when it happens.
"A compliance requirement mandates a NIST-approved hash"; "upstream
ships no release for 24 months"; "the vendor removes the v1 API".
The test is the same — a stranger could answer yes-or-no today.

**Unobservable triggers** are as bad as none. If the trigger names a
metric the project does not collect, either start collecting it or
rewrite the trigger against a signal that exists. The periodic sweep
below reports these explicitly.

## Good and bad triggers {#good-bad}

| Trigger | Verdict | Why |
|---------|---------|-----|
| "p99 delivery latency drops below 100 s, per the monitoring dashboard" | Good | Metric, threshold, observation point. |
| "hot-path CPU exceeds 80 % in the weekly perf run" | Good | Fires from a run that already happens. |
| "upstream ships no release for 24 months" | Good | Event; checkable from the repository today. |
| "a compliance mandate requires a NIST-approved hash" | Good | Unambiguous external event. |
| "when it breaks" | Bad | Breakage is undefined and arrives too late to be a review. |
| "later" / "at some point" | Bad | Never fires. This is "revisit: never" in costume. |
| "when we refactor" | Bad | Names an unrelated activity, not a signal about *this* decision. |
| "if it becomes a problem" | Bad | No metric, no threshold, no observer — pure vibes. |

The mechanical test: *"when it breaks" is not a trigger; a trigger
is a measurable signal.* If a stranger with access to the
observation point could not answer "has it fired?" in five minutes,
rewrite it.

## The periodic sweep {#periodic-sweep}

Triggers do not fire themselves. Nothing pages anyone when p99
crosses 100 s — unless the project wires an alert, and most
decision-grade signals never earn one. The mechanism that actually
fires triggers is **re-reading**, on a rhythm:

- **Opportunistic:** whenever a session touches a document, glance
  at the triggers of the records in it. Cost: seconds.
- **Periodic:** weekly, or at each milestone close — whichever
  rhythm the project already keeps — sweep all records and check
  every trigger against current data. Delegate the sweep:

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

The sweep's output is a review queue, not an edit. Firing a trigger
is a human decision-point, because what fires is a *question*, not
an answer.

## When a trigger fires {#when-fired}

Re-open the record — do not silently edit the value. The failure to
avoid: the constant changes in code, the record keeps the old why,
and the spec now testifies to a reason that no longer produced the
value. That is drift manufactured at the exact point built to
prevent it.

Procedure:

1. **Name the fired state.** "The trigger fired: p99 has been at
   82 s for three consecutive weeks, per the dashboard." This
   sentence opens the re-litigation legitimately — and it is the
   only thing that does.
2. **Re-run the decision with current data.** The old rejected
   alternatives get first look: their rejection reasons may have
   expired along with the trigger.
3. **Land one of two outcomes:**
   - **Reaffirmed.** The decision stands on new data. Refresh the
     why with the new evidence and set a *new* trigger — a fired
     trigger is spent.
   - **Changed.** Rewrite the record in place — new decision, new
     why, new rejections (the old winner joins them, with its
     reason), new trigger. Add a dated line to the document's
     version history. Git keeps the old text.
4. **Commit citing the anchor.** The commit body names the trigger
   state that opened the record and cites the record's anchor; the
   spec carries the reasoning, the commit points at it.

Never delete a record when a decision changes — rewrite it. The
anchor stays stable, every citation into it stays live, and the
superseded reasoning remains one `git log -p` away.

## Summary {#summary}

- A decision without a revisit condition becomes a sacred cow — or a
  permanent re-litigation target. The trigger prevents both.
- A trigger is metric + threshold + observation point, or an
  unambiguous external event. "Later" and "when it breaks" are not
  triggers.
- Unobservable triggers are as bad as none: collect the signal or
  rewrite the trigger.
- Triggers fire by being re-read: opportunistically on touch,
  periodically by sweep. Delegate the sweep; keep the reopening
  decision human.
- A fired trigger re-opens the record, never silently edits the
  value. Reaffirm with a fresh why and a new trigger, or rewrite in
  place with a changelog line. Git is the history.
