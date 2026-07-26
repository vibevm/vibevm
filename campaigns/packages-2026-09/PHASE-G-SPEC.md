# Phase G — Documentation as a package that cites and never copies {#root}

<status stage="spec" state="plan" comment="drafted 2026-07-26 from the owner's directive; not ratified"/>

**Placement:** after Phase F (the credibility report). Wave 1 deferred its own
Phase G for want of inputs; this is that work, re-specified.
**No letter collision:** this campaign's Phases are A–F, **T**, and **G**. Wave
1's Phase F (the judgment-marking pass) is named by its work everywhere in this
campaign, never by its letter — see `deferrals.md#inherited`.

---

## 0. The owner's directive, 2026-07-26 {#directive}

> Перед началом написания документации, перенести всю существующую документацию
> в `docs-legacy`. Сделать отдельный пакет с документацией (внутри `packages`):
> `org.vibevm.doc/doc`. Внутри документации оставлять ссылки на настоящие
> спецификации, откуда взялась информация. Связи однонаправленные, документация
> ссылается на спеки, спеки о документации не знают. В дальнейшем сделаем
> `org.vibevm.doc/web` … можно зарезервировать пакет под сайт сразу, стек —
> node.js, quick.dev (v2), tailwind, typescript. Сайт пока не разрабатывать.

## 1. What exists today, measured {#current}

| | |
|---|---|
| `docs/` | **43** markdown files — `architecture.md`, `authoring-{flow,feat,stack}.md`, `commands/**` |
| root | `README.md`, `DEV-GUIDE.md` |
| observed by the campaign | **none of it** |

That last row is the finding hiding in this phase's premise. `progress.toml`
includes `spec/**` and two package namespaces; **`docs/` is in no include glob**,
so 43 files of user-facing documentation have never been marked, never been
verified, and are absent from every count this campaign has produced. They are
not stale by measurement — they are **unmeasured**, which is the weaker claim
and the more uncomfortable one.

Existing groups under `packages/`: `org.vibevm.ai-native`, `org.vibevm.world`,
`org.vibevm.fractality`, `org.vibevm.vibeapp`. `org.vibevm.doc` is the fifth.

## 2. Step 1 — the archive move {#archive}

`docs/` → **`docs-legacy/`**, wholesale, in one commit that moves and changes
nothing else.

The precedent is `legacy-spec/`, and it carries its rule with it: **nothing in
the living corpus may cite into `docs-legacy/` as a normative source** —
archive-provenance pointers only. It is history kept readable, not a place to
draw from.

- ##G-ARCHIVE-NOT-DELETE It is an archive, not a deletion. The 43 files are the only record of
  what was documented, and the new tree is written *against* them — a fact
  present there and absent in the new docs is a **regression to report**, not a
  simplification.
- ##G-ARCHIVE-STAYS-UNOBSERVED `docs-legacy/` joins no include glob. It was unobserved before the
  move and stays so; moving it does not make it a contract.
- ##G-README-DEVGUIDE-DECIDED `README.md` and `DEV-GUIDE.md` are **not** part of the move by
  default — a repository root's README is its front door and a dev guide is
  load-bearing setup documentation (the `dev-runtime-docs` flow governs it, and
  that flow requires setup docs to change in the same commit as the toolchain
  they describe). Moving either is a separate owner call.

## 3. Step 2 — the package {#package}

```
packages/org.vibevm.doc/doc/v0.1.0/     the documentation
packages/org.vibevm.doc/web/v0.1.0/     RESERVED — manifest only, no site
```

### 3.1 The one-way law, and why it is right here {#one-way}

*Owner: «Связи однонаправленные, документация ссылается на спеки, спеки о
документации не знают.»*

**Documentation cites a spec unit by its `spec://…#anchor` URI. It never
restates the fact.**

This is not merely a preference about coupling, and it is worth stating why,
because the reason is the single most-repeated finding of this campaign: **a
documented fact that restates a spec fact is a second statement of one truth,
with its own writer, and nothing forces the two to agree.** This campaign has
now found that shape five times — a caret, a hand-written timestamp, three stale
projections, a gate's target, a ledger line that put a false premise into two
task files. Documentation is the largest surface on which it could happen, and
the one-way citation law is what prevents it structurally rather than by
vigilance.

