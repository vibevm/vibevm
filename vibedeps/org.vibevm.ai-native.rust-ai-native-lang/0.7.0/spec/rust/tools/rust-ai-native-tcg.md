# Tool Spec (high-level): `rust-ai-native-tcg` — Type-Aware Constrained Generation for Rust {#root}

<status stage="spec" state="done"/>

@fact:status-line *Status: vision / component brief for the vibevm tool suite. NOT an implementation plan.* @status:spec/done

@fact:VERY-FAR-FUTURE-DISPOSITION *VERY-FAR-FUTURE per the owner's standing disposition (2026-07-07): the decode-loop stages below wait on `vibe-llm` and a local inference substrate, exactly like the TypeScript token-level sibling.* @status:spec/done

@fact:RENAMED-FROM-VIBE-TCG *Renamed `vibe-tcg.md` → `rust-ai-native-tcg.md` (the D13 language-suffix policy, then its PROP-028 §2.4 family-prefix supersession) — the bare name `vibe-tcg` belongs solely to vibevm's language-generic product crate.* *Specified, not built — the rename happened and the policy resolves; the crate the name is reserved FOR does not exist. This file carries the new name, and the supersession chain it cites is authored (PROP-028 §2.4, `##D13-SUPERSEDED` → `##D13-LANGUAGE-LEADS` → `##D13-NEUTRAL-OUTSIDE`). But `vibe-tcg` was deleted with the whole multiplexed-product topology (PROP-026 in vibe-mcp, `##TCG-CRATE-DELETED`), and vibevm's crate roster carries no `vibe-tcg`: every tcg crate in the tree is per-family (`rust-ai-native-tcg`, `typescript-ai-native-tcg`, `go-ai-native-tcg`). The `vibe-*` stem is still reserved for language-neutral vibevm crates as a POLICY; the specific reservation this sentence asserts is held by nothing.* @status:spec/done

