# When to write what {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** A routing table from *situation* to
*target genre*, a companion table of misfiling smells that tell you a
document landed in the wrong genre, and one worked example of the
two-way linking law. @status:impl/done

@fact:USE-IT-AT-THE-MOMENT-YOU-ARE-ABOUT-TO-WRITE Use it at the moment you are about to write —
before the genre is fixed by habit rather than choice. @status:impl/done

## The routing table {#routing}

@fact:one-row-per-situation One row per situation. @status:impl/done

@fact:READ-WRITE-AND-WIRE-THE-LINK Read the situation, write to the target, and —
where a link column is filled — wire the link before you close the
file. @status:impl/done

| Situation | Target genre | Also do |
|-----------|-------------|---------|
| @fact:ROW-SITUATION-NEW-CONSTRAINT A new constraint or invariant is discovered @status:impl/done | **Contract section** @status:impl/done | Add a changelog line naming what changed and why @status:impl/done |
| @fact:ROW-SITUATION-BIG-NARRATIVE-WHY A big narrative "why we chose this" @status:impl/done | **Design doc** @status:impl/done | Link it to the contract; add the back-link at the anchor @status:impl/done |
| @fact:ROW-SITUATION-COMPETITOR-STUDY A study of a competitor or ecosystem tool @status:impl/done | **Research doc** @status:impl/done | Cite the version/date studied; it will be superseded @status:impl/done |
| @fact:ROW-SITUATION-MULTI-SESSION-CHANGE A multi-session change with phases @status:impl/done | **Campaign plan** @status:impl/done | Name the gate that ends each phase @status:impl/done |
| @fact:ROW-SITUATION-WHERE-WORK-STANDS Where the work stands right now @status:impl/done | **The checkpoint** @status:impl/done | Overwrite the old state; do not append @status:impl/done |
| @fact:ROW-SITUATION-DURABLE-CHOICE A durable choice (library, constant, protocol shape) @status:impl/done | **Decision record at the contract anchor** @status:impl/done | The long-form story, if any, goes in a linked design doc @status:impl/done |
| @fact:ROW-SITUATION-STANDING-INSTRUCTION A standing instruction every session needs @status:impl/done | **Boot file** @status:impl/done | Keep it small; link out to the full protocol @status:impl/done |

@fact:two-rows-are-easy-to-confuse Two rows are easy to confuse. @status:impl/done

@fact:A-DURABLE-CHOICE-GOES-AT-THE-CONTRACT-ANCHOR A **durable choice** goes *at the
contract anchor* as a compact record — decision, why, alternatives,
revisit trigger — because it is binding. @status:impl/done

@fact:A-BIG-NARRATIVE-WHY-GOES-IN-A-DESIGN-DOC A **big narrative why** goes
in a *design doc* because it is lore. @status:impl/done

@fact:MOST-DECISIONS-PRODUCE-BOTH Most real decisions produce
both: a tight record at the anchor, and — only when the reasoning is
large — a linked design doc holding the full story. @status:impl/done

@fact:THE-RECORD-IS-THE-CONTRACT-THE-DESIGN-DOC-ITS-FOOTNOTE The record is the
contract; the design doc is its footnote, not its replacement. @status:impl/done

### When a situation spans two genres {#spanning}

@fact:MANY-TASKS-LAND-ON-TWO-ROWS-AT-ONCE Many real writing tasks land on two rows at once — a session that
both settles a constraint *and* produced pages of reasoning is a
contract edit *and* a design doc. @status:impl/done

@fact:SPLIT-ALONG-THE-BINDING-LINE That is not ambiguity to resolve by
picking one; it is a signal to **split along the binding line**: @status:impl/done

- @fact:SPAN-THE-BINDING-HALF-GOES-TO-THE-CONTRACT The binding half — the constraint, the invariant, the chosen value
  — goes to the contract, in contract shape. @status:impl/done
- @fact:SPAN-THE-NARRATIVE-HALF-GOES-TO-THE-DESIGN-DOC The narrative half — the forks, the precedents, the path — goes to
  the design doc, in story shape. @status:impl/done
- @fact:SPAN-THE-TWO-WAY-LINK-JOINS-THEM The two-way link joins them, so neither half is lost and the
  authority is unambiguous. @status:impl/done

@fact:THE-TELL-IS-THE-WORD-AND The tell that you are facing a spanning situation is the word "and":
"we decided the timeout is 600 s *and* here is the whole latency
study." @status:impl/done

@fact:THE-AND-IS-THE-SEAM-CUT-THERE The "and" is the seam; cut there. @status:impl/done

## Misfiling smells {#smells}

