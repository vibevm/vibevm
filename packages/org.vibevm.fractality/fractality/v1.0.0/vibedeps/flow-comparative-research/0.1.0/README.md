# `flow:comparative-research` — study the competition, evergreen {#root}

A `flow` package that installs the **comparative research** genre
into a project. When you study an external system — a competitor, a
predecessor, an adjacent tool — the output is a self-contained,
evergreen document: readable months later without the original
sources, structured as a two-way gap analysis, closing with numbered
roadmap deltas that the study *proposes but never ratifies*.

The genre exists because the cheap alternatives fail. A bookmark
rots when the URL moves. A quick verbal "they do X, we should too"
evaporates and gets re-argued next quarter. A one-directional rant
about a rival's shiny feature is marketing you wrote for them. A
real study quotes the subject verbatim with dates, measures it in
*both* directions, and turns the actionable trailing gaps into
numbered, prioritized, homed proposals a human can weigh.

This package ships three pieces of content plus a boot snippet:

- `spec/flows/comparative-research/COMPARATIVE-RESEARCH-PROTOCOL.md`
  — the genre law: what a comparative study is, why it exists, the
  five laws (self-containedness, quote-first, two-way gaps,
  deltas-not-decrees, the re-fetch list), when to write one, and a
  re-derive prompt.
- `spec/flows/comparative-research/research-template.md` — a
  copy-ready skeleton (purpose, source table, reading shape, the
  subject in its own words, inventory, trail, lead, numbered deltas,
  open questions, re-fetch list), clause-by-clause commentary, and a
  short worked fragment.
- `spec/flows/comparative-research/from-research-to-roadmap.md` — the
  downstream pipeline: delta → owner review → accepted deltas become
  recorded decisions, rejected deltas stay archived with their
  reason, plus refresh discipline and the honesty rule.
- `spec/boot/52-flow-comparative-research.md` — boot snippet: the
  genre exists, reach for the template on request, hold the laws.

## Install {#install}

```bash
vibe install flow:comparative-research
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:comparative-research
```

Uninstalling removes every file the package wrote, including the boot
snippet. User-owned files are never touched.

## Composition {#composition}

- `flow:discovery-prompt` — a discovery session is *how* the raw
  study conversation runs; this genre is *where* its output
  crystallizes into a durable, evergreen document.
- `flow:decision-records` — an accepted delta lands as a recorded
  decision with a revisit trigger at its target anchor; the study
  points at the record.
- `flow:spec-genres` — this is the research genre's own package; the
  full genre map (research alongside PROP, FEAT, WAL, and the rest)
  lives there.

## Philosophical background {#background}

The genre is crystallized from the origin project's research-doc
practice — evergreen backgrounders that outlived their sources and
fed a research → roadmap-delta → contract pipeline. The collection's
spirit is the book *AI-native development*, which ships in Russian
inside `flow:redbook` at `spec/book/ru/`. Short version: a competitor
you have not studied is a roadmap mistake waiting to happen; a
competitor you have studied — in both directions — is intelligence.

## License {#license}

UPL-1.0. See [`LICENSE.md`](LICENSE.md).
