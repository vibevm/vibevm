# PROP-035: The spec compiler — directive preprocessor, package formats, and the two-mode boot linker {#root}

<status stage="impl" state="done" action="continue" comment="C 2026-07-25: §5-§13 shipped as vibe-spec (07-15), rename (07-16), normal+static AOT end to end (07-20); the §13 JIT loader and §10 link tables remain"/>

@fact:status-line **Status: IMPLEMENTED** (designed 2026-07-14 at the owner's request — the flagship "static-compiler vision"; verified against the tree 2026-07-25 by the spec-actualization campaign). §17 records the compiler shipping three times over: §5–§13 as the `vibe-spec` crate wired into `bootgen` (2026-07-15), the link-type rename (2026-07-16), and `normal + static` compiled end to end with the `link × format` question resolved as eager AOT (2026-07-20). **What remains:** the structural / JIT loader of §13 (`normal + dynamic`) and the §10 link tables, both still marked *(provisional)*. @status:impl/done

@fact:extends **Extends:** [PROP-009](PROP-009-loading-model.md) (the loading model — inclusion types, the two-tree model, the `STATIC.md` / `INDEX.md` artifacts). This PROP turns PROP-009's "ordered list of contributions" into a real **preprocessor + linker**. @status:spec/done

@fact:supersedes-line **Supersedes / folds in:** [PROP-034](PROP-034-transitive-links-boot-graph.md) (transitive links + the static boot-link graph). PROP-034's linker becomes the *emission layer* of this system (§12); PROP-034 is retained as the narrower, already-drafted contract for that layer until this PROP is ratified. @status:spec/done

@fact:related **Related:** [PROP-028](../../common/PROP-028-package-families.md) (families — the aggregator role), [PROP-029](../../common/PROP-029-fully-qualified-addresses.md) (`spec://` addressing, the `/` group↔name joiner), [PROP-008](../vibe-registry/PROP-008-qualified-naming.md) (pkgref grammar `kind:group/name@version`), the `addressable-specs` flow (anchor / section grammar), [`spec/design/loading-and-boot-model.md`](../../design/loading-and-boot-model.md) (the static/dynamic-linking metaphor this PROP completes), [PROP-014 specmap](../../../packages/org.vibevm.ai-native/core-ai-native/v0.7.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md) (the `uri↔file` index the router extends). @status:spec/done

---

## 1. Motivation — boot loading must become a toolchain {#motivation}

- @fact:frame-established PROP-009 established the frame: **installing a dependency must never modify authored spec** — "the C++ rule that you do not paste a header's text into your `#include`" (PROP-009 §1) — and gave each direct edge an inclusion type (`static` / `dynamic`, PROP-009 §2.4). @status:impl/done
- @fact:predecessors-limits PROP-034 added transitive links and pinned the boot as a static-linked, deduplicated, topologically-ordered graph. Both are correct and both are **package-granular and directive-free**. @status:impl/done

@fact:gaps-lead Three things they do not yet give: @status:impl/done

1. @fact:GAP-PREPROCESSOR **A preprocessor.** There is no way for one spec to say "splice this exact section here" (`#embed`), "I depend on this — pull it, in order, before me" (`#use`), or "this contract is implemented over there" (`#source`). Cross-spec composition is done today by prose citation, which the loader does not act on. @status:impl/done
2. @fact:GAP-RESOLVER **A resolver.** Every directive takes a `spec://` address, and the static compiler must turn it into a concrete file/section **algorithmically, without an LLM**. The codebase has no `spec:// → path` function today — resolution is the *inverse* (a filesystem scan mints `path → URI` into `specmap.json`). See §6. @status:impl/done
3. @fact:GAP-SECTION-GRAIN **Section granularity.** Real economy needs the unit of loading to be a **section of a document**, not a whole package or file. PROP-034's graph is over packages; the cascade this PROP needs (`a` pulls one file of a big package, which pulls the next) requires a graph over document **sections** (§5). @status:impl/done

- @fact:context-budget-stake The stake is the context budget. Loading vibevm itself already spends ~10% of a 1M window; an operating-system-scale project authored in Spec-Driven Development does not fit at all if every dependency loads whole. @status:spec/done
- @fact:TWO-MODE-COMPILER This PROP makes boot a **two-mode compiler**: an algorithmic *static* compiler that statically links a whole closure into one file, and a *structural* loader that reads only what is actually used, lazily, in dependency order — the same economy delegation-first buys for *work*, bought here for *loading*. @status:impl/done

---

## 2. Two build modes — static vs structural (AOT vs JIT) {#modes}

@fact:ONE-SEMANTICS-TWO-EXECUTORS The system is one directive semantics with two executors, exactly the GraalVM / Project Leyden split the owner names: @status:impl/done

- @fact:MODE-STATIC **Static build** — packages are concatenated into one (or few) files (`STATIC.md`). Directives are resolved **statically, by code, without an LLM**. This is the AOT / devirtualized path: no runtime indirection, the agent reads a finished file. @status:impl/done
- @fact:MODE-STRUCTURAL **Structural build** — the agent reads specs **on demand**, following directives as it meets them. This is the JIT / late-binding path; it subsumes PROP-009's current `static` and `dynamic` modes. @status:spec/done

- @fact:EQUIVALENCE-INVARIANT **The equivalence invariant.** Both executors MUST produce the *same effective spec* — as AOT and JIT must run the same program. The **static compiler is the reference semantics**; the structural loader (a prompt today, a hard algorithmic agent later, §13) is checked against it. @status:impl/done
- @fact:DIFF-TEST-DEFERRED Differential testing of the two is real, empirical, and expensive; it is **deferred and planned separately** (§16), not part of the first build. Until then, the structural side is best-effort and the static side is authoritative. @status:spec/done

@fact:contract-lead Two executors, one contract: @status:impl/done

- @fact:EXEC-STATIC **Static compiler** — code, fully algorithmic. Buildable now (§8). @status:impl/done
- @fact:EXEC-STRUCTURAL **Structural loader** — a set of first-loaded instructions (§13) that make the agent honour the directives, pending the future algorithmic agent. @status:spec/done

---

## 3. Package formats — `simple` and `normal` {#formats}

@fact:FORMAT-FIELD A new `vibe.toml` `[package]` field `format`, alongside `version`: @status:impl/done

- @fact:FORMAT-SIMPLE **`format = "simple"`** — **the default** (absent `format`, a package is `simple`). Legacy / adapted prompts, carried **whole**, with no VibeVM-specific structure — for importing existing corpora without rewriting them, and the fail-safe posture. Rules: inclusion in `[requires.packages]` means (a) structural — the agent reads the file; (b) static — its text is compiled into the target. If `[boot_snippet].source` names a file, only that file is read/spliced; **absent even that, every file in the package is read/spliced by a recursive walk** — the over-load is the author's problem, the deliberate cost of not adopting `normal`. @status:impl/done
- @fact:FORMAT-NORMAL **`format = "normal"`** — the VibeVM-native form, **opt-in**: the `contract` / `source` split (§4), directives (§7), and the compiler (§8). A `normal` package is **not read just because it is present** — it participates only when something actually `#use`s it (§7.2). This is tree-shaking; the optimized posture for authors who understand the machinery, at the price of structuring the package correctly. @status:impl/done

- @fact:SIMPLE-DEFAULT-WHY **Why `simple` is the default (owner decision, 2026-07-15).** A forgotten `format` must **fail safe, not silent**. @status:impl/done
- @fact:fail-safe-argument With `normal` as the default, a naive or mis-built package that nobody `#use`s loads **nothing** — a silent no-op, the worst failure. With `simple` as the default it loads **everything** — noisy and unoptimized, but visibly working; the author opts into `normal` and its discipline deliberately. @status:spec/done
- @fact:migration-non-event Migration (§15) is thereby a non-event: the existing corpus keeps working as `simple`, and packages convert to `normal` one at a time as their authors optimize them. @status:spec/done

---

## 4. Normal packages — the contract / source split {#contract-source}

@fact:hpp-cpp-inspiration Inspired by C/C++ `.h` / `.cpp`: @status:impl/done

- @fact:DIR-CONTRACT **`contract/`** — small, simple, boot-snippet-like. The surface a package exposes outward; short files, cheap to load. The analogue of a header. @status:impl/done
- @fact:DIR-SOURCE **`source/`** — large, heavy. The full implementation; pulled only when actually needed. The analogue of a translation unit. @status:impl/done

- @fact:SOURCE-HACK Because the structural executor lacks a C++ compiler's global view of the source tree, we use a deliberate hack: **the contract author declares what implements it, via `#source`** (§7.3). The author hand-draws edges a globally-aware compiler would infer. @status:impl/done
- @fact:link-tables-give-back That global view *does* exist in static mode and can be pre-compiled into **link tables** (§10), which is how we give the same knowledge back to the structural executor cheaply and make the hand-drawn `#source` edges verifiable. @status:spec/work

@fact:CONTRACT-IS-SURFACE The `contract` surface is what other packages include; `source` is reached only through a resolved contract edge or an explicit `#use`/`#embed` into it. @status:impl/done

---

## 5. The document IR — one hierarchical tree, two frontends {#ir}

@fact:DOCUMENT-IR Everything downstream operates on a single **document IR**: a DOM-like tree. Markdown and (future) XML are two frontends parsed into the same tree, so algorithms written against the IR scale to deeply nested XML for free. @status:impl/done

- @fact:IR-NODE **Node** = `{ id (anchor / tag), depth, kind, body-span, children[] }`. @status:impl/done
- @fact:IR-MD-FRONTEND **Markdown frontend.** Headings form the tree by level (`#` ⊃ `##` ⊃ `###`). A node's `id` is its explicit `{#anchor}`. A node's **body span runs from its anchored heading to the next heading of the same or higher level** (the owner-fixed rule); its children are the nested headings inside that span. @status:impl/done
- @fact:IR-FACT-LEAVES **Fact leaves** *(fact amendment, owner-ratified 2026-07-24)*. A `##<ID>` first token of a paragraph or list item (the fact unit of PROP-014 §2.1 / PROP-043 §3.8) is a **leaf node** of the IR: `kind = fact`, `id = <ID>`, body-span = the carrying paragraph or item with its continuation lines; its parent is the enclosing section node. Fact ids share the document's one anchor namespace. The resolver (§6) resolves a fact address like any node; `#embed` of a fact splices exactly its unit (§7.1's arbitrary granularity, unchanged); `#use` of a fact address pulls the top-level anchored ancestor of its **enclosing section** (the existing rule, unchanged). @status:impl/done
- @fact:IR-XML-FUTURE **XML frontend (future).** Elements already are the tree; `tag`/`id` is the address. Held for later, but the IR is designed for it now. @status:spec/done
- @fact:IR-ADDRESS-DEPTH **Addressing depth.** A `spec://…#a.b.c` fragment is a **path down the tree** (`a` → `b` → `c`), which already matches addressable-specs' dotted anchors (`#verification.timeout`). Sections at any depth are addressable. @status:impl/done

