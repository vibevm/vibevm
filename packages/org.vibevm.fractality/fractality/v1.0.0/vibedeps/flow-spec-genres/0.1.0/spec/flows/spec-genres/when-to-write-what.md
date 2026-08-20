# When to write what {#root}

**Scope of this document.** A routing table from *situation* to
*target genre*, a companion table of misfiling smells that tell you a
document landed in the wrong genre, and one worked example of the
two-way linking law. Use it at the moment you are about to write —
before the genre is fixed by habit rather than choice.

## The routing table {#routing}

One row per situation. Read the situation, write to the target, and —
where a link column is filled — wire the link before you close the
file.

| Situation | Target genre | Also do |
|-----------|-------------|---------|
| A new constraint or invariant is discovered | **Contract section** | Add a changelog line naming what changed and why |
| A big narrative "why we chose this" | **Design doc** | Link it to the contract; add the back-link at the anchor |
| A study of a competitor or ecosystem tool | **Research doc** | Cite the version/date studied; it will be superseded |
| A multi-session change with phases | **Campaign plan** | Name the gate that ends each phase |
| Where the work stands right now | **The checkpoint** | Overwrite the old state; do not append |
| A durable choice (library, constant, protocol shape) | **Decision record at the contract anchor** | The long-form story, if any, goes in a linked design doc |
| A standing instruction every session needs | **Boot file** | Keep it small; link out to the full protocol |

Two rows are easy to confuse. A **durable choice** goes *at the
contract anchor* as a compact record — decision, why, alternatives,
revisit trigger — because it is binding. A **big narrative why** goes
in a *design doc* because it is lore. Most real decisions produce
both: a tight record at the anchor, and — only when the reasoning is
large — a linked design doc holding the full story. The record is the
contract; the design doc is its footnote, not its replacement.

### When a situation spans two genres {#spanning}

Many real writing tasks land on two rows at once — a session that
both settles a constraint *and* produced pages of reasoning is a
contract edit *and* a design doc. That is not ambiguity to resolve by
picking one; it is a signal to **split along the binding line**:

- The binding half — the constraint, the invariant, the chosen value
  — goes to the contract, in contract shape.
- The narrative half — the forks, the precedents, the path — goes to
  the design doc, in story shape.
- The two-way link joins them, so neither half is lost and the
  authority is unambiguous.

The tell that you are facing a spanning situation is the word "and":
"we decided the timeout is 600 s *and* here is the whole latency
study." The "and" is the seam; cut there.

## Misfiling smells {#smells}

A document in the wrong genre gives off a smell. When you catch one,
the fix is to move the misplaced material to its genre and leave a
link behind — not to leave it where it is because moving is work.

| Smell | Diagnosis | Fix |
|-------|-----------|-----|
| A contract quotes three paragraphs of history | Lore leaked into a contract | Move the history to a design doc; leave a one-line why + link |
| A design doc is cited as if binding ("per the design doc we must…") | A contract is hiding inside lore | Extract the normative sentence to the contract; the design doc keeps the story |
| A checkpoint entry is older than the last release | State posing as truth | The checkpoint is stale; overwrite it — real invariants belong in a contract |
| A campaign plan is quoted to justify a behaviour | An execution doc treated as a spec | The behaviour's authority is the contract; the plan only tracked the work |
| A research doc prescribes what *we* must build | Research drifted into design/contract | Split: the external study stays research; our resulting choice becomes a contract decision |

The unifying tell: **binding weight in a non-binding genre, or
narrative bulk in a binding one.** Either way the genre and the
authority have come apart, which is exactly the confusion genre
typing exists to prevent.

## The linking law, worked {#linking-example}

The two-way link is easy to state and easy to half-do. Here is the
full shape, both ends wired.

At the contract — a `Related` line points *out* to the lore, so a
session reading the contract during boot finds the rationale without
being told it exists:

```
## Timeout is 600 s {#timeout}

The verification timeout is 600 s. Below this, high-latency clients
time out mid-run (measured 2026-03-05, 128 users).

Related: design/verification-timeout.md — the full latency study and
the three thresholds weighed before 600 s.
```

At the design doc — the header names the contract section it
explains, so a reader who arrives at the lore first can walk back to
the authoritative value:

```
# Design: choosing the verification timeout {#root}

Explains and is subordinate to: modules/verify/PROP.md#timeout.
If this document and that section disagree, that section wins and
this one is corrected.

[…the latency study, the 300/600/900 s forks, why 600 won…]
```

Now a cold reader entering from *either* side reaches the other, and
the precedence line in the design-doc header records — in-place — who
wins if they ever drift. That single pair of links is the difference
between lore that survives a cold start and lore that is silently
lost at the next session boundary.

## Summary {#summary}

- Route by situation: constraint → contract; narrative why → design
  doc; external study → research; phased change → campaign plan;
  current state → checkpoint; durable choice → record at the anchor.
- A decision usually produces both a compact record at the contract
  anchor and — if large — a linked design doc; the record is binding,
  the design doc is its footnote.
- Smells flag misfilings: history in a contract, a design doc cited
  as binding, a checkpoint older than the last release.
- Wire both link directions or neither — a one-way link loses the
  lore from the other side.
