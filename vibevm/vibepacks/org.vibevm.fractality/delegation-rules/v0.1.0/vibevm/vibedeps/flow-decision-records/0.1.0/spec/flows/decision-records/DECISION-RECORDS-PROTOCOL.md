# Decision Records Protocol {#root}

**Scope of this document.** This file defines the difference between
a *fact* and a *decision*, why recording decisions is load-bearing
in a human-AI team, the four fields every record carries, *where*
records live (at the governing spec anchor, not in an ADR silo),
*when* to write one, and the two section patterns for larger
documents. Copy-ready template: [`record-template.md`](record-template.md);
trigger design: [`revisit-triggers.md`](revisit-triggers.md).

## Facts versus decisions {#facts-vs-decisions}

"We hash with blake3" is a **fact**. Any reader — human or agent —
recovers it from the code in a second with one grep.

"We hash with blake3 because SHA-256 drags in an OpenSSL dependency,
and we need minimal binary size for edge servers on weak hardware"
is a **decision**. No grep recovers it: the constraint lives outside
the code, and the code is byte-identical either way.

The asymmetry is absolute: **a fact is recoverable from the code in
a second; a decision cannot be recovered at all.** Once reasoning
leaves working memory — the human's after two months, the agent's at
session end — it is gone unless written down. The decision form is
worth ten of the fact form.

## Why this is load-bearing in a human-AI team {#why}

In a pure-human team, unrecorded reasoning survives as tribal
knowledge: "Why this library?" — "Vasya tried five alternatives
three months ago; only this one worked with our glibc."

The agent cannot ask Vasya. Every session is a brilliant new
developer with zero project memory: knowledge that is not in a file
the agent can read does not exist for it. Two failure modes follow:

- **Re-derivation.** The code shows the value, not the constraint.
  The agent sees `TIMEOUT = 600`, finds no reason, concludes the
  number is arbitrary, and proposes 300 s "for performance" — the
  15 % of VPN users who needed the 600 s are invisible in the code.
- **Re-litigation.** Every unrecorded decision is re-opened by every
  future reader, at an hour of re-analysis per re-open — with no
  guarantee of the same answer, because the original data is gone.