@fact:granularity-rules-lead **Granularity rules** (owner-set), stated over the IR: @status:impl/done

- @fact:USE-ANCESTOR-RULE **`#use` pulls the whole top-level anchored ancestor** of the addressed node — reference a subsection, load its enclosing top-level section as one connected block. It does **not** pull that ancestor's siblings; siblings are read only when themselves needed. @status:impl/done
- @fact:EMBED-EXACT-RULE **`#embed` has arbitrary granularity** — it splices exactly the addressed node, no more. @status:impl/done
- @fact:ANCESTOR-PARAMETER "Top-level anchored ancestor" is a **parameter of the resolver**, not a hardcoded heading level, so the XML frontend can define it structurally. @status:impl/done

---

## 6. `spec://` addressing and the resolver ("router") {#addressing}

- @fact:FORMAT-CHOICE **Format choice.** `spec://` stays. Alternatives are strictly worse: path-based is fragile across `packages/`→`vibedeps/` materialization and drops versions; content-addressed (by hash) is unreadable and cannot "name a section"; query-based (by tag) is non-deterministic (may match N nodes) where a preprocessor needs exactly one. `spec://` is already symbolic, human-readable, and carries `group`/`name`/`path`/`anchor`. The gap is not the format — it is the **missing resolver**. @status:impl/done

