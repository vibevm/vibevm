# Spec authoring rules {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** How to write spec units that stay
addressable and stay true: what a unit is, how normativity is
marked, why norm and rationale live apart, how deviations are
recorded, the size budgets, when to split a document, changelog
lines, and the anchor-stability contract. @status:impl/done

@fact:protocol-document-pointer The addressing scheme
itself is defined in
[`ADDRESSABLE-SPECS-PROTOCOL.md`](ADDRESSABLE-SPECS-PROTOCOL.md). @status:impl/done

## The unit of meaning {#unit}

@fact:A-SPEC-UNIT-IS-A-HEADING-SPAN A **spec unit** is the span from an anchored heading to the next
heading of the same or higher level. @status:impl/done

@fact:ONE-UNIT-CARRIES-ONE-DECISION One unit carries **one
decision**. @status:impl/done

@fact:AND-ALSO-MEANS-TWO-UNITS If a unit needs "and also", it is two units. @status:impl/done

@fact:A-UNIT-MUST-MAKE-SENSE-ALONE The unit is the page of the agent's working memory: it must make
sense *alone* when pulled into a context window without its
neighbours. @status:impl/done

@fact:a-context-dependent-unit-is-not-addressable A unit that only works when the reader has just read the
three sections above it is not addressable — its anchor points at a
fragment, not a thought. @status:impl/done

@fact:SOFT-CEILING-ONE-PAGE-PER-UNIT Soft ceiling: a unit fits on a page, roughly 120 lines. @status:impl/done

@fact:longer-units-page-badly Longer units
page badly and churn on every edit. @status:spec/done

@fact:A-FENCE-CARRIES-NO-ANCHOR-AND-THAT-IS-DELIBERATE **Content inside a fenced
block carries no anchor, and cannot be given one.** A fence is a payload — the
reader copies it out, pastes it somewhere, runs it — so an anchor written
inside would travel with the copy and corrupt what it is a copy of. Directives
inside a fence are ignored for exactly the reason headings inside one are. @status:impl/done

@fact:AN-INSTRUCTION-INSIDE-A-FENCE-IS-UNVERIFIED-BY-CONSTRUCTION The consequence
is the part authors underestimate: **a fence carrying an INSTRUCTION rather
than an illustration is unverified by construction.** No instrument can test
its claim, none can register that a fix landed, and whether any of it is ever
checked depends on which nearby anchor a reader happens to pick up. And these
are exactly the lines most likely to be *run*: quick-start blocks, copy-paste
prompts, worked commands. @status:spec/done

@fact:A-CHECKABLE-CLAIM-BELONGS-OUTSIDE-THE-FENCE So put every checkable claim
**outside** the fence, in an anchored sentence next to it, and keep inside only
what must be copied verbatim. A path, a version, a command name asserted inside
a fence is a fact with no address and no reader — assert it in the prose that
introduces the block, where it can be judged, and let the block demonstrate it. @status:spec/done

## Normativity is marked, not implied {#normativity}

@fact:A-READER-MUST-NEVER-GUESS-WHETHER-A-SENTENCE-BINDS A reader — human or model — must never guess whether a sentence
binds. @status:impl/done

@fact:CONTRACT-STATEMENTS-USE-RFC-2119-VERBS Contract statements use RFC-2119 verbs (MUST / SHOULD / MAY);
everything without such a verb is explanation, not obligation. @status:impl/done

| Statement kind | Carries | Binds? |
|----------------|---------|--------|
| @fact:ROW-KIND-DECISION decision @status:impl/done | a choice plus its rationale (*why*) @status:impl/done | the choice binds; the rationale does not @status:impl/done |
| @fact:ROW-KIND-CONTRACT contract @status:impl/done | a normative requirement (MUST / SHOULD / MAY) @status:impl/done | yes @status:impl/done |
| @fact:ROW-KIND-DESIGN design @status:impl/done | the shape of a solution (*how*, provisional) @status:impl/done | no @status:impl/done |
| @fact:ROW-KIND-GUIDE guide @status:impl/done | usage documentation @status:impl/done | no @status:impl/done |

@fact:MARK-THE-KIND-WHERE-AMBIGUITY-IS-POSSIBLE Mark the kind where ambiguity is possible — a one-line note under the
heading is enough. @status:impl/done

@fact:THE-CHEAPEST-FORM-IS-A-CONTRACT-ONLY-UNIT The cheapest form: put contracts in their own
units and let the anchor name say so (`#verification.timeout`, not
`#some-thoughts-on-timeouts`). @status:impl/done

## Norm and rationale live apart {#norm-rationale}

@fact:the-must-changes-rarely-the-why-does-not The MUST changes rarely; the *why* evolves freely. @status:impl/done

@fact:mixing-them-makes-every-edit-look-like-a-contract-change When both live in
one paragraph, every rationale edit looks like a contract change, and
every reader must re-verify code against a unit that did not actually
move. @status:spec/done

