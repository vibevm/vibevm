# Tool Spec (high-level): `typescript-ai-native-tcg` — Token-Level Type-Constrained Generation for TypeScript {#root}

<status stage="spec" state="done"/>

##status-line *Status: vision / component brief — **DELIBERATELY HELD AT STUB DEPTH**, and
**dispositioned VERY-FAR-FUTURE by the owner (2026-07-07)**: token-level
(logit-mask) TCG requires an inference substrate vibevm does not have —
`vibe-llm` is an M0 stub, and hosted agent APIs never expose logits — so
this line waits, explicitly and indefinitely, on local-LLM plumbing.* @spec/done

##AGENTIC-DELIVERY-SHIPS-FIRST *The
AGENTIC delivery of the same value shipped FIRST: see the full-parity
sibling brief [`vibe-agentic-tcg-ts.md`](vibe-agentic-tcg-ts.md)
(AGENTIC-TCG-TS-PLAN v0.1) — a consultation oracle over MCP/CLI, whose
language-service core is the SAME oracle a future logit-masker will query.* @impl/done

##NOTHING-BUILT-THERE-IS-THROWN-AWAY *Nothing built there is thrown away here.* @spec/done

> ##PARITY-NOTE **Parity note.** The agentic sibling is at full seven-section parity;
> THIS file intentionally stops at the asymmetry, the layering, and the
> staged ambition. The deferred-to-parity sections — the decode-loop
> design stance (speculative decoding + backtracking; language-service
> latency inside a tight decode loop; the Corsa/TS7 native-compiler
> angle), the inference-substrate component shape, the full risk
> register, and the max-perfection soundness path (the `L(A) ⊆ L`
> completeness property the PLDI'25 paper formalizes) — are authored
> when THIS line is commissioned, not before. @impl/done

## 1. The asymmetry with the Rust tool {#asymmetry}

##RUST-HAS-NOTHING-AND-A-MULTI-YEAR-HORIZON For Rust, type-aware constrained decoding does not exist and must be built
from scratch over a multi-year horizon (rust-analyzer as the oracle;
trait/lifetime constraining is the open research). @spec/done

##TYPESCRIPT-TCD-KNOWLEDGE-EXISTS **For TypeScript the
KNOWLEDGE already exists**: Mündler et al. (PLDI'25) demonstrated
type-constrained decoding for a non-trivial subset of TypeScript and
measured ~74.8% reduction in compile errors (~94% of TypeScript compile
errors are type-level). @spec/done

##TYPESCRIPT-EXPOSES-ITS-CHECKER-PROGRAMMATICALLY TypeScript also exposes its checker
programmatically (Compiler API / language service), which Rust at decode
time does not. @spec/done

##CLEAN-ROOM-RULE **Clean-room rule (owner directive, boot-resident in the dev tree):**
the PLDI'25 reproduction repository (`eth-sri/type-constrained-code-
generation`) is inspiration-only — its code is NEVER copied, adapted,
ported, or vendored. @impl/done

##REIMPLEMENT-FROM-THE-PAPER-WHEN-COMMISSIONED When this line is commissioned, the algorithm is
reimplemented from the PAPER's published ideas in structurally different
code (our oracle stands on the real LanguageService, not a bespoke
subset type engine). @spec/done

##earlier-framing-is-withdrawn The earlier "wrap and extend the existing
implementation" framing predates that directive and is withdrawn. @impl/done

## 2. What it is {#what-it-is}

##service-masks-completions-lead A generation-time service that masks each completion to **type-valid,
discipline-conformant** TypeScript continuations: @spec/done
- ##LAYER-1-SYNTACTIC **Layer 1 — syntactic:** a TS grammar mask (mature CFG tooling,
  permissively licensed). @spec/done
- ##LAYER-2-SEMANTIC **Layer 2 — semantic:** at each completion point, query the type
  oracle for in-scope, type-valid continuations and mask to them. The
  oracle EXISTS since the agentic campaign (`tools/ts-oracle`,
  TCG-ORACLE-v0.1) — this line adds the completability discipline and
  the decode-loop integration, not the checker. @spec/done
- ##LAYER-3-DISCIPLINE-PROFILES **Layer 3 — discipline profiles:** compile AI-Native TypeScript rules
  into masks — forbid sampling `any`/`as`/`!`/`@ts-ignore` (§8 of the
  guide), require a branded type where a bare primitive crosses a seam
  (§4), require `unknown` + validator at a boundary (§2). The guide's
  bans become generation-time masks. (The agentic sibling already ships
  these as ADVICE; here they harden into masks.) @spec/done

## 3. Staged ambition {#staged-ambition}

- ##STAGE-0-PREREQUISITE **Stage 0 — prerequisite (DONE via the agentic campaign):** the
  language-service oracle with overlays, protocol, latency facts. @impl/done
- ##STAGE-1-DECODE-LOOP-INTEGRATION **Stage 1 — decode-loop integration:** wire the oracle into a local
  inference runtime (`vibe-llm`) as a completability filter —
  speculative span + validate + backtrack first (IterGen-style), true
  prefix masks second. @spec/done
- ##STAGE-2-EXTEND-COVERAGE **Stage 2 — extend coverage** toward fuller TypeScript within the
  idiomatic band (staying inside the central law, not chasing the OOD
  type-level tail). @spec/done
- ##STAGE-3-DISCIPLINE-PROFILES-AS-MASKS **Stage 3 — discipline profiles** as masks (Layer 3). @spec/done
- ##STAGE-4-CAPABILITY-ROUTING **Stage 4 — capability routing.** On for the weak swarm; optional/off
  for strong authors (over-constraint can distort strong models —
  DR1-015). @spec/done

## 4. Licensing posture {#licensing}
- ##LICENSING-TS-COMPILER-API TypeScript Compiler API: Apache-2.0 — clean. @spec/done
- ##LICENSING-CFG-GRAMMAR-TOOLING CFG/grammar tooling: permissive options exist (XGrammar/Outlines
  class); avoid GPL grammar tooling. @spec/done
- ##LICENSING-PLDI-REPOSITORY-IS-NOT-A-CODE-SOURCE The PLDI'25 repository: **not a code source under any circumstances**
  (clean-room rule above); the paper's published ideas are the
  reference. @spec/done
- ##licensing-net Net: buildable permissively; no viral-license trap on the critical
  path. @spec/done

## 5. The honest note {#honest-note}

##THE-MEASURED-RESULT-IS-GENERATION-TIME The 74.8% is real and TypeScript-native (not a transfer claim) — but it
is a *generation*-time result. @spec/done

##OPEN-QUESTION-IS-UNCHANGED The Discipline's open question (does
scaffolding help *modification*, not just generation) is unchanged:
`typescript-ai-native-tcg` makes a weak agent *write* well-typed TypeScript by
construction; whether it then *modifies* existing TypeScript safely is
still the pilot's job. @spec/done

##WELL-TYPED-CODE-CAN-STILL-BE-WRONG The tool pairs with Classes C/D (runtime
contracts, differential oracles) that check INTENT, since well-typed
code can still be wrong — and TypeScript's erasure means well-typed code
can still lie at runtime if `as` slipped through (hence the Layer-3 ban
on `as` matters even with the type oracle on). @spec/done

##DELIVERY-EXPERIMENTS-SHOULD-INFORM-STAGE-1 The agentic sibling's
two-arm battery is already measuring the consultation form of this
question; its numbers should inform whether Stage 1 here is ever worth
its cost. @spec/done
