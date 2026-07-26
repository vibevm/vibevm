# Decision Records Protocol {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines the difference between
a *fact* and a *decision*, why recording decisions is load-bearing
in a human-AI team, the four fields every record carries, *where*
records live (at the governing spec anchor, not in an ADR silo),
*when* to write one, and the two section patterns for larger
documents. @impl/done

##companion-document-pointers Copy-ready template: [`record-template.md`](record-template.md);
trigger design: [`revisit-triggers.md`](revisit-triggers.md). @impl/done

## Facts versus decisions {#facts-vs-decisions}

##BLAKE3-LINE-IS-A-FACT "We hash with blake3" is a **fact**. @spec/done

##a-fact-is-recovered-with-one-grep Any reader — human or agent —
recovers it from the code in a second with one grep. @spec/done

##BLAKE3-WITH-ITS-REASON-IS-A-DECISION "We hash with blake3 because SHA-256 drags in an OpenSSL dependency,
and we need minimal binary size for edge servers on weak hardware"
is a **decision**. @spec/done

##no-grep-recovers-a-decision No grep recovers it: the constraint lives outside
the code, and the code is byte-identical either way. @spec/done

##ASYMMETRY-IS-ABSOLUTE The asymmetry is absolute: **a fact is recoverable from the code in
a second; a decision cannot be recovered at all.** @spec/done

##reasoning-is-gone-once-it-leaves-working-memory Once reasoning
leaves working memory — the human's after two months, the agent's at
session end — it is gone unless written down. @spec/done

##DECISION-FORM-IS-WORTH-TEN-FACT-FORMS The decision form is
worth ten of the fact form. @spec/done

## Why this is load-bearing in a human-AI team {#why}

##tribal-knowledge-in-a-pure-human-team In a pure-human team, unrecorded reasoning survives as tribal
knowledge: "Why this library?" — "Vasya tried five alternatives
three months ago; only this one worked with our glibc." @spec/done

##AGENT-CANNOT-ASK-VASYA The agent cannot ask Vasya. @spec/done

##EVERY-SESSION-IS-A-NEW-DEVELOPER-WITH-ZERO-MEMORY Every session is a brilliant new
developer with zero project memory: knowledge that is not in a file
the agent can read does not exist for it. @spec/done

##two-failure-modes-lead Two failure modes follow: @spec/done

- ##FAILURE-RE-DERIVATION **Re-derivation.** The code shows the value, not the constraint.
  The agent sees `TIMEOUT = 600`, finds no reason, concludes the
  number is arbitrary, and proposes 300 s "for performance" — the
  15 % of VPN users who needed the 600 s are invisible in the code. @spec/done
- ##FAILURE-RE-LITIGATION **Re-litigation.** Every unrecorded decision is re-opened by every
  future reader, at an hour of re-analysis per re-open — with no
  guarantee of the same answer, because the original data is gone. @spec/done

##RECORD-IS-IMMUNITY-FROM-RE-LITIGATION A recorded decision is **immunity from re-litigation**: the next
proposal to "optimise" the timeout is answered by the record in one
read, not an hour of archaeology. @spec/done

