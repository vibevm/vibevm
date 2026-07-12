# Flow: Spec Genres {#root}

This project's documents are **genre-typed**. One undifferentiated
pile of markdown rots: contracts bloat with narrative, narrative
gets treated as binding, and nobody knows what wins. Every document
belongs to exactly one genre; the genre decides where it lives, how
it may change, and what authority it carries.

## The genre map {#genre-map}

| Genre | Holds | Binding? |
|-------|-------|----------|
| Boot files | Standing instructions read at session start | yes |
| Foundational decisions | Choices that cross every module | yes |
| Module contracts | What each module does (here: PROP / FEAT) | yes |
| Design docs | Why we chose what we chose — the lore | no |
| Research docs | What *other* projects did | no |
| Campaign plans | Phases and gates of one multi-session change | no |
| The checkpoint | Where work stands right now | state, not truth |

Full charters, mutability rules, and conflict authority:
[`spec/flows/spec-genres/SPEC-GENRES-PROTOCOL.md`](../flows/spec-genres/SPEC-GENRES-PROTOCOL.md).

## Core rule {#core-rule}

**Before writing any project document, name its genre first.** Then:

1. **Contract wins over lore.** When a design document and the
   contract it explains disagree, the contract wins and the design
   document is corrected — lore never silently diverges.
2. **Keep the two-way links.** A contract section that has lore
   links to it; the lore names the section it explains. A cold
   reader entering from either side finds the other.

Routing table for new material:
[`spec/flows/spec-genres/when-to-write-what.md`](../flows/spec-genres/when-to-write-what.md).
The contract/lore split in practice:
[`spec/flows/spec-genres/design-docs.md`](../flows/spec-genres/design-docs.md).

## Why this matters in a human-AI team {#why}

The agent reads the tree cold every session. If binding and
non-binding prose look alike, it will implement a parked idea out of
a design doc, or soften a contract because the narrative around it
sounded tentative. Genre typing is what lets a cold reader assign
authority to a sentence without asking anyone.

## Never {#never}

- Never put normative language — "must", "shall", requirement
  lists — in a design doc. Extract it to the contract; link back.
- Never resolve a contract-vs-lore conflict by editing the contract
  to match the lore. The correction runs the other way.
- Never create a document without deciding its genre.
- Never let lore go unlinked from its contract — an unlinked design
  doc is invisible at the next cold start.
