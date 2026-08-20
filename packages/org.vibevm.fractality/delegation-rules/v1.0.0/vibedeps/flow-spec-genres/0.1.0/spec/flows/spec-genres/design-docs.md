# Design docs: the contract/lore split in practice {#root}

**Scope of this document.** The genre table says a module contract
is binding and a design doc is lore. This file is the practical
edge: *what spills out* of a contract into a design doc, *what never
leaves*, the fork-by-fork shape a design doc uses to record a
decision session, the orthogonal-decomposition lesson that makes big
designs shippable, and how a design doc grows stale honestly.

## What stays in the contract, what spills out {#split}

A contract is read by an implementer under time pressure. Everything
in it must earn its place. So the split is not "short versus long" —
it is "load-bearing versus narrative".

**Never leaves the contract:**

- The **decision itself** — the value, the shape, the chosen option.
- Its **one-paragraph load-bearing why** — the single reason that, if
  forgotten, would get the decision reverted by accident.
- The **constraints** the decision must satisfy.
- The **invariants** it establishes for everything downstream.

**Spills out into the design doc:**

- **Precedent studies** — how three other tools solved this, at
  length. (Contrast with the research genre: research studies an
  external system for its own sake; a precedent study in a design doc
  is marshalled to justify *our* choice.)
- **Forks weighed at length** — every option considered, not just the
  winner.
- **Parked ideas** — the "we could later…" branches, explicitly
  marked as not-yet-decided so no one implements them.
- **The narrative "how we got here"** — the path of the discussion,
  the analogies, the owner's mental model.

The test at the boundary: *if deleting this sentence would change
what an implementer builds, it is contract.* If deleting it only
costs a future reader the story, it is lore.

## The fork-by-fork decision record {#forks}

The most valuable thing a design doc preserves is the set of forks a
session resolved — because those are exactly the questions a later
session will otherwise re-litigate. Record each fork as a fixed
skeleton, so the reasoning is scannable rather than buried in prose:

```
### Fork N — <the question in one line>

- Options:   <A> | <B> | <C>
- Chosen:    <the winner>
- Why:       <the deciding reason — a constraint, a cost, a precedent>
- Rejected:  <A> — because <reason>
             <C> — because <reason>
- Consequence: <what this fork now forces downstream, if anything>
```

Two properties make this shape pay off. First, the **Rejected** lines
are load-bearing: "we considered X and rejected it because Y" is the
single sentence that stops the same X being re-proposed next quarter.
Second, a fork with a **Consequence** that constrains a contract is a
signal to check the two-way link — that consequence probably belongs,
in one line, at the contract anchor too.

## The orthogonal-decomposition lesson {#decomposition}

When a design is large, the highest-leverage move is usually to split
it into **independently-shippable axes that compose** — decisions
that have different cost, different dependencies, and can ship as
different milestones. Keeping the axes separate is often the single
most clarifying move of a design session: it turns one intimidating
change into a short chain where the cheap, independent part ships
first and unblocks the rest.

A worked sketch. A request to "make the tool multi-package with
proper naming" decomposes into four axes:

```
Axis A — Workspace      : project = a set of modules   | independent → do first
Axis B — Selective pub  : each module opts in/out      | depends on A
Axis C — Qualified names: reverse-FQDN identity        | depends on the index
Axis D — Conflict-aware : collisions fail loudly       | depends on C
```

The finding that decomposition surfaces: Axis A alone closes most of
the request and depends on nothing, while the expensive part (naming,
discovery) is separable and can come later. Without the split, the
whole thing looks like one big-bang change; with it, the sequencing
is obvious. The design doc is where this decomposition and its
dependency chain are recorded in full; the contract for each axis
carries only that axis's decision.

## Growing stale honestly {#staleness}

A design doc records **what was true at the moment it was written**.
That is not a flaw to be patched away — it is the genre's nature, and
pretending otherwise causes more harm than the staleness itself.

The honest posture:

- **Corrections happen on contract conflict.** When the contract
  moves and the design doc now contradicts it, fix the contradicting
  passage and note what changed. This is targeted, not cosmetic.
- **Wholesale rewrites do not happen** just because time passed. A
  design doc is not living documentation to be kept current sentence
  by sentence; that would cost real effort to no benefit and would
  erase the record of how the thinking actually evolved.
- **A superseded design doc is archived, not deleted.** The path from
  a rejected early design to the shipped one is itself lore the next
  reader may need.

So a design doc is allowed to read as a snapshot with a date on it.
What it is never allowed to do is quietly diverge from the contract
and then get cited as current — that is the exact failure the
precedence law
([`SPEC-GENRES-PROTOCOL.md#precedence`](SPEC-GENRES-PROTOCOL.md#precedence))
exists to prevent.

## Summary {#summary}

- Contract keeps the decision, its one-paragraph why, constraints,
  and invariants; everything narrative spills into the design doc.
- The boundary test: if deleting a sentence changes what gets built,
  it is contract; if it only costs the story, it is lore.
- Record forks as a fixed skeleton — options, chosen, why, rejected,
  consequence — so settled questions are not re-litigated.
- Decompose large designs into independent, composable axes; ship the
  cheap independent one first.
- A design doc is a dated snapshot; correct it on contract conflict,
  never rewrite it wholesale to fake currency.