##TRIGGER-FIELD-KEEPS-THE-IMMUNITY-HONEST The trigger field keeps the
immunity honest — without it the decision hardens into a sacred cow
([`revisit-triggers.md` §sacred-cows](revisit-triggers.md#sacred-cows)). @spec/done

## The four-field record {#four-fields}

##EVERY-RECORD-CARRIES-EXACTLY-FOUR-FIELDS Every record carries exactly four fields: @impl/done

| Field | Requirement |
|-------|-------------|
| ##ROW-FIELD-DECISION **Decision** @impl/done | The chosen value or approach. One line. @impl/done |
| ##ROW-FIELD-WHY **Why** @impl/done | The observation that forced the choice — concrete, measured, cited. Name the data: log path, sample size, benchmark, incident, upstream constraint. @impl/done |
| ##ROW-FIELD-CONSIDERED-AND-REJECTED **Considered and rejected** @impl/done | One line per alternative, each carrying the reason it lost. A loser without a reason invites the evaluation to be re-run. @impl/done |
| ##ROW-FIELD-WHEN-TO-REVISIT **When to revisit** @impl/done | A measurable trigger: metric + threshold + where it is observed. "Later" is not a trigger. @impl/done |

##why-three-of-the-four-fields-exist Three of the four fields exist so the argument is *never re-had*:
the why answers "is this arbitrary?", the rejections "did you
consider X?", the trigger "is it time to reconsider?". @spec/done

##template-and-examples-pointer Copy-ready
shape and worked examples: [`record-template.md`](record-template.md). @impl/done

## Where records live {#placement}

##RECORDS-LIVE-AT-THE-GOVERNING-ANCHOR **At the spec anchor that governs the value.** @impl/done

##timeout-record-sits-under-the-verification-heading The timeout record
lives in the spec section that defines verification timing — under
the very heading a reader lands on when asking "why 600?". @spec/done

##departure-from-classic-adr-lead This
deliberately departs from classic ADR practice: @impl/done

| Classic ADR | This protocol |
|-------------|---------------|
| ##ROW-ADR-SILO `adr/0007-use-blake3.md` — a separate silo @spec/done | The record sits inside the governing spec section @spec/done |
| ##ROW-ADR-IMMUTABLE Immutable, append-only; changes chain via "superseded by ADR-0042" @spec/done | The section is edited in place; a dated changelog line notes the change; git holds the history @spec/done |
| ##ROW-ADR-MUST-BE-SOUGHT The reader must know the ADR exists and go find it @spec/done | The record rides along with every read of the section @spec/done |
| ##ROW-ADR-NUMBERED-BY-TIME Numbered by time of decision @spec/done | Anchored by the thing decided @spec/done |

##the-reason-is-the-reader The reason is the reader. @spec/done

##agent-loads-context-by-anchor An agent loads context by anchor: when it
reads the section governing the value, a co-located record arrives
for free, at the exact moment of temptation to "fix" the value. @spec/done

##a-record-in-a-silo-is-never-looked-up A
record in a silo requires the agent to know to look — and it will
not, because nothing at the anchor points at `adr/0007`. @spec/done

##SILOS-PRESERVE-TECHNICALLY-AND-LOSE-PRACTICALLY Silos
preserve reasoning technically and lose it practically. @spec/done

##placement-consequences-lead Consequences of the placement rule: @impl/done

- ##CONSEQUENCE-NO-ADR-DIRECTORY **No `adr/` directory.** The spec tree is the only home. @impl/done
- ##CONSEQUENCE-SPEC-SECTION-IS-THE-RECORD **The spec section IS the record.** No second artefact to sync. @impl/done
- ##CONSEQUENCE-EVOLUTION-IS-AN-EDIT **Evolution is an edit.** Rewrite the record in place, add a dated
  changelog line; the old text is one `git log -p` away. Procedure:
  [`revisit-triggers.md` §when-fired](revisit-triggers.md#when-fired). @impl/done
- ##CONSEQUENCE-RECORDS-ARE-CITABLE **Records are citable.** Give every record's heading an explicit
  anchor — `spec://<project>/<doc>#<anchor>` or any stable
  path-plus-anchor form — so comments and commits can point at it. @impl/done

## When to write one {#when}

##WRITE-A-RECORD-FOR-ANY-REOPENABLE-CHOICE Write a record for any choice a future reader could plausibly
re-open: @impl/done

| Occasion | Example |
|----------|---------|
| ##ROW-OCCASION-LIBRARY-PICK Library / dependency pick @impl/done | blake3 over SHA-256; one HTTP client over another @impl/done |
| ##ROW-OCCASION-CONSTANT Constant with consequences @impl/done | timeouts, retry counts, buffer sizes, thresholds @impl/done |
| ##ROW-OCCASION-PROTOCOL-SHAPE Protocol / format shape @impl/done | wire format, schema, identity scheme, directory layout @impl/done |
| ##ROW-OCCASION-REJECTED-APPROACH Rejected approach @impl/done | a road not taken that someone will propose again @impl/done |
| ##ROW-OCCASION-PROCESS-RULE Process rule @impl/done | a review gate, a commit convention, a naming law @impl/done |

##DO-NOT-RECORD-WHAT-HAS-NO-PLAUSIBLE-ALTERNATIVE Do **not** write records for facts with no plausible alternative, or
for implementation details the next refactor invalidates — those rot
faster than they pay back. @impl/done

##THE-COMPETENT-NEWCOMER-TEST The test: *would a competent newcomer,
reading the code cold, plausibly propose changing this?* @impl/done

##WRITE-IN-THE-SESSION-THAT-DECIDES The moment of writing is the moment of deciding — same session,
before it ends. @impl/done

##backfilled-reasoning-is-fiction Reasoning unwritten at session end does not survive
it; a record backfilled a week later is fiction with confidence. @spec/done

## The rejected-alternatives section {#rejected-alternatives}

##bigger-documents-outgrow-a-single-record Bigger documents — design proposals, subsystem specs — accumulate
more rejected options than fit in one record. @spec/done

##CLOSE-BIG-DOCUMENTS-WITH-A-REJECTED-OPTIONS-SECTION Close the document
with a section where **every rejected option gets one line with its
reason**: @impl/done

```markdown
## Rejected alternatives {#rejected}

- **A one-time hardening pass instead of a recurring process** —
  rejected: a one-shot pass decays the day after it lands.
- **Rely on the per-commit gate alone** — rejected: the gate is a
  regression detector, blind by construction to uncovered code.
- **Fully automate the audit now** — deferred, not rejected: value
  is breadth plus judgment; automation grows category by category.
```

##why-the-line-with-reason-format The line-with-reason format is the entire value: a bare list of
losers invites the evaluation to be re-run; a reasoned list answers
the future proposal before it is made. @spec/done

##MARK-HONEST-DEFERRALS-AS-DEFERRED-NOT-REJECTED Mark honest deferrals
*deferred, not rejected* — a deferral has a built-in revisit. @impl/done

## The invariants restatement {#invariants-restatement}

##FOUNDATIONAL-DOCUMENTS-CLOSE-WITH-INVARIANTS Foundational documents — the ones pinning decisions every other
document assumes — close with an **Invariants** section: the most
load-bearing decisions as one-liners, each pointing at its record. @impl/done

```markdown
## Invariants {#invariants}

(Restated from the records above. If anything below seems violated
in practice, stop and reconcile before proceeding.)

1. **Hashing is blake3.** No OpenSSL-linked digest anywhere. See §hashing.
2. **Verification timeout is 600 s.** Not 300. See §verification.timeout.
```

##why-the-restatement-earns-its-place The restatement is a concession to attention: a hurried reader — or
an agent deep into a long session — reads the invariants even when
reading nothing else; each line points back at the full reasoning. @spec/done

## Re-derive for your project {#re-derive}

##COPY-THE-TASK-NOT-THE-EXAMPLES Do not copy this protocol's examples verbatim — copy the *task*, and
let the agent re-derive the records the project actually needs: @impl/done

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

- ##SUM-RECORD-DECISIONS-NOT-FACTS A fact is recoverable from the code in a second; a decision cannot
  be recovered at all. Record decisions. @spec/done
- ##SUM-FOUR-FIELDS-ALWAYS Four fields, always: Decision / Why (measured, cited) / Considered
  and rejected (one line each, with reasons) / When to revisit (a
  measurable trigger). @impl/done
- ##SUM-RECORDS-LIVE-AT-THE-ANCHOR Records live at the governing spec anchor. No ADR silo; the
  section is the record; git is the history. @impl/done
- ##SUM-RECORD-IN-THE-SAME-SESSION Record in the same session the decision is made. @impl/done
- ##SUM-CLOSING-SECTIONS-BY-DOCUMENT-SIZE Bigger docs close with rejected alternatives; foundational docs
  with an invariants restatement. @impl/done
- ##SUM-IMMUNITY-AND-ITS-TRIGGER A recorded decision is immunity from re-litigation; the trigger
  keeps it from becoming dogma. @spec/done
