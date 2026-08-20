# Executable Scaffolds — The Runnable-Capital Catalog {#root}

<status stage="spec" state="done"/>

@fact:status-line **Discipline v0.2 · status: BETA · T1** @status:impl/done

@fact:CATALOG-IS-THE-OPERATIONAL-CORE *The operational core of "explanation capital must be runnable capital" (Manifesto §5).* @status:impl/done

@fact:NINE-CLASSES-CARRY-COGNITION *Nine classes of artifact that carry a strong author's cognition in a form a weak reader CONSUMES rather than re-derives.* @status:impl/done

@fact:EACH-PATTERN-IS-ALSO-A-CARD *Each pattern is also a card, shipped per-language in each stack's `cards/`; this catalog is the language-neutral overview and the build order.* @status:impl/done

## 0. Definition {#definition}

@fact:two-properties-at-once-lead An **executable scaffold** has two properties at once: @status:impl/done
1. @fact:PROPERTY-RUNS-OR-MACHINE-CHECKS **It runs or machine-checks** — compiles, executes, or evaluates as a checker, emitting a pass/fail (or typed) signal. Prose has no signal. @status:impl/done
2. @fact:PROPERTY-CARRIES-COGNITION **It carries cognition** — a weak reader USES it instead of re-deriving the understanding. @status:impl/done

@fact:prose-fails-the-first-property Prose about a pattern fails (1). @status:impl/done

@fact:random-utility-fails-the-second-property A random utility fails (2). @status:impl/done

@fact:SCAFFOLD-IS-UNDERSTANDING-WITH-A-PULSE A scaffold is materialized understanding with a pulse. @status:impl/done

@fact:empirical-basis-esolang-follow-up **Empirical basis.** EsoLang follow-up (Jun 2026): weak agents given a *written* strategy barely moved (Sonnet 12→12); given an *executable* helper library, they leapt (12→64). @status:spec/done

@fact:mechanism-not-the-idea-but-the-code Mechanism: mid-tier models lack not the idea but the ability to build the reusable code that carries it (R2C-008). @status:spec/done

@fact:TRANSFER-TAG-LEGEND **Transfer tags.** [E-strong] = directly supported by that generation result; [E-mid] = supported by other ledger findings; [E-hyp] = first-principles, validate in pilot. @status:impl/done

@fact:transfer-to-modification-is-the-pilots-job The result is about *generation*; transfer to *modification* is the pilot's job. @status:spec/done

@fact:floor-haiku-stayed-near-it **The floor.** Even with the executable library, Haiku-4.5 stayed near the floor. @status:spec/done

@fact:SCAFFOLDS-AMPLIFY-NOT-CREATE Scaffolds amplify capability; they do not create it. @status:spec/done

## 1. The nine classes {#nine-classes}

| Class | Name | Carries | Transfer | Card |
|---|---|---|---|---|
| @fact:ROW-CLASS-A **A** @status:impl/done | Generators / codegen @status:impl/done | structural decisions, as named inputs to an emitter @status:impl/done | [E-strong] @status:impl/done | `scaffold-a-generators` @status:impl/done |
| @fact:ROW-CLASS-B **B** @status:impl/done | Typed builders / typestate @status:impl/done | protocol correctness, as types the compiler checks @status:impl/done | [E-mid] @status:impl/done | `scaffold-b-typed-builders` @status:impl/done |
| @fact:ROW-CLASS-C **C** @status:impl/done | Runnable contracts @status:impl/done | invariants, as executing assertions/proofs @status:impl/done | [E-mid] @status:impl/done | `scaffold-c-runnable-contracts` @status:impl/done |
| @fact:ROW-CLASS-D **D** @status:impl/done | Differential / characterization oracles @status:impl/done | behavior, as a runnable old-vs-new check @status:impl/done | [E-mid] @status:impl/done | `scaffold-d-differential-oracle` @status:impl/done |
| @fact:ROW-CLASS-E **E** @status:impl/done | Per-cell fast verification loop @status:impl/done | the substrate that makes all signals fast enough @status:impl/done | [E-strong] @status:impl/done | `scaffold-e-fast-loop` @status:impl/done |
| @fact:ROW-CLASS-F **F** @status:impl/done | Structured, REQ-citing diagnostics @status:impl/done | debugging cognition, in the error text @status:impl/done | [E-mid] @status:impl/done | `scaffold-f-structured-diagnostics` @status:impl/done |
| @fact:ROW-CLASS-G **G** @status:impl/done | Executable examples / doctests @status:impl/done | canonical usage, as compiled examples that cannot lie @status:impl/done | [E-strong] @status:impl/done | `scaffold-g-doctests` @status:impl/done |
| @fact:ROW-CLASS-H **H** @status:impl/done | Local simulators / reference models @status:impl/done | subsystem semantics, as a runnable model @status:impl/done | [E-strong] @status:impl/done | `scaffold-h-simulators` @status:impl/done |
| @fact:ROW-CLASS-I **I** @status:impl/done | Scaffolded edit operations / codemods @status:impl/done | a multi-file change, as one checked operation @status:impl/done | [E-hyp] @status:impl/done | `scaffold-i-codemods` @status:impl/done |

