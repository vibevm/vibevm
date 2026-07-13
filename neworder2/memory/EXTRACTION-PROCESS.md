# The generalized process — extracting a project's knowledge into packages

_Owner-commissioned meta-goal (2026-07-14): turn the procedure we are running on vibevm into a
**reusable, project-agnostic process** for dissolving any project's embedded knowledge into a
rich ecosystem of dependency-loaded packages. This doc is the running capture; it is itself a
candidate for a package (**recursion**: the extraction produces packages, and its own method
becomes one — working name `org.vibevm.world/knowledge-extraction`). Keep it current as
findings accrue. Companion: `00-understanding.md` (the vibevm-specific model + git state),
`concepts.md` (the package plan), `report.md` (the engine requirements)._

---

## 0. The thesis

A mature project accumulates **reusable knowledge** — disciplines, protocols, postures,
patterns — tangled inside project-specific files (a foundational conventions doc, agent
instruction files, per-module specs). That knowledge is trapped: it cannot be reused by
another project, versioned independently, or improved once and propagated. **Extraction frees
it**: each reusable idea moves into its own installable package (single source of truth); the
project becomes a **thin consumer** that declares the package as a dependency and lets the
loading model deliver it. The project shrinks to what is genuinely project-specific.

The economic frame: this is the highest-leverage documentation refactor — it converts N
projects each re-writing the same discipline into one package N projects depend on.

## 1. The end-state (what "done" looks like)

- Every reusable idea lives in **its own package** (content there, once).
- The host **section is deleted or reduced to a thin stub** (a link + any project-specific
  residue — e.g. "we have two mirrors: github + gitverse, see <pkg>"). **No restated content.**
- The package is a **real dependency** in the host manifest, **loaded statically** (its boot
  snippet is in the computed boot sequence, forced every session).
- **The host says NOTHING about how loading works.** Declaring the dependency IS the delivery.
  Never write "its boot snippet delivers X" — that prose is the smell of a missing dependency.
- Hierarchical topics become **families**: an aggregator package whose members are separate
  sub-packages in its dependencies.
- **The C++ `#include` rule**: installing a dependency never edits the host's authored text.
  The host is edited *by the refactorer* (delete the extracted section); the package's content
  arrives via the materialized-deps tree + the generated boot index, never pasted in.

## 2. Anti-patterns (learned the hard way on vibevm v1)

1. **Cite-in-place** — leaving the content in the host and adding a `see <pkg>` link. WRONG:
   the content must MOVE; the host keeps at most a stub.
2. **No new packages** — only citing packages that already happened to exist. WRONG: extraction
   *creates* the packages the corpus needs.
3. **Loading prose** — explaining in the host how the boot snippet loads. WRONG: the dependency
   mechanism handles it; the host is silent about loading.
4. **Timidity of scope** — marking a pattern "project-specific, stays" because it *appears*
   in a project-specific file. A codeword like «move fast and break things» still works in
   *other* projects → it is reusable → its own package. Default to extractable; reserve "stays"
   for genuine machinery (the tool's own resolver, registry, install engine).
5. **A read-only zone** — treating some files as off-limits. For a sanctioned refactor, **every
   file is editable** (the owner grants it). Foundational/boot/frozen files are the *richest*
   sources; mine them under double attention.

## 3. The procedure

### Phase S — setup (the checklists exist so nothing is missed)
1. **Safety envelope** — a dedicated branch, a rollback tag, a durable workspace dir; a backup
   branch before any destructive reset.
2. **Scope manifest** — every file classified in/out with a reason. For a full extraction the
   in-scope set is *all authored project files*; out = generated/vendored/third-party trees.
3. **Package inventory** — index the packages that already exist (reuse targets) and the
   package registry conventions (address form, layout template, boot-snippet slots).
4. **Trace baseline** — snapshot the spec↔code graph (dangling set) + the prose-link set +
   the anchor set, so a moved unit's edges can be proven intact after.
5. **Source control table** — one row per in-scope file, status columns, one notesfile each.

### Phase A — classify (verified traversal, double attention on the foundation)
For **every unit** (a section, a rule, a protocol) decide:
- **(reusable)** → which package? existing (reuse/extend) or new (author)? standalone or a
  family member? Record the target in the concept registry (the dedup axis: best-version wins).
- **(project-specific)** → stays. But probe for a hidden reusable seam first (the owner:
  "there may be MANY packages"). The tool's own machinery genuinely stays.
Order by **pattern density, highest first** — the foundational conventions doc and the agent
instruction files carry the most, so mine them first and let later files dedup into the
packages they establish.

### Phase M — move (the per-package capsule; gate + commit each)
For each target package P:
1. **Author/extend P** — create/patch `<group>/<name>/<ver>/` (manifest, README, LICENSE,
   `boot/NN-flow-<name>.md`, `spec/flows/<name>/*.md`) with the content **moved out of the
   host**, generalized to be project-agnostic (strip the host's proper nouns into "your
   project"). For a **family**, P's manifest depends on its member sub-packages.
2. **Delete/thin the host source** — remove the extracted section, or reduce to a stub (a link
   + project-specific residue). Delete the whole file where it becomes empty. No loading prose.
3. **Declare the dependency** — add `"<kind>:<group>/<name>" = { version, link = "static" }`
   to the host manifest (or the family aggregator's).
4. **Retarget inbound edges** — every citer of a removed anchor (code `#[spec]`/`scope!` edges
   AND prose `spec://…#anchor` links, tree-wide) → the package address. **Load-bearing
   code-cited anchors move WITH their content to the package** and the code edge is repointed;
   a move that would orphan a code edge must be refused until the edge is handled.
5. **Install + gate + commit** — materialize + regenerate the boot index; then the gate ladder
   (§4). One topic commit per package/family.

### Phase R — report
The by-hand moves are the requirements spec for a future *engine* that automates them
(`report.md`). The chief automatable operation is the **graph-wide citer rewrite** (see §5 R2).

## 4. The gate ladder (a move is done only when the model re-checks clean)
1. **No new dangling edges** — the code↔spec graph's dangling set does not grow vs baseline.
2. **No new broken prose links** — every `spec://` citation still resolves.
3. **Full self-check green** — fmt / tests / lint / spec-lint / cross-file-identity gates.
4. **Boot resolves** — reinstall regenerates the boot index; every path it names exists.

## 5. Operational findings (portable gotchas)
- **Install requires the tool's own live agent-servers OFF** — an MCP server running from a
  materialized-deps slot locks it; a full re-materialize then fails. Stop them for the run.
- **Local resolution** — resolving from the in-tree package registry needs the right flag/mode
  (here `--registry packages`); a plain dev binary is not an "installed" tool and gets no
  ambient registry.
- **Address forms** — host unit `spec://<host-ns>/<path>#<anchor>`; package unit
  `spec://<group>/<name>/<path>#<anchor>`.
- **Static vs inline loading** — core practices load **static** (boot snippet in the index,
  read every session). `inline` (verbatim into a first-read priority file) is the sparingly-used
  emergency lane. The dependency's boot snippet is what carries the content — so the host never
  restates it.
- **Verbatim guard** — some text is an *authorisation* (an owner's exact words); a paraphrase
  is a different authorisation. Mark such units do-not-reword; move them verbatim.
- **Load-bearing anchors** — anchors a compiler/linter cites (via code edges) cannot be dropped;
  the content + anchor move together and the edge is repointed.

## 6. Recursion note
This process, generalized and stripped of "vibevm", is itself the package
`knowledge-extraction` (working name). The vibevm run is its first application and its proof.
