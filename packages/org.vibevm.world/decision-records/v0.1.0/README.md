# `flow:decision-records` — record why, not just what {#root}

A `flow` package that installs the **decisions, not facts**
discipline into a project. A fact ("timeout is 600 s") is
recoverable from the code in a second; the *reason* cannot be
recovered at all. So every choice a future reader could plausibly
re-open gets a four-field record — Decision / Why / Considered and
rejected / When to revisit — at the spec anchor that governs the
value.

In a pure-human team, unrecorded reasoning limps along as tribal
knowledge — someone can still ask Vasya why the library was chosen.
The agent cannot ask Vasya. It re-derives from the code, the code
shows the value but not the constraint, and the decision gets
re-litigated: "600 s looks arbitrary, propose 300 s for
performance." A recorded decision is immunity from re-litigation;
the revisit trigger keeps the immunity from hardening into dogma.

There is deliberately no `adr/` directory and no immutable numbered
log: the governing spec section IS the record, evolution is an edit
plus a changelog line, and history lives in git.

This package ships three pieces of content plus a boot snippet:

- `spec/flows/decision-records/DECISION-RECORDS-PROTOCOL.md` — full
  protocol: facts versus decisions, why the asymmetry is load-bearing
  in a human-AI team, the four-field record, placement at the
  governing anchor (contrasted with classic ADR), when to write a
  record, the rejected-alternatives and invariants section patterns,
  and a re-derive prompt for adapting the practice to any project.
- `spec/flows/decision-records/record-template.md` — the copy-ready
  template, two fully worked examples (a timeout constant, a library
  choice), and the anti-pattern table.
- `spec/flows/decision-records/revisit-triggers.md` — trigger anatomy
  (metric + threshold + observation point), good/bad trigger table,
  the periodic sweep that actually fires triggers, and the reopening
  procedure.
- `spec/boot/25-flow-decision-records.md` — boot snippet loaded at
  session start: the core rule, the in-session recording duty, and
  the never-do list.

## Install {#install}

```bash
vibe install flow:decision-records
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:decision-records
```

Uninstalling removes every file the package wrote, including the boot
snippet. User-owned files are never touched.

## Composition {#composition}

- `flow:addressable-specs` — records live at anchors; the stable
  `spec://…#anchor` address is what makes a record citable from code
  comments, commits, and the session journal.
- `flow:sync-from-code` — its spec delta (value + reason + revisit
  trigger) is precisely a decision record born from a code-first
  change; this package generalises that trio to *all* decisions,
  whichever direction they arrive from.
- `flow:wal` — WAL Constraints entries cite recorded decisions
  ("timeout is 600 s, not 300 → §verification.timeout") instead of
  restating the reasoning.
- `flow:atomic-commits` — commit bodies cite the record, never
  replace it: the spec carries the why, the commit points at the
  anchor.

## Philosophical background {#background}

The practice is extracted from *AI-native development*, chapter 3
(*«Архитектура памяти»*, subsection *«Решения, а не факты»*) and the
memory-architecture conclusions of chapter 1. The book ships in
Russian inside `flow:redbook` at `spec/book/ru/`. Short version: the
agent has no memory between sessions — a recorded decision is the
only kind of memory it can ever have.

## License {#license}

UPL-1.0. See [`LICENSE.md`](LICENSE.md).
