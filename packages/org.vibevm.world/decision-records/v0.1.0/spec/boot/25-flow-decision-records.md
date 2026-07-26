# Flow: Decision Records {#root}

<status stage="impl" state="done"/>

##PROJECT-RECORDS-DECISIONS-NOT-FACTS This project records **decisions, not facts**. @impl/done

##FACT-IS-RECOVERABLE-FROM-THE-CODE A fact ("timeout is
600 s") is recoverable from the code in a second. @spec/done

##REASON-IS-LOST-UNLESS-WRITTEN-DOWN The reason it is
600 s cannot be recovered at all — unless it was written down when
the decision was made. @spec/done

## Core rule {#core-rule}

##ANY-REOPENABLE-CHOICE-GETS-A-RECORD Any choice a future reader could plausibly re-open — a library pick,
a constant with consequences, a protocol shape, a rejected approach —
gets a **four-field record at the spec anchor that governs the value**: @impl/done

| Field | Requirement |
|-------|-------------|
| ##ROW-FIELD-DECISION **Decision** @impl/done | The chosen value or approach. One line. @impl/done |
| ##ROW-FIELD-WHY **Why** @impl/done | Concrete and cited: a measurement, a constraint, an incident — with data. @impl/done |
| ##ROW-FIELD-CONSIDERED-AND-REJECTED **Considered and rejected** @impl/done | One line per alternative, each carrying its rejection reason. @impl/done |
| ##ROW-FIELD-WHEN-TO-REVISIT **When to revisit** @impl/done | A measurable trigger: metric + threshold + where it is observed. @impl/done |

##NO-SEPARATE-ADR-DIRECTORY There is no separate ADR directory and no immutable numbered log. @impl/done

##GOVERNING-SPEC-SECTION-IS-THE-RECORD The spec section that governs the value IS the record; evolution is
an edit plus a changelog line; history lives in git. @impl/done

##full-protocol-pointer Full protocol:
[`spec/flows/decision-records/DECISION-RECORDS-PROTOCOL.md`](../flows/decision-records/DECISION-RECORDS-PROTOCOL.md). @impl/done

## In session {#in-session}

##in-session-duty-lead When the user makes a decision during a session: @impl/done

1. ##PROPOSE-THE-RECORD-BEFORE-THE-SESSION-ENDS Propose recording it at the governing anchor, with all four
   fields, **before the session ends** — an unrecorded decision does
   not survive the session boundary. @impl/done
2. ##ASK-RATHER-THAN-INVENT-DATA If the why or the trigger is not known, ask. Do not invent data;
   do not record a two-field stub. @impl/done
3. ##READ-THE-RECORD-BEFORE-CHANGING-THE-VALUE Before changing any value governed by a record, read the record.
   Re-open it only by naming its trigger state ("the trigger fired:
   …" / "the trigger has not fired, but …"). @impl/done

##template-and-examples-pointer Copy-ready template and worked examples:
[`spec/flows/decision-records/record-template.md`](../flows/decision-records/record-template.md). @impl/done

##trigger-design-pointer Trigger design and the periodic sweep:
[`spec/flows/decision-records/revisit-triggers.md`](../flows/decision-records/revisit-triggers.md). @impl/done

## Why this matters in a human-AI team {#why}

##AGENT-CANNOT-ASK-VASYA The agent cannot ask Vasya why the library was chosen. @spec/done

##re-derivation-ends-in-re-litigation It re-derives
from what it can read, and the code shows the value, not the
constraint — so it re-litigates: "600 s looks arbitrary, propose
300 s for performance." @spec/done

##RECORD-IS-IMMUNITY-FROM-RE-LITIGATION A recorded decision is immunity from
re-litigation. @spec/done

##TRIGGER-KEEPS-IMMUNITY-FROM-HARDENING-INTO-DOGMA A recorded trigger is what keeps the immunity from
hardening into dogma. @spec/done

## Never {#never}

- ##NEVER-WRITE-BECAUSE-IT-IS-BETTER Never write "because it is better" — a why cites a measurement, a
  constraint, or an incident, or it is not a why. @impl/done
- ##NEVER-RE-LITIGATE-WITHOUT-NAMING-THE-TRIGGER-STATE Never re-litigate a recorded decision without naming its trigger
  state first. @impl/done
- ##NEVER-PUT-THE-WHY-IN-THE-COMMIT-ONLY Never put a decision's why into a commit message only. The commit
  cites the record; the spec carries it. @impl/done
- ##NEVER-RECORD-A-MISSING-REASON-OR-TRIGGER Never record a decision with a missing reason or a missing revisit
  trigger — that is a fact with decoration, not a record. @impl/done
