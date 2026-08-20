# The book {#root}

This directory carries the source text of *AI-native development* —
the book the redbook collection distills. The collection takes the
**general spirit of the process** from it; the member flows are its
practices in installable form.

## Editions {#editions}

- `ru/` — the Russian text, included **as-is** from the author's
  manuscript. Currently the only edition, and therefore the
  authoritative one.
- `en/` — reserved. An English edition will sit alongside the
  Russian one; once it exists, the English text takes priority and
  the Russian edition remains as a translation. Until then, the
  Russian text is the reference.

## Contents {#contents}

- `ru/chapter-1-two-process-model.md` — Два процесса, одна задача:
  the coprocessor model, the cognitive load split, shared memory.
- `ru/chapter-2-shared-state-and-files.md` — Shared state: файлы как
  IPC: addressability, atomicity, the conflict protocol, the WAL
  pattern.
- `ru/chapter-3-memory-individual.md` — Архитектура памяти: the
  memory hierarchy, decisions-not-facts, the working day.

The chapters are reference depth, not standing instructions: open
them when a *why* question arises. The day-to-day rules live in the
member flows' boot snippets and protocol documents.

Chapter texts are verbatim; internal references to material outside
these chapters (e.g. the Safe Harbor note) resolve at the book's own
public home, not inside this package.
