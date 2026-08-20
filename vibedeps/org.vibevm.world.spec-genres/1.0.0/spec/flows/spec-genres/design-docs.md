# Design docs: the contract/lore split in practice {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** The genre table says a module contract
is binding and a design doc is lore. @status:impl/done

@fact:this-file-is-the-practical-edge This file is the practical
edge: *what spills out* of a contract into a design doc, *what never
leaves*, the fork-by-fork shape a design doc uses to record a
decision session, the orthogonal-decomposition lesson that makes big
designs shippable, and how a design doc grows stale honestly. @status:impl/done

## What stays in the contract, what spills out {#split}

@fact:a-contract-is-read-under-time-pressure A contract is read by an implementer under time pressure. @status:spec/done

@fact:EVERYTHING-IN-A-CONTRACT-MUST-EARN-ITS-PLACE Everything
in it must earn its place. @status:impl/done

@fact:THE-SPLIT-IS-LOAD-BEARING-VERSUS-NARRATIVE So the split is not "short versus long" —
it is "load-bearing versus narrative". @status:impl/done

@fact:never-leaves-the-contract-lead **Never leaves the contract:** @status:impl/done

- @fact:KEEP-THE-DECISION-ITSELF The **decision itself** — the value, the shape, the chosen option. @status:impl/done
- @fact:KEEP-THE-ONE-PARAGRAPH-LOAD-BEARING-WHY Its **one-paragraph load-bearing why** — the single reason that, if
  forgotten, would get the decision reverted by accident. @status:impl/done
- @fact:KEEP-THE-CONSTRAINTS The **constraints** the decision must satisfy. @status:impl/done
- @fact:KEEP-THE-INVARIANTS The **invariants** it establishes for everything downstream. @status:impl/done

@fact:spills-out-into-the-design-doc-lead **Spills out into the design doc:** @status:impl/done

- @fact:SPILL-PRECEDENT-STUDIES **Precedent studies** — how three other tools solved this, at
  length. (Contrast with the research genre: research studies an
  external system for its own sake; a precedent study in a design doc
  is marshalled to justify *our* choice.) @status:impl/done
- @fact:SPILL-FORKS-WEIGHED-AT-LENGTH **Forks weighed at length** — every option considered, not just the
  winner. @status:impl/done
- @fact:SPILL-PARKED-IDEAS **Parked ideas** — the "we could later…" branches, explicitly
  marked as not-yet-decided so no one implements them. @status:impl/done
- @fact:SPILL-THE-NARRATIVE-HOW-WE-GOT-HERE **The narrative "how we got here"** — the path of the discussion,
  the analogies, the owner's mental model. @status:impl/done

@fact:THE-BOUNDARY-TEST-CONTRACT-SIDE The test at the boundary: *if deleting this sentence would change
what an implementer builds, it is contract.* @status:impl/done

@fact:THE-BOUNDARY-TEST-LORE-SIDE If deleting it only
costs a future reader the story, it is lore. @status:impl/done

## The fork-by-fork decision record {#forks}

@fact:FORKS-ARE-THE-MOST-VALUABLE-THING-A-DESIGN-DOC-PRESERVES The most valuable thing a design doc preserves is the set of forks a
session resolved — because those are exactly the questions a later
session will otherwise re-litigate. @status:impl/done

@fact:RECORD-EACH-FORK-AS-A-FIXED-SKELETON Record each fork as a fixed
skeleton, so the reasoning is scannable rather than buried in prose: @status:impl/done

```
### Fork N — <the question in one line>

- Options:   <A> | <B> | <C>
- Chosen:    <the winner>
- Why:       <the deciding reason — a constraint, a cost, a precedent>
- Rejected:  <A> — because <reason>
             <C> — because <reason>
- Consequence: <what this fork now forces downstream, if anything>
```

@fact:two-properties-make-the-shape-pay-off Two properties make this shape pay off. @status:impl/done

@fact:REJECTED-LINES-ARE-LOAD-BEARING First, the **Rejected** lines
are load-bearing: "we considered X and rejected it because Y" is the
single sentence that stops the same X being re-proposed next quarter. @status:impl/done

@fact:A-CONSEQUENCE-SIGNALS-A-TWO-WAY-LINK-CHECK Second, a fork with a **Consequence** that constrains a contract is a
signal to check the two-way link — that consequence probably belongs,
in one line, at the contract anchor too. @status:impl/done

## The orthogonal-decomposition lesson {#decomposition}

