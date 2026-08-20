# Spec Genres Protocol {#root}

**Scope of this document.** This file defines *why* project
documents are sorted into genres, *which* genres exist and what each
one is for, *who wins* when two of them disagree, and *how* the
two-way links keep the non-binding genres reachable from a cold
start. It is the taxonomy; the sibling documents are the routing
table and the contract/lore split in practice.

## Why genres exist {#why}

Left alone, a spec tree becomes one undifferentiated pile of
markdown, and three failures follow, every time:

- **Contracts bloat with narrative.** A discussion log — every fork,
  every analogy, every rejected branch — gets poured into the
  document that an implementer must read as a contract. Now the
  contract is unreadable as a contract.
- **Narrative gets treated as binding.** A paragraph of "here is how
  we were thinking about it" sits next to a requirement, in the same
  font, and a later reader implements the musing.
- **Nobody knows what wins.** Two documents say different things and
  there is no rule for which one is authoritative, so the conflict is
  resolved by whoever edited last, not by design.

Genre typing removes all three. Each document declares what kind of
thing it is; the kind fixes its charter, its mutability, its
audience, and — decisively — its authority when it collides with
another document.

## The genre table {#genres}

Each genre has a **charter** (what it is for), a **mutability** (how
it is allowed to change), a **reader**, and an **authority** (what
wins on conflict).

| Genre | Charter | Mutability | Reader | Authority |
|-------|---------|-----------|--------|-----------|
| **Boot files** | Standing instructions loaded at session start; small and stable | Rare, deliberate edits | Every session, first | Binding |
| **Foundational decisions** | Choices that cross every module (identity, versioning, licensing) | Amended by decision record | Everyone | Binding |
| **Module contracts** | What one module does — its behaviour, constraints, invariants (this convention: PROP / FEAT) | Edit + changelog line | Implementers | Binding |
| **Design docs** | Why we chose what we chose — the narrative lore behind a contract | Rewritten freely; corrected on contract conflict | Anyone tracing intent | Non-binding |
| **Research docs** | What *other* projects/systems did — external studies, prior art | Superseded by newer study | Anyone surveying the field | Non-binding |
| **Campaign plans** | The phases and gates of one multi-session change | Ticked as executed; archived when done | The crew running it | Non-binding (execution, not truth) |
| **The checkpoint** | Where work stands *right now* — branch, blocker, next step | Overwritten every session | The next session, first | State, not truth |

Two axes separate the binding genres from the rest. **Boot,
foundational decisions, and module contracts are binding** — they say
what the system *is* and *must do*. **Design and research are
non-binding** — they explain, they do not require. **Campaign plans
and the checkpoint are volatile** — they track execution and state,
which change faster than truth and must never be mistaken for it.

### Contract versus lore, precisely {#contract-vs-lore}

The line that does the most work is between a **module contract** and
a **design doc**. The load-bearing rationale — the decision itself,
its one-paragraph why, the constraints, the invariants — stays
*inside* the contract. The narrative rationale — the full discussion,
the forks weighed at length, the precedents studied, the ideas parked
for later — moves *out* into a design doc. The contract stays
readable as a contract; the lore stays available to anyone who wants
the whole story.

Design and research both point away from the contract, but in
opposite directions: **research studies what other projects did;
design records why we chose what we chose.** A backgrounder on a
competitor is research. The reasoning behind our own module is
design.

## The precedence law {#precedence}

> When a design document and the contract it explains disagree, the
> contract wins and the design document is corrected.

This is not a tie-breaker of convenience; it is the rule that keeps
lore honest. Lore is allowed to be wrong — it records what was true
when it was written, and the world moves. What lore is *not* allowed
to do is silently diverge and then get cited as if current. So the
correction always runs one way:

- The contract changed; a design doc now contradicts it → **fix the
  design doc**, add a line noting what moved.
- A design doc proposes something the contract forbids → the design
  doc is a **parked idea**, marked as such, never mistaken for the
  rule.
- Never edit the contract to match the lore. That inverts the
  authority and lets a stale musing rewrite the system.

The same ordering holds for the volatile genres against the binding
ones: a campaign plan or a checkpoint entry never overrides a
contract. If the checkpoint says one thing and the contract another,
the checkpoint is stale — state posing as truth — and gets corrected.

## The two-way linking law {#linking}

A non-binding document is only useful if a cold reader can *find* it.
The link is the mechanism that makes the lore survive a cold start.

> A contract section that has lore links to it. The lore links back.
> A cold reader entering from either side finds the other.

Concretely:

- Every design doc names the contract section(s) it explains.
- Every contract section that has a design doc links to it — from a
  `Related` line, a footnote, an anchor reference — so a session
  reading the contract during boot discovers the rationale *without
  being told it exists*.
- A one-directional link is a latent break: follow the tree from the
  unlinked side and the other half is invisible. Both directions or
  neither.

This is why an unlinked design doc counts as a defect, not merely
untidy. It holds real reasoning that the next cold reader will never
reach, and so the project will re-derive — or re-litigate — a
question it already settled.

## Placing a document {#placing}

The routine is short, and it runs *before* the first sentence:

1. **Name the genre.** Is this binding or explanatory? Does it
   describe our system (contract/design) or someone else's
   (research)? Is it truth (contract) or state (checkpoint/plan)?
2. **Put it in that genre's home** and give it that genre's shape —
   a contract reads as a contract, a design doc reads as a story.
3. **Wire the links** if it is lore: name its contract, and add the
   back-link from the contract.
4. **Do not blend.** If the draft is half requirement and half
   story, it is two documents — split it along the binding line.

The decision table for common situations lives in
[`when-to-write-what.md`](when-to-write-what.md); the contract/lore
split, with the fork-by-fork record shape, lives in
[`design-docs.md`](design-docs.md).

## Re-derive for your project {#re-derive}

Do not adopt this table verbatim — your project already has genres,
named or not. Have the agent surface them and map yours onto this
frame:

```
Read spec/flows/spec-genres/ in full, then adapt it to this project:
1. Inventory every kind of document this repo already holds (specs,
   ADRs, RFCs, design docs, wikis, READMEs, runbooks, planning docs,
   status files). Group them by the genre they actually play.
2. For each genre, name its charter, mutability, reader, and what
   wins when it conflicts with another — flag any genre with no
   conflict rule.
3. Find contract documents carrying paragraphs of narrative history,
   and design/planning documents written in binding language. List
   each as a misfiling to correct.
4. Check the binding<->lore links: list contract sections whose
   rationale lives elsewhere but is not linked, and lore documents
   that name no contract.
5. Show me the genre map and the misfilings as a plan. Change
   nothing until I approve.
```

## Summary {#summary}

- One undifferentiated pile of markdown rots three ways: contracts
  bloat, narrative goes binding, nobody knows what wins.
- Seven genres, three authorities: binding (boot, foundational,
  contracts), non-binding (design, research), volatile (plans, the
  checkpoint).
- Contract wins over lore. Lore is corrected on conflict, never the
  other way, and never silently diverges.
- Two-way links are the mechanism: a cold reader entering from either
  the contract or the lore finds the other.
- Name the genre before writing the first sentence.