@fact:UNIFIED-GRAMMAR **Unified grammar** (reconciled with the pkgref grammar of PROP-008): @status:impl/done

```
spec://<group>/<name>[@<version>]/<doc-path>#<anchor>[.<sub>…][~r<N>]
```

- @fact:URI-JOINER `group` ↔ `name` joiner is **`/`, never `.`** (PROP-029). @status:impl/done
- @fact:URI-VERSION-OPTIONAL `@<version>` is **optional — a feature, never an obligation (owner-ruled 2026-08-04, B-028: «указание версий — опциональная фича; если версия не указана — используется самая свежая»)**; absent, the address resolves against the **freshest installed version** of the package (semver-newest among the materialised `vibedeps/` slots — the one deterministic offline reading of «самая свежая»). Several coexisting slots are therefore not an ambiguity but an ordered set with a defined maximum; an explicit `@<version>` still picks its exact slot, including a non-newest one. @status:impl/done
- @fact:URI-TREE-PATH `#<anchor>.<sub>…` is a **tree path** into the document IR (§5). @status:impl/done
- @fact:URI-REVISION-PIN `~r<N>` pins a spec-unit revision (PROP-014), not a package version. @status:spec/done

@fact:THE-ROUTER **The router** is the new component: a deterministic function `spec:// → IR node`, evaluated over the resolved, materialized tree. It is the prerequisite everything else stands on. It must handle, without an LLM: @status:impl/done

- @fact:ROUTER-SELF-COORDINATE **The self coordinate (B-031, owner-approved 2026-08-04)** — the root project's own `<group>/<name>` (declared in `[project]`, e.g. `org.vibevm.core/vibevm`) resolves to the workspace's **authored `spec/` tree**, matched before any `vibedeps/` slot lookup and never versioned; an undotted authority (the retired host token, illustrative fixtures) parses but never resolves — a hard error carrying the rename hint. @status:impl/done
- @fact:ROUTER-DOC-ID **Doc-id truncation** — `PROP-NNN` / `FEAT-NNN` in a URI resolve to `PROP-NNN-<slug>.md`; other docs use the full stem. (This is `canonical_doc_path` in the specmap engine, reused, not reinvented.) @status:impl/done
- @fact:ROUTER-VIBEDEPS **`packages/` (source) vs `vibedeps/` (materialized slot)** — the compiler resolves against the **materialized `vibedeps/` tree** (the installed reality), consistent with the specmap engine, which never scans `packages/`. @status:impl/done
- @fact:ROUTER-VERSION **Version selection when `@version` is absent — the freshest installed** (semver-newest slot; owner-ruled 2026-08-04, B-028, superseding the earlier lockfile wording). A lockfile-informed selection layer above the router remains possible machinery, but the router's own default is the newest materialised slot — deterministic over the installed set. @status:impl/done

- @fact:ROUTER-DETERMINISM Determinism is a hard requirement: the static compiler must resolve every address to exactly one node or fail loudly. @status:impl/done
- @fact:ROUTER-EXTENDS-SPECMAP The router extends the `specmap.json` index (already a `uri↔file` table) rather than duplicating it. @status:impl/done

---

## 7. The three directives {#directives}

- @fact:DIRECTIVE-SHAPE Common shape: `#<directive> [options] <specpath>`, where `specpath` is a `spec://` address (§6) — a section, or a whole file. @status:impl/done
- @fact:DIRECTIVE-MANIFEST-AGREE Every file referenced by any directive MUST be declared in the package's `vibe.toml` (the directive graph and the manifest cannot disagree). @status:impl/done
- @fact:DIRECTIVES-FIRST-LOADED The directive instructions are among the first things loaded, in every project, package, and library (§13). @status:spec/done

### 7.1 `#embed` — the macro (materialization-time) {#embed}

@fact:EMBED-DEF The simplest preprocessor directive: splice one section (or whole file) into another — a C-macro substitution over `spec://` addresses. @status:impl/done

- @fact:EMBED-FIRES **Fires at materialization** (`vibe install` into `vibedeps`) **and** must be fully expanded during `STATIC.md` compilation. **No unexpanded `#embed` may survive in a compiled `STATIC.md`.** @status:impl/done
- @fact:EMBED-MODE-INDEPENDENT **Mode-independent.** `#embed` is purely a materialization concern — it produces the same text in both build modes. Therefore `vibedeps` stores embeds **already expanded**, while `#use`/`#source` (mode-dependent) remain as directives. This split removes a large part of the ordering problem: embeds are fixed early and identically for both executors. @status:impl/done
- @fact:EMBED-CONTRACT-RULE **Contract-section rule.** An `#embed` targeting a `contract`-section of a `normal` package inherits the full **`#source` compilation rules** (§7.3) for that paragraph — so the merged (contract+source) text is what gets spliced. @status:impl/done
- @fact:EMBED-UNRESTRICTED **Unrestricted otherwise.** `#embed` may splice any paragraph of any spec, or a whole spec (including `format = "simple"`), with no structural checks. @status:impl/done
- @fact:EMBED-PURPOSE **Arbitrary granularity** (§5). Algorithmic; the tool of building highly-optimized statically-assembled package hierarchies — its main purpose. @status:impl/done

### 7.2 `#use` — the dependency edge (tree-shaking) {#use}

@fact:USE-DEF The harder directive: pull only the packages actually needed. @status:impl/done

