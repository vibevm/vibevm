# Flow: Decision Records {#root}

<status stage="impl" state="done"/>

@fact:PROJECT-RECORDS-DECISIONS-NOT-FACTS This project records **decisions, not facts**. @status:impl/done

@fact:FACT-IS-RECOVERABLE-FROM-THE-CODE A fact ("timeout is
600 s") is recoverable from the code in a second. @status:spec/done

@fact:REASON-IS-LOST-UNLESS-WRITTEN-DOWN The reason it is
600 s cannot be recovered at all — unless it was written down when
the decision was made. @status:spec/done

## Core rule {#core-rule}

@fact:ANY-REOPENABLE-CHOICE-GETS-A-RECORD Any choice a future reader could plausibly re-open — a library pick,
a constant with consequences, a protocol shape, a rejected approach —
gets a **four-field record at the spec anchor that governs the value**: @status:impl/done

| Field | Requirement |
|-------|-------------|
| @fact:ROW-FIELD-DECISION **Decision** @status:impl/done | The chosen value or approach. One line. @status:impl/done |
| @fact:ROW-FIELD-WHY **Why** @status:impl/done | Concrete and cited: a measurement, a constraint, an incident — with data. @status:impl/done |
| @fact:ROW-FIELD-CONSIDERED-AND-REJECTED **Considered and rejected** @status:impl/done | One line per alternative, each carrying its rejection reason. @status:impl/done |
| @fact:ROW-FIELD-WHEN-TO-REVISIT **When to revisit** @status:impl/done | A measurable trigger: metric + threshold + where it is observed. @status:impl/done |

@fact:NO-SEPARATE-ADR-DIRECTORY There is no separate ADR directory and no immutable numbered log. @status:impl/done

@fact:GOVERNING-SPEC-SECTION-IS-THE-RECORD The spec section that governs the value IS the record; evolution is
an edit plus a changelog line; history lives in git. @status:impl/done

@fact:full-protocol-pointer Full protocol:
@spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root. @status:impl/done

## In session {#in-session}

@fact:in-session-duty-lead When the user makes a decision during a session: @status:impl/done

1. @fact:PROPOSE-THE-RECORD-BEFORE-THE-SESSION-ENDS Propose recording it at the governing anchor, with all four
   fields, **before the session ends** — an unrecorded decision does
   not survive the session boundary. @status:impl/done
2. @fact:ASK-RATHER-THAN-INVENT-DATA If the why or the trigger is not known, ask. Do not invent data;
   do not record a two-field stub. @status:impl/done
3. @fact:READ-THE-RECORD-BEFORE-CHANGING-THE-VALUE Before changing any value governed by a record, read the record.
   Re-open it only by naming its trigger state ("the trigger fired:
   …" / "the trigger has not fired, but …"). @status:impl/done

@fact:template-and-examples-pointer Copy-ready template and worked examples:
@spec://org.vibevm.world/decision-records/flows/decision-records/record-template#root. @status:impl/done

@fact:trigger-design-pointer Trigger design and the periodic sweep:
@spec://org.vibevm.world/decision-records/flows/decision-records/revisit-triggers#root. @status:impl/done

## Why this matters in a human-AI team {#why}

@fact:AGENT-CANNOT-ASK-VASYA The agent cannot ask Vasya why the library was chosen. @status:spec/done

@fact:re-derivation-ends-in-re-litigation It re-derives
from what it can read, and the code shows the value, not the
constraint — so it re-litigates: "600 s looks arbitrary, propose
300 s for performance." @status:spec/done

@fact:RECORD-IS-IMMUNITY-FROM-RE-LITIGATION A recorded decision is immunity from
re-litigation. @status:spec/done

@fact:TRIGGER-KEEPS-IMMUNITY-FROM-HARDENING-INTO-DOGMA A recorded trigger is what keeps the immunity from
hardening into dogma. @status:spec/done

## Never {#never}

- @fact:NEVER-WRITE-BECAUSE-IT-IS-BETTER Never write "because it is better" — a why cites a measurement, a
  constraint, or an incident, or it is not a why. @status:impl/done
- @fact:NEVER-RE-LITIGATE-WITHOUT-NAMING-THE-TRIGGER-STATE Never re-litigate a recorded decision without naming its trigger
  state first. @status:impl/done
- @fact:NEVER-PUT-THE-WHY-IN-THE-COMMIT-ONLY Never put a decision's why into a commit message only. The commit
  cites the record; the spec carries it. @status:impl/done
- @fact:NEVER-RECORD-A-MISSING-REASON-OR-TRIGGER Never record a decision with a missing reason or a missing revisit
  trigger — that is a fact with decoration, not a record. @status:impl/done