@fact:a-misfiled-document-gives-off-a-smell A document in the wrong genre gives off a smell. @status:impl/done

@fact:MOVE-THE-MISPLACED-MATERIAL-AND-LEAVE-A-LINK When you catch one,
the fix is to move the misplaced material to its genre and leave a
link behind — not to leave it where it is because moving is work. @status:impl/done

| Smell | Diagnosis | Fix |
|-------|-----------|-----|
| @fact:ROW-SMELL-CONTRACT-QUOTES-HISTORY A contract quotes three paragraphs of history @status:impl/done | Lore leaked into a contract @status:impl/done | Move the history to a design doc; leave a one-line why + link @status:impl/done |
| @fact:ROW-SMELL-DESIGN-DOC-CITED-AS-BINDING A design doc is cited as if binding ("per the design doc we must…") @status:impl/done | A contract is hiding inside lore @status:impl/done | Extract the normative sentence to the contract; the design doc keeps the story @status:impl/done |
| @fact:ROW-SMELL-CHECKPOINT-OLDER-THAN-THE-RELEASE A checkpoint entry is older than the last release @status:impl/done | State posing as truth @status:impl/done | The checkpoint is stale; overwrite it — real invariants belong in a contract @status:impl/done |
| @fact:ROW-SMELL-CAMPAIGN-PLAN-AS-SPEC A campaign plan is quoted to justify a behaviour @status:impl/done | An execution doc treated as a spec @status:impl/done | The behaviour's authority is the contract; the plan only tracked the work @status:impl/done |
| @fact:ROW-SMELL-RESEARCH-PRESCRIBES A research doc prescribes what *we* must build @status:impl/done | Research drifted into design/contract @status:impl/done | Split: the external study stays research; our resulting choice becomes a contract decision @status:impl/done |

@fact:THE-UNIFYING-TELL The unifying tell: **binding weight in a non-binding genre, or
narrative bulk in a binding one.** @status:impl/done

@fact:genre-and-authority-have-come-apart Either way the genre and the
authority have come apart, which is exactly the confusion genre
typing exists to prevent. @status:impl/done

## The linking law, worked {#linking-example}

@fact:easy-to-state-and-easy-to-half-do The two-way link is easy to state and easy to half-do. @status:spec/done

@fact:here-is-the-full-shape Here is the
full shape, both ends wired. @status:impl/done

@fact:AT-THE-CONTRACT-A-RELATED-LINE-POINTS-OUT At the contract — a `Related` line points *out* to the lore, so a
session reading the contract during boot finds the rationale without
being told it exists: @status:impl/done

```
## Timeout is 600 s {#timeout}

The verification timeout is 600 s. Below this, high-latency clients
time out mid-run (measured 2026-03-05, 128 users).

Related: design/verification-timeout.md — the full latency study and
the three thresholds weighed before 600 s.
```

@fact:AT-THE-DESIGN-DOC-THE-HEADER-NAMES-THE-CONTRACT At the design doc — the header names the contract section it
explains, so a reader who arrives at the lore first can walk back to
the authoritative value: @status:impl/done

```
# Design: choosing the verification timeout {#root}

Explains and is subordinate to: modules/verify/PROP.md#timeout.
If this document and that section disagree, that section wins and
this one is corrected.

[…the latency study, the 300/600/900 s forks, why 600 won…]
```

@fact:BOTH-ENDS-WIRED-MEANS-EITHER-SIDE-REACHES-THE-OTHER Now a cold reader entering from *either* side reaches the other, and
the precedence line in the design-doc header records — in-place — who
wins if they ever drift. @status:impl/done

@fact:what-the-pair-of-links-buys That single pair of links is the difference
between lore that survives a cold start and lore that is silently
lost at the next session boundary. @status:spec/done

## Summary {#summary}

- @fact:SUM-ROUTE-BY-SITUATION Route by situation: constraint → contract; narrative why → design
  doc; external study → research; phased change → campaign plan;
  current state → checkpoint; durable choice → record at the anchor. @status:impl/done
- @fact:SUM-A-DECISION-PRODUCES-BOTH A decision usually produces both a compact record at the contract
  anchor and — if large — a linked design doc; the record is binding,
  the design doc is its footnote. @status:impl/done
- @fact:SUM-SMELLS-FLAG-MISFILINGS Smells flag misfilings: history in a contract, a design doc cited
  as binding, a checkpoint older than the last release. @status:impl/done
- @fact:SUM-WIRE-BOTH-DIRECTIONS-OR-NEITHER Wire both link directions or neither — a one-way link loses the
  lore from the other side. @status:impl/done