- @fact:use-problem **Problem it solves.** Specs refer to entities defined elsewhere, often without an explicit link, and linking constantly is tedious. Without `#use`, the smallest script would have to pull the whole standard library. @status:spec/done
- @fact:USE-TREE-SHAKING **Tree-shaking default.** A `format = "normal"` package that nobody `#use`s does not participate — not read, not used, anywhere. The moment any text does `#use spec://…`, that package **enters the build** and MUST be linked **before** its user in topological order. @status:impl/done
- @fact:USE-STRUCTURAL **Structural mode.** `#use` is an instruction to read the target's content **when it is needed** — not eagerly, but definitely **before using anything inside it**. Reads **cascade**: `a` `#use`s `b`, `b` `#use`s `c`, so using `a` loads `b` and `c` transitively. The cascade is what lets a big package be entered through one file and expand from there, rather than loaded whole. @status:spec/done
- @fact:USE-INLINE **Inline mode.** The same, statically: the `#use`d library's text is **fully copied higher up in `STATIC.md`** so it is available before the user. @status:impl/done
- @fact:USE-GRANULARITY **Granularity.** Pulls the whole top-level anchored ancestor of the addressed node (§5). @status:impl/done
- @fact:USE-CONTRACT-RULE **Contract-section rule.** A `#use` into a `contract`-section of a `normal` package inherits the `#source` rules (§7.3). Otherwise `#use` may pull any paragraph or whole spec (even `simple`) with no structural checks. @status:impl/done
- @fact:USE-AS-ALIAS **The `as` clause (B-011, owner-approved 2026-08-04).** `#use [options] <specpath> as <Alias>` additionally binds `<Alias>` — an identifier under the anchor-segment grammar (§6) — to the directive's address. The alias is **file-scoped** (visible throughout the declaring file regardless of declaration position, never exported, never inherited by `#use`-ers); two declarations of one alias name in one file are a compile error. The alias binds to the **address**, never to any compiled text — so it survives splicing and any later cleaning of the compiled lane. A trailing `as` clause is ignored by pre-B-011 scanners (the token tail after the address was never parsed), so the clause is backward-compatible by construction. @status:spec/work

### 7.3 `#source` — contract↔implementation virtual linking {#source}

@fact:SOURCE-DEF Like a C++ interface, but with section-level merging. `contract` sections are the exposed surface; `#source` names the file(s) that implement them. Sections are treated as the analogue of class methods, needing a merge (in the static build) or virtual-lookup (in structural mode) mechanism. @status:impl/done

@fact:merge-algorithm-lead Merge algorithm, per section (by matching `{#tag}`): @status:impl/done

1. @fact:MERGE-CONTRACT-ONLY **In contract, absent in source** — structural: a full part of the spec, readable at will; static: compiled into the build whole. @status:impl/done
2. @fact:MERGE-SOURCE-ONLY **In source, absent in contract** — always counted; structural: readable at will; static: compiled in whole. *(Calling a section that exists only in the implementation is poor taste, but permitted — we deliberately impose no `private`/`public` access control.)* @status:impl/done
3. @fact:MERGE-BOTH **In both, same `{#tag}`** — merged by the tag's mode: @status:impl/done
   - @fact:MERGE-REPLACE **`:replace`** — `# name {#tag} :replace` — the contract text is ignored; the source text is canonical (read by the agent / put in the static build; an already-read contract text is explicitly superseded). @status:impl/done
   - @fact:MERGE-ADD **`:add`** — `# name {#tag} :add` — the result is the **sum**: contract text first, then source text. Static: compile the concatenation. Structural: the agent reads both and weights them equally. @status:impl/done
   - @fact:MERGE-DEFAULT-ADD **Default is `:add`** (absent a `:`-suffix) — so the interface text need not be duplicated to appear in the result. @status:impl/done

@fact:fact-inheritance-lead **Fact inheritance** *(fact amendment, owner-ratified 2026-07-24 — closes F-022)*: @status:impl/done

