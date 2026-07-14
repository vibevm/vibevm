# PROP-035: The spec compiler — directive preprocessor, package formats, and the two-mode boot linker {#root}

**Status:** DESIGN — provisional, 2026-07-14 (owner-requested; the flagship "static-compiler vision"). Requirements captured from an owner design dialogue; **not implementation-locked**. Sections marked *(provisional)* are held for the implementation task, not yet decided.
**Extends:** [PROP-009](PROP-009-loading-model.md) (the loading model — inclusion types, the two-tree model, the `STATIC.md` / `INDEX.md` artifacts). This PROP turns PROP-009's "ordered list of contributions" into a real **preprocessor + linker**.
**Supersedes / folds in:** [PROP-034](PROP-034-transitive-links-boot-graph.md) (transitive links + the static boot-link graph). PROP-034's linker becomes the *emission layer* of this system (§12); PROP-034 is retained as the narrower, already-drafted contract for that layer until this PROP is ratified.
**Related:** [PROP-028](../../common/PROP-028-package-families.md) (families — the aggregator role), [PROP-029](../../common/PROP-029-fully-qualified-addresses.md) (`spec://` addressing, the `/` group↔name joiner), [PROP-008](../vibe-registry/PROP-008-qualified-naming.md) (pkgref grammar `kind:group/name@version`), the `addressable-specs` flow (anchor / section grammar), [`spec/design/loading-and-boot-model.md`](../../design/loading-and-boot-model.md) (the static/dynamic-linking metaphor this PROP completes), [PROP-014 specmap](../../../packages/org.vibevm.ai-native/core-ai-native/v0.7.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md) (the `uri↔file` index the router extends).

---

## 1. Motivation — boot loading must become a toolchain {#motivation}

PROP-009 established the frame: **installing a dependency must never modify authored spec** — "the C++ rule that you do not paste a header's text into your `#include`" (PROP-009 §1) — and gave each direct edge an inclusion type (`static` / `dynamic`, PROP-009 §2.4). PROP-034 added transitive links and pinned the boot as a static-linked, deduplicated, topologically-ordered graph. Both are correct and both are **package-granular and directive-free**. Three things they do not yet give:

1. **A preprocessor.** There is no way for one spec to say "splice this exact section here" (`#embed`), "I depend on this — pull it, in order, before me" (`#use`), or "this contract is implemented over there" (`#source`). Cross-spec composition is done today by prose citation, which the loader does not act on.
2. **A resolver.** Every directive takes a `spec://` address, and the static compiler must turn it into a concrete file/section **algorithmically, without an LLM**. The codebase has no `spec:// → path` function today — resolution is the *inverse* (a filesystem scan mints `path → URI` into `specmap.json`). See §6.
3. **Section granularity.** Real economy needs the unit of loading to be a **section of a document**, not a whole package or file. PROP-034's graph is over packages; the cascade this PROP needs (`a` pulls one file of a big package, which pulls the next) requires a graph over document **sections** (§5).

The stake is the context budget. Loading vibevm itself already spends ~10% of a 1M window; an operating-system-scale project authored in Spec-Driven Development does not fit at all if every dependency loads whole. This PROP makes boot a **two-mode compiler**: an algorithmic *static* compiler that statically links a whole closure into one file, and a *structural* loader that reads only what is actually used, lazily, in dependency order — the same economy delegation-first buys for *work*, bought here for *loading*.

---

## 2. Two build modes — static vs structural (AOT vs JIT) {#modes}

The system is one directive semantics with two executors, exactly the GraalVM / Project Leyden split the owner names:

- **Static build** — packages are concatenated into one (or few) files (`STATIC.md`). Directives are resolved **statically, by code, without an LLM**. This is the AOT / devirtualized path: no runtime indirection, the agent reads a finished file.
- **Structural build** — the agent reads specs **on demand**, following directives as it meets them. This is the JIT / late-binding path; it subsumes PROP-009's current `static` and `dynamic` modes.

**The equivalence invariant.** Both executors MUST produce the *same effective spec* — as AOT and JIT must run the same program. The **static compiler is the reference semantics**; the structural loader (a prompt today, a hard algorithmic agent later, §13) is checked against it. Differential testing of the two is real, empirical, and expensive; it is **deferred and planned separately** (§16), not part of the first build. Until then, the structural side is best-effort and the static side is authoritative.

