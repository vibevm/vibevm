# When to write what {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** A routing table from *situation* to
*target genre*, a companion table of misfiling smells that tell you a
document landed in the wrong genre, and one worked example of the
two-way linking law. @impl/done

##USE-IT-AT-THE-MOMENT-YOU-ARE-ABOUT-TO-WRITE Use it at the moment you are about to write —
before the genre is fixed by habit rather than choice. @impl/done

## The routing table {#routing}

##one-row-per-situation One row per situation. @impl/done

##READ-WRITE-AND-WIRE-THE-LINK Read the situation, write to the target, and —
where a link column is filled — wire the link before you close the
file. @impl/done

| Situation | Target genre | Also do |
|-----------|-------------|---------|
| ##ROW-SITUATION-NEW-CONSTRAINT A new constraint or invariant is discovered @impl/done | **Contract section** @impl/done | Add a changelog line naming what changed and why @impl/done |
| ##ROW-SITUATION-BIG-NARRATIVE-WHY A big narrative "why we chose this" @impl/done | **Design doc** @impl/done | Link it to the contract; add the back-link at the anchor @impl/done |
| ##ROW-SITUATION-COMPETITOR-STUDY A study of a competitor or ecosystem tool @impl/done | **Research doc** @impl/done | Cite the version/date studied; it will be superseded @impl/done |
| ##ROW-SITUATION-MULTI-SESSION-CHANGE A multi-session change with phases @impl/done | **Campaign plan** @impl/done | Name the gate that ends each phase @impl/done |
| ##ROW-SITUATION-WHERE-WORK-STANDS Where the work stands right now @impl/done | **The checkpoint** @impl/done | Overwrite the old state; do not append @impl/done |
| ##ROW-SITUATION-DURABLE-CHOICE A durable choice (library, constant, protocol shape) @impl/done | **Decision record at the contract anchor** @impl/done | The long-form story, if any, goes in a linked design doc @impl/done |
| ##ROW-SITUATION-STANDING-INSTRUCTION A standing instruction every session needs @impl/done | **Boot file** @impl/done | Keep it small; link out to the full protocol @impl/done |

##two-rows-are-easy-to-confuse Two rows are easy to confuse. @impl/done

##A-DURABLE-CHOICE-GOES-AT-THE-CONTRACT-ANCHOR A **durable choice** goes *at the
contract anchor* as a compact record — decision, why, alternatives,
revisit trigger — because it is binding. @impl/done

##A-BIG-NARRATIVE-WHY-GOES-IN-A-DESIGN-DOC A **big narrative why** goes
in a *design doc* because it is lore. @impl/done

##MOST-DECISIONS-PRODUCE-BOTH Most real decisions produce
both: a tight record at the anchor, and — only when the reasoning is
large — a linked design doc holding the full story. @impl/done

##THE-RECORD-IS-THE-CONTRACT-THE-DESIGN-DOC-ITS-FOOTNOTE The record is the
contract; the design doc is its footnote, not its replacement. @impl/done

### When a situation spans two genres {#spanning}

##MANY-TASKS-LAND-ON-TWO-ROWS-AT-ONCE Many real writing tasks land on two rows at once — a session that
both settles a constraint *and* produced pages of reasoning is a
contract edit *and* a design doc. @impl/done

##SPLIT-ALONG-THE-BINDING-LINE That is not ambiguity to resolve by
picking one; it is a signal to **split along the binding line**: @impl/done

- ##SPAN-THE-BINDING-HALF-GOES-TO-THE-CONTRACT The binding half — the constraint, the invariant, the chosen value
  — goes to the contract, in contract shape. @impl/done
- ##SPAN-THE-NARRATIVE-HALF-GOES-TO-THE-DESIGN-DOC The narrative half — the forks, the precedents, the path — goes to
  the design doc, in story shape. @impl/done
- ##SPAN-THE-TWO-WAY-LINK-JOINS-THEM The two-way link joins them, so neither half is lost and the
  authority is unambiguous. @impl/done

##THE-TELL-IS-THE-WORD-AND The tell that you are facing a spanning situation is the word "and":
"we decided the timeout is 600 s *and* here is the whole latency
study." @impl/done

##THE-AND-IS-THE-SEAM-CUT-THERE The "and" is the seam; cut there. @impl/done

## Misfiling smells {#smells}

