# Spec authoring rules {#root}

**Scope of this document.** How to write spec units that stay
addressable and stay true: what a unit is, how normativity is
marked, why norm and rationale live apart, how deviations are
recorded, the size budgets, when to split a document, changelog
lines, and the anchor-stability contract. The addressing scheme
itself is defined in
[`ADDRESSABLE-SPECS-PROTOCOL.md`](ADDRESSABLE-SPECS-PROTOCOL.md).

## The unit of meaning {#unit}

A **spec unit** is the span from an anchored heading to the next
heading of the same or higher level. One unit carries **one
decision**. If a unit needs "and also", it is two units.

The unit is the page of the agent's working memory: it must make
sense *alone* when pulled into a context window without its
neighbours. A unit that only works when the reader has just read the
three sections above it is not addressable — its anchor points at a
fragment, not a thought.

Soft ceiling: a unit fits on a page, roughly 120 lines. Longer units
page badly and churn on every edit.

## Normativity is marked, not implied {#normativity}

A reader — human or model — must never guess whether a sentence
binds. Contract statements use RFC-2119 verbs (MUST / SHOULD / MAY);
everything without such a verb is explanation, not obligation.

| Statement kind | Carries | Binds? |
|----------------|---------|--------|
| decision       | a choice plus its rationale (*why*) | the choice binds; the rationale does not |
| contract       | a normative requirement (MUST / SHOULD / MAY) | yes |
| design         | the shape of a solution (*how*, provisional) | no |
| guide          | usage documentation | no |

Mark the kind where ambiguity is possible — a one-line note under the
heading is enough. The cheapest form: put contracts in their own
units and let the anchor name say so (`#verification.timeout`, not
`#some-thoughts-on-timeouts`).

## Norm and rationale live apart {#norm-rationale}

The MUST changes rarely; the *why* evolves freely. When both live in
one paragraph, every rationale edit looks like a contract change, and
every reader must re-verify code against a unit that did not actually
move. Keep the contract sentence tight and let the reasoning follow
it in its own paragraph — or its own unit — so each can change
without casting suspicion on the other.

## The spec never restates *how* {#no-shadow-code}

The spec states *what* and *why*. Implementation detail belongs next
to the code — doc comments, module docs — where it cannot drift from
the code it describes. **A spec that mirrors code is shadow code and
drift fuel**: two copies of the same *how*, one of which is compiled
and one of which rots.

The test: if a refactor with identical behaviour would force a spec
edit, the spec is standing too close to the code. Step back to the
contract.

## Write testably {#testable}

A contract unit should imply its own verification. If you cannot
name the test that would verify the statement, it is design, not
contract — either sharpen it until a test is imaginable, or demote
it and stop pretending it binds. Record the test name in the unit
once it exists (`Test: …`); that line is half of the bidirectional
graph described in the
[protocol §graph](ADDRESSABLE-SPECS-PROTOCOL.md#graph).

## Deviations are first-class {#deviations}

When reality intentionally differs from the spec, the deviation is
recorded at the point where it happens, with the reason — in the
code near the deviating item, citing the violated anchor:

```
// DEVIATES: spec://com.example.shop/PROP-001#verification.timeout
// Reason: staging uses 60 s so the suite finishes under a minute;
// production keeps the spec value. Revisit if staging flakes.
```

A recorded deviation is a decision awaiting review. An undocumented
deviation discovered later is a defect — not because the code is
wrong, but because the channel lied. Honest deviations are what keep
the spec authoritative even while reality is ahead of it.

## Size budgets {#budgets}

Control-plane files are read every session; every token in them is a
tax multiplied by the number of sessions the project will ever run.

| File | Budget | When over |
|------|--------|-----------|
| Boot file (always loaded) | ≤ 500 tokens | cut; move detail into flow docs and cite |
| WAL / continuation state  | ≤ 3000 tokens | collapse completed items to one line each |
| One module spec document  | ≤ 5000 tokens | split into submodules |

Rule of thumb: 500 tokens is roughly 375 English words — one page.
The numbers are budgets, not physics; the direction of the pressure
is what matters. A spec that keeps growing past its budget is not a
big spec, it is two specs sharing a file.

## When to split a document {#splitting}

Split when any of these fires:

- the document is over its size budget (§[budgets](#budgets));
- a unit needs "and also" — that is two units, and often two homes;
- two audiences have emerged (contract readers vs tutorial readers);
- one section is cited from outside far more than the rest — promote
  it to its own document so its URI shortens and its neighbours stop
  riding along into every context window.

Splitting is cheap precisely because citations point at anchors, not
page numbers — provided the anchors survive the move
(§[anchor-stability](#anchor-stability)).

## Changelog lines {#changelog}

Each semantic change to a document appends one dated line with the
reason:

```markdown
## Changelog {#changelog}
- [2026-02-17] §verification.timeout: 300 s → 600 s — VPN users
  do not fit in 300 s.
```

The changelog is a signal, not history — history lives in git. The
changelog exists for the reader who last saw this file a week ago
and needs the delta in five seconds, without running a diff.

## Anchor stability {#anchor-stability}

**Anchors are immutable once cited.** An anchor that appears in code
markers, commit bodies, other specs, or the WAL is a public symbol;
renaming it is a breaking change that silently snaps every citation
— the exact failure addressability exists to prevent.

- Never rename a cited anchor. If the heading text must change, the
  `{#id}` stays.
- Never reuse an anchor for a different meaning. An address that
  once meant one thing and now means another is worse than a dead
  link.
- Retire with a tombstone, do not delete:
  `<!-- RETIRED: superseded by {#new-anchor} -->` under the old
  heading location.
- Moving a unit to another document leaves a tombstone at the old
  address pointing to the new one.

Delegate the audit — it is mechanical and the agent is good at it:

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

- One unit, one decision; a unit makes sense alone, on one page.
- Normativity is marked with RFC-2119 verbs — nobody guesses what
  binds.
- Contract and rationale are separate; the spec never restates
  *how*. A spec that mirrors code is shadow code.
- Every contract implies a test; the unit names it.
- Deviations are recorded where they happen, with reasons; the
  undocumented one is the defect.
- Budgets: boot ≤ 500 tokens, WAL ≤ 3000, module spec ≤ 5000 —
  split when over.
- Changelog: one dated line per semantic change, with the reason.
- Anchors are immutable once cited; retirement is a tombstone,
  never a deletion or a silent rename.