Two executors, one contract:

- **Static compiler** — code, fully algorithmic. Buildable now (§8).
- **Structural loader** — a set of first-loaded instructions (§13) that make the agent honour the directives, pending the future algorithmic agent.

---

## 3. Package formats — `simple` and `normal` {#formats}

A new `vibe.toml` `[package]` field `format`, alongside `version`:

- **`format = "simple"`** — **the default** (absent `format`, a package is `simple`). Legacy / adapted prompts, carried **whole**, with no VibeVM-specific structure — for importing existing corpora without rewriting them, and the fail-safe posture. Rules: inclusion in `[requires.packages]` means (a) structural — the agent reads the file; (b) static — its text is compiled into the target. If `[boot_snippet].source` names a file, only that file is read/spliced; **absent even that, every file in the package is read/spliced by a recursive walk** — the over-load is the author's problem, the deliberate cost of not adopting `normal`.
- **`format = "normal"`** — the VibeVM-native form, **opt-in**: the `contract` / `source` split (§4), directives (§7), and the compiler (§8). A `normal` package is **not read just because it is present** — it participates only when something actually `#use`s it (§7.2). This is tree-shaking; the optimized posture for authors who understand the machinery, at the price of structuring the package correctly.

**Why `simple` is the default (owner decision, 2026-07-15).** A forgotten `format` must **fail safe, not silent**. With `normal` as the default, a naive or mis-built package that nobody `#use`s loads **nothing** — a silent no-op, the worst failure. With `simple` as the default it loads **everything** — noisy and unoptimized, but visibly working; the author opts into `normal` and its discipline deliberately. Migration (§15) is thereby a non-event: the existing corpus keeps working as `simple`, and packages convert to `normal` one at a time as their authors optimize them.

---

## 4. Normal packages — the contract / source split {#contract-source}

Inspired by C/C++ `.h` / `.cpp`:

- **`contract/`** — small, simple, boot-snippet-like. The surface a package exposes outward; short files, cheap to load. The analogue of a header.
- **`source/`** — large, heavy. The full implementation; pulled only when actually needed. The analogue of a translation unit.

Because the structural executor lacks a C++ compiler's global view of the source tree, we use a deliberate hack: **the contract author declares what implements it, via `#source`** (§7.3). The author hand-draws edges a globally-aware compiler would infer. That global view *does* exist in static mode and can be pre-compiled into **link tables** (§10), which is how we give the same knowledge back to the structural executor cheaply and make the hand-drawn `#source` edges verifiable.

The `contract` surface is what other packages include; `source` is reached only through a resolved contract edge or an explicit `#use`/`#embed` into it.

---

## 5. The document IR — one hierarchical tree, two frontends {#ir}

Everything downstream operates on a single **document IR**: a DOM-like tree. Markdown and (future) XML are two frontends parsed into the same tree, so algorithms written against the IR scale to deeply nested XML for free.

- **Node** = `{ id (anchor / tag), depth, kind, body-span, children[] }`.
- **Markdown frontend.** Headings form the tree by level (`#` ⊃ `##` ⊃ `###`). A node's `id` is its explicit `{#anchor}`. A node's **body span runs from its anchored heading to the next heading of the same or higher level** (the owner-fixed rule); its children are the nested headings inside that span.
- **XML frontend (future).** Elements already are the tree; `tag`/`id` is the address. Held for later, but the IR is designed for it now.
- **Addressing depth.** A `spec://…#a.b.c` fragment is a **path down the tree** (`a` → `b` → `c`), which already matches addressable-specs' dotted anchors (`#verification.timeout`). Sections at any depth are addressable.

**Granularity rules** (owner-set), stated over the IR:

- **`#use` pulls the whole top-level anchored ancestor** of the addressed node — reference a subsection, load its enclosing top-level section as one connected block. It does **not** pull that ancestor's siblings; siblings are read only when themselves needed.
- **`#embed` has arbitrary granularity** — it splices exactly the addressed node, no more.
- "Top-level anchored ancestor" is a **parameter of the resolver**, not a hardcoded heading level, so the XML frontend can define it structurally.

---

## 6. `spec://` addressing and the resolver ("router") {#addressing}