A recorded decision is **immunity from re-litigation**: the next
proposal to "optimise" the timeout is answered by the record in one
read, not an hour of archaeology. The trigger field keeps the
immunity honest — without it the decision hardens into a sacred cow
([`revisit-triggers.md` §sacred-cows](revisit-triggers.md#sacred-cows)).

## The four-field record {#four-fields}

Every record carries exactly four fields:

| Field | Requirement |
|-------|-------------|
| **Decision** | The chosen value or approach. One line. |
| **Why** | The observation that forced the choice — concrete, measured, cited. Name the data: log path, sample size, benchmark, incident, upstream constraint. |
| **Considered and rejected** | One line per alternative, each carrying the reason it lost. A loser without a reason invites the evaluation to be re-run. |
| **When to revisit** | A measurable trigger: metric + threshold + where it is observed. "Later" is not a trigger. |

Three of the four fields exist so the argument is *never re-had*:
the why answers "is this arbitrary?", the rejections "did you
consider X?", the trigger "is it time to reconsider?". Copy-ready
shape and worked examples: [`record-template.md`](record-template.md).

## Where records live {#placement}

**At the spec anchor that governs the value.** The timeout record
lives in the spec section that defines verification timing — under
the very heading a reader lands on when asking "why 600?". This
deliberately departs from classic ADR practice:

| Classic ADR | This protocol |
|-------------|---------------|
| `adr/0007-use-blake3.md` — a separate silo | The record sits inside the governing spec section |
| Immutable, append-only; changes chain via "superseded by ADR-0042" | The section is edited in place; a dated changelog line notes the change; git holds the history |
| The reader must know the ADR exists and go find it | The record rides along with every read of the section |
| Numbered by time of decision | Anchored by the thing decided |

The reason is the reader. An agent loads context by anchor: when it
reads the section governing the value, a co-located record arrives
for free, at the exact moment of temptation to "fix" the value. A
record in a silo requires the agent to know to look — and it will
not, because nothing at the anchor points at `adr/0007`. Silos
preserve reasoning technically and lose it practically.

Consequences of the placement rule:

- **No `adr/` directory.** The spec tree is the only home.
- **The spec section IS the record.** No second artefact to sync.
- **Evolution is an edit.** Rewrite the record in place, add a dated
  changelog line; the old text is one `git log -p` away. Procedure:
  [`revisit-triggers.md` §when-fired](revisit-triggers.md#when-fired).
- **Records are citable.** Give every record's heading an explicit
  anchor — `spec://<project>/<doc>#<anchor>` or any stable
  path-plus-anchor form — so comments and commits can point at it.

## When to write one {#when}

Write a record for any choice a future reader could plausibly
re-open:

| Occasion | Example |
|----------|---------|
| Library / dependency pick | blake3 over SHA-256; one HTTP client over another |
| Constant with consequences | timeouts, retry counts, buffer sizes, thresholds |
| Protocol / format shape | wire format, schema, identity scheme, directory layout |
| Rejected approach | a road not taken that someone will propose again |
| Process rule | a review gate, a commit convention, a naming law |

Do **not** write records for facts with no plausible alternative, or
for implementation details the next refactor invalidates — those rot
faster than they pay back. The test: *would a competent newcomer,
reading the code cold, plausibly propose changing this?*

The moment of writing is the moment of deciding — same session,
before it ends. Reasoning unwritten at session end does not survive
it; a record backfilled a week later is fiction with confidence.

## The rejected-alternatives section {#rejected-alternatives}

Bigger documents — design proposals, subsystem specs — accumulate
more rejected options than fit in one record. Close the document
with a section where **every rejected option gets one line with its
reason**:

```markdown
## Rejected alternatives {#rejected}

- **A one-time hardening pass instead of a recurring process** —
  rejected: a one-shot pass decays the day after it lands.
- **Rely on the per-commit gate alone** — rejected: the gate is a
  regression detector, blind by construction to uncovered code.
- **Fully automate the audit now** — deferred, not rejected: value
  is breadth plus judgment; automation grows category by category.
```

The line-with-reason format is the entire value: a bare list of
losers invites the evaluation to be re-run; a reasoned list answers
the future proposal before it is made. Mark honest deferrals
*deferred, not rejected* — a deferral has a built-in revisit.

## The invariants restatement {#invariants-restatement}

Foundational documents — the ones pinning decisions every other
document assumes — close with an **Invariants** section: the most
load-bearing decisions as one-liners, each pointing at its record.

```markdown
## Invariants {#invariants}

(Restated from the records above. If anything below seems violated
in practice, stop and reconcile before proceeding.)

1. **Hashing is blake3.** No OpenSSL-linked digest anywhere. See §hashing.
2. **Verification timeout is 600 s.** Not 300. See §verification.timeout.
```

The restatement is a concession to attention: a hurried reader — or
an agent deep into a long session — reads the invariants even when
reading nothing else; each line points back at the full reasoning.

## Re-derive for your project {#re-derive}

Do not copy this protocol's examples verbatim — copy the *task*, and
let the agent re-derive the records the project actually needs:

```
Read spec/flows/decision-records/ in full, then adapt the practice
to this project:
1. Inventory where design decisions currently live here (spec tree,
   ADR directory, wiki exports, README sections, commit messages).
2. List the ten decisions a newcomer would most plausibly re-open —
   library picks, constants with consequences, protocol shapes,
   rejected approaches — with the file and anchor governing each.
3. For each, draft the four-field record (Decision / Why /
   Considered and rejected / When to revisit) at that governing
   anchor. Where you cannot source a why, write TODO(owner) — never
   invent data.
4. Propose revisit triggers only from signals this project actually
   observes: CI timings, monitoring, benchmarks, dependency audits.
5. Show me the drafts as diffs. Apply nothing until I approve.
```

## Summary {#summary}

- A fact is recoverable from the code in a second; a decision cannot
  be recovered at all. Record decisions.
- Four fields, always: Decision / Why (measured, cited) / Considered
  and rejected (one line each, with reasons) / When to revisit (a
  measurable trigger).
- Records live at the governing spec anchor. No ADR silo; the
  section is the record; git is the history.
- Record in the same session the decision is made.
- Bigger docs close with rejected alternatives; foundational docs
  with an invariants restatement.
- A recorded decision is immunity from re-litigation; the trigger
  keeps it from becoming dogma.