@fact:KEEP-THE-CONTRACT-SENTENCE-TIGHT Keep the contract sentence tight and let the reasoning follow
it in its own paragraph — or its own unit — so each can change
without casting suspicion on the other. @status:impl/done

## The spec never restates *how* {#no-shadow-code}

@fact:THE-SPEC-STATES-WHAT-AND-WHY The spec states *what* and *why*. @status:impl/done

@fact:IMPLEMENTATION-DETAIL-BELONGS-NEXT-TO-THE-CODE Implementation detail belongs next
to the code — doc comments, module docs — where it cannot drift from
the code it describes. @status:impl/done

@fact:A-SPEC-THAT-MIRRORS-CODE-IS-SHADOW-CODE **A spec that mirrors code is shadow code and
drift fuel**: two copies of the same *how*, one of which is compiled
and one of which rots. @status:impl/done

@fact:THE-REFACTOR-TEST The test: if a refactor with identical behaviour would force a spec
edit, the spec is standing too close to the code. @status:impl/done

@fact:step-back-to-the-contract Step back to the
contract. @status:impl/done

## Write testably {#testable}

@fact:A-CONTRACT-UNIT-IMPLIES-ITS-OWN-VERIFICATION A contract unit should imply its own verification. @status:impl/done

@fact:UNNAMEABLE-TEST-MEANS-DESIGN-NOT-CONTRACT If you cannot
name the test that would verify the statement, it is design, not
contract — either sharpen it until a test is imaginable, or demote
it and stop pretending it binds. @status:impl/done