**Format choice.** `spec://` stays. Alternatives are strictly worse: path-based is fragile across `packages/`→`vibedeps/` materialization and drops versions; content-addressed (by hash) is unreadable and cannot "name a section"; query-based (by tag) is non-deterministic (may match N nodes) where a preprocessor needs exactly one. `spec://` is already symbolic, human-readable, and carries `group`/`name`/`path`/`anchor`. The gap is not the format — it is the **missing resolver**.

**Unified grammar** (reconciled with the pkgref grammar of PROP-008):

```
spec://<group>/<name>[@<version>]/<doc-path>#<anchor>[.<sub>…][~r<N>]
```

- `group` ↔ `name` joiner is **`/`, never `.`** (PROP-029).
- `@<version>` is **optional**; absent, the version is taken from the lockfile / current install. This closes one of today's ambiguities (the URI carried no version while several versions coexist on disk).
- `#<anchor>.<sub>…` is a **tree path** into the document IR (§5).
- `~r<N>` pins a spec-unit revision (PROP-014), not a package version.

**The router** is the new component: a deterministic function `spec:// → IR node`, evaluated over the resolved, materialized tree. It is the prerequisite everything else stands on. It must handle, without an LLM:

- **Doc-id truncation** — `PROP-NNN` / `FEAT-NNN` in a URI resolve to `PROP-NNN-<slug>.md`; other docs use the full stem. (This is `canonical_doc_path` in the specmap engine, reused, not reinvented.)
- **`packages/` (source) vs `vibedeps/` (materialized slot)** — the compiler resolves against the **materialized `vibedeps/` tree** (the installed reality), consistent with the specmap engine, which never scans `packages/`.
- **Version selection** from the lockfile when `@version` is absent.

Determinism is a hard requirement: the static compiler must resolve every address to exactly one node or fail loudly. The router extends the `specmap.json` index (already a `uri↔file` table) rather than duplicating it.

---

## 7. The three directives {#directives}

Common shape: `#<directive> [options] <specpath>`, where `specpath` is a `spec://` address (§6) — a section, or a whole file. Every file referenced by any directive MUST be declared in the package's `vibe.toml` (the directive graph and the manifest cannot disagree). The directive instructions are among the first things loaded, in every project, package, and library (§13).

### 7.1 `#embed` — the macro (materialization-time) {#embed}

The simplest preprocessor directive: splice one section (or whole file) into another — a C-macro substitution over `spec://` addresses.

- **Fires at materialization** (`vibe install` into `vibedeps`) **and** must be fully expanded during `STATIC.md` compilation. **No unexpanded `#embed` may survive in a compiled `STATIC.md`.**
- **Mode-independent.** `#embed` is purely a materialization concern — it produces the same text in both build modes. Therefore `vibedeps` stores embeds **already expanded**, while `#use`/`#source` (mode-dependent) remain as directives. This split removes a large part of the ordering problem: embeds are fixed early and identically for both executors.
- **Contract-section rule.** An `#embed` targeting a `contract`-section of a `normal` package inherits the full **`#source` compilation rules** (§7.3) for that paragraph — so the merged (contract+source) text is what gets spliced.
- **Unrestricted otherwise.** `#embed` may splice any paragraph of any spec, or a whole spec (including `format = "simple"`), with no structural checks.
- **Arbitrary granularity** (§5). Algorithmic; the tool of building highly-optimized statically-assembled package hierarchies — its main purpose.

### 7.2 `#use` — the dependency edge (tree-shaking) {#use}

The harder directive: pull only the packages actually needed.

- **Problem it solves.** Specs refer to entities defined elsewhere, often without an explicit link, and linking constantly is tedious. Without `#use`, the smallest script would have to pull the whole standard library.
- **Tree-shaking default.** A `format = "normal"` package that nobody `#use`s does not participate — not read, not used, anywhere. The moment any text does `#use spec://…`, that package **enters the build** and MUST be linked **before** its user in topological order.
- **Structural mode.** `#use` is an instruction to read the target's content **when it is needed** — not eagerly, but definitely **before using anything inside it**. Reads **cascade**: `a` `#use`s `b`, `b` `#use`s `c`, so using `a` loads `b` and `c` transitively. The cascade is what lets a big package be entered through one file and expand from there, rather than loaded whole.
- **Inline mode.** The same, statically: the `#use`d library's text is **fully copied higher up in `STATIC.md`** so it is available before the user.
- **Granularity.** Pulls the whole top-level anchored ancestor of the addressed node (§5).
- **Contract-section rule.** A `#use` into a `contract`-section of a `normal` package inherits the `#source` rules (§7.3). Otherwise `#use` may pull any paragraph or whole spec (even `simple`) with no structural checks.

