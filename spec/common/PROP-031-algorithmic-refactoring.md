# PROP-031 — Algorithmic refactoring: the codemod engine and the write-side of the model {#root}

<status stage="spec" state="done" action="continue" comment="B0 2026-07-24: design proposal v0.1, drafted for review; open to challenge until ratified; fact grain 2026-07-24"/>

@fact:status-line **Status.** Design proposal v0.1 — not implementation-locked. Drafted for review; every decision below is open to challenge until ratified. This PROP establishes a *direction and a model*; it schedules no implementation of its own — its first consumer is the SPECMAP Unit-Mobility Plan (archived: `legacy-spec/terraforms/SPECMAP-UNIT-MOBILITY-PLAN-v0.1.md`), which builds the first operation. @status:spec/done

@fact:companions **Companions.** [PROP-032 — the project model & agent-first IDE substrate](spec://org.vibevm.core/vibevm/common/PROP-032#root) (the universal typed graph these operations mutate; this PROP is its *mutation* half) · [PROP-014 — specmap bidirectional traceability](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#index) (the read-side model this PROP makes writable; its `#llm-boundary` and `#open` §7.3 are extended here) · [PROP-003 — dependency evolution](spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003) §2.5.3 (the LLM-boundary philosophy) · [PROP-000 §3 License](spec://org.vibevm.core/vibevm/common/PROP-000#license) (permissive-only dependency posture) · [PROP-029 — fully-qualified addresses](spec://org.vibevm.core/vibevm/common/PROP-029) and [PROP-012 — managed redirect block](spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#markers) (the address + redirect substrate) · the AI-Native discipline card `scaffold-i-codemods` (the existing beachhead) · the delegation-first directive in `CLAUDE.md` (the economic thesis this PROP serves). @status:spec/done

---

## 1. Problem statement {#problem}

@fact:problem-lead Refactoring in this repository is done one of two ways today, and both are wrong for the volume ahead: @status:spec/done

1. @fact:PROBLEM-BY-HAND **By hand.** The naming campaigns in the git log (`refactor(spec): spec:// authority joins group and name with /`; `repoint every reference to the new package groups`) were large, mechanical, cross-file renames performed by a human or an LLM walking files. Correct, but slow and not repeatable. @status:spec/done
2. @fact:PROBLEM-LLM-REWRITE **By LLM file-rewriting.** An agent opens each file and rewrites it. For an elementary change — rename a `spec://` address, move a spec unit into a package — this can take **hours**, cost a fortune in tokens, and is **unreliable**: a single missed call-site silently severs a spec↔code edge, and nothing catches it unless a gate happens to run. @status:spec/done

- @fact:DELEGATION-THESIS Both violate the repository's own delegation-first thesis (`CLAUDE.md`): *mechanical transforms should cost `O(decision)`, not `O(files)`.* @status:spec/done
- @fact:RENAME-IS-TOOL-CALL A rename has **zero** generative content — it is a deterministic function of the tree — so it should not consume any model at all, cheap or expensive; it should be a **tool call**. @status:spec/done

- @fact:BEACHHEAD-SCAFFOLD-I The beachhead exists. The AI-Native discipline already ships **Scaffold I — codemods** (`scaffold-i-codemods`): "a recurring multi-file change offered as ONE parameterized, checked, atomic operation," explicitly naming `codemod rename-seam --from X --to Y`. @status:spec/done
- @fact:BEACHHEAD-LIMITS But today it is (a) **one operation** (`add-cell` scaffolding only), (b) marked **`[E-hyp]`** (unvalidated hypothesis), and (c) **a single language stack's card**, not a cross-layer capability with a ratified model. @status:spec/done
- @fact:EXTRACTION-DEMAND The upcoming cultural-pattern extraction refactoring will perform thousands of unit moves and address renames; without an algorithmic engine it is economically and operationally impossible to do well. @status:spec/done

@fact:PROP-PURPOSE This PROP names the capability, fixes the model, and — critically — states what every artifact must do **now** so the engine, when it lands, has a clean model to operate on. @status:spec/done

## 2. Decisions {#decisions}

### 2.1 A refactoring is the write-side of the traceability model {#write-side}

- @fact:READ-SIDE `prop r1` — specmap (PROP-014) is a **read-only, deterministic, queryable graph** of spec↔code: `index` builds it, `check` gates it, `explain` renders it. @status:spec/done
- @fact:WRITE-SIDE-DEF An algorithmic refactoring is the **write-side of that same model**: it mutates a node or an edge (rename a `spec://` address, move a unit, retarget an edge), the engine **emits the corresponding file edits**, and then **re-checks the graph**. @status:spec/done
- @fact:COMPLETION-NOT-SUBSYSTEM Algorithmic refactoring is therefore not a new subsystem — it is the natural completion of specmap: *read → write*. @status:spec/done
- @fact:SUBSTRATE-FREE Everything already built (cross-package resolution, revisions, suspects, dangling detection, determinism) becomes the refactorer's substrate at no additional cost. @status:spec/done
- @fact:GRAPH-GENERALISED The graph itself is generalised by [PROP-032 §2.1](spec://org.vibevm.core/vibevm/common/PROP-032#graph) into a **symmetric model over spec *and* code nodes**; these operations mutate that graph, so as it grows (`code://` nodes, spec→spec / spec→code edges) the same operations extend to the new directions for free. @status:spec/done

@fact:ADDRESSABILITY-COROLLARY **Corollary (load-bearing):** *nothing is refactorable that is not first addressable in a machine-readable model.* What lives only in prose cannot be mechanically transformed. This is the root of the build-in-anticipation discipline (§3). @status:spec/done

### 2.2 The LLM boundary: the model emits typed commands, not free-form edits {#llm-boundary}

- @fact:LLM-DIVISION `req r1` — The division of labour is fixed: **the LLM SELECTS and PARAMETERIZES a typed refactoring operation; a deterministic engine EXECUTES and GATES it.** @status:spec/done
- @fact:LLM-JUDGEMENT The LLM decides *what* to refactor and *why* (extract this pattern? into which package? is the text clean or mixed?) — irreducibly a judgement. @status:spec/done
- @fact:LLM-MUST-NOT-REWRITE The LLM MUST NOT rewrite files to perform a *mechanical* refactoring; it emits an operation from the algebra (§2.6) with its parameters, and the engine does the rest. @status:spec/done

- @fact:BOUNDARY-COMPLETION This is the completion of [PROP-014 §2.7](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#llm-boundary) and PROP-003 §2.5.3 — *the LLM emits facts and renderings; deterministic machinery decides.* @status:spec/done
- @fact:TYPED-COMMAND A refactoring is a **typed command the LLM proposes**, reviewed like any diff, not free-form editing. @status:spec/done
- @fact:ECONOMIC-CONSEQUENCE The consequence is the economic one: LLM cost drops from `O(files touched)` to `O(one decision)`. @status:spec/done

### 2.3 The gated invariant: a refactoring is *done* only when the model re-checks clean {#gated-invariant}

@fact:GATED-INVARIANT-LEAD `req r1` — Every operation is: @status:spec/done

- @fact:OP-ATOMIC **Atomic** — all-or-nothing; a failed post-check rolls **every** write back (the pattern `codemod add-cell` already implements, `rust-ai-native-cli/src/codemod.rs`). @status:spec/done
- @fact:OP-DETERMINISTIC **Deterministic** — same tree + same parameters → same edits; twice-run is byte-identical (the specmap determinism contract, PROP-014 §2.5). @status:spec/done
- @fact:OP-DRY-RUNNABLE **Dry-runnable** — `--dry-run` prints the exact plan and touches nothing; the human reviews before the write. @status:spec/done
- @fact:OP-GATED **Gated** — the operation is **not complete until the model re-checks clean**: `specmap --check` (0 dangling, 0 new suspects) plus the tier's own gate (`self-check.sh`, `cargo check`). @status:spec/done

@fact:TRUST-PROPERTY This gated invariant is the **trust property that LLM file-rewriting structurally lacks**: an LLM rename is "done" when the model says it is done; an engine rename is done when the *invariant* says so. That difference is the whole value. @status:spec/done

### 2.4 The three-tier stack {#three-tier}

@fact:THREE-TIER `prop r1` — Refactoring operations live at three tiers, mirroring the discipline's existing product / stack / core structure. Each tier owns the model it can address: @status:spec/done

| Tier | Owns / knows | Representative operations |
|---|---|---|
| @fact:TIER-PRODUCT **product — vibevm** @status:spec/done | the project object-model: lockfile, `vibe.toml` graph, boot manifest (`spec/boot/INDEX.md`), the embedded registry (PROP-030), package FQIDs (PROP-029) @status:spec/done | `rename-package`, `move-package-between-groups`, `repoint-dependency`, `relocate-boot-snippet`, `split-package` / `merge-package` @status:spec/done |
| @fact:TIER-DISCIPLINE **discipline — specmark / specmap** (language-neutral) @status:spec/done | the `spec://` address space and the spec↔code edge graph @status:spec/done | `rename-address`, `move-unit` (the capsule), `retarget-edge`, `bump-revision` + re-affirm pins @status:spec/done |
| @fact:TIER-LANGUAGE **language — rust-ai-native / typescript-ai-native** @status:spec/done | code symbols + their spec tags, per language @status:spec/done | `rename-symbol` (+ its `#[spec]`/`scope!` tags), `move-item` (+ its edges), `change-signature` — **wrapping** an existing engine (§2.5) @status:spec/done |

- @fact:TIER-COMPOSE Operations **compose across tiers**: `move-unit` (discipline) = `rename-address` ∘ `relocate-text` ∘ `external-specs-upkeep`; a package rename (product) may drive N `rename-address` (discipline) + N `rename-symbol` (language). @status:spec/done
- @fact:GROWTH-BY-COMPOSITION The engine grows by composition, not by bespoke commands. @status:spec/done

### 2.5 Wrap permissive engines; never reimplement AST surgery {#wrap-engines}

- @fact:WRAP-LAW `req r1` — The language tier does **not** reimplement rename/move on raw ASTs. It **orchestrates an existing, permissively-licensed refactoring backend** and adds the one thing those backends do not know: spec-awareness (updating `#[spec]`/`scope!` tags and keeping the specmap graph consistent). @status:spec/done
- @fact:WRAP-CANDIDATES Candidate backends — `rust-analyzer` (Apache-2.0/MIT), `ast-grep` (MIT), `ts-morph` (MIT), `comby` (Apache-2.0) — all satisfy [PROP-000 §3](spec://org.vibevm.core/vibevm/common/PROP-000#license) (permissive-only; GPL/AGPL/LGPL forbidden as dependencies). @status:spec/done
- @fact:CLEAN-ROOM Study of any research refactoring codebase follows the repository's clean-room rule (`spec/boot/90-user.md`): understand the approach, write structurally different code; never port. @status:spec/done
- @fact:LICENSE-REVERIFY License fields re-verified before any code-level reuse (§5). @status:spec/done

### 2.6 The operation algebra {#algebra}

@fact:ALGEBRA `prop r1` — The engine exposes a small, growing **algebra** of typed operations, each with named parameters, a dry-run, and a post-check. v0.1 catalogue (not exhaustive; grows by owner amendment): @status:spec/done

```
rename-address   <from-uri> <to-uri>              # retarget every citing edge to a new spec:// address
move-unit        <from-uri> <to-doc>[#anchor]     # relocate a spec unit across a boundary (the capsule)
retarget-edge    <symbol> <from-uri> <to-uri>     # repoint one code→spec edge
bump-revision    <uri> --to r<N>                  # bump a unit's revision; list the pins it makes suspect
rename-symbol    <from> <to>                       # rename a code symbol + its spec tags (wraps §2.5)
rename-package   <from-fqid> <to-fqid>            # rename a package + repoint every consumer (product tier)
repoint-dependency <consumer> <from> <to>          # move a dependency edge in the manifest graph
relocate-boot-snippet <slot> <to>                  # move a boot entry + regenerate INDEX.md
```

- @fact:OP-PURE-FUNCTION Each is a pure function `(tree, params) → edits`, applied atomically and gated (§2.3). @status:spec/done
- @fact:FIRST-OPERATION `move-unit` is the first to be built (SPECMAP Unit-Mobility Plan Phase 3), and it is built **as a composition** on top of `rename-address` — which is the purest instance and therefore the one that validates the whole loop first. @status:spec/done

## 3. Build-in-anticipation discipline (what to do NOW) {#anticipation}

@fact:ANTICIPATION-LAW `req r1` — The engine does not exist yet, but its arrival is a **standing assumption** from this PROP forward. Everything authored between now and then MUST keep the model refactor-ready, so the engine inherits a clean substrate rather than a swamp: @status:spec/done

1. @fact:ANT-ADDRESS-EVERYTHING **Address everything; prose-reference nothing.** Every cross-reference is a resolvable URI (`spec://…#anchor`, a package FQID, a symbol path), never "see above" / "the section on X". Generalises [PROP-014 §3.1.9](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#index) from a style rule to a **precondition**: the engine can only rename what it can address. An un-addressable reference is invisible to the refactorer and will silently rot. @status:spec/done
2. @fact:ANT-STABLE-IDENTITY **Stable identity for anything that can move.** Spec units keep immutable `{#anchor}`s; code carries `#[spec]`/`scope!`; packages carry FQIDs; boot entries carry INDEX ids. Renaming preserves the anchor; retiring tombstones it (PROP-014 §2.1). Identity is what an operation targets. @status:spec/done
3. @fact:ANT-GRAPH-OVER-PROSE **Prefer the graph over prose for anything you may later refactor.** If a link must survive a rename, it belongs in the machine-readable graph, not in a paragraph. **This resolves the SPECMAP plan's D3 in favour of graph-edges:** prose `spec://` references should become first-class specmap edges (extend `mdspec`), because that makes them not merely *gated* but *refactorable*. A standalone prose-link checker gates; a graph edge gates **and** moves. @status:spec/done
4. @fact:ANT-ALGEBRA-INSTANCES **Build bespoke tools as instances of the algebra, not one-offs.** The SPECMAP `move-unit` command is written as the first operation of *this* engine (§2.6), sharing the dry-run / atomic / gated contract, so the second operation is composition, not a rewrite. @status:spec/done
5. @fact:ANT-GATE-NOW **Gate the models now.** A write-side is only trustworthy over a read-side that is enforced. Wiring `specmap --check` into `self-check.sh` (SPECMAP Plan M1) is a precondition for *any* refactoring operation: the invariant the engine checks against must already be green and gated. @status:spec/done

## 4. Rejected alternatives {#rejected}

1. @fact:REJ-LLM-REWRITE **LLM free-form file-rewriting (the status quo).** The problem, not a solution: `O(files)` cost, no atomicity, no gate, silent edge-severing. Retained only for the *judgement* half (deciding what to refactor), never the *mechanical* half. @status:spec/done
2. @fact:REJ-SIDECAR-DB **A sidecar refactoring database** (operations recorded outside the tree). Violates "project facts live in the repo" (`CLAUDE.md` memory discipline) and PROP-014's rejection of sidecar maps — the model is the tree (specmap graph + manifests), and the engine reads/writes it directly. @status:spec/done
3. @fact:REJ-REIMPLEMENT-AST **Reimplementing AST surgery per language.** Wasteful and fragile against language evolution; §2.5 wraps mature permissive engines instead, adding only spec-awareness. @status:spec/done
4. @fact:REJ-IDE-ONLY **IDE-only refactoring (LSP rename in the editor).** Necessary but insufficient: not scriptable, not headless, not gated by the discipline, not composable into swarm/`fractality` runs, and blind to `spec://` addresses and package graphs. The engine is CLI-first and gate-first; IDE integration is a later surface over the same operations. @status:spec/done
5. @fact:REJ-DEFER-MODEL **Deferring the model until the engine is built.** Rejected as the whole point of this PROP: the *anticipation discipline* (§3) is what makes the eventual engine cheap. Building the model late means retrofitting addressability across a corpus that grew without it. @status:spec/done

## 5. Prior art & license posture {#prior-art}

@fact:prior-art-lead Conventions and ideas are free; code is not (PROP-000 §3). Roles explicit; license fields re-verified before any code-level reuse. @status:spec/done

| System | License (verify) | Role here |
|---|---|---|
| @fact:PA-OPENREWRITE OpenRewrite @status:spec/done | Apache-2.0 @status:spec/done | **Study — the closest prior art.** Typed, composable, *gated* refactoring "recipes" over a Lossless Semantic Tree. The recipe = a typed operation; the LST = the model. Borrow the *shape* (recipes + a checked model), not code. @status:spec/done |
| @fact:PA-RUST-ANALYZER rust-analyzer @status:spec/done | Apache-2.0 / MIT @status:spec/done | Wrap candidate for the Rust language tier (rename, SSR) via its library crates or LSP. @status:spec/done |
| @fact:PA-AST-GREP ast-grep @status:spec/done | MIT @status:spec/done | Wrap candidate — structural search/replace, multi-language, CLI + rules. @status:spec/done |
| @fact:PA-TS-MORPH ts-morph @status:spec/done | MIT @status:spec/done | Wrap candidate for the TypeScript language tier (AST refactoring library). @status:spec/done |
| @fact:PA-COMBY comby @status:spec/done | Apache-2.0 @status:spec/done | Study/wrap — structural rewriting across languages. @status:spec/done |
| @fact:PA-LSP LSP rename / SSR @status:spec/done | n/a (protocol) @status:spec/done | The interaction model the CLI operations mirror headlessly. @status:spec/done |
| @fact:PA-JSCODESHIFT jscodeshift / codemod.com @status:spec/done | MIT @status:spec/done | Study — the codemod-as-script lineage (the discipline's Scaffold I already draws on it). @status:spec/done |

@fact:DIFFERENTIATORS **Differentiators.** @status:spec/done

1. @fact:DIFF-SPEC-AWARE operations are **spec-aware** — they update the `spec://` edge graph, which no general refactoring tool knows about; @status:spec/done
2. @fact:DIFF-GATED they are **gated by the discipline's own invariant** (`specmap --check` / `self-check`), not merely "compiles"; @status:spec/done
3. @fact:DIFF-LLM-PARTICIPANT an **LLM is a first-class participant** — strictly as the proposer/parameterizer behind a deterministic core (§2.2); @status:spec/done
4. @fact:DIFF-COMPOSE-TIERS operations **compose across three tiers** (spec / code / package) because the models are addressable end-to-end. @status:spec/done

## 6. Open questions {#open}

<status stage="spec" state="work" comment="B1 2026-07-24: six questions open; ratification pending"/>

1. @fact:open-mechanism-home **Where the discipline mechanism lives.** This host PROP is the vibevm-facing anchor. Elevating Scaffold I from an `[E-hyp]` card to a ratified language-neutral **mechanism** in `flow:org.vibevm.ai-native/core-ai-native` (a sibling of PROP-014) is a separate, later, owner-decided step. Decide when the second operation lands. @status:spec/work
2. @fact:open-weak-tier **Weak-tier parameterization (the `[E-hyp]` question, inherited from Scaffold I).** Can the weakest swarm tier correctly *parameterize* an operation, or must the weakest tier be restricted to fixed-parameter invocations? The prime pilot question; answered empirically once `rename-address` / `move-unit` ship. @status:spec/work
3. @fact:open-transactional **Transactional multi-tier operations.** A `rename-package` fans out into discipline + language operations across many files; is the whole fan-out one transaction (all-or-nothing across tiers), or a checkpointed sequence? Lean: one transaction with a single post-gate, checkpointing only if wall-time forces it. @status:spec/work
4. @fact:open-concurrency **Concurrency with in-flight human edits.** An operation assumes a quiescent tree; define the posture when the working tree is dirty (refuse? operate only on committed state?). @status:spec/work
5. @fact:open-undo **Undo / history.** Beyond git revert — does the engine keep an operation journal for structured inverse operations, or is git the only undo? Lean: git is the undo; operations are designed to invert cleanly (a rename's inverse is a rename). @status:spec/work
6. @fact:open-redirect-default **Redirect vs eager-retarget as the default** (PROP-014 §7.3). `rename-address` eager-retargets; redirect stubs are the deferred/parallel path. Which is the default for a swarm run is a policy the operation exposes, not a fixed choice (SPECMAP Plan D1). @status:spec/work

---

- @fact:ratification-note *This PROP is a design proposal. Ratification — and the first operation's implementation — happens through PR review against this document and the SPECMAP Unit-Mobility Plan.* @status:spec/done
- @fact:unexercised-removed *Any mechanism specified here that is not exercised by the second shipped operation is removed from the spec rather than carried as aspirational documentation (the PROP-014 §335 discipline, inherited).* @status:spec/done
