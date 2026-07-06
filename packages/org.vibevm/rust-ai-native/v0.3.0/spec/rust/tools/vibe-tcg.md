# Tool Spec (high-level): `vibe-tcg` — Type-Aware Constrained Generation for Rust
*Status: vision / component brief for the vibevm tool suite. NOT an implementation plan. Derived from R2C-005 (type-constrained decoding is per-language manual work; no Rust impl exists), DR2-012/DR1-014 (the 74.8% compile-error reduction in TypeScript), and the constrained-decoding ecosystem scan (SynCode, XGrammar-2, IterGen, Mündler PLDI'25).*

## 1. What problem it solves

LLMs emit Rust that fails to compile; per the PLDI'25 evidence, ~94% of compile errors are TYPE errors, not syntax — and only ~6% are syntactic. Pure grammar/CFG constraining (mature: SynCode, XGrammar-2) catches the 6% and leaves the 94%. The gap for Rust specifically: no type-aware constrained-generation tool exists (the PLDI'25 authors built it only for a TypeScript subset and state plainly it must be re-implemented per language). `vibe-tcg` is that missing tool for Rust, delivered as a vibevm component so the swarm's weak agents generate well-typed Rust by construction rather than by retry.

**Strategic placement in the scaffold catalog:** this is the generation-time complement to the post-generation `cargo check` loop (Class E). The loop catches errors AFTER a full generation; `vibe-tcg` prevents a class of them DURING generation. Both are wanted; the loop is buildable today, `vibe-tcg` is the harder, higher-leverage bet for the weak-agent swarm (DR1-015: constraints help weak models most).

## 2. Design stance (consequences of what we read)

- **Do NOT reimplement rustc's type system.** The PLDI'25 cost was high precisely because they hand-built type-reachability. Rust's trait resolution + lifetime inference is far heavier than TypeScript's. Building a from-scratch incremental Rust type checker is a multi-year effort and a likely dead end.
- **Stand on `rust-analyzer` instead.** It already IS an incremental, query-based (salsa) analyzer that answers "what's in scope here, with what type" at a cursor — exactly the oracle a type-aware constrainer needs. The tool's core bet: expose rust-analyzer's existing analysis as a generation-time completion oracle, rather than rebuild it.
- **Two-layer constraint, matching the literature's split:**
  - *Layer 1 — syntactic (mature, cheap):* a Rust CFG mask via an existing engine (XGrammar-2/SynCode-class). Guarantees parseable Rust. This is solved tech; integrate, don't invent.
  - *Layer 2 — semantic (the novel, valuable part):* at each completion point where an identifier/expression is sampled, query rust-analyzer for the set of in-scope, type-valid continuations (callable functions whose signature fits, fields of the right type, trait methods in scope, variants for an exhaustive match) and mask to that set. This is the Rust analogue of Mündler's "search over inhabitable types," but backed by rust-analyzer rather than a bespoke type engine.
- **Speculative + backtracking, per IterGen:** full per-token rust-analyzer queries are too slow. Speculatively decode multi-token spans, validate the span against the analyzer, backtrack on rejection. The interpreter-budget result (R2C: feedback amplifies capable agents) implies the tool should expose WHY a span was rejected, not just reject it.

## 3. Component shape (how it fits vibevm)

- **Surface:** an inference-time service the agent harness calls during generation, parameterized by (a) the file/cursor context, (b) the assembled cell context from the pager, (c) the active constraint profile.
- **Constraint profiles (tie to the discipline):** profiles are not just "valid Rust" but "valid AI-Native Rust" — e.g. a profile that forbids sampling a `.unwrap()` continuation in a domain cell, or forbids constructing a primitive where a newtype seam exists (Class B enforcement at generation time), or requires the next item to carry a contract block (R3-002). The discipline's bans become generation-time masks, not just post-hoc lints. This is the deepest integration: **the guide's rules compile into tcg constraint profiles.**
- **Output:** well-typed (and discipline-conformant) Rust spans + a structured trace of what was masked and why (feeds Class F diagnostics, R3-011).
- **Determinism note:** the masking is deterministic given (model logits, analyzer state, profile); only the model's sampling is stochastic. This keeps the tool auditable (A1) — every rejected continuation has a recorded reason.

## 4. Staged ambition (easy wins first, per the licensing/realism posture)

**Stage 1 — syntactic profile only.** Integrate a CFG engine (XGrammar-2-class, Apache-2.0) with a Rust grammar. Catches the ~6%. Cheap, ships fast, immediately useful. Validates the harness integration.

**Stage 2 — scope/name constraining.** Use rust-analyzer to mask identifier completions to in-scope names with the right arity. Catches a chunk of the "hallucinated library/feature that doesn't exist" failure (the exact failure Matt Welsh reports for Rust newbies in 2026). No full type inference yet.

**Stage 3 — type-valid continuations.** Full Layer 2: mask to type-inhabitable continuations via rust-analyzer. This is the PLDI'25-equivalent leap, and where the 74.8%-class gains would come from IF they transfer to Rust (unproven — measure).

**Stage 4 — discipline profiles.** Compile AI-Native Rust rules into constraint masks (no-unwrap-in-domain, newtype-at-seam, contract-block-required). The discipline enforced at generation time.

**Maximum-perfection hard path (separate, flagged):** a soundness layer proving the mask never excludes a valid completion (the L(A) ⊆ L vs ⊇ L property the PLDI'25 paper formalizes). Likely needs a formal model of the supported Rust subset. High cost; only after Stages 1–3 demonstrate value.

## 5. Licensing posture (per project policy)
- `rust-analyzer`: MIT/Apache-2.0 — permissive, safe to build on. The central dependency is clean.
- CFG engines: XGrammar (Apache-2.0), Outlines (Apache-2.0), SynCode (check current license) — permissive options exist; avoid any GPL-licensed grammar tooling.
- The PLDI'25 reproduction package (eth-sri/type-constrained-code-generation) is a DESIGN reference (read the algorithm), not a dependency — and it's TypeScript-specific regardless.
- Net: the whole stack is buildable permissively. No viral-license trap on the critical path.

## 6. The honest risk register
- **Transfer unproven:** the 74.8% is TypeScript. Rust's richer types may yield smaller gains, or the per-completion analyzer latency may make Stage 3 impractical for interactive generation. Measure at Stage 2 before committing to Stage 3.
- **rust-analyzer was built for IDEs, not decoding loops:** query latency and partial-file analysis under a half-written buffer may need work; rust-analyzer's tolerance for incomplete code is an asset here but its per-query cost in a tight decode loop is the open engineering risk.
- **Over-constraint can hurt strong models (DR1-015: Hermes-4-405B dropped 92.5%→35.0%).** The tool must be CAPABILITY-ROUTED: on for the weak swarm, optional/off for strong authors. A profile that helps Qwen-32B may distort Opus.
- **It does not fix semantics, only well-typedness:** well-typed wrong code still compiles. `vibe-tcg` is necessary-not-sufficient; it pairs with Class C/D oracles (contracts, differential tests) that check INTENT, not just types. (The CITYWALK false-positive trap, DR2-012 caveat.)

## 7. One-line summary
`vibe-tcg` makes a weak agent generate well-typed, discipline-conformant Rust *by construction* — by masking each completion to rust-analyzer-validated continuations under a constraint profile compiled from the AI-Native Rust guide — standing on rust-analyzer rather than reimplementing the type system, and routed by capability so it lifts the swarm without distorting strong authors.
