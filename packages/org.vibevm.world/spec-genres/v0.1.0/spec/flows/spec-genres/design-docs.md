# Design docs: the contract/lore split in practice {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** The genre table says a module contract
is binding and a design doc is lore. @impl/done

##this-file-is-the-practical-edge This file is the practical
edge: *what spills out* of a contract into a design doc, *what never
leaves*, the fork-by-fork shape a design doc uses to record a
decision session, the orthogonal-decomposition lesson that makes big
designs shippable, and how a design doc grows stale honestly. @impl/done

## What stays in the contract, what spills out {#split}

##a-contract-is-read-under-time-pressure A contract is read by an implementer under time pressure. @spec/done

##EVERYTHING-IN-A-CONTRACT-MUST-EARN-ITS-PLACE Everything
in it must earn its place. @impl/done

##THE-SPLIT-IS-LOAD-BEARING-VERSUS-NARRATIVE So the split is not "short versus long" —
it is "load-bearing versus narrative". @impl/done

##never-leaves-the-contract-lead **Never leaves the contract:** @impl/done

- ##KEEP-THE-DECISION-ITSELF The **decision itself** — the value, the shape, the chosen option. @impl/done
- ##KEEP-THE-ONE-PARAGRAPH-LOAD-BEARING-WHY Its **one-paragraph load-bearing why** — the single reason that, if
  forgotten, would get the decision reverted by accident. @impl/done
- ##KEEP-THE-CONSTRAINTS The **constraints** the decision must satisfy. @impl/done
- ##KEEP-THE-INVARIANTS The **invariants** it establishes for everything downstream. @impl/done

##spills-out-into-the-design-doc-lead **Spills out into the design doc:** @impl/done

- ##SPILL-PRECEDENT-STUDIES **Precedent studies** — how three other tools solved this, at
  length. (Contrast with the research genre: research studies an
  external system for its own sake; a precedent study in a design doc
  is marshalled to justify *our* choice.) @impl/done
- ##SPILL-FORKS-WEIGHED-AT-LENGTH **Forks weighed at length** — every option considered, not just the
  winner. @impl/done
- ##SPILL-PARKED-IDEAS **Parked ideas** — the "we could later…" branches, explicitly
  marked as not-yet-decided so no one implements them. @impl/done
- ##SPILL-THE-NARRATIVE-HOW-WE-GOT-HERE **The narrative "how we got here"** — the path of the discussion,
  the analogies, the owner's mental model. @impl/done

##THE-BOUNDARY-TEST-CONTRACT-SIDE The test at the boundary: *if deleting this sentence would change
what an implementer builds, it is contract.* @impl/done

##THE-BOUNDARY-TEST-LORE-SIDE If deleting it only
costs a future reader the story, it is lore. @impl/done

## The fork-by-fork decision record {#forks}

##FORKS-ARE-THE-MOST-VALUABLE-THING-A-DESIGN-DOC-PRESERVES The most valuable thing a design doc preserves is the set of forks a
session resolved — because those are exactly the questions a later
session will otherwise re-litigate. @impl/done

##RECORD-EACH-FORK-AS-A-FIXED-SKELETON Record each fork as a fixed
skeleton, so the reasoning is scannable rather than buried in prose: @impl/done

```
### Fork N — <the question in one line>

- Options:   <A> | <B> | <C>
- Chosen:    <the winner>
- Why:       <the deciding reason — a constraint, a cost, a precedent>
- Rejected:  <A> — because <reason>
             <C> — because <reason>
- Consequence: <what this fork now forces downstream, if anything>
```

##two-properties-make-the-shape-pay-off Two properties make this shape pay off. @impl/done

##REJECTED-LINES-ARE-LOAD-BEARING First, the **Rejected** lines
are load-bearing: "we considered X and rejected it because Y" is the
single sentence that stops the same X being re-proposed next quarter. @impl/done

##A-CONSEQUENCE-SIGNALS-A-TWO-WAY-LINK-CHECK Second, a fork with a **Consequence** that constrains a contract is a
signal to check the two-way link — that consequence probably belongs,
in one line, at the contract anchor too. @impl/done

## The orthogonal-decomposition lesson {#decomposition}

