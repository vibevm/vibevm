# Spec authoring rules {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** How to write spec units that stay
addressable and stay true: what a unit is, how normativity is
marked, why norm and rationale live apart, how deviations are
recorded, the size budgets, when to split a document, changelog
lines, and the anchor-stability contract. @impl/done

##protocol-document-pointer The addressing scheme
itself is defined in
[`ADDRESSABLE-SPECS-PROTOCOL.md`](ADDRESSABLE-SPECS-PROTOCOL.md). @impl/done

## The unit of meaning {#unit}

##A-SPEC-UNIT-IS-A-HEADING-SPAN A **spec unit** is the span from an anchored heading to the next
heading of the same or higher level. @impl/done

##ONE-UNIT-CARRIES-ONE-DECISION One unit carries **one
decision**. @impl/done

##AND-ALSO-MEANS-TWO-UNITS If a unit needs "and also", it is two units. @impl/done

##A-UNIT-MUST-MAKE-SENSE-ALONE The unit is the page of the agent's working memory: it must make
sense *alone* when pulled into a context window without its
neighbours. @impl/done

##a-context-dependent-unit-is-not-addressable A unit that only works when the reader has just read the
three sections above it is not addressable — its anchor points at a
fragment, not a thought. @impl/done

##SOFT-CEILING-ONE-PAGE-PER-UNIT Soft ceiling: a unit fits on a page, roughly 120 lines. @impl/done

##longer-units-page-badly Longer units
page badly and churn on every edit. @spec/done

## Normativity is marked, not implied {#normativity}

##A-READER-MUST-NEVER-GUESS-WHETHER-A-SENTENCE-BINDS A reader — human or model — must never guess whether a sentence
binds. @impl/done

##CONTRACT-STATEMENTS-USE-RFC-2119-VERBS Contract statements use RFC-2119 verbs (MUST / SHOULD / MAY);
everything without such a verb is explanation, not obligation. @impl/done

| Statement kind | Carries | Binds? |
|----------------|---------|--------|
| ##ROW-KIND-DECISION decision @impl/done | a choice plus its rationale (*why*) @impl/done | the choice binds; the rationale does not @impl/done |
| ##ROW-KIND-CONTRACT contract @impl/done | a normative requirement (MUST / SHOULD / MAY) @impl/done | yes @impl/done |
| ##ROW-KIND-DESIGN design @impl/done | the shape of a solution (*how*, provisional) @impl/done | no @impl/done |
| ##ROW-KIND-GUIDE guide @impl/done | usage documentation @impl/done | no @impl/done |

##MARK-THE-KIND-WHERE-AMBIGUITY-IS-POSSIBLE Mark the kind where ambiguity is possible — a one-line note under the
heading is enough. @impl/done

##THE-CHEAPEST-FORM-IS-A-CONTRACT-ONLY-UNIT The cheapest form: put contracts in their own
units and let the anchor name say so (`#verification.timeout`, not
`#some-thoughts-on-timeouts`). @impl/done

## Norm and rationale live apart {#norm-rationale}

##the-must-changes-rarely-the-why-does-not The MUST changes rarely; the *why* evolves freely. @impl/done

##mixing-them-makes-every-edit-look-like-a-contract-change When both live in
one paragraph, every rationale edit looks like a contract change, and
every reader must re-verify code against a unit that did not actually
move. @spec/done

##KEEP-THE-CONTRACT-SENTENCE-TIGHT Keep the contract sentence tight and let the reasoning follow
it in its own paragraph — or its own unit — so each can change
without casting suspicion on the other. @impl/done

## The spec never restates *how* {#no-shadow-code}

##THE-SPEC-STATES-WHAT-AND-WHY The spec states *what* and *why*. @impl/done

##IMPLEMENTATION-DETAIL-BELONGS-NEXT-TO-THE-CODE Implementation detail belongs next
to the code — doc comments, module docs — where it cannot drift from
the code it describes. @impl/done

