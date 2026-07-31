# `flow:spec-genres` — what goes where, and who wins {#root}

<status stage="doc" state="done" audience="user"/>

##PACKAGE-INSTALLS-A-GENRE-TAXONOMY A `flow` package that installs a **genre taxonomy** for a project's
documents. @impl/done

##pile-rots-three-ways One undifferentiated pile of markdown rots three ways: @spec/done

- ##ROT-CONTRACTS-BLOAT-WITH-NARRATIVE contracts bloat with narrative, @spec/done
- ##ROT-NARRATIVE-GETS-TREATED-AS-BINDING narrative gets treated as binding, @spec/done
- ##ROT-NOBODY-KNOWS-WHAT-WINS and nobody knows what wins. @spec/done

##PACKAGE-SORTS-FIXES-AND-PINS-THE-TWO-LAWS This package sorts every document into a
genre, fixes what each genre may do, and pins the two laws that keep
the pile honest — contract wins over lore, and the two-way link that
lets a cold reader find the lore behind a contract. @impl/done

##genres-lead The genres: @impl/done

- ##GENRE-GROUP-BINDING binding (boot files, foundational decisions, module
  contracts), @impl/done
- ##GENRE-GROUP-NON-BINDING non-binding (design docs, research docs), @impl/done
- ##GENRE-GROUP-VOLATILE and volatile
  (campaign plans, the checkpoint). @impl/done

##CONVENTION-NAMES-MODULE-CONTRACTS-PROP-FEAT The convention here names module
contracts PROP / FEAT — that is a naming choice this collection
carries from its origin, and you can rename it for your project; the
taxonomy is what matters, not the labels. @spec/done

##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done

- ##CONTENT-THE-PROTOCOL `spec/flows/spec-genres/SPEC-GENRES-PROTOCOL.md` — the taxonomy:
  why genres exist, the full genre table (charter, mutability,
  reader, authority), the precedence law, the two-way linking law,
  and a re-derive prompt for adapting the map to any project. @impl/done
- ##CONTENT-THE-DESIGN-DOCS-GUIDE `spec/flows/spec-genres/design-docs.md` — the contract/lore split
  in practice: what spills out of a contract, what never leaves, the
  fork-by-fork record skeleton, the orthogonal-decomposition lesson,
  and how a design doc grows stale honestly. @impl/done
- ##CONTENT-THE-ROUTING-GUIDE `spec/flows/spec-genres/when-to-write-what.md` — the routing table
  (situation → genre), the misfiling-smells table, and one worked
  example of the linking law with both ends wired. @impl/done
- ##CONTENT-THE-BOOT-SNIPPET `spec/boot/17-flow-spec-genres.md` — boot snippet: the genre map,
  the name-the-genre-first rule, and the never-do list. @impl/done

## Install {#install}

```bash
vibe install flow:spec-genres
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:spec-genres
```

##UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @impl/done

##USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @impl/done

## Composition {#composition}

- ##COMPOSES-ADDRESSABLE-SPECS `flow:addressable-specs` — the spec tree layout and stable anchors
  live there; genres classify *what* the tree holds, that package
  fixes *how* it is addressed. @impl/done
- ##COMPOSES-DECISION-RECORDS `flow:decision-records` — a decision record lives at the contract
  anchor; a design doc is where its long-form story goes. This
  package draws the line between the two. @impl/done
- ##COMPOSES-COMPARATIVE-RESEARCH `flow:comparative-research` — the research genre has its own
  package; this taxonomy points at it rather than duplicating it. @impl/done
- ##COMPOSES-CAMPAIGN-PLANS `flow:campaign-plans` — the campaign-plan genre has its own
  package; here it is one row in the genre table (execution, not
  truth). @impl/done

## Philosophical background {#background}

##genre-model-crystallized-from-the-origin-project The genre model is crystallized from the origin project's own
design-doc genre law: the load-bearing rationale stays inside each
contract, the narrative rationale — the lore — moves into a linked
design doc, and the link is the mechanism that makes the lore survive
a cold start. @spec/done

##collection-spirit-is-the-book The collection's spirit is the book *AI-native
development* (in Russian inside `flow:redbook` at `spec/book/ru/`). @spec/done

##AGENT-READS-THE-TREE-COLD-EVERY-SESSION Short version: an agent reads the tree cold every session, so a
sentence's genre — and therefore its authority — must be legible
without asking anyone. @spec/done

## License {#license}

##license-line UPL-1.0. See [`LICENSE.md`](LICENSE.md). @impl/done