@fact:row-details-live-in-the-card Each row's full Applicability/Routine/Checker is in its card. @status:impl/done

## 2. Build order (transfer-strength × weak-reader leverage) {#build-order}

1. @fact:BUILD-ORDER-E **E (fast loop)** — substrate; nothing pays off without it. First. @status:impl/done
2. @fact:BUILD-ORDER-G-AND-F **G (doctests) + F (diagnostics)** — cheapest runnable capital; guaranteed-truthful few-shot signal. [E-strong]. @status:impl/done
3. @fact:BUILD-ORDER-B-AND-C **B (typed builders) + C (contracts)** — convert hallucinations to compile/assert failures at seams. [E-mid]. @status:impl/done
4. @fact:BUILD-ORDER-D **D (differential oracles)** — the modification-specific safety net; the class most worth validating for our actual task. [E-mid]. @status:impl/done
5. @fact:BUILD-ORDER-A-AND-H **A (generators) + H (simulators)** — highest ceiling, highest cost. [E-strong]. @status:impl/done
6. @fact:BUILD-ORDER-I **I (codemods)** — potentially decisive for the swarm, but [E-hyp]; prototype and measure before the guide commits to it. @status:impl/done

## 3. The scaffold-reality checklist (all four must hold) {#reality-checklist}
- [ ] @fact:CHECKLIST-RUNS-OR-CHECKS **Runs/checks:** emits pass/fail or typed signal, not prose. @status:impl/done
- [ ] @fact:CHECKLIST-CARRIES-COGNITION **Carries cognition:** encodes a decision a weak reader would otherwise re-derive. @status:impl/done
- [ ] @fact:CHECKLIST-FAST-ENOUGH **Fast enough:** signal returns inside the per-cell loop budget (<~60s). @status:impl/done
- [ ] @fact:CHECKLIST-CANNOT-SILENTLY-LIE **Cannot silently lie:** if it drifts from reality it FAILS (compile error / assert / red test), never misleads. (The doctest-vs-comment distinction.) @status:impl/done

## 4. The build/use boundary (a pilot hypothesis worth stating) {#build-use-boundary}
@fact:sharp-capability-line-hypothesis There may be a sharp capability line between *building* a scaffold (Classes A, I — emit/parameterize) and *using* one (Classes G, H — consume). @status:spec/done

@fact:evidence-haiku-did-not-improve Evidence: Haiku did not improve even with the executable library, suggesting the barrier is not the scaffold's presence but the ability to wield it. @status:spec/done

@fact:WEAKEST-TIER-GETS-CONSUME-ONLY-SCAFFOLDS If true, the weakest swarm tier should receive consume-only scaffolds (G/H) and invoke-only operations (I as a fixed command), never build-it-yourself scaffolds. @status:spec/done

@fact:prime-pilot-question Prime pilot question (R4). @status:spec/done