##A-SPEC-THAT-MIRRORS-CODE-IS-SHADOW-CODE **A spec that mirrors code is shadow code and
drift fuel**: two copies of the same *how*, one of which is compiled
and one of which rots. @impl/done

##THE-REFACTOR-TEST The test: if a refactor with identical behaviour would force a spec
edit, the spec is standing too close to the code. @impl/done

##step-back-to-the-contract Step back to the
contract. @impl/done

## Write testably {#testable}

##A-CONTRACT-UNIT-IMPLIES-ITS-OWN-VERIFICATION A contract unit should imply its own verification. @impl/done

##UNNAMEABLE-TEST-MEANS-DESIGN-NOT-CONTRACT If you cannot
name the test that would verify the statement, it is design, not
contract — either sharpen it until a test is imaginable, or demote
it and stop pretending it binds. @impl/done

##RECORD-THE-TEST-NAME-IN-THE-UNIT Record the test name in the unit
once it exists (`Test: …`); that line is half of the bidirectional
graph described in the
[protocol §graph](ADDRESSABLE-SPECS-PROTOCOL.md#graph) — and where the
project mechanizes that graph, the same half is authored as a tag on the
test and rendered back at the unit instead, per that section. @impl/done

## Deviations are first-class {#deviations}

##A-DEVIATION-IS-RECORDED-WHERE-IT-HAPPENS When reality intentionally differs from the spec, the deviation is
recorded at the point where it happens, with the reason — in the
code near the deviating item, citing the violated anchor: @impl/done

```
// DEVIATES: spec://com.example.shop/PROP-001#verification.timeout
// Reason: staging uses 60 s so the suite finishes under a minute;
// production keeps the spec value. Revisit if staging flakes.
```

##A-RECORDED-DEVIATION-AWAITS-REVIEW A recorded deviation is a decision awaiting review. @impl/done

##AN-UNDOCUMENTED-DEVIATION-IS-A-DEFECT An undocumented
deviation discovered later is a defect — not because the code is
wrong, but because the channel lied. @impl/done

##honest-deviations-keep-the-spec-authoritative Honest deviations are what keep
the spec authoritative even while reality is ahead of it. @spec/done

## Size budgets {#budgets}

##control-plane-tokens-are-a-per-session-tax Control-plane files are read every session; every token in them is a
tax multiplied by the number of sessions the project will ever run. @spec/done

| File | Budget | When over |
|------|--------|-----------|
| ##ROW-BUDGET-BOOT-FILE Boot file (always loaded) @impl/done | ≤ 500 tokens @impl/done | cut; move detail into flow docs and cite @impl/done |
| ##ROW-BUDGET-WAL WAL / continuation state @impl/done | ≤ 3000 tokens @impl/done | collapse completed items to one line each @impl/done |
| ##ROW-BUDGET-MODULE-SPEC One module spec document @impl/done | ≤ 5000 tokens @impl/done | split into submodules @impl/done |

##rule-of-thumb-500-tokens-is-a-page Rule of thumb: 500 tokens is roughly 375 English words — one page. @impl/done

##THE-NUMBERS-ARE-BUDGETS-NOT-PHYSICS The numbers are budgets, not physics; the direction of the pressure
is what matters. @impl/done

##A-SPEC-PAST-ITS-BUDGET-IS-TWO-SPECS A spec that keeps growing past its budget is not a
big spec, it is two specs sharing a file. @impl/done

## When to split a document {#splitting}

##split-triggers-lead Split when any of these fires: @impl/done

- ##SPLIT-WHEN-OVER-BUDGET the document is over its size budget (§[budgets](#budgets)); @impl/done
- ##SPLIT-WHEN-A-UNIT-NEEDS-AND-ALSO a unit needs "and also" — that is two units, and often two homes; @impl/done
- ##SPLIT-WHEN-TWO-AUDIENCES-EMERGE two audiences have emerged (contract readers vs tutorial readers); @impl/done
- ##SPLIT-WHEN-ONE-SECTION-IS-CITED-FAR-MORE one section is cited from outside far more than the rest — promote
  it to its own document so its URI shortens and its neighbours stop
  riding along into every context window. @impl/done

##SPLITTING-IS-CHEAP-IF-ANCHORS-SURVIVE Splitting is cheap precisely because citations point at anchors, not
page numbers — provided the anchors survive the move
(§[anchor-stability](#anchor-stability)). @impl/done

## Changelog lines {#changelog}

##EACH-SEMANTIC-CHANGE-APPENDS-A-DATED-LINE Each semantic change to a document appends one dated line with the
reason: @impl/done

```markdown
## Changelog {#changelog}
- [2026-02-17] §verification.timeout: 300 s → 600 s — VPN users
  do not fit in 300 s.
```

##THE-CHANGELOG-IS-A-SIGNAL-NOT-HISTORY The changelog is a signal, not history — history lives in git. @impl/done

##who-the-changelog-is-for The
changelog exists for the reader who last saw this file a week ago
and needs the delta in five seconds, without running a diff. @spec/done

## Anchor stability {#anchor-stability}

##ANCHORS-ARE-IMMUTABLE-ONCE-CITED **Anchors are immutable once cited.** @impl/done

##A-CITED-ANCHOR-IS-A-PUBLIC-SYMBOL An anchor that appears in code
markers, commit bodies, other specs, or the WAL is a public symbol;
renaming it is a breaking change that silently snaps every citation
— the exact failure addressability exists to prevent. @impl/done

- ##NEVER-RENAME-A-CITED-ANCHOR Never rename a cited anchor. If the heading text must change, the
  `{#id}` stays. @impl/done
- ##NEVER-REUSE-AN-ANCHOR-FOR-A-DIFFERENT-MEANING Never reuse an anchor for a different meaning. An address that
  once meant one thing and now means another is worse than a dead
  link. @impl/done
- ##RETIRE-WITH-A-TOMBSTONE-DO-NOT-DELETE Retire with a tombstone, do not delete:
  `<!-- RETIRED: superseded by {#new-anchor} -->` under the old
  heading location. @impl/done
- ##A-MOVED-UNIT-LEAVES-A-TOMBSTONE Moving a unit to another document leaves a tombstone at the old
  address pointing to the new one. @impl/done

##delegate-the-audit-lead Delegate the audit — it is mechanical and the agent is good at it: @impl/done

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

- ##SUM-ONE-UNIT-ONE-DECISION One unit, one decision; a unit makes sense alone, on one page. @impl/done
- ##SUM-NORMATIVITY-IS-MARKED Normativity is marked with RFC-2119 verbs — nobody guesses what
  binds. @impl/done
- ##SUM-CONTRACT-AND-RATIONALE-ARE-SEPARATE Contract and rationale are separate; the spec never restates
  *how*. A spec that mirrors code is shadow code. @impl/done
- ##SUM-EVERY-CONTRACT-IMPLIES-A-TEST Every contract implies a test; the unit
  names it, by a `Test:` line or by a rendered edge where the graph is
  mechanized. @impl/done
- ##SUM-DEVIATIONS-ARE-RECORDED-WHERE-THEY-HAPPEN Deviations are recorded where they happen, with reasons; the
  undocumented one is the defect. @impl/done
- ##SUM-THE-SIZE-BUDGETS Budgets: boot ≤ 500 tokens, WAL ≤ 3000, module spec ≤ 5000 —
  split when over. @impl/done
- ##SUM-THE-CHANGELOG-LINE Changelog: one dated line per semantic change, with the reason. @impl/done
- ##SUM-ANCHORS-ARE-IMMUTABLE Anchors are immutable once cited; retirement is a tombstone,
  never a deletion or a silent rename. @impl/done
