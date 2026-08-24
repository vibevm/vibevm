# `flow:comparative-research` — study the competition, evergreen {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-COMPARATIVE-RESEARCH-GENRE A `flow` package that installs the **comparative research** genre
into a project. @status:impl/done

@fact:THE-OUTPUT-IS-A-SELF-CONTAINED-EVERGREEN-DOCUMENT When you study an external system — a competitor, a
predecessor, an adjacent tool — the output is a self-contained,
evergreen document: readable months later without the original
sources, structured as a two-way gap analysis, closing with numbered
roadmap deltas that the study *proposes but never ratifies*. @status:impl/done

@fact:the-genre-exists-because-the-alternatives-fail The genre exists because the cheap alternatives fail. @status:spec/done

@fact:a-bookmark-rots-when-the-url-moves A bookmark
rots when the URL moves. @status:spec/done

@fact:a-verbal-comparison-evaporates-and-gets-re-argued A quick verbal "they do X, we should too"
evaporates and gets re-argued next quarter. @status:spec/done

@fact:a-one-directional-rant-is-marketing-for-them A one-directional rant
about a rival's shiny feature is marketing you wrote for them. @status:spec/done

@fact:A-REAL-STUDY-QUOTES-MEASURES-AND-PROPOSES A
real study quotes the subject verbatim with dates, measures it in
*both* directions, and turns the actionable trailing gaps into
numbered, prioritized, homed proposals a human can weigh. @status:impl/done

@fact:package-contents-lead This package ships three pieces of content plus a boot snippet: @status:impl/done

- @fact:CONTENT-THE-GENRE-LAW `spec/flows/comparative-research/COMPARATIVE-RESEARCH-PROTOCOL.xml`
  — the genre law: what a comparative study is, why it exists, the
  five laws (self-containedness, quote-first, two-way gaps,
  deltas-not-decrees, the re-fetch list), when to write one, and a
  re-derive prompt. @status:impl/done
- @fact:CONTENT-THE-RESEARCH-TEMPLATE `spec/flows/comparative-research/research-template.xml` — a
  copy-ready skeleton (purpose, source table, reading shape, the
  subject in its own words, inventory, trail, lead, numbered deltas,
  open questions, re-fetch list), clause-by-clause commentary, and a
  short worked fragment. @status:impl/done
- @fact:CONTENT-THE-ROADMAP-PIPELINE `spec/flows/comparative-research/from-research-to-roadmap.xml` — the
  downstream pipeline: delta → owner review → accepted deltas become
  recorded decisions, rejected deltas stay archived with their
  reason, plus refresh discipline and the honesty rule. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/52-flow-comparative-research.xml` — boot snippet: the
  genre exists, reach for the template on request, hold the laws. @status:impl/done

## Install {#install}

```bash
vibe install flow:comparative-research
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:comparative-research
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @status:impl/done

## Composition {#composition}

- @fact:COMPOSES-DISCOVERY-PROMPT `flow:discovery-prompt` — a discovery session is *how* the raw
  study conversation runs; this genre is *where* its output
  crystallizes into a durable, evergreen document. @status:impl/done
- @fact:COMPOSES-DECISION-RECORDS `flow:decision-records` — an accepted delta lands as a recorded
  decision with a revisit trigger at its target anchor; the study
  points at the record. @status:impl/done
- @fact:COMPOSES-SPEC-GENRES `flow:spec-genres` — this is the research genre's own package; the
  full genre map (research alongside PROP, FEAT, WAL, and the rest)
  lives there. @status:impl/done

## Philosophical background {#background}

@fact:crystallized-from-the-origin-projects-research-doc-practice The genre is crystallized from the origin project's research-doc
practice — evergreen backgrounders that outlived their sources and
fed a research → roadmap-delta → contract pipeline. @status:spec/done

@fact:collections-spirit-is-the-redbook The collection's
spirit is the book *AI-native development*, which ships in Russian
inside `flow:redbook` at `spec/book/ru/`. @status:spec/done

@fact:short-version-a-studied-competitor-is-intelligence Short version: a competitor
you have not studied is a roadmap mistake waiting to happen; a
competitor you have studied — in both directions — is intelligence. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See [`LICENSE.md`](LICENSE.md). @status:impl/done