##a-misfiled-document-gives-off-a-smell A document in the wrong genre gives off a smell. @impl/done

##MOVE-THE-MISPLACED-MATERIAL-AND-LEAVE-A-LINK When you catch one,
the fix is to move the misplaced material to its genre and leave a
link behind — not to leave it where it is because moving is work. @impl/done

| Smell | Diagnosis | Fix |
|-------|-----------|-----|
| ##ROW-SMELL-CONTRACT-QUOTES-HISTORY A contract quotes three paragraphs of history @impl/done | Lore leaked into a contract @impl/done | Move the history to a design doc; leave a one-line why + link @impl/done |
| ##ROW-SMELL-DESIGN-DOC-CITED-AS-BINDING A design doc is cited as if binding ("per the design doc we must…") @impl/done | A contract is hiding inside lore @impl/done | Extract the normative sentence to the contract; the design doc keeps the story @impl/done |
| ##ROW-SMELL-CHECKPOINT-OLDER-THAN-THE-RELEASE A checkpoint entry is older than the last release @impl/done | State posing as truth @impl/done | The checkpoint is stale; overwrite it — real invariants belong in a contract @impl/done |
| ##ROW-SMELL-CAMPAIGN-PLAN-AS-SPEC A campaign plan is quoted to justify a behaviour @impl/done | An execution doc treated as a spec @impl/done | The behaviour's authority is the contract; the plan only tracked the work @impl/done |
| ##ROW-SMELL-RESEARCH-PRESCRIBES A research doc prescribes what *we* must build @impl/done | Research drifted into design/contract @impl/done | Split: the external study stays research; our resulting choice becomes a contract decision @impl/done |

##THE-UNIFYING-TELL The unifying tell: **binding weight in a non-binding genre, or
narrative bulk in a binding one.** @impl/done

##genre-and-authority-have-come-apart Either way the genre and the
authority have come apart, which is exactly the confusion genre
typing exists to prevent. @impl/done

## The linking law, worked {#linking-example}

##easy-to-state-and-easy-to-half-do The two-way link is easy to state and easy to half-do. @spec/done

##here-is-the-full-shape Here is the
full shape, both ends wired. @impl/done

##AT-THE-CONTRACT-A-RELATED-LINE-POINTS-OUT At the contract — a `Related` line points *out* to the lore, so a
session reading the contract during boot finds the rationale without
being told it exists: @impl/done

```
## Timeout is 600 s {#timeout}

The verification timeout is 600 s. Below this, high-latency clients
time out mid-run (measured 2026-03-05, 128 users).

Related: design/verification-timeout.md — the full latency study and
the three thresholds weighed before 600 s.
```

##AT-THE-DESIGN-DOC-THE-HEADER-NAMES-THE-CONTRACT At the design doc — the header names the contract section it
explains, so a reader who arrives at the lore first can walk back to
the authoritative value: @impl/done

```
# Design: choosing the verification timeout {#root}

Explains and is subordinate to: modules/verify/PROP.md#timeout.
If this document and that section disagree, that section wins and
this one is corrected.

[…the latency study, the 300/600/900 s forks, why 600 won…]
```

##BOTH-ENDS-WIRED-MEANS-EITHER-SIDE-REACHES-THE-OTHER Now a cold reader entering from *either* side reaches the other, and
the precedence line in the design-doc header records — in-place — who
wins if they ever drift. @impl/done

##what-the-pair-of-links-buys That single pair of links is the difference
between lore that survives a cold start and lore that is silently
lost at the next session boundary. @spec/done

## Summary {#summary}

- ##SUM-ROUTE-BY-SITUATION Route by situation: constraint → contract; narrative why → design
  doc; external study → research; phased change → campaign plan;
  current state → checkpoint; durable choice → record at the anchor. @impl/done
- ##SUM-A-DECISION-PRODUCES-BOTH A decision usually produces both a compact record at the contract
  anchor and — if large — a linked design doc; the record is binding,
  the design doc is its footnote. @impl/done
- ##SUM-SMELLS-FLAG-MISFILINGS Smells flag misfilings: history in a contract, a design doc cited
  as binding, a checkpoint older than the last release. @impl/done
- ##SUM-WIRE-BOTH-DIRECTIONS-OR-NEITHER Wire both link directions or neither — a one-way link loses the
  lore from the other side. @impl/done
