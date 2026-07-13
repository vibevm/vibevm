# The generalized process — extracting a project's knowledge into packages

_Owner-commissioned meta-goal (2026-07-14): turn the procedure we are running on vibevm into a
**reusable, project-agnostic methodology** for dissolving any project's embedded knowledge into
a rich ecosystem of dependency-loaded packages. This doc is the running capture and the full
method — it is itself a candidate package (**recursion**: extraction produces packages, and its
own method becomes one — working name `org.vibevm.world/knowledge-extraction`). Companions:
`00-understanding.md` (project-specific model + git state), `scope.md` / `oldpacks.md` /
`concepts.md` / `allspecs.md` / `trace-baseline/` (the live checklists), `report.md` (the
engine requirements). Keep this current as findings accrue._

---

## 1. The thesis

A mature project accumulates **reusable knowledge** — disciplines, protocols, postures,
patterns — tangled inside project-specific files (a foundational conventions doc, agent
instruction files, per-module specs). That knowledge is trapped: it cannot be reused by another
project, versioned independently, or improved once and propagated. **Extraction frees it**: each
reusable idea moves into its own installable package (single source of truth); the project
becomes a **thin consumer** that declares the package as a dependency and lets the loading model
deliver it. This is the highest-leverage documentation refactor — it converts N projects each
re-writing the same discipline into one package N projects depend on.

## 2. The end-state (what "done" looks like)

- Every reusable idea lives in **its own package** (content there, once).
- The host **section is deleted or reduced to a thin stub** — a link + any project-specific
  residue (e.g. "we have two mirrors: github + gitverse, see <pkg>"). **No restated content.**
- The package is a **real dependency** in the host manifest, **loaded statically** (its boot
  snippet is in the computed boot sequence, forced every session).
- **The host says NOTHING about how loading works.** Declaring the dependency IS the delivery.
  Never write "its boot snippet delivers X" — that prose is the smell of a missing dependency.
- Hierarchical topics become **families**: an aggregator package whose members are separate
  sub-packages in its dependencies.
- **The C++ `#include` rule**: installing a dependency never edits the host's authored text.
  The host is edited *by the refactorer* (delete the extracted section); the package's content
  arrives via the materialized-deps tree + the generated boot index, never pasted in.

## 3. Anti-patterns (learned the hard way on vibevm v1)

1. **Cite-in-place** — leaving content in the host and adding a `see <pkg>` link. WRONG: content
   must MOVE; the host keeps at most a stub.
2. **No new packages** — only citing packages that already happened to exist. WRONG: extraction
   *creates* the packages the corpus needs.
3. **Loading prose** — explaining in the host how the boot snippet loads. WRONG: the dependency
   mechanism handles it; the host is silent about loading.
4. **Timidity of scope** — marking a pattern "project-specific, stays" because it appears in a
   project-specific file. A codeword like «move fast and break things» still works in *other*
   projects → reusable → its own package. Default extractable; reserve "stays" for genuine
   machinery (the tool's own resolver, registry, install engine).
5. **A read-only zone** — treating files as off-limits. For a sanctioned refactor **every file is
   editable**; foundational/boot/frozen files are the *richest* sources — mine under double
   attention.

## 4. The method engine — verified sequential traversal

Every phase runs the same loop, so nothing is missed and a restart/compaction is lossless:

1. **Obtain the list** (of files, of packages, of concepts).
2. **Make a control file** with an `object` column and one or more **status** columns.
3. **Walk the list**; do each object's work.
4. **Mark a status ATOMICALLY** — set it *only when that status's work is FULLY done* (e.g.
   `analyzed` only when BOTH the in-file markers AND the notesfile are complete). A half-done
   object stays unmarked.
5. When the list is exhausted, **re-scan and loop over any unmarked** until all are marked.
6. **Commit after every unit** — the control files + the package are git-tracked, so re-entry
   after an interruption re-reads, finishes what is missing, and never duplicates.

## 5. The checklist artifacts (the "miss nothing" machinery)

Built once in setup, kept live throughout. All under a durable workspace dir (`neworder2/`),
git-tracked.

- **`scope.md` — the in/out manifest.** Every file classified into zones with a **reason for
  every exclusion**: *in-scope* (authored project files — for a full extraction, effectively all
  of them), *out* (generated blocks, vendored/materialized dep copies, third-party clones, build
  artifacts, pure state/journals). Plus a **scanning-hygiene rule**: prune
  `node_modules/target/dist/.git/<vendored-deps>/<cache>` at any depth before counting or reading
  — materialized-dep copies duplicate the same files many times (on vibevm: of 1379 raw `.md`
  under `packages/`, only 360 were real source). "Skip nothing" means skip nothing *in-scope*.
- **`oldpacks.md` — the existing-package index (reuse targets).** Distil every package that
  already exists: `object | covers (1-line) | exported spec:// namespace | has a checker?`.
  Records the **address form** and the **package layout template** (manifest, README, LICENSE,
  `boot/NN-flow-<name>.md`, `spec/flows/<name>/*.md`) and the **free boot-snippet slots**. This
  is what lets Phase A choose *reuse vs author*, and Phase M author new packages to convention.
- **`trace-baseline/` — the "before" graph (the gate comparators).**
  - `specmap.snapshot` — the code↔spec graph's **dangling set** from a live index build (record
    any pre-existing orphan so it does not count as "new"); this is what gate 1 protects.
  - `prose-links.tsv` — every inline `spec://…#anchor` citation across the corpus (gate 2: no new
    broken link).
  - `anchors.tsv` — every `{#anchor}` definition (the resolution target set; a moved anchor keeps
    its `{#…}` so its identity survives the move).