- ##G-CITE-NOT-COPY A doc page carries **prose the reader needs and citations for every
  claim it makes**. Where it would state a normative value, it cites the anchor
  that governs it.
- ##G-EDGE-IS-DOCUMENTS The citation is a `documents` edge (PROP-014's edge kinds:
  `implements · verifies · documents · deviates · informs`), so the link is
  machine-visible, not merely a markdown href.
- ##G-SPEC-STAYS-IGNORANT **No spec file gains a back-link.** The spec tree does not know the
  documentation exists. A spec edit is never blocked by a doc edit.

### 3.2 The consequence the one-way law creates, and its instrument {#one-way-cost}

One-way linking has a real cost and it must be paid deliberately: **a spec unit
can change under a doc page that cites it, and nothing in the spec tree will
notice.**

That is exactly what `implements`/`verifies` already solve for code, and the
same machinery answers it here: PROP-014's **two-tier revisions** —
author-asserted semantic revision plus content hash, with **asymmetric
invalidation** (spec bump ⇒ edges suspect; code change ⇒ edges stay valid). A
`documents` edge pinned `~r<N>` goes suspect when its target's revision moves.

**So the direction of the check is the reverse of the link**: the docs link to
the spec, and the *tooling* reports which doc edges a spec change has
invalidated. One-way authorship, two-way detection. Without this, the one-way
law buys decoupling at the price of silent rot — and silent rot is what the
whole campaign exists to remove.

### 3.3 Is the doc package observed? {#observed}

**Recommendation: yes, and it is cheap precisely because of §3.1.** A doc page
that cites rather than restates carries few facts of its own; most of its
sentences are navigation and explanation, which the two anchor registers already
distinguish (`##kebab-case` service units). Marking it closes the loop:
`progress check` then reports doc pages whose citations dangle.

**Gotcha, and it is the same one three times now:** `progress.toml` is
**include-only by design**. A new group is invisible until its glob is added.
`packages/org.vibevm.doc/**/*.md` must be added explicitly, or the phase
produces documentation that is unmeasured exactly like the tree it replaced.

## 4. What the phase consumes — and none of it exists yet {#inputs}

This is the part wave 1 got wrong, and amendment **A1** exists because of it:
wave 1's Phase C listed «harvest cards written while knowledge is hot» among its
steps, gated on something else, skipped the step at no cost, and **Phase G
arrived to consume an empty directory and had to be deferred**. That is this
phase's failure mode, inherited.

