# Multiple sources for one contract, and the plugin form {#root}

<status stage="spec" state="done" comment="the B-056 design; the owner closed the SHAPE with four rulings on 2026-08-04 (##B056-RULED) and this is the build design over them. Non-normative: PROP-035 and the backlog rulings win. Authored 2026-08-05, волна Г."/>

##companion-line **Companion to:** [`BACKLOG.md` B-056](../../BACKLOG.md#b-056) (the four rulings and the honest cost), [B-055](../../BACKLOG.md#b-055) (today's silence on a second directive). Contract: [PROP-035 `#source`](../modules/vibe-workspace/PROP-035-spec-compiler.md#source) and its `##NO-DEADLOCK-INVARIANT` (§9). This document is lore — it records why the build is shaped this way, and the PROPs win wherever they disagree. @spec/done

## 1. The measured basis {#basis}

##basis-one-source **Folding takes exactly one source, and it is the first one.** `crates/vibe-spec/src/pipeline.rs:221` is `.find(|d| d.kind == DirectiveKind::Source).map(|d| d.address)` — the first `#source` directive wins and every later one is dropped without a word. That silence is [B-055](../../BACKLOG.md#b-055); this design is what replaces it. @impl/done

##basis-two-inputs **The folder's signature is binary.** `merge::fold_source(contract: &DocTree, source: &DocTree) -> String` walks the contract's top-level heading sections, looks each anchor up in THE source, and emits `:add` (contract text minus redeclared facts, then source text) or `:replace` (source text only); source-only sections are appended after. Generalising it is the centre of this build. @impl/done

##basis-no-recursion **The fold does not follow a source's own `#source`.** `use_graph` says so explicitly: `#embed` and `#source` are not dependency edges and are ignored by its traversal. So a source that itself names a source contributes only its own text today. @impl/done

##basis-resolver-is-pointwise **The resolver maps one package coordinate to one installed slot.** There is no enumeration-by-pattern, which is the whole of what the plugin form needs. @impl/done

## 2. What the owner's four rulings settle, and what they leave {#rulings}

##rulings-recap **Settled (2026-08-04).** *(1)* `:replace` from ANY source discards only the CONTRACT text; the sources still sum among themselves, in order. *(2)* A glob MUST be expanded in sorted order. *(3)* Both spellings live: several `#source` lines, and a `*` pattern. *(4)* Recursion follows C++/Java — contracts include recursively and deduplicated, implementations do not include implementations recursively, and an implementation including an implementation is legal only up to a cycle. @spec/done

##rulings-replace-is-a-flag **Why ruling (1) removed a problem instead of deciding it.** The boss had proposed making two `:replace` on one anchor a build error. Under the owner's formulation the conflict cannot arise: `:replace` stops being «whose text is canonical» and becomes a flag meaning «drop the contract side», after which the sources sum in declaration order no matter how many of them carried it. Degenerate check: with ONE source the result is byte-identical to today's behaviour, so the generalisation is backward compatible. @spec/done

##rulings-left **Left to the build.** The N-input fold, the flag's threading, a cycle guard and a dedup for the fold, and enumeration in the resolver. Everything below is that. @spec/plan

## 3. The fold, generalised {#fold}

##fold-signature **One new entry point, the old one kept as its degenerate case.** `fold_sources(contract: &DocTree, sources: &[&DocTree]) -> String`, with `fold_source(c, s)` retained as `fold_sources(c, &[s])`. Keeping the binary name is not politeness: it is the regression test — every existing fold test must pass through the new path unchanged, and any that does not is a behaviour change to be argued rather than absorbed. @spec/plan

##fold-per-section **The per-section rule, stated once.** For each top-level contract section with anchor `a`, let `S(a)` be the sub-sequence of sources carrying a section with that anchor, in the order the `#source` directives were declared (a glob contributing its members in sorted order). Then: `S(a)` empty → the contract section is emitted unchanged; any member of `S(a)` carrying `:replace` → the contract text is dropped and the members of `S(a)` are emitted in order; otherwise → the contract text is emitted MINUS every fact any member of `S(a)` redeclares, then the members in order. @spec/plan

##fold-per-section-delta **The delta from today's law is two words.** «Any» and «every»: one source becomes a sequence, and fact-override becomes a union over that sequence rather than a lookup in a single document. Nothing else about the section rule moves. @spec/plan

