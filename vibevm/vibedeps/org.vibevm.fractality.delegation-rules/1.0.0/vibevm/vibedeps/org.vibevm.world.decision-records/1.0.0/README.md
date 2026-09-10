# `flow:decision-records` — record why, not just what {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-DECISIONS-NOT-FACTS-DISCIPLINE A `flow` package that installs the **decisions, not facts**
discipline into a project. @status:impl/done

@fact:FACT-IS-RECOVERABLE-REASON-IS-NOT A fact ("timeout is 600 s") is
recoverable from the code in a second; the *reason* cannot be
recovered at all. @status:spec/done

@fact:FOUR-FIELD-RECORD-AT-THE-GOVERNING-ANCHOR So every choice a future reader could plausibly
re-open gets a four-field record — Decision / Why / Considered and
rejected / When to revisit — at the spec anchor that governs the
value. @status:impl/done

@fact:tribal-knowledge-in-a-pure-human-team In a pure-human team, unrecorded reasoning limps along as tribal
knowledge — someone can still ask Vasya why the library was chosen. @status:spec/done

@fact:AGENT-CANNOT-ASK-VASYA The agent cannot ask Vasya. @status:spec/done

@fact:re-derivation-ends-in-re-litigation It re-derives from the code, the code
shows the value but not the constraint, and the decision gets
re-litigated: "600 s looks arbitrary, propose 300 s for
performance." @status:spec/done

@fact:RECORD-IS-IMMUNITY-TRIGGER-KEEPS-IT-HONEST A recorded decision is immunity from re-litigation;
the revisit trigger keeps the immunity from hardening into dogma. @status:spec/done

@fact:NO-ADR-DIRECTORY-NO-NUMBERED-LOG There is deliberately no `adr/` directory and no immutable numbered
log: the governing spec section IS the record, evolution is an edit
plus a changelog line, and history lives in git. @status:impl/done

@fact:package-contents-lead This package ships three pieces of content plus a boot snippet: @status:impl/done

- @fact:CONTENT-THE-PROTOCOL `spec/flows/decision-records/DECISION-RECORDS-PROTOCOL.xml` — full
  protocol: facts versus decisions, why the asymmetry is load-bearing
  in a human-AI team, the four-field record, placement at the
  governing anchor (contrasted with classic ADR), when to write a
  record, the rejected-alternatives and invariants section patterns,
  and a re-derive prompt for adapting the practice to any project. @status:impl/done
- @fact:CONTENT-THE-RECORD-TEMPLATE `spec/flows/decision-records/record-template.xml` — the copy-ready
  template, two fully worked examples (a timeout constant, a library
  choice), and the anti-pattern table. @status:impl/done
- @fact:CONTENT-THE-REVISIT-TRIGGERS `spec/flows/decision-records/revisit-triggers.xml` — trigger anatomy
  (metric + threshold + observation point), good/bad trigger table,
  the periodic sweep that actually fires triggers, and the reopening
  procedure. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/25-flow-decision-records.xml` — boot snippet loaded at
  session start: the core rule, the in-session recording duty, and
  the never-do list. @status:impl/done

## Install {#install}

```bash
vibe install flow:decision-records
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:decision-records
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @status:impl/done

## Composition {#composition}

- @fact:COMPOSES-ADDRESSABLE-SPECS `flow:addressable-specs` — records live at anchors; the stable
  `spec://…#anchor` address is what makes a record citable from code
  comments, commits, and the session journal. @status:impl/done
- @fact:COMPOSES-SYNC-FROM-CODE `flow:sync-from-code` — its spec delta (value + reason + revisit
  trigger) is precisely a decision record born from a code-first
  change; this package generalises that trio to *all* decisions,
  whichever direction they arrive from. @status:impl/done
- @fact:COMPOSES-WAL `flow:wal` — WAL Constraints entries cite recorded decisions
  ("timeout is 600 s, not 300 → §verification.timeout") instead of
  restating the reasoning. @status:impl/done
- @fact:COMPOSES-ATOMIC-COMMITS `flow:git-atomic-commits` — commit bodies cite the record, never
  replace it: the spec carries the why, the commit points at the
  anchor. @status:impl/done

## Philosophical background {#background}

@fact:practice-extracted-from-the-book The practice is extracted from *AI-native development*, chapter 3
(*«Архитектура памяти»*, subsection *«Решения, а не факты»*) and the
memory-architecture conclusions of chapter 1. @status:spec/done

@fact:BOOK-SHIPS-IN-RUSSIAN-INSIDE-REDBOOK The book ships in
Russian inside `flow:redbook` at `spec/book/ru/`. @status:impl/done

@fact:AGENT-HAS-NO-MEMORY-BETWEEN-SESSIONS Short version: the
agent has no memory between sessions — a recorded decision is the
only kind of memory it can ever have. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See [`LICENSE.md`](LICENSE.md). @status:impl/done