@fact:RECORD-THE-TEST-NAME-IN-THE-UNIT Record the test name in the unit
once it exists (`Test: …`); that line is half of the bidirectional
graph described in the
[protocol §graph](ADDRESSABLE-SPECS-PROTOCOL.md#graph) — and where the
project mechanizes that graph, the same half is authored as a tag on the
test and rendered back at the unit instead, per that section. @status:impl/done

## Deviations are first-class {#deviations}

@fact:A-DEVIATION-IS-RECORDED-WHERE-IT-HAPPENS When reality intentionally differs from the spec, the deviation is
recorded at the point where it happens, with the reason — in the
code near the deviating item, citing the violated anchor: @status:impl/done

```
// DEVIATES: spec://com.example.shop/PROP-001#verification.timeout
// Reason: staging uses 60 s so the suite finishes under a minute;
// production keeps the spec value. Revisit if staging flakes.
```

@fact:A-RECORDED-DEVIATION-AWAITS-REVIEW A recorded deviation is a decision awaiting review. @status:impl/done

@fact:AN-UNDOCUMENTED-DEVIATION-IS-A-DEFECT An undocumented
deviation discovered later is a defect — not because the code is
wrong, but because the channel lied. @status:impl/done

@fact:honest-deviations-keep-the-spec-authoritative Honest deviations are what keep
the spec authoritative even while reality is ahead of it. @status:spec/done

## Size budgets {#budgets}

@fact:control-plane-tokens-are-a-per-session-tax Control-plane files are read every session; every token in them is a
tax multiplied by the number of sessions the project will ever run. @status:spec/done

| File | Budget | When over |
|------|--------|-----------|
| @fact:ROW-BUDGET-BOOT-FILE Boot file, authored (always loaded) @status:impl/done | ≤ 500 tokens @status:impl/done | cut; move detail into flow docs and cite @status:impl/done |
| @fact:ROW-BUDGET-WAL WAL / continuation state @status:impl/done | ≤ 3000 tokens @status:impl/done | collapse completed items to one line each @status:impl/done |
| @fact:ROW-BUDGET-MODULE-SPEC One module spec document @status:impl/done | ≤ 5000 tokens @status:impl/done | split into submodules @status:impl/done |

@fact:BUDGETS-GOVERN-AUTHORED-DOCUMENTS-ONLY Every budget above governs an
**authored** document. A **generated** artifact carries none — and the proof
is in the last column: "cut", "collapse" and "split" are instructions to a
writer, and a compiler's output has no writer to receive them. Its size is
whatever the resolution graph says the session must have, so trimming it means
changing what the project declared it needs, which is a different decision in a
different file. @status:impl/done

@fact:COMPILING-INTO-THE-LANE-IS-NOT-A-BUDGET-BREACH It follows that a project
which *compiles* its boot lane out of installed sources is not in breach when
that lane grows, and "the artifact is over budget" is not a finding against it.
The cost that stays real is the **session's** context — a live measurement
about one run, not a static rule about one file, and this table is not where it
is settled. @status:impl/done

@fact:rule-of-thumb-500-tokens-is-a-page Rule of thumb: 500 tokens is roughly 375 English words — one page. @status:impl/done

@fact:THE-NUMBERS-ARE-BUDGETS-NOT-PHYSICS The numbers are budgets, not physics; the direction of the pressure
is what matters. @status:impl/done

@fact:A-SPEC-PAST-ITS-BUDGET-IS-TWO-SPECS A spec that keeps growing past its budget is not a
big spec, it is two specs sharing a file. @status:impl/done

## When to split a document {#splitting}

@fact:split-triggers-lead Split when any of these fires: @status:impl/done

- @fact:SPLIT-WHEN-OVER-BUDGET the document is over its size budget (§[budgets](#budgets)); @status:impl/done
- @fact:SPLIT-WHEN-A-UNIT-NEEDS-AND-ALSO a unit needs "and also" — that is two units, and often two homes; @status:impl/done
- @fact:SPLIT-WHEN-TWO-AUDIENCES-EMERGE two audiences have emerged (contract readers vs tutorial readers); @status:impl/done
- @fact:SPLIT-WHEN-ONE-SECTION-IS-CITED-FAR-MORE one section is cited from outside far more than the rest — promote
  it to its own document so its URI shortens and its neighbours stop
  riding along into every context window. @status:impl/done

@fact:SPLITTING-IS-CHEAP-IF-ANCHORS-SURVIVE Splitting is cheap precisely because citations point at anchors, not
page numbers — provided the anchors survive the move
(§[anchor-stability](#anchor-stability)). @status:impl/done

## Changelog lines {#changelog}

@fact:EACH-SEMANTIC-CHANGE-APPENDS-A-DATED-LINE Each semantic change to a document appends one dated line with the
reason: @status:impl/done

```markdown
## Changelog {#changelog}
- [2026-02-17] §verification.timeout: 300 s → 600 s — VPN users
  do not fit in 300 s.
```

@fact:THE-CHANGELOG-IS-A-SIGNAL-NOT-HISTORY The changelog is a signal, not history — history lives in git. @status:impl/done

@fact:who-the-changelog-is-for The
changelog exists for the reader who last saw this file a week ago
and needs the delta in five seconds, without running a diff. @status:spec/done

## Anchor stability {#anchor-stability}

@fact:ANCHORS-ARE-IMMUTABLE-ONCE-CITED **Anchors are immutable once cited.** @status:impl/done

@fact:A-CITED-ANCHOR-IS-A-PUBLIC-SYMBOL An anchor that appears in code
markers, commit bodies, other specs, or the WAL is a public symbol;
renaming it is a breaking change that silently snaps every citation
— the exact failure addressability exists to prevent. @status:impl/done

- @fact:NEVER-RENAME-A-CITED-ANCHOR Never rename a cited anchor. If the heading text must change, the
  `{#id}` stays. @status:impl/done
- @fact:NEVER-REUSE-AN-ANCHOR-FOR-A-DIFFERENT-MEANING Never reuse an anchor for a different meaning. An address that
  once meant one thing and now means another is worse than a dead
  link. @status:impl/done
- @fact:RETIRE-WITH-A-TOMBSTONE-DO-NOT-DELETE Retire with a tombstone, do not delete:
  `<!-- RETIRED: superseded by {#new-anchor} -->` under the old
  heading location. @status:impl/done
- @fact:A-MOVED-UNIT-LEAVES-A-TOMBSTONE Moving a unit to another document leaves a tombstone at the old
  address pointing to the new one. @status:impl/done

@fact:delegate-the-audit-lead Delegate the audit — it is mechanical and the agent is good at it: @status:impl/done

```
Audit this repository for addressability:
1. List every spec heading that states a decision, constraint, or
   contract but carries no {#anchor}.
2. List every normative value that appears in more than one file.
3. List every spec:// citation (specs, code comments, commit log)
   that no longer resolves to an existing anchor.
Report as three tables: location, problem, suggested fix.
Do not edit anything yet.
```

## Summary {#summary}

- @fact:SUM-ONE-UNIT-ONE-DECISION One unit, one decision; a unit makes sense alone, on one page. @status:impl/done
- @fact:SUM-NORMATIVITY-IS-MARKED Normativity is marked with RFC-2119 verbs — nobody guesses what
  binds. @status:impl/done
- @fact:SUM-CONTRACT-AND-RATIONALE-ARE-SEPARATE Contract and rationale are separate; the spec never restates
  *how*. A spec that mirrors code is shadow code. @status:impl/done
- @fact:SUM-EVERY-CONTRACT-IMPLIES-A-TEST Every contract implies a test; the unit
  names it, by a `Test:` line or by a rendered edge where the graph is
  mechanized. @status:impl/done
- @fact:SUM-DEVIATIONS-ARE-RECORDED-WHERE-THEY-HAPPEN Deviations are recorded where they happen, with reasons; the
  undocumented one is the defect. @status:impl/done
- @fact:SUM-THE-SIZE-BUDGETS Budgets: boot ≤ 500 tokens, WAL ≤ 3000, module spec ≤ 5000 —
  split when over. @status:impl/done
- @fact:SUM-THE-CHANGELOG-LINE Changelog: one dated line per semantic change, with the reason. @status:impl/done
- @fact:SUM-ANCHORS-ARE-IMMUTABLE Anchors are immutable once cited; retirement is a tombstone,
  never a deletion or a silent rename. @status:impl/done