@fact:DERIVED-FROM-THE-EVIDENCE *Derived from R2C-005 (type-constrained decoding is per-language manual work; no Rust impl exists), DR2-012/DR1-014 (the 75.3 %/70.2 % compile-error reduction in TypeScript), and the constrained-decoding ecosystem scan (SynCode, XGrammar-2, IterGen, Mündler PLDI'25).* *One of the three cited ids does not resolve: `R2C-005` (ATLAS `##FINDING-R2C-005`) and `DR2-012` (`##FINDING-DR2-012`) are authored; `DR1-014` is not — the roster runs 001–013 and 015–024 with 014 the only gap, and no document in the tree defines it. The reduction result stands on DR2-012 (whose canonical pair 75.3 %/70.2 % this line now cites) and on the cited paper; the second id is a dead reference.* @status:spec/done

> @fact:AGENTIC-SIBLING-SHIPPED **The agentic sibling SHIPPED (2026-07-07).** The tcg line's AGENTIC
> delivery exists for BOTH languages: TypeScript first
> (`typescript/tools/vibe-agentic-tcg-ts.md` in the TS stack), and now
> Rust — a consultation oracle over the consumer's own rust-analyzer
> (validate / scope / type-valid completions / quick info over LSP
> overlays, discipline-enriched in-process by the same conform engine
> as the gate) behind the SAME language-parameterised `tcg_*` MCP
> tools — see [`vibe-agentic-tcg-rust.md`](vibe-agentic-tcg-rust.md)
> and vibevm's PROP-026. That delivery is this brief's Stage 2 made
> consultable instead of masking; the decode-loop stages below remain
> gated on an inference substrate. The far-backlogged `ra_ap_*`
> embedding (vibevm ROADMAP.md, Far backlog) is the capability upgrade
> both the agentic line and a future masker would share. @status:impl/done

## 1. What problem it solves {#problem}

@fact:LLM-COMPILE-ERRORS-ARE-MOSTLY-TYPE-ERRORS LLMs emit Rust that fails to compile; per the PLDI'25 evidence, ~94% of compile errors are TYPE errors, not syntax — and only ~6% are syntactic. @status:spec/done

@fact:CFG-CONSTRAINING-CATCHES-ONLY-THE-SYNTACTIC-SIX-PERCENT Pure grammar/CFG constraining (mature: SynCode, XGrammar-2) catches the 6% and leaves the 94%. @status:spec/done

@fact:NO-TYPE-AWARE-TOOL-EXISTS-FOR-RUST The gap for Rust specifically: no type-aware constrained-generation tool exists (the PLDI'25 authors built it only for a TypeScript subset and state plainly it must be re-implemented per language). @status:spec/done

@fact:RUST-AI-NATIVE-TCG-IS-THAT-MISSING-TOOL `rust-ai-native-tcg` is the name reserved for that missing tool for Rust — to be delivered as a vibevm component so the swarm's weak agents generate well-typed Rust by construction rather than by retry. The binary shipping under that name today is the AGENTIC consultation oracle (`vibe.toml` `[[binary]]`, `crates/rust-ai-native-tcg`); the token-level generation tier this brief specifies is held VERY-FAR-FUTURE per @fact:status-line and is not built. @status:spec/done

@fact:STRATEGIC-PLACEMENT-IN-THE-CATALOG **Strategic placement in the scaffold catalog:** this is the generation-time complement to the post-generation `cargo check` loop (Class E). @status:spec/done

@fact:LOOP-CATCHES-AFTER-TCG-PREVENTS-DURING The loop catches errors AFTER a full generation; `rust-ai-native-tcg` prevents a class of them DURING generation. @status:spec/done

@fact:BOTH-ARE-WANTED-TCG-IS-THE-HIGHER-LEVERAGE-BET Both are wanted; the loop is buildable today, `rust-ai-native-tcg` is the harder, higher-leverage bet for the weak-agent swarm (DR1-015: constraints help weak models most). @status:spec/done

## 2. Design stance (consequences of what we read) {#design-stance}

- @fact:DO-NOT-REIMPLEMENT-RUSTCS-TYPE-SYSTEM **Do NOT reimplement rustc's type system.** The PLDI'25 cost was high precisely because they hand-built type-reachability. Rust's trait resolution + lifetime inference is far heavier than TypeScript's. Building a from-scratch incremental Rust type checker is a multi-year effort and a likely dead end. @status:spec/done
- @fact:STAND-ON-RUST-ANALYZER-INSTEAD **Stand on `rust-analyzer` instead.** It already IS an incremental, query-based (salsa) analyzer that answers "what's in scope here, with what type" at a cursor — exactly the oracle a type-aware constrainer needs. The tool's core bet: expose rust-analyzer's existing analysis as a generation-time completion oracle, rather than rebuild it. @status:spec/done
- @fact:TWO-LAYER-CONSTRAINT **Two-layer constraint, matching the literature's split:** @status:spec/done
  - @fact:LAYER-1-SYNTACTIC *Layer 1 — syntactic (mature, cheap):* a Rust CFG mask via an existing engine (XGrammar-2/SynCode-class). Guarantees parseable Rust. This is solved tech; integrate, don't invent. @status:spec/done
  - @fact:LAYER-2-SEMANTIC *Layer 2 — semantic (the novel, valuable part):* at each completion point where an identifier/expression is sampled, query rust-analyzer for the set of in-scope, type-valid continuations (callable functions whose signature fits, fields of the right type, trait methods in scope, variants for an exhaustive match) and mask to that set. This is the Rust analogue of Mündler's "search over inhabitable types," but backed by rust-analyzer rather than a bespoke type engine. @status:spec/done
- @fact:SPECULATIVE-PLUS-BACKTRACKING-PER-ITERGEN **Speculative + backtracking, per IterGen:** full per-token rust-analyzer queries are too slow. Speculatively decode multi-token spans, validate the span against the analyzer, backtrack on rejection. The interpreter-budget result (R2C: feedback amplifies capable agents) implies the tool should expose WHY a span was rejected, not just reject it. @status:spec/done

## 3. Component shape (how it fits vibevm) {#component-shape}

- @fact:COMPONENT-SURFACE **Surface:** an inference-time service the agent harness calls during generation, parameterized by (a) the file/cursor context, (b) the assembled cell context from the pager, (c) the active constraint profile. @status:spec/done
- @fact:CONSTRAINT-PROFILES-TIE-TO-THE-DISCIPLINE **Constraint profiles (tie to the discipline):** profiles are not just "valid Rust" but "valid AI-Native Rust" — e.g. a profile that forbids sampling a `.unwrap()` continuation in a domain cell, or forbids constructing a primitive where a newtype seam exists (Class B enforcement at generation time), or requires the next item to carry a contract block (R3-002). The discipline's bans become generation-time masks, not just post-hoc lints. This is the deepest integration: **the guide's rules compile into tcg constraint profiles.** @status:spec/done
- @fact:COMPONENT-OUTPUT **Output:** well-typed (and discipline-conformant) Rust spans + a structured trace of what was masked and why (feeds Class F diagnostics, R3-011). @status:spec/done
- @fact:DETERMINISM-NOTE **Determinism note:** the masking is deterministic given (model logits, analyzer state, profile); only the model's sampling is stochastic. This keeps the tool auditable (A1) — every rejected continuation has a recorded reason. @status:spec/done

## 4. Staged ambition (easy wins first, per the licensing/realism posture) {#staged-ambition}

@fact:STAGE-1-SYNTACTIC-PROFILE-ONLY **Stage 1 — syntactic profile only.** Integrate a CFG engine (XGrammar-2-class, Apache-2.0) with a Rust grammar. Catches the ~6%. Cheap, ships fast, immediately useful. Validates the harness integration. @status:spec/done

@fact:STAGE-2-SCOPE-NAME-CONSTRAINING **Stage 2 — scope/name constraining.** Use rust-analyzer to mask identifier completions to in-scope names with the right arity. Catches a chunk of the "hallucinated library/feature that doesn't exist" failure (the exact failure Matt Welsh reports for Rust newbies in 2026). No full type inference yet. @status:spec/done

@fact:STAGE-3-TYPE-VALID-CONTINUATIONS **Stage 3 — type-valid continuations.** Full Layer 2: mask to type-inhabitable continuations via rust-analyzer. This is the PLDI'25-equivalent leap, and where the 74.8%-class gains would come from IF they transfer to Rust (unproven — measure). @status:spec/done

@fact:STAGE-4-DISCIPLINE-PROFILES **Stage 4 — discipline profiles.** Compile AI-Native Rust rules into constraint masks (no-unwrap-in-domain, newtype-at-seam, contract-block-required). The discipline enforced at generation time. @status:spec/done

@fact:MAXIMUM-PERFECTION-HARD-PATH **Maximum-perfection hard path (separate, flagged):** a soundness layer proving the mask never excludes a valid completion (the L(A) ⊆ L vs ⊇ L property the PLDI'25 paper formalizes). Likely needs a formal model of the supported Rust subset. High cost; only after Stages 1–3 demonstrate value. @status:spec/done

## 5. Licensing posture (per project policy) {#licensing}
- @fact:LICENSING-RUST-ANALYZER `rust-analyzer`: MIT/Apache-2.0 — permissive, safe to build on. The central dependency is clean. @status:spec/done
- @fact:LICENSING-CFG-ENGINES CFG engines: XGrammar (Apache-2.0), Outlines (Apache-2.0), SynCode (check current license) — permissive options exist; avoid any GPL-licensed grammar tooling. @status:spec/done
- @fact:LICENSING-PLDI-REPRODUCTION-PACKAGE The PLDI'25 reproduction package (eth-sri/type-constrained-code-generation) is a DESIGN reference (read the algorithm), not a dependency — and it's TypeScript-specific regardless. @status:spec/done
- @fact:licensing-net Net: the whole stack is buildable permissively. No viral-license trap on the critical path. @status:spec/done

## 6. The honest risk register {#risk-register}
- @fact:RISK-TRANSFER-UNPROVEN **Transfer unproven:** the PLDI'25 reduction — 75.3% on synthesis and 70.2% on translation (DR2-012) — is TypeScript. Rust's richer types may yield smaller gains, or the per-completion analyzer latency may make Stage 3 impractical for interactive generation. Measure at Stage 2 before committing to Stage 3. @status:spec/done
- @fact:RISK-RUST-ANALYZER-BUILT-FOR-IDES **rust-analyzer was built for IDEs, not decoding loops:** query latency and partial-file analysis under a half-written buffer may need work; rust-analyzer's tolerance for incomplete code is an asset here but its per-query cost in a tight decode loop is the open engineering risk. @status:spec/done
- @fact:RISK-OVER-CONSTRAINT-HURTS-STRONG-MODELS **Over-constraint can hurt strong models (DR1-015: Hermes-4-405B dropped 92.5%→35.0%).** The tool must be CAPABILITY-ROUTED: on for the weak swarm, optional/off for strong authors. A profile that helps Qwen-32B may distort Opus. @status:spec/done
- @fact:RISK-DOES-NOT-FIX-SEMANTICS **It does not fix semantics, only well-typedness:** well-typed wrong code still compiles. `rust-ai-native-tcg` is necessary-not-sufficient; it pairs with Class C/D oracles (contracts, differential tests) that check INTENT, not just types. (The CITYWALK false-positive trap, DR2-012 caveat.) @status:spec/done

## 7. One-line summary {#summary}
@fact:ONE-LINE-SUMMARY `rust-ai-native-tcg` makes a weak agent generate well-typed, discipline-conformant Rust *by construction* — by masking each completion to rust-analyzer-validated continuations under a constraint profile compiled from the AI-Native Rust guide — standing on rust-analyzer rather than reimplementing the type system, and routed by capability so it lifts the swarm without distorting strong authors. @status:spec/done
