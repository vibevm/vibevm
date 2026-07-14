# Design rationale: Loading & boot composition model

**Companion to:** PROP-009 (loading model — forthcoming, [`spec/modules/vibe-workspace/PROP-009-loading-model.md`](../modules/vibe-workspace/PROP-009-loading-model.md)).
**Status:** non-normative design record. Captured 2026-05-21 in an owner design session.
**Authority:** the PROP is the contract. If this document and PROP-009 disagree, the PROP wins.

---

## 1. What this document is

PROP-007 shipped the workspace data model (M1.17) but deliberately left one question
open — [PROP-007 §6 question 3](../modules/vibe-workspace/PROP-007-workspace.md#open),
the *per-member materialisation target*: when a dependency is resolved for member M,
into which member's `spec/` does its content land?

This session answered it. The answer turned out to be far larger than picking a
directory: it is a redesign of vibevm's entire **loading model** — how a dependency's
content is materialised, how the boot sequence is composed across a workspace
hierarchy, and how an AI agent consumes it at session start. This document records
the *why* and the fork-by-fork reasoning; **PROP-009** will be the contract.

It belongs to the workspace arc and continues
[`workspace-and-qualified-naming.md`](workspace-and-qualified-naming.md).

---

## 2. The problem the deferred question was hiding

PROP-007 §6 q3 reads as a narrow placement question. It is not.

vibevm's boot model ([`VIBEVM-SPEC.md` §6](../../VIBEVM-SPEC.md)) is a **flat, single,
shared, mutable namespace** — `spec/boot/NN-*.md`, read in filename order, one
sequence, one entry point. It is correct for exactly one project shape: one project →
one boot sequence → one entry point.

A workspace breaks every one of those assumptions at once:

- **N nodes** — the root plus every member.
- **N entry points** — a developer `cd`s into any member and opens an agent there.
  This is PROP-007's load-bearing principle: "the user works in a sub-project and
  doesn't notice it is part of something bigger."
- **N boot sequences** — each entry point needs a sequence coherent *for that node*.
- **One shared dependency set** — unified resolution (PROP-007 §2.4) means one version
  of each external dependency across the whole workspace.

The flat model cannot be stretched over this. It must be replaced.

The owner's hard constraint, stated at the outset: **installing a dependency must
never modify anyone's authored spec.** The owner's analogy is C++ — you do not rewrite
your `#include` directives into the literal text of the dependency's headers. Merging
a dependency's spec into the consuming spec is the broken option, and it is what a
naïve "bubble everything into the root `spec/boot/`" would amount to.

---

## 3. The owner's mental model — boot as linking

The owner framed the loading mechanism directly as **static vs dynamic linking**, and
that framing became the spine of the design.

| C++ / the linker | vibevm |
|---|---|
| Compile + link time | `vibe install` |
| Load + run time | start of an AI session |
| Object file / library | a package (flow / feat / stack / tool) |
| The executable | a node's effective boot sequence |
| The dynamic loader (`ld.so`) | the AI agent following INCLUDE pointers |
| `DT_NEEDED` entry | an INCLUDE pointer (a `spec://` reference) |
| Static linking (`.a` copied into the binary) | `vibe install` inlines a dependency's boot |
| Dynamic linking (`.so` referenced, resolved at load) | the INCLUDE pointer stays; the agent resolves it |
| `vendor/` / `~/.cargo/registry` | the materialised-dependency tree |

The load-bearing consequence: **at session start, the AI agent is the loader.** Every
reference the agent resolves itself costs tool calls, and tool calls are — the owner's
words — *безумно дорого*. Static linking moves resolution to `vibe install` time (done
once, by the machine); dynamic linking leaves it to the agent (every session, by the
LLM). Exactly the C++ tradeoff: a statically-linked binary loads with no help; a
dynamically-linked one needs the loader to find and map every `.so`.

---

## 4. The model — four principles

**P1 — Two trees.** A node's authored `spec/` (only the node's author writes it) and
the materialised dependencies (a separate tree, only `vibe` writes it) are physically
separate and never intermixed. Installing a dependency never touches authored `spec/`.
This is "your code vs `vendor/`". Under unified resolution a dependency is materialised
**once** for the whole workspace.

