# `flow:spec-genres` — what goes where, and who wins {#root}

A `flow` package that installs a **genre taxonomy** for a project's
documents. One undifferentiated pile of markdown rots three ways:
contracts bloat with narrative, narrative gets treated as binding,
and nobody knows what wins. This package sorts every document into a
genre, fixes what each genre may do, and pins the two laws that keep
the pile honest — contract wins over lore, and the two-way link that
lets a cold reader find the lore behind a contract.

The genres: binding (boot files, foundational decisions, module
contracts), non-binding (design docs, research docs), and volatile
(campaign plans, the checkpoint). The convention here names module
contracts PROP / FEAT — that is a naming choice this collection
carries from its origin, and you can rename it for your project; the
taxonomy is what matters, not the labels.

This package ships four pieces of content plus a boot snippet:

- `spec/flows/spec-genres/SPEC-GENRES-PROTOCOL.md` — the taxonomy:
  why genres exist, the full genre table (charter, mutability,
  reader, authority), the precedence law, the two-way linking law,
  and a re-derive prompt for adapting the map to any project.
- `spec/flows/spec-genres/design-docs.md` — the contract/lore split
  in practice: what spills out of a contract, what never leaves, the
  fork-by-fork record skeleton, the orthogonal-decomposition lesson,
  and how a design doc grows stale honestly.
- `spec/flows/spec-genres/when-to-write-what.md` — the routing table
  (situation → genre), the misfiling-smells table, and one worked
  example of the linking law with both ends wired.
- `spec/boot/17-flow-spec-genres.md` — boot snippet: the genre map,
  the name-the-genre-first rule, and the never-do list.

## Install {#install}

```bash
vibe install flow:spec-genres
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:spec-genres
```

Uninstalling removes every file the package wrote, including the boot
snippet. User-owned files are never touched.

## Composition {#composition}

- `flow:addressable-specs` — the spec tree layout and stable anchors
  live there; genres classify *what* the tree holds, that package
  fixes *how* it is addressed.
- `flow:decision-records` — a decision record lives at the contract
  anchor; a design doc is where its long-form story goes. This
  package draws the line between the two.
- `flow:comparative-research` — the research genre has its own
  package; this taxonomy points at it rather than duplicating it.
- `flow:campaign-plans` — the campaign-plan genre has its own
  package; here it is one row in the genre table (execution, not
  truth).

## Philosophical background {#background}

The genre model is crystallized from the origin project's own
design-doc genre law: the load-bearing rationale stays inside each
contract, the narrative rationale — the lore — moves into a linked
design doc, and the link is the mechanism that makes the lore survive
a cold start. The collection's spirit is the book *AI-native
development* (in Russian inside `flow:redbook` at `spec/book/ru/`).
Short version: an agent reads the tree cold every session, so a
sentence's genre — and therefore its authority — must be legible
without asking anyone.

## License {#license}

UPL-1.0. See [`LICENSE.md`](LICENSE.md).
