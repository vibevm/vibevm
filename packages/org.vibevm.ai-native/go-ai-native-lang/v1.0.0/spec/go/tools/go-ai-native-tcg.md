# Tool Spec (high-level): `go-ai-native-tcg` — Token-Level Type-Constrained Generation for Go {#root}

<status stage="spec" state="done"/>

@fact:status-line *Status: vision / component brief — **DELIBERATELY HELD AT STUB DEPTH**, and
**dispositioned VERY-FAR-FUTURE by the owner (2026-07-17, the GO-AI-NATIVE
mandate: «TCG если реализовывать, то только agentic, для token level мы
пока не готовы»)**: token-level (logit-mask) TCG requires an inference
substrate vibevm does not have — `vibe-llm` is an M0 stub, and hosted
agent APIs never expose logits — so this line waits, explicitly and
indefinitely, on local-LLM plumbing.* @status:spec/done

@fact:AGENTIC-DELIVERY-SHIPS-FIRST *The AGENTIC delivery of the same
value ships FIRST and is this package's actual tcg surface: see the
full-parity sibling brief [`vibe-agentic-tcg-go.md`](vibe-agentic-tcg-go.md),
whose gopls oracle is the SAME oracle a future logit-masker would query.* @status:impl/done

@fact:NOTHING-BUILT-THERE-IS-THROWN-AWAY *Nothing built there is thrown away here.* @status:spec/done

> @fact:PARITY-NOTE **Parity note.** The agentic sibling is at full seven-section parity;
> THIS file intentionally stops at the asymmetry, the layering, and the
> staged ambition — the same stub depth as the TS token-level brief. The
> deferred-to-parity sections (decode-loop design stance, the
> inference-substrate component shape, the full risk register, the
> soundness path) are authored when THIS line is commissioned, not
> before. @status:impl/done

## 1. The asymmetry with the Rust and TS tools {#asymmetry}

@fact:TYPESCRIPT-TCD-KNOWLEDGE-EXISTS For TypeScript the type-constrained-decoding KNOWLEDGE exists (Mündler
et al., PLDI'25: ~74.8% compile-error reduction on a TS subset — a
generation-time result, clean-room-fenced). @status:spec/done

@fact:RUST-HAS-NOTHING-AND-THE-HARDEST-TYPE-SYSTEM For Rust nothing exists and
the type system is the hardest of the set (traits, lifetimes). @status:spec/done

@fact:GO-SITS-AT-THE-EASY-END **Go
sits at the easy end of the spectrum:** the type system is deliberately
small — no higher-kinded machinery, no lifetimes, modest generics with
explicit constraints — and `go/types` is a stable, public, documented
stdlib API. @status:spec/done

@fact:LAYER-2-MASK-IS-MORE-TRACTABLE-FOR-GO A Layer-2 semantic mask (in-scope, type-valid
continuations) is more tractable for Go than for either sibling. @status:spec/done

@fact:WHAT-GO-LACKS-IS-AN-INFERENCE-SUBSTRATE What
Go lacks is the same thing everyone lacks: an inference substrate that
exposes logits. @status:spec/done

@fact:THE-ABSENCE-IS-WHY-THIS-LINE-IS-VERY-FAR-FUTURE That absence, not the type theory, is why this line is
very-far-future. @status:spec/done

## 2. What it is {#what-it-is}

@fact:service-masks-completions-lead A generation-time service that masks each completion to **type-valid,
discipline-conformant** Go continuations: @status:spec/done

- @fact:LAYER-1-SYNTACTIC **Layer 1 — syntactic:** a Go grammar mask (Go's grammar is famously
  small; mature CFG tooling, permissively licensed). @status:spec/done
- @fact:LAYER-2-SEMANTIC **Layer 2 — semantic:** at each completion point, query the type
  oracle for in-scope, type-valid continuations and mask to them. The
  oracle EXISTS since the agentic campaign (the gopls bridge,
  TCG-ORACLE-GO-v0.1); a `go/types`-backed completability answer is the
  natural embedding when the decode loop demands per-token latency LSP
  cannot give. @status:spec/done
- @fact:LAYER-3-DISCIPLINE-PROFILES **Layer 3 — discipline profiles:** compile AI-Native Go rules into
  masks — forbid sampling `init()` or an ambient default inside a cell
  (§2/§7 of the guide), require a defined type where a bare primitive
  crosses a seam (§4-B), forbid a `default:` arm on a closed-set switch
  (§5). The guide's bans become generation-time masks. (The agentic
  sibling already ships these as ADVICE; here they would harden into
  masks.) @status:spec/done

## 3. Staged ambition {#staged-ambition}

- @fact:STAGE-0-PREREQUISITE **Stage 0 — prerequisite (DONE via the agentic campaign):** the gopls
  oracle with overlays, protocol, latency posture. @status:impl/done
- @fact:STAGE-1-DECODE-LOOP-INTEGRATION **Stage 1 — decode-loop integration:** wire a completability oracle
  into a local inference runtime (`vibe-llm`) — speculative span +
  validate + backtrack first (IterGen-style), true prefix masks second. @status:spec/done
- @fact:STAGE-2-COVERAGE **Stage 2 — coverage** toward fuller Go within the idiomatic band
  (staying inside the central law). @status:spec/done
- @fact:STAGE-3-DISCIPLINE-PROFILES-AS-MASKS **Stage 3 — discipline profiles** as masks (Layer 3). @status:spec/done
- @fact:STAGE-4-CAPABILITY-ROUTING **Stage 4 — capability routing.** On for the weak swarm; optional/off
  for strong authors (over-constraint can distort strong models —
  DR1-015). @status:spec/done

## 4. Licensing posture {#licensing}

- @fact:LICENSING-GO-STDLIB go/types, go/parser: stdlib (BSD-3) — clean. @status:spec/done
- @fact:LICENSING-CFG-GRAMMAR-TOOLING CFG/grammar tooling: permissive options exist; avoid GPL grammar
  tooling. @status:spec/done
- @fact:LICENSING-NO-THIRD-PARTY-RESEARCH-CODE No third-party research code is needed: Go's masking layers would be
  built on stdlib type-checking APIs; the PLDI'25 repository remains
  clean-room-fenced as everywhere in this project. @status:spec/done

## 5. The honest note {#honest-note}

@fact:SEMANTIC-MASK-IS-MORE-BUILDABLE-BUT-THE-QUESTION-IS-UNCHANGED Go's small type system makes the semantic mask MORE buildable than
either sibling's — and the Discipline's open question is unchanged:
type-constrained decoding is a *generation*-time technique; whether
scaffolds help *modification* is still the pilot's job. @status:spec/done

@fact:WELL-TYPED-CODE-CAN-STILL-BE-WRONG Well-typed code
can still be wrong: the tool pairs with Classes C/D (contracts,
differential fuzz oracles) that check INTENT, not just types. @status:spec/done

@fact:DELIVERY-EXPERIMENTS-SHOULD-INFORM-STAGE-1 The
agentic sibling's delivery experiments should inform whether Stage 1
here is ever worth its cost — measured, not assumed. @status:spec/done