**P2 — The boot sequence is computed, not assembled by hand.** Each node has an
*effective boot sequence* = inherited foundation (from ancestors, flowing down) + the
node's own authored boot + the boot of its transitive dependencies (flowing up) + user
overrides. `vibe` computes it from the unified resolution. This is the owner's
"matryoshka" — but computed directly per level from the resolution graph, not
physically copied leaf-to-root (copying drifts; computation does not). Every level is
self-contained: a session opened at any node gets a sequence coherent for that node's
subtree. The root's sequence is the union of everything; a small member's is small —
the hierarchy gives cost-scoping for free.

**P3 — One generated index per entry point; the agent never walks the graph.**
`vibe install` generates, for each entry-point node, the boot artifacts (§6). The
agent reads them in a flat, predictable loop — reads parallelise, collapsing latency.
No recursion, no discovery, no cycle-detection on the agent's side: `vibe` did that
once. Boot stays **pure file-reading** — the `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`
redirect points at generated files; it does *not* become "run `vibe`". This preserves
the zero-dependency cross-agent property that is the whole point of VIBEVM-SPEC §6.1.

**P4 — Three inclusion types: `inline`, `static`, `dynamic`.** Declared per dependency
in the consumer's `vibe.toml`; default `static`. They are the points on the linker
spectrum (§6).

---

## 5. The fork-by-fork record

Four forks were put to the owner. The options, the resolution, the reasoning.

### Fork 1 — the form of static boot delivery

Options offered: a single inlined aggregate (cheapest at boot, but duplication and
the lost-in-the-middle risk §6.4 already rejected); a resolved path index (clean, but
1 + N reads); a hybrid.

**Resolution — a refined hybrid.** Not one of the three: a model where `inline`,
`static`, and `dynamic` *all coexist* as per-dependency inclusion types (§6), plus a
dedicated generated `STATIC.md`. The owner's reasoning: `STATIC.md` is an *emergency
priority lane*. For the highest-importance content — top-level skills, critical
disciplines — the boot text is concatenated verbatim into one file read first, so its
priority is guaranteed by *position* and does not depend on the agent performing
resolution correctly. This is the honest answer to lost-in-the-middle (§6.5): you do
not fix attention degradation by being clever about file counts; you fix it by putting
the must-not-be-missed content physically first.

### Fork 2 — where materialised dependencies live

**Resolution — a committed dependency tree.** A fresh clone is bootable with no
`vibe install`; the dependencies are visible and diffable; it is consistent with the
spec-driven ethos — the spec corpus *is* the product. Plus: a `vibe` command to
regenerate a subtree of the materialised state on demand — for when dependencies are
believed stale or a previous index pass was wrong.

### Fork 3 — uniform vs workspace-only

**Resolution — uniform.** A single-package project is a degenerate (zero-member)
workspace; one loading model everywhere. Consistent with `Workspace::discover`, which
already degenerates cleanly to "just this one node". The cost — every existing
project's layout migrates — is acceptable: vibevm is pre-release, and M1.17's
no-legacy hard break already set the precedent.

### Fork 4 — milestone scope

**Resolution — boot + effective spec, unified.** The boot index and the `vibe build`
effective spec (VIBEVM-SPEC §4.6) are the same idea: a computed, layered, materialised
view of the workspace. PROP-009 specifies one *computed-view engine*; the boot index
("what to read at session start") and the effective spec ("the full merged corpus for
build") are two views it emits.

---

## 6. The three inclusion types — the refined Fork-1 answer in detail

Each dependency in a consumer's `vibe.toml` carries an inclusion type (working syntax:
`link = "static" | "static" | "dynamic"`); the default is `static`. At `vibe install`
time, for each entry-point node `vibe` generates:

- **`STATIC.md`** — the verbatim concatenation of every `static`-typed contribution in
  the node's effective boot, in priority order. Read first; one read; maximum attention
  weight. The emergency lane — used sparingly, for top-level skills and critical
  disciplines. Generated only when the node has inline contributions.
- **`INDEX.md`** — the ordered, resolved manifest of the rest of the boot sequence.
  `static` entries appear as resolved file paths the agent reads directly (a flat,
  parallelisable loop). `dynamic` entries appear as INCLUDE pointers the agent resolves
  at boot.