##fold-override-is-a-union **Fact override widens to the union, and that is the same rule, not a new one.** Today `overridden_facts` drops the contract facts whose id THE source redeclares. With N sources the dropped set is the union over all of them — one id, one unit, and whoever redeclares it takes it. A fact redeclared by two sources at once is not a fold problem: both redeclarations survive into the merged text and the compiler's anchor-uniqueness recheck fails on the duplicate, loudly, which is the behaviour the owner's model already relies on. @spec/plan

##fold-source-only-collision **A source-only anchor declared twice IS an error, and the asymmetry is deliberate.** A source section matching a contract anchor is an addition to something already declared — summing is right. A source-only section is a new declaration, and two of them are two definitions of one name. C++ says the same thing about a `.cpp` including a `.cpp`: **declaration is idempotent, definition is not.** The fold need not detect this itself; it appends in order and the post-merge uniqueness check does the failing, with the anchor named. @spec/done

##fold-order-is-declaration-order **Order is declaration order, and a glob is sorted before it joins.** So the composed document is a pure function of (tree, lockfile): the lockfile fixes which slots exist, sorting fixes their order, and nothing depends on filesystem enumeration order. @spec/plan

## 4. Recursion — an existing law, not a new one {#recursion}

##recursion-already-canon **Ruling (4) is already this project's law; it just never reached the fold.** PROP-035 §9 carries it as `##NO-DEADLOCK-INVARIANT` — «the contract layer is where cycles are legal; the source layer is where topological order is obligatory» — and `crates/vibe-spec/src/use_graph.rs` implements it for `#use`: a three-colour DFS, dedup by construction («a node reached by several paths appears once»), an `is_contract` predicate keyed on the path segment, a loop admitted only when EVERY node in it is a contract, and a loop touching a source rejected outright. The work is to extend that reach, not to invent the rule. @impl/done

##recursion-what-to-extend **What the fold needs, concretely.** Its own traversal over `#source` edges with the same three colours and the same `is_contract` predicate: a contract reached by several paths folds once; a cycle among contracts is legal; a cycle that touches an implementation is a hard error naming the path. Reuse the existing walker rather than writing a second one — two implementations of one cycle law is the same defect as two implementations of one hash. @spec/plan

##recursion-dedup-asymmetry **What must NOT be claimed while it is unmeasured.** The static traversal deduplicates by construction. The structural (dynamic) mode is executed by an LLM from the first instructions, so there dedup is a property of the prompt, not of a machine. Do not state the symmetry until it is measured. @impl/work

## 5. The plugin form {#plugins}

##plugins-only-enumeration-is-new **The glob adds exactly one capability.** `#source spec://org.vibevm.plugins/plugin-*` needs the resolver to enumerate installed packages matching a pattern instead of resolving one coordinate. Everything downstream is the sequence fold of §3. @spec/plan

##plugins-reproducible-by-lockfile **Reproducibility is not at risk, and the reason is already in the tree.** «What is installed» is not an ambient property of the machine — it is the set the lockfile pins. One tree plus one lockfile, expanded in sorted order, give one document. @spec/done

##plugins-empty-is-legal **A glob that matches nothing is an empty set, never a missing source** — and that closes a neighbouring argument for free. The privacy-tier discussion keeps circling «declared but not shipped» versus «lost»; globs degrade naturally and pointed addresses do not, so the plugin form simply does not have that problem. @spec/done

## 6. The honest cost, and the order to build in {#cost}

##cost-halves **Only the `:add` half is cheap.** The default mode is a sum, and a sum is associative: contract + s1 + s2 + … composes with no new entity, and a section present only in a source is simply appended. The resolver's enumeration, the `:replace` flag's threading, ordering and recursion are a separate build with its own design — which is this document. @spec/done

##cost-reading-consequence **A section assembled from five plugins is long, and the long-section threshold warning ([B-021](../../BACKLOG.md#b-021)) will fire on it.** That is right rather than annoying — a reader deserves to know a section is composed — but it should be expected rather than discovered. @spec/done

##cost-order **Build order, each step landable alone.** *(1)* `fold_sources` over an explicit list, with `fold_source` as its degenerate case and every existing test passing unchanged. *(2)* The pipeline stops taking `.find(…)` and passes every `#source` in declaration order — this alone closes [B-055](../../BACKLOG.md#b-055). *(3)* The fold's cycle guard and dedup over the extended `use_graph` walker. *(4)* Resolver enumeration for the glob, sorted. Steps 1–2 are the sum; 3–4 are the rest. @spec/plan