### 7.3 `#source` — contract↔implementation virtual linking {#source}

Like a C++ interface, but with section-level merging. `contract` sections are the exposed surface; `#source` names the file(s) that implement them. Sections are treated as the analogue of class methods, needing a merge (in the static build) or virtual-lookup (in structural mode) mechanism.

Merge algorithm, per section (by matching `{#tag}`):

1. **In contract, absent in source** — structural: a full part of the spec, readable at will; static: compiled into the build whole.
2. **In source, absent in contract** — always counted; structural: readable at will; static: compiled in whole. *(Calling a section that exists only in the implementation is poor taste, but permitted — we deliberately impose no `private`/`public` access control.)*
3. **In both, same `{#tag}`** — merged by the tag's mode:
   - **`:replace`** — `# name {#tag} :replace` — the contract text is ignored; the source text is canonical (read by the agent / put in the static build; an already-read contract text is explicitly superseded).
   - **`:add`** — `# name {#tag} :add` — the result is the **sum**: contract text first, then source text. Static: compile the concatenation. Structural: the agent reads both and weights them equally.
   - **Default is `:add`** (absent a `:`-suffix) — so the interface text need not be duplicated to appear in the result.

### 7.4 In-place use — the `@spec://` sigil {#in-place-use}

Implicit dependency without an explicit `#use`, made precise by a sigil:

- **`@spec://…`** (an `@` before `spec`) is an **in-place use**: the agent MUST read it (mandatory), exactly as if a `#use` had been declared at the top of the file.
- **Bare `spec://…`** (no `@`) is at the agent's discretion — read it if useful, skip it otherwise. (A future algorithmic agent narrows this further.)
- **Read once.** An `@spec` target is read only on **first** encounter, never re-read blindly — see the read-set (§below).
- **Resolution order.** Collect all explicit `#use` into a map, enrich it with the `@spec` in-place uses, then act on the combined map.

**The read-set (surviving compaction).** To honour "read once" across an agent's context compaction, a persistent, file-based **read-set** (`.vibe/session/read-set.json` or equivalent) records `{ specpath, content_hash }` on each read; the `content_hash` is reused from specmap, so a *changed* section is re-read. A first-loaded instruction (§13) tells the agent to consult the read-set before reading and append after — this survives compaction because the boot instructions are re-read. Crucially the read-set records *what exists and where*, **not what is currently in context**: compaction evicts the *text* but not the *fact*, and reads are cheap (files sit in `vibedeps`). So the rule is: read an `@spec` target if (a) it is not in the read-set, **or** (b) it is, but its content is no longer in context. A clean solution without a harness compaction signal is out of scope; the file-based read-set plus a boot instruction is the pragmatic floor, and the future algorithmic agent keeps the read-set rigorously. Mental model: a **linker symbol table, but for what has been read**.

---

## 8. The compilation pipeline — the standard order {#pipeline}

The single ordering standard both executors follow (the "procedure of macro-substitution" the owner asked to pin down):

1. **Parse.** Build the IR tree (§5) of every participating file; collect all directives with their positions.
2. **Build the use-graph and topologically sort.** Nodes = document sections (or packages, at the coarser tier); edges = explicit `#use` + `@spec` in-place uses (§7.4) + implicit references. Topological order = every dependency before its dependent.
3. **Source-merge.** For every `contract` section, resolve `#source` (§7.3) into its effective (merged) body. This runs **before** embed because an embed may target a merged contract section (§7.1).
4. **Embed-expand.** Apply `#embed` (§7.1) as textual substitution, **top-down within a file**, in **topological order over the embed-graph across files** (a package is fully compiled — its source-merge and its own embeds done — before it is embedded), **recursively to a fixed point**, with cycle guards (§9).
5. **Emit.** Concatenate in topological order with open/close markers (§11). For the static build: `STATIC.md`. For structural: the loader consults the same order lazily.

Determinism: independent nodes are tie-broken by a stable key (category → boot-snippet slot → fully-qualified name), as PROP-034 §2.3 already specifies for the emission layer.

---

## 9. Cycles and guards {#cycles}

