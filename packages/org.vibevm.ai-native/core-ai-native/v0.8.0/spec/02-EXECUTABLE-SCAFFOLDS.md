# Executable Scaffolds — The Runnable-Capital Catalog {#root}

<status stage="spec" state="done"/>

##status-line **Discipline v0.2 · status: BETA · T1** @impl/done

##CATALOG-IS-THE-OPERATIONAL-CORE *The operational core of "explanation capital must be runnable capital" (Manifesto §5).* @impl/done

##NINE-CLASSES-CARRY-COGNITION *Nine classes of artifact that carry a strong author's cognition in a form a weak reader CONSUMES rather than re-derives.* @impl/done

##EACH-PATTERN-IS-ALSO-A-CARD *Each pattern is also a card, shipped per-language in each stack's `cards/`; this catalog is the language-neutral overview and the build order.* @impl/done

## 0. Definition {#definition}

##two-properties-at-once-lead An **executable scaffold** has two properties at once: @impl/done
1. ##PROPERTY-RUNS-OR-MACHINE-CHECKS **It runs or machine-checks** — compiles, executes, or evaluates as a checker, emitting a pass/fail (or typed) signal. Prose has no signal. @impl/done
2. ##PROPERTY-CARRIES-COGNITION **It carries cognition** — a weak reader USES it instead of re-deriving the understanding. @impl/done

##prose-fails-the-first-property Prose about a pattern fails (1). @impl/done

##random-utility-fails-the-second-property A random utility fails (2). @impl/done

##SCAFFOLD-IS-UNDERSTANDING-WITH-A-PULSE A scaffold is materialized understanding with a pulse. @impl/done

##empirical-basis-esolang-follow-up **Empirical basis.** EsoLang follow-up (Jun 2026): weak agents given a *written* strategy barely moved (Sonnet 12→12); given an *executable* helper library, they leapt (12→64). @spec/done

##mechanism-not-the-idea-but-the-code Mechanism: mid-tier models lack not the idea but the ability to build the reusable code that carries it (R2C-008). @spec/done

##TRANSFER-TAG-LEGEND **Transfer tags.** [E-strong] = directly supported by that generation result; [E-mid] = supported by other ledger findings; [E-hyp] = first-principles, validate in pilot. @impl/done

##transfer-to-modification-is-the-pilots-job The result is about *generation*; transfer to *modification* is the pilot's job. @spec/done

##floor-haiku-stayed-near-it **The floor.** Even with the executable library, Haiku-4.5 stayed near the floor. @spec/done

##SCAFFOLDS-AMPLIFY-NOT-CREATE Scaffolds amplify capability; they do not create it. @spec/done

## 1. The nine classes {#nine-classes}

| Class | Name | Carries | Transfer | Card |
|---|---|---|---|---|
| ##ROW-CLASS-A **A** @impl/done | Generators / codegen @impl/done | structural decisions, as named inputs to an emitter @impl/done | [E-strong] @impl/done | `scaffold-a-generators` @impl/done |
| ##ROW-CLASS-B **B** @impl/done | Typed builders / typestate @impl/done | protocol correctness, as types the compiler checks @impl/done | [E-mid] @impl/done | `scaffold-b-typed-builders` @impl/done |
| ##ROW-CLASS-C **C** @impl/done | Runnable contracts @impl/done | invariants, as executing assertions/proofs @impl/done | [E-mid] @impl/done | `scaffold-c-runnable-contracts` @impl/done |
| ##ROW-CLASS-D **D** @impl/done | Differential / characterization oracles @impl/done | behavior, as a runnable old-vs-new check @impl/done | [E-mid] @impl/done | `scaffold-d-differential-oracle` @impl/done |
| ##ROW-CLASS-E **E** @impl/done | Per-cell fast verification loop @impl/done | the substrate that makes all signals fast enough @impl/done | [E-strong] @impl/done | `scaffold-e-fast-loop` @impl/done |
| ##ROW-CLASS-F **F** @impl/done | Structured, REQ-citing diagnostics @impl/done | debugging cognition, in the error text @impl/done | [E-mid] @impl/done | `scaffold-f-structured-diagnostics` @impl/done |
| ##ROW-CLASS-G **G** @impl/done | Executable examples / doctests @impl/done | canonical usage, as compiled examples that cannot lie @impl/done | [E-strong] @impl/done | `scaffold-g-doctests` @impl/done |
| ##ROW-CLASS-H **H** @impl/done | Local simulators / reference models @impl/done | subsystem semantics, as a runnable model @impl/done | [E-strong] @impl/done | `scaffold-h-simulators` @impl/done |
| ##ROW-CLASS-I **I** @impl/done | Scaffolded edit operations / codemods @impl/done | a multi-file change, as one checked operation @impl/done | [E-hyp] @impl/done | `scaffold-i-codemods` @impl/done |

##row-details-live-in-the-card Each row's full Applicability/Routine/Checker is in its card. @impl/done

## 2. Build order (transfer-strength × weak-reader leverage) {#build-order}

1. ##BUILD-ORDER-E **E (fast loop)** — substrate; nothing pays off without it. First. @impl/done
2. ##BUILD-ORDER-G-AND-F **G (doctests) + F (diagnostics)** — cheapest runnable capital; guaranteed-truthful few-shot signal. [E-strong]. @impl/done
3. ##BUILD-ORDER-B-AND-C **B (typed builders) + C (contracts)** — convert hallucinations to compile/assert failures at seams. [E-mid]. @impl/done
4. ##BUILD-ORDER-D **D (differential oracles)** — the modification-specific safety net; the class most worth validating for our actual task. [E-mid]. @impl/done
5. ##BUILD-ORDER-A-AND-H **A (generators) + H (simulators)** — highest ceiling, highest cost. [E-strong]. @impl/done
6. ##BUILD-ORDER-I **I (codemods)** — potentially decisive for the swarm, but [E-hyp]; prototype and measure before the guide commits to it. @impl/done

## 3. The scaffold-reality checklist (all four must hold) {#reality-checklist}
- [ ] ##CHECKLIST-RUNS-OR-CHECKS **Runs/checks:** emits pass/fail or typed signal, not prose. @impl/done
- [ ] ##CHECKLIST-CARRIES-COGNITION **Carries cognition:** encodes a decision a weak reader would otherwise re-derive. @impl/done
- [ ] ##CHECKLIST-FAST-ENOUGH **Fast enough:** signal returns inside the per-cell loop budget (<~60s). @impl/done
- [ ] ##CHECKLIST-CANNOT-SILENTLY-LIE **Cannot silently lie:** if it drifts from reality it FAILS (compile error / assert / red test), never misleads. (The doctest-vs-comment distinction.) @impl/done

## 4. The build/use boundary (a pilot hypothesis worth stating) {#build-use-boundary}
##sharp-capability-line-hypothesis There may be a sharp capability line between *building* a scaffold (Classes A, I — emit/parameterize) and *using* one (Classes G, H — consume). @spec/done

##evidence-haiku-did-not-improve Evidence: Haiku did not improve even with the executable library, suggesting the barrier is not the scaffold's presence but the ability to wield it. @spec/done

##WEAKEST-TIER-GETS-CONSUME-ONLY-SCAFFOLDS If true, the weakest swarm tier should receive consume-only scaffolds (G/H) and invoke-only operations (I as a fixed command), never build-it-yourself scaffolds. @spec/done

##prime-pilot-question Prime pilot question (R4). @spec/done