- **`allspecs.md` — the source control table.** One row per in-scope file:
  `file | analyzed | distilled | refactored | disposition | notesfile`. The verified-traversal
  checklist; its columns drive resume ("find the first unmarked row and continue").
- **`notes/` — one notesfile per source, MIRRORING the source tree** (`spec/common/PROP-000.md`
  → `neworder2/notes/spec/common/PROP-000.md`; collision-free). First line = the source path.
  Holds the per-file analysis and the marked candidates (each wrapped in a
  `<!-- MARKER:NAME -->…<!-- /MARKER -->` pair). `analyzed` is atomic on markers + notesfile.
- **`concepts.md` — the concept registry (the dedup axis + the exhaustive package plan).**
  One row per reusable concept: `concept | target package | new/exists/family | host source(s) |
  host fate`. The dedup rule: if a package already owns a pattern, do not duplicate — reuse it,
  merging into the single **best** version (best-wins, not first-wins). Families are recorded as
  aggregator + members. This is the master map the whole move phase executes against.

## 6. The procedure (phases, driven by §5's artifacts)

### Phase S — setup
Safety envelope (a dedicated branch, a rollback tag, a **backup branch before any destructive
reset**, the durable workspace) → build `scope.md` → `oldpacks.md` → `trace-baseline/` →
`allspecs.md` + empty notesfiles.

### Phase A — classify (verified traversal, double attention on the foundation)
Walk the in-scope files **ordered by pattern density, highest first** (the foundational
conventions doc and the agent instruction files carry the most — mine them first so later files
dedup into the packages they establish). For every unit decide **reusable → which package**
(existing/new/family, into `concepts.md`) or **project-specific → stays** (but probe for a hidden
reusable seam first — "there may be MANY packages"). Mark `analyzed` atomically. Do a one-time
**comprehension trial** — take the single clearest pattern all the way through Phase M's capsule
+ gates before the mass run, to catch a misunderstanding on one capsule instead of seventy.

### Phase M — move (the per-package capsule; gate + commit each)
For each target package P in `concepts.md`:
1. **Author/extend P** — `<group>/<name>/<ver>/` (manifest, README, LICENSE,
   `boot/NN-flow-<name>.md`, `spec/flows/<name>/*.md`) with content **moved out of the host**,
   generalized project-agnostic (strip the host's proper nouns into "your project"). For a
   **family**, P's manifest depends on its member sub-packages.
2. **Delete/thin the host source** — remove the extracted section, or reduce to a stub (a link +
   project-specific residue). Delete the whole file where it empties. No loading prose.
3. **Declare the dependency** — `"<kind>:<group>/<name>" = { version, link = "static" }` in the
   host manifest (or the family aggregator's).
4. **Retarget inbound edges** — every citer of a removed anchor, code `#[spec]`/`scope!` edges AND
   prose `spec://…#anchor` links, tree-wide → the package address. **Load-bearing code-cited
   anchors move WITH their content to the package** and the code edge is repointed; a move that
   would orphan a code edge must be refused until the edge is handled.
5. **Install + gate + commit** — materialize + regenerate the boot index; the gate ladder (§8);
   one topic commit per package/family. Update `allspecs.md` + the notesfile disposition.

### Phase R — report
The by-hand moves are the requirements spec for a future *engine* that automates them
(`report.md`); the chief automatable operation is the graph-wide citer rewrite (§9 R2).

## 7. The per-package capsule, in one line
author package (content in) → delete/thin host → declare static dep → repoint citers → install →
gate → commit. Nothing about loading is ever written in the host.

## 8. The gate ladder (a move is done only when the model re-checks clean)
1. **No new dangling edges** — the code↔spec dangling set does not grow vs `trace-baseline`.
2. **No new broken prose links** — every `spec://` citation still resolves.
3. **Full self-check green** — fmt / tests / lint / spec-lint / cross-file-identity gates.
4. **Boot resolves** — reinstall regenerates the boot index; every path it names exists.

## 9. Operational findings (portable gotchas)
- **Install requires the tool's own live agent-servers OFF** — an MCP server running from a
  materialized-deps slot locks it; a full re-materialize then fails ("Access denied"). Stop them.
- **Local resolution** — resolving from the in-tree package registry needs the right flag/mode
  (here `--registry packages`); a plain dev binary is not an "installed" tool and gets no ambient
  registry.
- **Static vs inline loading** — core practices load **static** (boot snippet in the index, read
  every session). `inline` (verbatim into a first-read priority file) is the sparingly-used
  emergency lane. The dependency's boot snippet carries the content — the host never restates it.
- **Verbatim guard** — some text is an *authorisation* (an owner's exact words); a paraphrase is
  a different authorisation. Mark such units do-not-reword; move them verbatim.
- **Address forms** — host unit `spec://<host-ns>/<path>#<anchor>`; package unit
  `spec://<group>/<name>/<path>#<anchor>`.
- **Package families** model hierarchical topics (aggregator + member sub-packages).

## 10. Recursion note
This process, generalized and stripped of "vibevm", is itself the package `knowledge-extraction`
(working name). The vibevm run is its first application and its proof — see the checklists in
this directory and `report.md`.

---

### Applied-run log (kept current)
- **Reference package proven:** `dev-runtime-docs` (from PROP-000 §19) — owner-confirmed the
  shape (2026-07-14). The v2 capsule + gate ladder are validated end-to-end.
- Backup of the wrong v1: branch `cultural-backup` @ `0eb3202`. Baseline: `pre-cultural-refactor`
  @ `8831a14`. Redo lands on `cultural-refactor`.
