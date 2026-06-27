# Tool Spec (high-level): `vibe-tcg-ts` — Type-Aware Constrained Generation for TypeScript
*Status: vision / component brief — **DELIBERATELY HELD AT STUB DEPTH**. The TypeScript counterpart to `rust/tools/vibe-tcg.md` — but with a fundamentally different cost profile, because for TypeScript the hard part already exists.*

> **Parity note (read first).** Unlike every other artifact in this package, this brief is **not** brought to Rust parity, by owner decision: the `tcg` line is not being worked now. The Rust counterpart (`rust/tools/vibe-tcg.md`) is a full seven-section component brief (problem · design stance · component shape · staged ambition + max-perfection hard path · licensing · risk register · summary); this file intentionally stops at the asymmetry, the layering, and the staged ambition. The deferred-to-parity sections — **Design stance** (speculative decoding + backtracking; language-service latency in a tight decode loop; the Corsa/TS7 native-compiler angle), **Component shape** (the vibevm integration surface: cursor context, constraint profiles, output trace, determinism/auditability), the **full risk register**, and the **max-perfection soundness path** (the `L(A) ⊆ L` completeness property the PLDI'25 paper formalizes) — are a tracked follow-up, to be authored when the `tcg` work begins. Everything else in this package is at full parity; this one file is the single conscious exception.

## 1. The asymmetry with the Rust tool

For Rust, type-aware constrained decoding does not exist and must be built from scratch over a multi-year horizon (rust-analyzer as the oracle; trait/lifetime constraining is the open research). **For TypeScript it already exists**: Mündler et al. (PLDI'25) implemented type-constrained decoding for a non-trivial subset of TypeScript and measured ~74.8% reduction in compile errors (~94% of TypeScript compile errors are type-level). So `vibe-tcg-ts` is not a from-scratch build — it is a **wrap-and-extend** of existing work, standing on a type checker that TypeScript, unlike Rust at decode time, exposes programmatically (the Compiler API / language service).

## 2. What it is

A generation-time service that masks each completion to **type-valid, discipline-conformant** TypeScript continuations:
- **Layer 1 — syntactic:** a TS grammar mask (mature CFG tooling).
- **Layer 2 — semantic (the valuable part, and the part that already has a reference implementation):** at each completion point, query the TypeScript type checker (Compiler API) for in-scope, type-valid continuations and mask to them. Extend the PLDI'25 subset toward fuller TypeScript.
- **Layer 3 — discipline profiles:** compile AI-Native TypeScript rules into masks — forbid sampling `any`/`as`/`!`/`@ts-ignore` (§8 of the guide), require a branded type where a bare primitive crosses a seam (§4), require `unknown` + validator at a boundary (§2). The guide's bans become generation-time masks.

## 3. Staged ambition
- **Stage 1 — wrap PLDI'25.** Reproduce/integrate the existing TypeScript type-constrained decoder as a vibevm service. Immediate value; the 74.8%-class result is TypeScript-native, not a transfer bet.
- **Stage 2 — extend coverage.** Push the supported subset toward fuller TypeScript (generics, conditional types within the idiomatic band — staying inside the central law, not chasing the OOD type-level tail).
- **Stage 3 — discipline profiles.** Layer-3 masks: the `unsafe`-set bans and branding/boundary requirements enforced at generation time.
- **Stage 4 — capability routing.** On for the weak swarm; optional/off for strong authors (over-constraint can distort strong models — DR1-015).

## 4. Licensing posture
- TypeScript Compiler API: Apache-2.0 — clean.
- The PLDI'25 reproduction package (`eth-sri`): a reference to wrap/extend; check its license before vendoring vs reimplementing the algorithm from the paper. Permissive CFG tooling exists (avoid GPL grammar tooling).
- Net: buildable permissively; no viral-license trap on the critical path.

## 5. The honest note
The 74.8% is real and TypeScript-native (not a transfer claim) — but it is a *generation*-time result. The Discipline's open question (does scaffolding help *modification*, not just generation) is unchanged: `vibe-tcg-ts` makes a weak agent *write* well-typed TypeScript by construction; whether it then *modifies* existing TypeScript safely is still the pilot's job. The tool pairs with Classes C/D (runtime contracts, differential oracles) that check INTENT, since well-typed code can still be wrong — and TypeScript's erasure means well-typed code can still lie at runtime if `as` slipped through (hence the Layer-3 ban on `as` matters even with the type oracle on).
