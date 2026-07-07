# Tool Spec (high-level): `typescript-ai-native-tcg` — Token-Level Type-Constrained Generation for TypeScript
*Status: vision / component brief — **DELIBERATELY HELD AT STUB DEPTH**, and
**dispositioned VERY-FAR-FUTURE by the owner (2026-07-07)**: token-level
(logit-mask) TCG requires an inference substrate vibevm does not have —
`vibe-llm` is an M0 stub, and hosted agent APIs never expose logits — so
this line waits, explicitly and indefinitely, on local-LLM plumbing. The
AGENTIC delivery of the same value shipped FIRST: see the full-parity
sibling brief [`vibe-agentic-tcg-ts.md`](vibe-agentic-tcg-ts.md)
(AGENTIC-TCG-TS-PLAN v0.1) — a consultation oracle over MCP/CLI, whose
language-service core is the SAME oracle a future logit-masker will query.
Nothing built there is thrown away here.*

> **Parity note.** The agentic sibling is at full seven-section parity;
> THIS file intentionally stops at the asymmetry, the layering, and the
> staged ambition. The deferred-to-parity sections — the decode-loop
> design stance (speculative decoding + backtracking; language-service
> latency inside a tight decode loop; the Corsa/TS7 native-compiler
> angle), the inference-substrate component shape, the full risk
> register, and the max-perfection soundness path (the `L(A) ⊆ L`
> completeness property the PLDI'25 paper formalizes) — are authored
> when THIS line is commissioned, not before.

## 1. The asymmetry with the Rust tool

For Rust, type-aware constrained decoding does not exist and must be built
from scratch over a multi-year horizon (rust-analyzer as the oracle;
trait/lifetime constraining is the open research). **For TypeScript the
KNOWLEDGE already exists**: Mündler et al. (PLDI'25) demonstrated
type-constrained decoding for a non-trivial subset of TypeScript and
measured ~74.8% reduction in compile errors (~94% of TypeScript compile
errors are type-level). TypeScript also exposes its checker
programmatically (Compiler API / language service), which Rust at decode
time does not.

**Clean-room rule (owner directive, boot-resident in the dev tree):**
the PLDI'25 reproduction repository (`eth-sri/type-constrained-code-
generation`) is inspiration-only — its code is NEVER copied, adapted,
ported, or vendored. When this line is commissioned, the algorithm is
reimplemented from the PAPER's published ideas in structurally different
code (our oracle stands on the real LanguageService, not a bespoke
subset type engine). The earlier "wrap and extend the existing
implementation" framing predates that directive and is withdrawn.

## 2. What it is

A generation-time service that masks each completion to **type-valid,
discipline-conformant** TypeScript continuations:
- **Layer 1 — syntactic:** a TS grammar mask (mature CFG tooling,
  permissively licensed).
- **Layer 2 — semantic:** at each completion point, query the type
  oracle for in-scope, type-valid continuations and mask to them. The
  oracle EXISTS since the agentic campaign (`tools/ts-oracle`,
  TCG-ORACLE-v0.1) — this line adds the completability discipline and
  the decode-loop integration, not the checker.
- **Layer 3 — discipline profiles:** compile AI-Native TypeScript rules
  into masks — forbid sampling `any`/`as`/`!`/`@ts-ignore` (§8 of the
  guide), require a branded type where a bare primitive crosses a seam
  (§4), require `unknown` + validator at a boundary (§2). The guide's
  bans become generation-time masks. (The agentic sibling already ships
  these as ADVICE; here they harden into masks.)

## 3. Staged ambition

- **Stage 0 — prerequisite (DONE via the agentic campaign):** the
  language-service oracle with overlays, protocol, latency facts.
- **Stage 1 — decode-loop integration:** wire the oracle into a local
  inference runtime (`vibe-llm`) as a completability filter —
  speculative span + validate + backtrack first (IterGen-style), true
  prefix masks second.
- **Stage 2 — extend coverage** toward fuller TypeScript within the
  idiomatic band (staying inside the central law, not chasing the OOD
  type-level tail).
- **Stage 3 — discipline profiles** as masks (Layer 3).
- **Stage 4 — capability routing.** On for the weak swarm; optional/off
  for strong authors (over-constraint can distort strong models —
  DR1-015).

## 4. Licensing posture
- TypeScript Compiler API: Apache-2.0 — clean.
- CFG/grammar tooling: permissive options exist (XGrammar/Outlines
  class); avoid GPL grammar tooling.
- The PLDI'25 repository: **not a code source under any circumstances**
  (clean-room rule above); the paper's published ideas are the
  reference.
- Net: buildable permissively; no viral-license trap on the critical
  path.

## 5. The honest note
The 74.8% is real and TypeScript-native (not a transfer claim) — but it
is a *generation*-time result. The Discipline's open question (does
scaffolding help *modification*, not just generation) is unchanged:
`typescript-ai-native-tcg` makes a weak agent *write* well-typed TypeScript by
construction; whether it then *modifies* existing TypeScript safely is
still the pilot's job. The tool pairs with Classes C/D (runtime
contracts, differential oracles) that check INTENT, since well-typed
code can still be wrong — and TypeScript's erasure means well-typed code
can still lie at runtime if `as` slipped through (hence the Layer-3 ban
on `as` matters even with the type oracle on). The agentic sibling's
two-arm battery is already measuring the consultation form of this
question; its numbers should inform whether Stage 1 here is ever worth
its cost.
