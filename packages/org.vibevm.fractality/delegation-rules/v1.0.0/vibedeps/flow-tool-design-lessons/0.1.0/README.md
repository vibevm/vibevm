# `flow:tool-design-lessons` — lessons for tool authors {#root}

A `flow` package that installs a **lessons catalog** for tool authors:
numbered, self-contained lessons distilled from building a self-updating
CLI and the package ecosystem around it. Each lesson is a scar — a
design that seemed reasonable, shipped, and taught its cost — captured
as **context + the law + why**, so you can read the one that governs the
decision in front of you and skip the rest.

**Audience: tool authors** — anyone building a self-updating CLI, an
installer, a version manager, or a package system. This is a lessons
catalog, not a protocol: there is no single procedure to follow, just
fourteen laws with the failures that earned them and three maxims that
sit above them.

This package ships four pieces of content plus a boot snippet:

- `spec/flows/tool-design-lessons/TOOL-DESIGN-LESSONS.md` — the catalog:
  what the lessons are, the index table (lesson id → one-line law →
  where it lives), the three cross-cutting maxims, the meta-lesson
  (record the *why*, not just the *what*), and a re-derive prompt for
  adapting the laws to your own platform.
- `spec/flows/tool-design-lessons/self-updating-tools.md` — lessons
  S1–S7: live-pointer activation, immutable instance directories, cheap
  identity, sources by reference, durable-environment edits, one
  runnable required-tools table, and safe removal / garbage collection.
- `spec/flows/tool-design-lessons/packaging-lessons.md` — lessons P1–P7:
  a package is a project, ship runtime not prose, identity is the
  source, build output goes elsewhere, vendor the bootstrap, spike
  before the irreversible move, and build the general mechanism on real
  demand.
- `spec/boot/70-flow-tool-design-lessons.md` — boot snippet loaded at
  session start: when to read a lesson first, and the never-do list.

## Install {#install}

```bash
vibe install flow:tool-design-lessons
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:tool-design-lessons
```

Uninstalling removes every file the package wrote, including the boot
snippet. User-owned files are never touched.

## Composition {#composition}

- `flow:managed-blocks` — the deep dive on writing safely inside a
  shared, human-owned file; lesson S5 (durable-environment edits) points
  there for the mechanics of the marked-block edit.
- `flow:qualified-naming` — the sibling on ecosystem identity; when a
  package is a project (P1) and many of them coexist, stable qualified
  names are what keep them addressable.
- `flow:manual-tests` — several of these lessons were caught only by a
  human-run walk on a real machine, not by an automated suite; the two
  test tiers reinforce each other.
- `flow:decision-records` — every lesson here is a decision record that
  earned its revisit trigger the hard way; that flow generalises the
  record-the-*why* discipline to all decisions, whichever direction they
  arrive from.

## Philosophical background {#background}

The catalog is crystallized from the origin project's version-manager
rework (the switch from environment-as-truth to a live pointer, immutable
instances, and cheap identity) and its code-bearing-packages laws (a
package is a project, identity is the source, vendor the bootstrap). The
collection's spirit is the book *AI-native development*, which ships in
Russian inside `flow:redbook` at `spec/book/ru/`. Short version: a tool
that manages itself edits a live machine, and every one of these laws is
a place where the obvious design quietly cost more than the careful one.

## License {#license}

UPL-1.0. See [`LICENSE.md`](LICENSE.md).