| input | supplied by | state today |
|---|---|---|
| **harvest cards** | Phase C's step (A1 makes it a gate condition) | `harvest/` empty |
| **`audience`** — `user` \| `author` \| `dev` | Phase B markup + the judgment pass | set on **one** file so far |
| **`actionstage="doc"`** markers | the judgment-marking pass (wave 1's Phase F, A3.i) | **zero** in the corpus |
| **the two guides' TOC** | `vibe progress report --view doc --audience user\|author` | produces nothing, correctly — no input |

`##AUDIENCE-DOC-USE` in PROP-043 states the mechanism plainly: *«`actionstage="doc"`
markers feed the two guides' tables of contents»*. **The generator is built and
its input is empty.** Phase G does not write a table of contents by hand; it
runs that command, and if the command returns nothing the *judgment pass* is
what is missing, not the documentation.

- ##G-DO-NOT-HAND-BUILD-TOC **Never hand-assemble what the report generates.** A hand-built TOC
  is a derived value with its own writer — §3.1's whole argument, committed
  inside the phase meant to demonstrate it.
- ##G-AUDIENCE-GAP-IS-KNOWN The `audience` vocabulary has a known gap (F-082): a package's boot
  snippet is read by a **consuming project's session**, which is none of
  `user` / `author` / `dev` cleanly. Phase G will hit it on every flow package.
  Resolve it as a vocabulary amendment before the guides, not during them.

## 5. The two guides {#guides}

- **User Guide** — `audience="user"`: writes specs in their own project,
  installs dependencies, never opens package internals.
- **Package Author Guide** — `audience="author"`: builds packages, wants depth.
  This one documents `packages/`, which is this campaign's own subject, so it is
  written last and benefits most from Phases B–F having run.

Both are assembled from marked units, not composed from scratch: the `audience`
axis already partitions the corpus, and that partition is the outline.

## 6. The genre question — documentation is not in the map {#genre}

`spec-genres` types every project document and its map has seven rows: boot
files, foundational decisions, module contracts, design docs, research docs,
campaign plans, the checkpoint. **Product documentation is in none of them.**

So the one-way rule is not a deviation from that flow's «keep the two-way links»
— that rule governs **lore explaining a contract**, and a User Guide is not
lore. It is a genre the map does not cover.

- ##G-ADD-GENRE-ROW Phase G adds the row: *documentation — holds what a consumer needs to
  use the product; binding? no; cites contracts one-way and is never cited by
  them*. Without it the next session must re-derive the same conclusion, and the
  flow's own law is «never create a document without deciding its genre».

## 7. The reserved website package {#web}

`packages/org.vibevm.doc/web/v0.1.0/` — **manifest only. No site is built.**

Declared stack: **node.js · Qwik v2 · Tailwind · TypeScript**.

- ##G-QWIK-READING **Reading to confirm before the manifest is written:** the directive
  says «quick.dev (v2)». Taken as **Qwik** (`qwik.dev`) v2 — it is the only
  framework matching that name and it fits node + Tailwind + TypeScript. A
  wrong framework name frozen into a published manifest is a rotting literal of
  exactly the kind this campaign keeps finding, so it is confirmed with the
  owner, not inferred silently.
- ##G-WEB-RESERVE-WHY Reserving now is right under `qualified-naming`: `(group, name)` is
  globally unique and **a rename is a new identity** with no version carry-over.
  Reserving costs a manifest; renaming later costs the coordinate.
- ##G-WEB-CONSUMES-DOC The site consumes `org.vibevm.doc/doc` as a dependency and adds no
  content of its own — the same content-minimal shape PROP-028 gives family
  aggregators. Its own facts would otherwise be a second copy, §3.1 again.
- ##G-WEB-IS-TYPESCRIPT-DISCIPLINE When it is built, it is TypeScript under the installed
  `typescript-ai-native` stack, not an exception to it.

## 8. Exit gate — enumerating this phase's own steps (A1) {#exit}

1. `docs/` moved to `docs-legacy/` in a move-only commit; nothing cites into it
   normatively.
2. `packages/org.vibevm.doc/doc/v0.1.0/` exists, is a valid package, and is
   added to `progress.toml`'s include globs.
3. **Every claim in the docs carries a `documents` edge** to the spec unit it
   came from; **zero** doc pages restate a normative value.
4. **Zero spec files link to documentation** — checked, not assumed.
5. Both guides' tables of contents are **generated**, not hand-written, and the
   command that generates them is recorded with its output.
6. Every fact present in `docs-legacy/` and absent from the new tree is either
   deliberately dropped with a reason or filed as a regression.
7. `spec-genres`' map carries the documentation row (§6).
8. `packages/org.vibevm.doc/web/v0.1.0/` exists as a manifest with the confirmed
   stack, and **no site code**.
9. `baseline.json` written at phase close (A6).

## 9. Predictions — each naming the step that tests it (A5) {#predictions}

1. **The `audience` axis, not the prose, is the phase's real cost.** Partitioning
   10 825 facts into user/author/dev is the judgment pass's work, and Phase G
   cannot start without it. *Tested by:* step 5 — if the generated TOC is empty
   or wrong, the input was missing.
2. **A substantial fraction of `docs-legacy/`'s 43 files documents behaviour
   that no longer exists.** They were never observed, never verified, and the
   product has moved through M1.17–M1.19 since. *Tested by:* step 6's
   regression list, which will separate "dropped because obsolete" from
   "dropped because forgotten".
3. **The one-way law will be violated first by a table, not by prose.** A
   command-reference table restating flag names and defaults is the most natural
   place to copy a normative value. *Tested by:* step 3's zero-restatement check.
4. **`audience` will need a fourth value** before the guides are written (F-082).
   *Tested by:* the vocabulary amendment in §4, which either happens or the
   guides ship with boot snippets miscategorised.

## 10. Prerequisites {#prereqs}

- **Phase F closed** — the credibility report is what the documentation must not
  contradict.
- **The judgment-marking pass run** (wave 1's Phase F, A3.i) — it supplies
  `audience` and `actionstage="doc"`. **Phase G cannot start without it**, and
  that pass still has no phase of its own in this plan; §4 is the argument for
  giving it one.
- **The harvest pass run** — A1 made it a Phase C exit condition precisely so
  this phase does not arrive at an empty directory twice.