The owner's C++ intuition made precise. In C++ two distinct mechanisms are at work: an `#include` cycle is broken by **include guards** (idempotent re-inclusion becomes a no-op), and mutual recursion of *types* is resolved by **forward declaration** — a *declaration* needs no *definition*. "Including only interfaces never deadlocks" precisely because declarations can close a cycle without bodies. Mapped onto us:

- **`#embed` cycle → hard error.** `#embed` is `#include` without a guard, so a cycle is an infinite substitution. We **forbid** it: a guard keyed on the `specpath` currently on the expansion stack detects it, aborts compilation, and emits **debug info naming the full cycle path** (`A → B → A`). (Owner-required: the guard's firing is reported, not silent.)
- **`#use` cycle between contracts → allowed** (the forward-declaration case). Because a contract is small and self-contained, static mode breaks the cycle by **emitting the contracts before any source bodies**. Structural mode is "read when needed", so a contract-level cycle simply resolves lazily.
- **`#use` cycle that needs a source body to compile itself → error** (the "incomplete type where a complete type is required" case).

**Invariant.** The **contract layer is where cycles are legal; the source layer is where topological order is mandatory.** This is the theoretical no-deadlock guarantee: as long as the contract hierarchy is acyclic-under-`#embed` and no source body participates in a `#use` cycle, the build always terminates.

---

## 10. Link tables — the vtable analogue *(provisional)* {#link-tables}

The owner's C++-virtual-dispatch analogy, held for the implementation task. Inline mode ≈ a non-virtual / devirtualized call (bound statically, no runtime indirection); structural mode ≈ a virtual call (late-bound at runtime); a **link table ≈ a vtable** — a table the compiler builds once so the runtime dispatches cheaply instead of searching.

Concretely: at **install-time** (or a dedicated compile phase) build, by code, the graph edges the structural executor otherwise lacks —

- an **anchor-index** per document (the IR tree, addressable),
- a **contract→source map** (the real edges behind every `#source`),
- the **use-graph**,

and persist them to a file table (a sibling/extension of `specmap.json`). The structural agent then **consults a cheap on-disk table instead of building the graph in context** — which directly answers the objection that the structural executor "lacks global knowledge because the project is too big for the agent's context": the edges are built by the compiler, not the agent. A bonus: hand-drawn `#source` edges become **verifiable** — the table knows the real edges and can flag divergence. This reuses the specmap infrastructure rather than adding a parallel one. Kept provisional and folded into the implementation task per the owner.

---

## 11. Markers in compiled output {#markers}

When a file's text is placed into a static file (e.g. `STATIC.md`), a path comment is added **both before and after** it (today only *before* — the after-comment is the closing tag). Around a package body (which contains several files) the same: a package-open comment, then many file open/close comments inside, then a package-close comment. This makes `STATIC.md` **reversible** — a compiled artifact can be decomposed back to its constituent files and packages, giving the same bidirectional traceability specmap already provides for code.

---

## 12. Transitive static — folding in PROP-034 {#transitive-inline}

`static-transitive` may be set at the top of a package hierarchy; then every element below it in the dependency graph is pulled `static`, **regardless of what it declared before**. This is safe precisely because the static build is algorithmic, not LLM-driven, and loses nothing. It is the path to large highly-optimized builds.

This is PROP-034 §2.1/§2.3 (transitive links + dedup + topological order + cycle rejection), which becomes the **emission layer** of this compiler: after §8's pipeline resolves directives, PROP-034's linker deduplicates and dependency-orders the node list into `STATIC.md` / `INDEX.md`. `transitive-static` / `transitive-dynamic` remain reserved (no use case yet), but the graph analyzer is built to operate at that level.

---

## 13. The structural loader — the "first instructions" {#loader-prompt}

Until hard algorithmic agents exist (§14), structural mode is executed by an LLM following instructions. Those instructions — how to honour `#use`, `#embed`, `#source`, `@spec`, and the read-set — MUST load **first, everywhere**: in every project, package, and library vibevm manages. A project or package **without** them is considered **broken**; the project- and package-creation tools MUST check for and inject them. This is one of the most critical loading mechanisms — nothing works without it. Inline compilation, by contrast, needs no LLM and can build the whole thing algorithmically today; its tooling is what remains to be built (the current `STATIC.md` machinery is naive by comparison).

---

## 14. Future algorithmic agents {#future-agents}

We are preparing for purpose-built algorithmic agents that run alongside Claude Code (and in specific cases instead of it) and honour every directive (`#use`, `#embed`, `#source`) rigidly and unconditionally. The design must not assume only an LLM executor: the directive semantics (§7), the pipeline (§8), and the link tables (§10) are all specified so a deterministic agent can execute them. Remember this executor is coming.

---

## 15. Migration {#migration}

Incremental, safety-first (owner-set):

- **Build and test on a demo fixture corpus first** — throwaway packages exercising `simple`/`normal`, `contract`/`source`, `#embed`/`#use`/`#source`, cycles, and `@spec`. These are **not** real packages, and experimenting on them must never break vibevm itself.
- **Migrate real packages gradually**, improving them onto the new format one at a time.
- **Convert vibevm itself last:** first the whole of `org.vibevm.world`, and only then (if at all) the core feature specs. With `simple` as the default (§3) there is **no blast radius** — an un-migrated package keeps loading as `simple` (whole), so conversion to `normal` is per-package and opt-in, never a flag-day.

---

## 16. Open questions {#open}

1. **Equivalence testing (§2).** Differential testing of the static vs structural executors — real, empirical, experiment-heavy; **planned separately**, meaningless before a working base exists. The static compiler is the reference semantics in the meantime.
2. **`@spec` read-set across compaction (§7.4).** No clean solution without a harness compaction signal; the file-based read-set + boot instruction is the floor. Revisit if the harness exposes a compaction event.
3. **XML frontend (§5).** Timing and the exact IR mapping; the data structures are designed for it now, the frontend is built later.
4. **Implicit-reference closure explosion (§7.4).** With bare `spec://` at the agent's discretion, and `@spec` mandatory, bound the transitive closure a single in-place use can pull; measure on the demo corpus.
5. **Link tables (§10).** Whether they land at install-time, static-compile-time, or a separate phase — folded into the implementation task.
6. **No access control (§7.3).** We deliberately omit `private`/`public`. Confirm this holds once real packages exercise cross-contract calls.
7. **`dynamic-transitive` (§12).** Inherited from PROP-034 §5; still reserved.

---

## 17. Version history {#history}

- **2026-07-14 — drafted (owner-requested), provisional.** Captures the "static-compiler vision" design dialogue: two build modes as AOT/JIT with the equivalence invariant (§2); `simple`/`normal` package formats (§3); the `contract`/`source` split (§4); the hierarchical document IR with MD and future-XML frontends (§5); the unified `spec://` grammar and the deterministic router (§6); the three directives `#embed` / `#use` / `#source` plus the `@spec` in-place-use sigil and the read-set (§7); the five-phase compilation pipeline and the embed-ordering standard (§8); the C++-derived cycle rules and the contract-layer no-deadlock invariant (§9); link tables as the vtable analogue (§10, provisional); reversible open/close markers (§11); `transitive-inline` folding in PROP-034 as the emission layer (§12); the first-loaded structural loader (§13); the future algorithmic executor (§14); and the demo-corpus-first, vibevm-last migration (§15). Implementation begins with the router (§6) under this contract.
- **2026-07-15 — implemented, and the default flipped to `simple`.** §5–§13 shipped as the `vibe-spec` crate and wired into `bootgen` (the payoff: `render_inline` runs `expand_embeds`, guarded); `transitive-inline` (§12) landed on `LinkType`. **§3's default changed from `normal` to `simple`** (owner decision): a forgotten `format` must fail *safe* (over-load, visibly working) rather than *silent* (a `normal` no-op), which also removes the §15 migration blast radius. Still open and under review: the `link` × `format` interaction (does a `normal` + `static` edge read eagerly or lazily?).
- **2026-07-16 — link-type rename (owner decision), the `link` set shrinks to two.** `LinkType::Inline → Static` (the verbatim `STATIC.md` lane — "the static compiler"), `Static → Dynamic` (the default, a by-reference `INDEX.md` read with an optional `when`), and the old `Dynamic` removed — a conditional load is now just a `dynamic` entry carrying a `when`. `inline-transitive → static-transitive`; `INLINE.md → STATIC.md`; `render_inline → render_static`; `compile_inline → compile_static`. Pure terminology, aligning vibevm with the CS static/dynamic-linking standard so "the static compiler" reads naturally; shipped across `vibe-core`, `vibe-workspace`, `vibe-spec`, the package manifests, and these specs.