##SPLIT-A-LARGE-DESIGN-INTO-INDEPENDENTLY-SHIPPABLE-AXES When a design is large, the highest-leverage move is usually to split
it into **independently-shippable axes that compose** — decisions
that have different cost, different dependencies, and can ship as
different milestones. @impl/done

##why-keeping-the-axes-separate-clarifies Keeping the axes separate is often the single
most clarifying move of a design session: it turns one intimidating
change into a short chain where the cheap, independent part ships
first and unblocks the rest. @spec/done

##worked-sketch-four-axes A worked sketch. A request to "make the tool multi-package with
proper naming" decomposes into four axes: @impl/done

```
Axis A — Workspace      : project = a set of modules   | independent → do first
Axis B — Selective pub  : each module opts in/out      | depends on A
Axis C — Qualified names: reverse-FQDN identity        | depends on the index
Axis D — Conflict-aware : collisions fail loudly       | depends on C
```

##the-finding-decomposition-surfaces The finding that decomposition surfaces: Axis A alone closes most of
the request and depends on nothing, while the expensive part (naming,
discovery) is separable and can come later. @spec/done

##without-the-split-it-looks-like-a-big-bang Without the split, the
whole thing looks like one big-bang change; with it, the sequencing
is obvious. @spec/done

##THE-DESIGN-DOC-RECORDS-THE-DECOMPOSITION-IN-FULL The design doc is where this decomposition and its
dependency chain are recorded in full; the contract for each axis
carries only that axis's decision. @impl/done

## Growing stale honestly {#staleness}

##A-DESIGN-DOC-RECORDS-WHAT-WAS-TRUE-WHEN-WRITTEN A design doc records **what was true at the moment it was written**. @impl/done

##STALENESS-IS-THE-GENRES-NATURE-NOT-A-FLAW That is not a flaw to be patched away — it is the genre's nature, and
pretending otherwise causes more harm than the staleness itself. @impl/done

##honest-posture-lead The honest posture: @impl/done

- ##CORRECTIONS-HAPPEN-ON-CONTRACT-CONFLICT **Corrections happen on contract conflict.** When the contract
  moves and the design doc now contradicts it, fix the contradicting
  passage and note what changed. This is targeted, not cosmetic. @impl/done
- ##WHOLESALE-REWRITES-DO-NOT-HAPPEN **Wholesale rewrites do not happen** just because time passed. A
  design doc is not living documentation to be kept current sentence
  by sentence; that would cost real effort to no benefit and would
  erase the record of how the thinking actually evolved. @impl/done
- ##A-SUPERSEDED-DESIGN-DOC-IS-ARCHIVED-NOT-DELETED **A superseded design doc is archived, not deleted.** The path from
  a rejected early design to the shipped one is itself lore the next
  reader may need. @impl/done

##A-DESIGN-DOC-MAY-READ-AS-A-DATED-SNAPSHOT So a design doc is allowed to read as a snapshot with a date on it. @impl/done

##IT-MAY-NEVER-DIVERGE-AND-BE-CITED-AS-CURRENT What it is never allowed to do is quietly diverge from the contract
and then get cited as current — that is the exact failure the
precedence law
([`SPEC-GENRES-PROTOCOL.md#precedence`](SPEC-GENRES-PROTOCOL.md#precedence))
exists to prevent. @impl/done

## Summary {#summary}

- ##SUM-WHAT-THE-CONTRACT-KEEPS Contract keeps the decision, its one-paragraph why, constraints,
  and invariants; everything narrative spills into the design doc. @impl/done
- ##SUM-THE-BOUNDARY-TEST The boundary test: if deleting a sentence changes what gets built,
  it is contract; if it only costs the story, it is lore. @impl/done
- ##SUM-RECORD-FORKS-AS-A-SKELETON Record forks as a fixed skeleton — options, chosen, why, rejected,
  consequence — so settled questions are not re-litigated. @impl/done
- ##SUM-DECOMPOSE-INTO-COMPOSABLE-AXES Decompose large designs into independent, composable axes; ship the
  cheap independent one first. @impl/done
- ##SUM-A-DATED-SNAPSHOT-CORRECTED-ON-CONFLICT A design doc is a dated snapshot; correct it on contract conflict,
  never rewrite it wholesale to fake currency. @impl/done