@fact:SPLIT-A-LARGE-DESIGN-INTO-INDEPENDENTLY-SHIPPABLE-AXES When a design is large, the highest-leverage move is usually to split
it into **independently-shippable axes that compose** — decisions
that have different cost, different dependencies, and can ship as
different milestones. @status:impl/done

@fact:why-keeping-the-axes-separate-clarifies Keeping the axes separate is often the single
most clarifying move of a design session: it turns one intimidating
change into a short chain where the cheap, independent part ships
first and unblocks the rest. @status:spec/done

@fact:worked-sketch-four-axes A worked sketch. A request to "make the tool multi-package with
proper naming" decomposes into four axes: @status:impl/done

```
Axis A — Workspace      : project = a set of modules   | independent → do first
Axis B — Selective pub  : each module opts in/out      | depends on A
Axis C — Qualified names: reverse-FQDN identity        | depends on the index
Axis D — Conflict-aware : collisions fail loudly       | depends on C
```

@fact:the-finding-decomposition-surfaces The finding that decomposition surfaces: Axis A alone closes most of
the request and depends on nothing, while the expensive part (naming,
discovery) is separable and can come later. @status:spec/done

@fact:without-the-split-it-looks-like-a-big-bang Without the split, the
whole thing looks like one big-bang change; with it, the sequencing
is obvious. @status:spec/done

@fact:THE-DESIGN-DOC-RECORDS-THE-DECOMPOSITION-IN-FULL The design doc is where this decomposition and its
dependency chain are recorded in full; the contract for each axis
carries only that axis's decision. @status:impl/done

## Growing stale honestly {#staleness}

@fact:A-DESIGN-DOC-RECORDS-WHAT-WAS-TRUE-WHEN-WRITTEN A design doc records **what was true at the moment it was written**. @status:impl/done

@fact:STALENESS-IS-THE-GENRES-NATURE-NOT-A-FLAW That is not a flaw to be patched away — it is the genre's nature, and
pretending otherwise causes more harm than the staleness itself. @status:impl/done

@fact:honest-posture-lead The honest posture: @status:impl/done

- @fact:CORRECTIONS-HAPPEN-ON-CONTRACT-CONFLICT **Corrections happen on contract conflict.** When the contract
  moves and the design doc now contradicts it, fix the contradicting
  passage and note what changed. This is targeted, not cosmetic. @status:impl/done
- @fact:WHOLESALE-REWRITES-DO-NOT-HAPPEN **Wholesale rewrites do not happen** just because time passed. A
  design doc is not living documentation to be kept current sentence
  by sentence; that would cost real effort to no benefit and would
  erase the record of how the thinking actually evolved. @status:impl/done
- @fact:A-SUPERSEDED-DESIGN-DOC-IS-ARCHIVED-NOT-DELETED **A superseded design doc is archived, not deleted.** The path from
  a rejected early design to the shipped one is itself lore the next
  reader may need. @status:impl/done

@fact:A-DESIGN-DOC-MAY-READ-AS-A-DATED-SNAPSHOT So a design doc is allowed to read as a snapshot with a date on it. @status:impl/done

@fact:IT-MAY-NEVER-DIVERGE-AND-BE-CITED-AS-CURRENT What it is never allowed to do is quietly diverge from the contract
and then get cited as current — that is the exact failure the
precedence law
([`SPEC-GENRES-PROTOCOL.md#precedence`](SPEC-GENRES-PROTOCOL.md#precedence))
exists to prevent. @status:impl/done

## Summary {#summary}

- @fact:SUM-WHAT-THE-CONTRACT-KEEPS Contract keeps the decision, its one-paragraph why, constraints,
  and invariants; everything narrative spills into the design doc. @status:impl/done
- @fact:SUM-THE-BOUNDARY-TEST The boundary test: if deleting a sentence changes what gets built,
  it is contract; if it only costs the story, it is lore. @status:impl/done
- @fact:SUM-RECORD-FORKS-AS-A-SKELETON Record forks as a fixed skeleton — options, chosen, why, rejected,
  consequence — so settled questions are not re-litigated. @status:impl/done
- @fact:SUM-DECOMPOSE-INTO-COMPOSABLE-AXES Decompose large designs into independent, composable axes; ship the
  cheap independent one first. @status:impl/done
- @fact:SUM-A-DATED-SNAPSHOT-CORRECTED-ON-CONFLICT A design doc is a dated snapshot; correct it on contract conflict,
  never rewrite it wholesale to fake currency. @status:impl/done
