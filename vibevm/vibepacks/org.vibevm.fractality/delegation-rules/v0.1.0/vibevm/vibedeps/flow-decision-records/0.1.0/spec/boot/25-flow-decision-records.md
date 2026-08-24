# Flow: Decision Records {#root}

This project records **decisions, not facts**. A fact ("timeout is
600 s") is recoverable from the code in a second. The reason it is
600 s cannot be recovered at all — unless it was written down when
the decision was made.

## Core rule {#core-rule}

Any choice a future reader could plausibly re-open — a library pick,
a constant with consequences, a protocol shape, a rejected approach —
gets a **four-field record at the spec anchor that governs the value**:

| Field | Requirement |
|-------|-------------|
| **Decision** | The chosen value or approach. One line. |
| **Why** | Concrete and cited: a measurement, a constraint, an incident — with data. |
| **Considered and rejected** | One line per alternative, each carrying its rejection reason. |
| **When to revisit** | A measurable trigger: metric + threshold + where it is observed. |

There is no separate ADR directory and no immutable numbered log.
The spec section that governs the value IS the record; evolution is
an edit plus a changelog line; history lives in git.

Full protocol:
[`spec/flows/decision-records/DECISION-RECORDS-PROTOCOL.md`](../flows/decision-records/DECISION-RECORDS-PROTOCOL.md).

## In session {#in-session}

When the user makes a decision during a session:

1. Propose recording it at the governing anchor, with all four
   fields, **before the session ends** — an unrecorded decision does
   not survive the session boundary.
2. If the why or the trigger is not known, ask. Do not invent data;
   do not record a two-field stub.
3. Before changing any value governed by a record, read the record.
   Re-open it only by naming its trigger state ("the trigger fired:
   …" / "the trigger has not fired, but …").

Copy-ready template and worked examples:
[`spec/flows/decision-records/record-template.md`](../flows/decision-records/record-template.md).
Trigger design and the periodic sweep:
[`spec/flows/decision-records/revisit-triggers.md`](../flows/decision-records/revisit-triggers.md).

## Why this matters in a human-AI team {#why}

The agent cannot ask Vasya why the library was chosen. It re-derives
from what it can read, and the code shows the value, not the
constraint — so it re-litigates: "600 s looks arbitrary, propose
300 s for performance." A recorded decision is immunity from
re-litigation. A recorded trigger is what keeps the immunity from
hardening into dogma.

## Never {#never}

- Never write "because it is better" — a why cites a measurement, a
  constraint, or an incident, or it is not a why.
- Never re-litigate a recorded decision without naming its trigger
  state first.
- Never put a decision's why into a commit message only. The commit
  cites the record; the spec carries it.
- Never record a decision with a missing reason or a missing revisit
  trigger — that is a fact with decoration, not a record.