- Session-start order: `CLAUDE.md` → `STATIC.md` → `INDEX.md` and the files it names.

Cost profile:

| Type | Reads at boot | Content on disk | Use |
|---|---|---|---|
| `inline` | ~1 (already in `STATIC.md`) | duplicated (bounded — few items) | critical disciplines, top-level skills |
| `static` | 1 + N (N parallelisable) | lives once | the default — ordinary dependencies |
| `dynamic` | 1 + N + graph-walk | lives once | conditional / context-gated boot |

`dynamic` is, mechanically, the subskill `lazy-pull` delivery mode (PROP-003 §2.5) —
the loading model generalises subskill delivery rather than inventing a parallel axis.

---

## 7. Consequences and findings

- **Numbering.** The `NN-` prefix namespace (10–89, author-chosen) cannot survive a
  workspace and is already admitted provisional (§6.5). In the computed model `vibe`
  owns the order in the generated artifacts; a package author declares only a
  *category* (the existing range bands — foundation / flow / stack / user-override —
  become categories) and optionally a coarse early/late hint. Prefix collisions become
  impossible by construction.
- **Mirror-layout breakage.** VIBEVM-SPEC §13.1's mirror layout — source path = target
  path — works today only because a dependency always lands at the same path in every
  project. Moving dependency content into a separate tree breaks that; a package's
  internal cross-references (a boot snippet pointing at its own protocol document) must
  become package-relative or `spec://` URIs, rewritten by `vibe` at materialisation.
- **Published-copy regeneration.** A package published by `vibe workspace publish` is
  consumed standalone — its boot index must be regenerated for the published shape,
  where dependencies are registry-resolved and version-pinned rather than path-sourced.
  This is exactly PROP-007 §2.5's dual-form `{ path, version }`; publish staging gains
  an index-regeneration step.
- **vibevm dogfoods itself.** The vibevm repository is itself a vibevm project;
  PROP-009 changes how *this* repository boots. `spec/boot/00-core.md` and
  `spec/boot/90-user.md` stay user-owned authored boot; the generated `STATIC.md` /
  `INDEX.md` join them. The migration is part of the milestone.

---

## 8. What this supersedes and parks

- **Workspace-aware `vibe install` / `vibe build`** (PROP-007 §9.3, §6 q3) is no longer
  a separate deferred item — it is *subsumed*: it becomes the install/build half of
  PROP-009.
- **`version = { workspace = true }`** (PROP-007 §6 q4) and the **publish-signalling
  polish** (`--archive` etc., PROP-007 §9.3) are parked behind PROP-009 — recorded, not
  dropped.
- **PROP-008** (qualified naming) is unaffected; it still follows PROP-005 (index).

---

## 9. Session log

- **2026-05-21.** Session restored from `CONTINUE.md` + `spec/WAL.md`. The owner
  reopened PROP-007 §6 q3 (the materialisation target). The discussion established
  that the question is a loading-model redesign, not a directory choice; produced the
  linker-model spine (§3), the four principles (§4), and the four-fork resolution
  recorded in §5. PROP-009 and its implementation milestone (M1.18) deferred to the
  contract-writing step. Out of band, a standing test-environment misdiagnosis was
  corrected — `os error 740` on `cargo test -p vibe-install` is Windows UAC installer
  detection (the test binary's name contains "install"), not Windows Defender; see the
  WAL.

---

## 10. Pointers

- PROP-009 (forthcoming) — [`spec/modules/vibe-workspace/PROP-009-loading-model.md`](../modules/vibe-workspace/PROP-009-loading-model.md) — the contract.
- [PROP-007](../modules/vibe-workspace/PROP-007-workspace.md) — the workspace data model; §6 q3 is the question this answers.
- [`workspace-and-qualified-naming.md`](workspace-and-qualified-naming.md) — the preceding design session.
- [`VIBEVM-SPEC.md`](../../VIBEVM-SPEC.md) — §6 (boot directory model), §4.2 (layout), §4.6 (effective spec), §13.1 (mirror layout).
- [PROP-003 §2.5](../modules/vibe-resolver/PROP-003-dep-evolution.md) — subskills and delivery modes.