1. @fact:FACT-SECTION-FATE **Section fate by default.** Facts ride their section: `:add` carries both sides' facts into the sum; `:replace` supersedes the contract's facts — text and fact anchors together. @status:impl/done
2. @fact:FACT-OVERRIDE **Per-fact override.** Within a merged `:add` section, a source fact redeclaring a contract fact's `##<ID>` **overrides** it: the contract fact's span is dropped from the merged output and the source's is canonical (last-wins in contract→source order). One id, one unit — redeclaration IS the override gesture, so refining a single statement never requires `:replace`-ing the whole section. @status:impl/done
3. @fact:FACT-MERGED-UNIQUENESS **The merged view holds uniqueness.** After merging, the compiler re-runs the anchor-uniqueness check — fact and heading ids, one namespace — over the merged document; a surviving duplicate (a non-override collision across sections, or fact-vs-heading) is a **build error**, never a warning. Per-file cleanliness of the inputs does not exempt the merged output. *(Precision, from the implementation review 2026-07-24: the gate flags a repeat only when at least one occurrence is a fact leaf — a pure heading-vs-heading repeat is the `:add` concatenation's own artifact (both section versions legitimately carry the same `{#tag}`), not a collision.)* @status:impl/done

@fact:sequence-lead **Several sources, and the plugin form** *(B-056, owner-ruled 2026-08-04, built 2026-08-05)*: @status:impl/done

1. @fact:SOURCE-SEQUENCE **A contract may declare more than one `#source`, and every one is honoured, in declaration order.** The algorithm above reads "the source" as that sequence: for an anchor `a`, let `S(a)` be the sub-sequence of sources carrying a section at `a`, in the order the directives were written (a glob contributing its members sorted). Before this the compiler took the FIRST `#source` and dropped the rest without a word — the silence B-055 (closed by `bc88e530`) recorded. @status:impl/done
2. @fact:SOURCE-REPLACE-IS-A-FLAG **`:replace` from ANY source discards the contract text only; the sources still sum among themselves, in order.** Over a sequence `:replace` stops being "whose text is canonical" and becomes a flag, after which two sources both carrying it is not a conflict to adjudicate. With one source the result is byte-identical to ##MERGE-REPLACE, so the generalisation is backward compatible. @status:impl/done
3. @fact:SOURCE-FACT-OVERRIDE-IS-A-UNION **Per-fact override widens to the union over `S(a)`.** A contract fact is dropped when ANY member redeclares its id — one id, one unit, and whoever redeclares it takes it. Two sources redeclaring one id is not a fold question: both survive into the merged text and @fact:FACT-MERGED-UNIQUENESS fails on the duplicate, loudly. @status:impl/done
4. @fact:SOURCE-ONLY-IS-A-DEFINITION **A source-only section declared by two sources is an error, and the asymmetry is deliberate.** A source section matching a contract anchor is an *addition* to something already declared, so summing is right; a source-only section is a *new declaration*, and two of them are two definitions of one name — declaration is idempotent, definition is not (the ODR parallel §9 draws). **Where it is caught matters:** @fact:FACT-MERGED-UNIQUENESS sees a document whose provenance is already gone and deliberately tolerates a repeated heading (that shape is the legitimate `:add` artefact), so the collision is judged in the fold, which still knows which source brought what. @status:impl/done
5. @fact:SOURCE-RECURSION **The fold follows `#source` recursively under §9's cycle law, and it carries an include guard.** A source that itself declares `#source` folds before it merges into its parent; a cycle among contracts is legal and its not-yet-folded member contributes nothing (the forward declaration); a cycle touching an implementation is a build error naming the path. This is @fact:NO-DEADLOCK-INVARIANT reaching the fold — the same walker, one more edge set, never a second traversal. **The guard is not bookkeeping:** visiting a shared node once does not make its TEXT appear once, because the fold is textual inclusion — without a guard a diamond carries the shared body in along both paths, and a shared source declaring a fact then fails the uniqueness check, turning an ordinary composition into an un-buildable error. A node's text therefore enters the composed document exactly once, by the first path in fold order. @status:impl/done
6. @fact:SOURCE-GLOB **The plugin form.** A `#source` address may carry a `*` in the package-NAME half; it names every installed package whose name matches AND that carries the addressed document, expanded in sorted order — so the composed document is a pure function of (tree, lockfile). A glob matching nothing is a legal empty set, never a missing source; a pointed address still fails loudly when its target is absent. Both the fold and the cycle guard read a document's `#source` edges through one function, so the graph the guard judges is the graph the fold walks. @status:impl/done

### 7.4 In-place use — the `@spec://` sigil {#in-place-use}

@fact:in-place-lead Implicit dependency without an explicit `#use`, made precise by a sigil: @status:impl/done

- @fact:AT-SPEC-MANDATORY **`@spec://…`** (an `@` before `spec`) is an **in-place use**: the agent MUST read it (mandatory), exactly as if a `#use` had been declared at the top of the file. @status:impl/done
- @fact:BARE-SPEC-DISCRETIONARY **Bare `spec://…`** (no `@`) is at the agent's discretion — read it if useful, skip it otherwise. (A future algorithmic agent narrows this further.) @status:impl/done
- @fact:READ-ONCE **Read once.** An `@spec` target is read only on **first** encounter, never re-read blindly — see the read-set (§below). @status:impl/done
- @fact:IN-PLACE-RESOLUTION-ORDER **Resolution order.** Collect all explicit `#use` into a map, enrich it with the `@spec` in-place uses, then act on the combined map. @status:impl/done
- @fact:AT-BANG-IS-AN-ALIASED-IN-PLACE-USE **`@!<Alias>` (B-011, owner-approved 2026-08-04)** is the aliased twin of `@spec://…`: a **mandatory** in-place use of the address the file's `as`-declared alias binds to (§7.2), with identical read-once / read-set semantics. `@!X` where `X` is not declared in the file is a compile error naming the file's known aliases. In the compiled static lane every `@!X` is rewritten to the full `@spec://…` address it denotes — the compiled output is self-describing without the alias table and resolvable after any cleaning; structural mode reads the declaration and the sigil directly. @status:spec/work
- @fact:SHORT-LABEL-LOOKUP-IS-TWO-SCOPE **The short-reference lookup rule (B-011).** An *unqualified* label reference (an intra-document `(#x)` link) resolves against exactly two scopes, in order: (1) the anchor namespace of the containing document; (2) the file's declared aliases. Found in neither, or in both: a **compile error listing the candidates** — the resolver never widens the search and never picks silently. @status:spec/work

- @fact:READ-SET **The read-set (surviving compaction).** To honour "read once" across an agent's context compaction, a persistent, file-based **read-set** (`.vibe/session/read-set.json` or equivalent) records `{ specpath, content_hash }` on each read; the `content_hash` is reused from specmap, so a *changed* section is re-read. @status:spec/done
- @fact:read-set-boot-instruction A first-loaded instruction (§13) tells the agent to consult the read-set before reading and append after — this survives compaction because the boot instructions are re-read. @status:spec/done
- @fact:read-set-records-existence Crucially the read-set records *what exists and where*, **not what is currently in context**: compaction evicts the *text* but not the *fact*, and reads are cheap (files sit in `vibedeps`). @status:spec/done
- @fact:READ-SET-RULE So the rule is: read an `@spec` target if (a) it is not in the read-set, **or** (b) it is, but its content is no longer in context. @status:spec/done
- @fact:read-set-floor A clean solution without a harness compaction signal is out of scope; the file-based read-set plus a boot instruction is the pragmatic floor, and the future algorithmic agent keeps the read-set rigorously. Mental model: a **linker symbol table, but for what has been read**. @status:spec/done

---

## 8. The compilation pipeline — the standard order {#pipeline}

@fact:pipeline-lead The single ordering standard both executors follow (the "procedure of macro-substitution" the owner asked to pin down): @status:impl/done

1. @fact:PIPE-PARSE **Parse.** Build the IR tree (§5) of every participating file; collect all directives with their positions. @status:impl/done
2. @fact:PIPE-USE-GRAPH **Build the use-graph and topologically sort.** Nodes = document sections (or packages, at the coarser tier); edges = explicit `#use` + `@spec` in-place uses (§7.4) + implicit references. Topological order = every dependency before its dependent. @status:impl/done
3. @fact:PIPE-SOURCE-MERGE **Source-merge.** For every `contract` section, resolve `#source` (§7.3) into its effective (merged) body. This runs **before** embed because an embed may target a merged contract section (§7.1). @status:impl/done
4. @fact:PIPE-EMBED-EXPAND **Embed-expand.** Apply `#embed` (§7.1) as textual substitution, **top-down within a file**, in **topological order over the embed-graph across files** (a package is fully compiled — its source-merge and its own embeds done — before it is embedded), **recursively to a fixed point**, with cycle guards (§9). @status:impl/done
5. @fact:PIPE-QUALIFY **Qualify (B-011, owner-approved 2026-08-04; per-node refinement — the B-006 rider, owner-approved 2026-08-04).** For the static build only: rewrite label definitions — heading `{#x}` and fact `##X`, one namespace (§7.3) — to the qualified form `<origin-slug>--<original>` (the slug is the lowercased `<group>/<name>` with dots and the joiner mapped to `-`/`--`; the original tail keeps its case), **per node, each node under its own authoring origin**: a compiled closure that splices cross-origin nodes qualifies every node under the origin that authored it, never the carrying entry's — provenance is never re-attributed by splicing. An intra-closure `(#x)` link rewrites to the qualified name of the **defining** node (so a cross-origin link stays resolvable under the two-scope lookup); a `simple` contribution is one node, where this reduces to B-011's original whole-contribution rewrite. Full `spec://` addresses, `@spec` uses, directive lines, and fenced content are never touched. The qualified name is a pure function of `(origin, original label)` — independent of splice order and composition, which is what makes late lane additions append-only. `@!X` rewrites to its full address here (§7.4). @status:spec/work
6. @fact:PIPE-EMIT **Emit.** Concatenate in topological order with open/close markers (§11), prefixed by the resolution preamble and the tombstone table (§11). For the static build: `STATIC.md`. For structural: the loader consults the same order lazily. @status:impl/done

@fact:PIPE-DETERMINISM Determinism: independent nodes are tie-broken by a stable key (category → boot-snippet slot → fully-qualified name), as PROP-034 §2.3 already specifies for the emission layer. @status:impl/done

---

## 9. Cycles and guards {#cycles}

@fact:cycles-intuition The owner's C++ intuition made precise. In C++ two distinct mechanisms are at work: an `#include` cycle is broken by **include guards** (idempotent re-inclusion becomes a no-op), and mutual recursion of *types* is resolved by **forward declaration** — a *declaration* needs no *definition*. "Including only interfaces never deadlocks" precisely because declarations can close a cycle without bodies. Mapped onto us: @status:impl/done

- @fact:EMBED-CYCLE-ERROR **`#embed` cycle → hard error.** `#embed` is `#include` without a guard, so a cycle is an infinite substitution. We **forbid** it: a guard keyed on the `specpath` currently on the expansion stack detects it, aborts compilation, and emits **debug info naming the full cycle path** (`A → B → A`). (Owner-required: the guard's firing is reported, not silent.) @status:impl/done
- @fact:USE-CYCLE-ALLOWED **`#use` cycle between contracts → allowed** (the forward-declaration case). Because a contract is small and self-contained, static mode breaks the cycle by **emitting the contracts before any source bodies**. Structural mode is "read when needed", so a contract-level cycle simply resolves lazily. @status:impl/done
- @fact:USE-BODY-CYCLE-ERROR **`#use` cycle that needs a source body to compile itself → error** (the "incomplete type where a complete type is required" case). @status:impl/done

@fact:NO-DEADLOCK-INVARIANT **Invariant.** The **contract layer is where cycles are legal; the source layer is where topological order is mandatory.** This is the theoretical no-deadlock guarantee: as long as the contract hierarchy is acyclic-under-`#embed` and no source body participates in a `#use` cycle, the build always terminates. @status:impl/done

---

## 10. Link tables — the vtable analogue *(provisional)* {#link-tables}

@fact:LINK-TABLE-ANALOGY The owner's C++-virtual-dispatch analogy, held for the implementation task. Inline mode ≈ a non-virtual / devirtualized call (bound statically, no runtime indirection); structural mode ≈ a virtual call (late-bound at runtime); a **link table ≈ a vtable** — a table the compiler builds once so the runtime dispatches cheaply instead of searching. @status:spec/work

@fact:link-tables-build-lead Concretely: at **install-time** (or a dedicated compile phase) build, by code, the graph edges the structural executor otherwise lacks — @status:spec/work

- @fact:TABLE-ANCHOR-INDEX an **anchor-index** per document (the IR tree, addressable), @status:spec/work
- @fact:TABLE-CONTRACT-SOURCE a **contract→source map** (the real edges behind every `#source`), @status:spec/work
- @fact:TABLE-USE-GRAPH the **use-graph**, @status:spec/work

@fact:link-tables-persist and persist them to a file table (a sibling/extension of `specmap.json`). The structural agent then **consults a cheap on-disk table instead of building the graph in context** — which directly answers the objection that the structural executor "lacks global knowledge because the project is too big for the agent's context": the edges are built by the compiler, not the agent. A bonus: hand-drawn `#source` edges become **verifiable** — the table knows the real edges and can flag divergence. This reuses the specmap infrastructure rather than adding a parallel one. Kept provisional and folded into the implementation task per the owner. @status:spec/work

---

## 11. Markers in compiled output {#markers}

- @fact:OPEN-CLOSE-MARKERS When a file's text is placed into a static file (e.g. `STATIC.md`), a path comment is added **both before and after** it (today only *before* — the after-comment is the closing tag). Around a package body (which contains several files) the same: a package-open comment, then many file open/close comments inside, then a package-close comment. @status:impl/done
- @fact:STATIC-REVERSIBLE This makes `STATIC.md` **reversible** — a compiled artifact can be decomposed back to its constituent files and packages, giving the same bidirectional traceability specmap already provides for code. @status:impl/done
- @fact:COMPILED-LABELS-ARE-QUALIFIED **Compiled labels are origin-qualified (B-011).** A compiled block's heading anchors and fact ids carry the §8 qualify phase's `<origin-slug>--` prefix, so the compiled document's label namespace is collision-free by construction; reversibility survives because the block's own marker key names the origin, and stripping that block's prefix restores the source labels. @status:spec/work
- @fact:STATIC-HEADER-RESOLUTION-PREAMBLE **The header carries the resolution preamble, first (B-011, owner addition 2026-08-04).** The compiled lane opens with a short generated preamble — the qualified-label convention, the alias semantics, the two-scope lookup rule, «unresolved short name → the tombstone table below», and «full addresses resolve against package sources, never against this file» — placed as the first lines of the first file a session reads, because boot files are re-read every session and after compaction, which is what makes the rules un-forgettable. @status:spec/work
- @fact:STATIC-TOMBSTONE-TABLE **The tombstone table sits directly under the header.** Every short name the qualify phase renamed is listed with its qualified heirs and their origins — a retired name never vanishes silently (the addressable-specs tombstone law applied to renames). A resolver that misses a short splice anchor answers with these candidates, never with emptiness. @status:spec/work
- @fact:COMPILED-LANE-IS-NOT-A-CITATION-TARGET **The compiled lane is not a citation target (B-011, §6.1 layer 1 of the design).** A `spec://` address whose document path names a generated `STATIC.md` is an illegal target for authored text — the lane is a cache; source-of-truth is the package source. The directive compiler rejects such an address; the gate lints the tree for them. @status:spec/work

---

## 12. Transitive static — folding in PROP-034 {#transitive-inline}

- @fact:STATIC-TRANSITIVE-FOLD `static-transitive` may be set at the top of a package hierarchy; then every element below it in the dependency graph is pulled `static`, **regardless of what it declared before**. @status:impl/done
- @fact:transitive-safety This is safe precisely because the static build is algorithmic, not LLM-driven, and loses nothing. It is the path to large highly-optimized builds. @status:impl/done

- @fact:EMISSION-LAYER This is PROP-034 §2.1/§2.3 (transitive links + dedup + topological order + cycle rejection), which becomes the **emission layer** of this compiler: after §8's pipeline resolves directives, PROP-034's linker deduplicates and dependency-orders the node list into `STATIC.md` / `INDEX.md`. @status:impl/done
- @fact:transitive-variants-reserved `transitive-static` / `transitive-dynamic` remain reserved (no use case yet), but the graph analyzer is built to operate at that level. @status:spec/done

---

## 13. The structural loader — the "first instructions" {#loader-prompt}

- @fact:LOADER-LLM-EXECUTED Until hard algorithmic agents exist (§14), structural mode is executed by an LLM following instructions. @status:spec/done
- @fact:LOADER-FIRST-EVERYWHERE Those instructions — how to honour `#use`, `#embed`, `#source`, `@spec`, and the read-set — MUST load **first, everywhere**: in every project, package, and library vibevm manages. @status:spec/done
- @fact:LOADER-BROKEN-WITHOUT A project or package **without** them is considered **broken**; the project- and package-creation tools MUST check for and inject them. This is one of the most critical loading mechanisms — nothing works without it. @status:spec/done
- @fact:LOADER-RESOLUTION-RULES **The resolution rules are part of the first instructions (B-011, owner addition 2026-08-04).** The first-loaded text includes the B-011 resolution rules: qualified-label convention and short↔qualified derivation, alias (`as` / `@!`) semantics, the two-scope lookup with fail-with-candidates, the tombstone table's role, and «full addresses resolve against package sources under `vibedeps/`, never against a compiled lane». B-011 wires this contract into a live boot for the first time — until it lands, the loader instructions remain the `structural-loader.md` hold. @status:spec/work
- @fact:inline-tooling-note Inline compilation, by contrast, needs no LLM and can build the whole thing algorithmically today; its tooling is what remains to be built (the current `STATIC.md` machinery is naive by comparison). @status:spec/done

---

## 14. Future algorithmic agents {#future-agents}

- @fact:FUTURE-ALGORITHMIC-AGENTS We are preparing for purpose-built algorithmic agents that run alongside Claude Code (and in specific cases instead of it) and honour every directive (`#use`, `#embed`, `#source`) rigidly and unconditionally. @status:spec/done
- @fact:DESIGN-FOR-DETERMINISTIC The design must not assume only an LLM executor: the directive semantics (§7), the pipeline (§8), and the link tables (§10) are all specified so a deterministic agent can execute them. Remember this executor is coming. @status:spec/done

---

## 15. Migration {#migration}

@fact:migration-lead Incremental, safety-first (owner-set): @status:impl/done

- @fact:MIG-DEMO-FIRST **Build and test on a demo fixture corpus first** — throwaway packages exercising `simple`/`normal`, `contract`/`source`, `#embed`/`#use`/`#source`, cycles, and `@spec`. These are **not** real packages, and experimenting on them must never break vibevm itself. @status:impl/done
- @fact:MIG-GRADUAL **Migrate real packages gradually**, improving them onto the new format one at a time. @status:spec/done
- @fact:MIG-VIBEVM-LAST **Convert vibevm itself last:** first the whole of `org.vibevm.world`, and only then (if at all) the core feature specs. With `simple` as the default (§3) there is **no blast radius** — an un-migrated package keeps loading as `simple` (whole), so conversion to `normal` is per-package and opt-in, never a flag-day. @status:spec/done

---

## 16. Open questions {#open}

1. @fact:OPEN-EQUIVALENCE **Equivalence testing (§2).** Differential testing of the static vs structural executors — real, empirical, experiment-heavy; **planned separately**, meaningless before a working base exists. The static compiler is the reference semantics in the meantime. @status:spec/work
2. @fact:OPEN-READ-SET-COMPACTION **`@spec` read-set across compaction (§7.4).** No clean solution without a harness compaction signal; the file-based read-set + boot instruction is the floor. Revisit if the harness exposes a compaction event. @status:spec/work
3. @fact:OPEN-XML-FRONTEND **XML frontend (§5).** Timing and the exact IR mapping; the data structures are designed for it now, the frontend is built later. @status:spec/work
4. @fact:OPEN-CLOSURE-EXPLOSION **Implicit-reference closure explosion (§7.4).** With bare `spec://` at the agent's discretion, and `@spec` mandatory, bound the transitive closure a single in-place use can pull; measure on the demo corpus. @status:spec/work
5. @fact:OPEN-LINK-TABLES **Link tables (§10).** Whether they land at install-time, static-compile-time, or a separate phase — folded into the implementation task. @status:spec/work
   - @fact:LINK-TABLES-ARE-DEFERRED-WITH-A-NAMED-TRIGGER **Deferred deliberately, by owner ruling 2026-07-29, and this records the reason so nobody re-derives it.** Built today: the graph and a deterministic dump (`crates/vibe-spec/src/link_table.rs`). Not built: the persisted on-disk format and the structural consumer. The tables are **the vtable of the §13 structural executor** — a prebuilt index so a late-bound reader dispatches instead of searching — and this project does not run that mode, so they are an optimisation of navigation cost, never a precondition of correctness. Meanwhile `#embed spec://…` resolves and splices at compile time (`render_static` → `expand_embeds`), and an `@spec://` pointer that costs a lookup is strictly better than the confidently wrong relative path it replaced. Building the layer mid-refactor would create code the refactor then has to refactor. @status:spec/done
   - @fact:LINK-TABLES-PROMOTION-TRIGGER **What promotes it, so the deferral cannot quietly become permanent:** either `@spec://` pointers in the boot lane are MEASURED to cost a reader more than the lane saves, or the §13 structural loader is opened — whichever comes first. Either makes the searching real rather than hypothetical; until one fires, the cost being optimised away is a cost nobody has paid. @status:spec/done
6. @fact:OPEN-NO-ACCESS-CONTROL **No access control (§7.3).** We deliberately omit `private`/`public`. Confirm this holds once real packages exercise cross-contract calls. @status:spec/work
7. @fact:OPEN-DYNAMIC-TRANSITIVE **`dynamic-transitive` (§12).** Inherited from PROP-034 §5; still reserved. @status:spec/work

---

## 17. Version history {#history}

- @fact:HISTORY-DRAFTED **2026-07-14 — drafted (owner-requested), provisional.** Captures the "static-compiler vision" design dialogue: two build modes as AOT/JIT with the equivalence invariant (§2); `simple`/`normal` package formats (§3); the `contract`/`source` split (§4); the hierarchical document IR with MD and future-XML frontends (§5); the unified `spec://` grammar and the deterministic router (§6); the three directives `#embed` / `#use` / `#source` plus the `@spec` in-place-use sigil and the read-set (§7); the five-phase compilation pipeline and the embed-ordering standard (§8); the C++-derived cycle rules and the contract-layer no-deadlock invariant (§9); link tables as the vtable analogue (§10, provisional); reversible open/close markers (§11); `transitive-inline` folding in PROP-034 as the emission layer (§12); the first-loaded structural loader (§13); the future algorithmic executor (§14); and the demo-corpus-first, vibevm-last migration (§15). Implementation begins with the router (§6) under this contract. @status:spec/done
- @fact:HISTORY-IMPLEMENTED **2026-07-15 — implemented, and the default flipped to `simple`.** §5–§13 shipped as the `vibe-spec` crate and wired into `bootgen` (the payoff: `render_inline` runs `expand_embeds`, guarded); `transitive-inline` (§12) landed on `LinkType`. **§3's default changed from `normal` to `simple`** (owner decision): a forgotten `format` must fail *safe* (over-load, visibly working) rather than *silent* (a `normal` no-op), which also removes the §15 migration blast radius. Still open and under review: the `link` × `format` interaction (does a `normal` + `static` edge read eagerly or lazily?). @status:spec/done
- @fact:HISTORY-RENAME **2026-07-16 — link-type rename (owner decision), the `link` set shrinks to two.** `LinkType::Inline → Static` (the verbatim `STATIC.md` lane — "the static compiler"), `Static → Dynamic` (the default, a by-reference `INDEX.md` read with an optional `when`), and the old `Dynamic` removed — a conditional load is now just a `dynamic` entry carrying a `when`. `inline-transitive → static-transitive`; `INLINE.md → STATIC.md`; `render_inline → render_static`; `compile_inline → compile_static`. Pure terminology, aligning vibevm with the CS static/dynamic-linking standard so "the static compiler" reads naturally; shipped across `vibe-core`, `vibe-workspace`, `vibe-spec`, the package manifests, and these specs. @status:spec/done
- @fact:HISTORY-B011-ALIASING **2026-08-04 — deterministic loading: qualified splice, aliases, the lookup rule (B-011, owner-approved).** The owner's highest-priority build lands its contract: the §8 pipeline gains the **qualify** phase (labels become `<origin-slug>--<original>`, a pure function of origin — splice-order-independent, append-only under late dynamic lane additions); §7.2 gains the `as <Alias>` clause and §7.4 the `@!Alias` mandatory in-place use plus the two-scope short-reference lookup (fail with candidates, never a silent pick); §11 gains the qualified-label, resolution-preamble, tombstone-table, and lane-is-not-a-citation-target facts; §13's first instructions gain the resolution rules as named content (the owner's priority-placement addition). Design rationale and the fork record: `spec/design/deterministic-loading-aliasing.md`; the commissioning entry: `BACKLOG.md` B-011. PROP-009 §2.3's «verbatim concatenation» becomes «anchor-qualified concatenation» in the same landing. @status:spec/work
- @fact:HISTORY-NORMAL-STATIC **2026-07-20 — `normal + static` compiled end to end; the `link × format` question resolved.** The `[package].format` field now **parses** (`PackageFormat` in `vibe-core`, default `simple`, `deny_unknown_fields`-clean), **threads** through the boot model (`DependencyBoot` / `UnitInput` → `BootEntry`, both emission paths), and **drives** the static renderer: a `normal`-format contribution pulled `static` is **compiled** to its `#use` / `#source`-resolved, tree-shaken, dependency-ordered closure (`vibe-workspace::boot_artifacts::normal::compile_normal_entry`, seeded at the contract's whole-document address and delegating to the shipped `vibe_spec::compile_static`), where a `simple` one stays a verbatim concatenation. This **resolves the open `link × format` question** raised in the 2026-07-15 entry ("does a `normal + static` edge read eagerly or lazily?"): it is **eager** — the tree-shaken, `#source`-merged closure is AOT-compiled into `STATIC.md`, which is the equivalence-invariant reading (§2), never the whole file. The default stays `simple`, so an un-migrated tree's static lane is byte-identical (§3/§15). `normal + dynamic` — the structural / JIT loader (§13) — is still pending; only the static (AOT) executor honours the format so far. @status:spec/done
