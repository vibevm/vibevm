# Executable Scaffolds — The Runnable-Capital Catalog
**Discipline v0.2 · status: BETA · T1**

*The operational core of "explanation capital must be runnable capital" (Manifesto §5). Nine classes of artifact that carry a strong author's cognition in a form a weak reader CONSUMES rather than re-derives. Each pattern is also a card in `cards/`; this catalog is the overview and the build order.*

## 0. Definition

An **executable scaffold** has two properties at once:
1. **It runs or machine-checks** — compiles, executes, or evaluates as a checker, emitting a pass/fail (or typed) signal. Prose has no signal.
2. **It carries cognition** — a weak reader USES it instead of re-deriving the understanding.

Prose about a pattern fails (1). A random utility fails (2). A scaffold is materialized understanding with a pulse.

**Empirical basis.** EsoLang follow-up (Jun 2026): weak agents given a *written* strategy barely moved (Sonnet 12→12); given an *executable* helper library, they leapt (12→64). Mechanism: mid-tier models lack not the idea but the ability to build the reusable code that carries it (R2C-008).

**Transfer tags.** [E-strong] = directly supported by that generation result; [E-mid] = supported by other ledger findings; [E-hyp] = first-principles, validate in pilot. The result is about *generation*; transfer to *modification* is the pilot's job.

**The floor.** Even with the executable library, Haiku-4.5 stayed near the floor. Scaffolds amplify capability; they do not create it.

## 1. The nine classes

| Class | Name | Carries | Transfer | Card |
|---|---|---|---|---|
| **A** | Generators / codegen | structural decisions, as named inputs to an emitter | [E-strong] | `scaffold-a-generators` |
| **B** | Typed builders / typestate | protocol correctness, as types the compiler checks | [E-mid] | `scaffold-b-typed-builders` |
| **C** | Runnable contracts | invariants, as executing assertions/proofs | [E-mid] | `scaffold-c-runnable-contracts` |
| **D** | Differential / characterization oracles | behavior, as a runnable old-vs-new check | [E-mid] | `scaffold-d-differential-oracle` |
| **E** | Per-cell fast verification loop | the substrate that makes all signals fast enough | [E-strong] | `scaffold-e-fast-loop` |
| **F** | Structured, REQ-citing diagnostics | debugging cognition, in the error text | [E-mid] | `scaffold-f-structured-diagnostics` |
| **G** | Executable examples / doctests | canonical usage, as compiled examples that cannot lie | [E-strong] | `scaffold-g-doctests` |
| **H** | Local simulators / reference models | subsystem semantics, as a runnable model | [E-strong] | `scaffold-h-simulators` |
| **I** | Scaffolded edit operations / codemods | a multi-file change, as one checked operation | [E-hyp] | `scaffold-i-codemods` |

Each row's full Applicability/Routine/Checker is in its card.

## 2. Build order (transfer-strength × weak-reader leverage)

1. **E (fast loop)** — substrate; nothing pays off without it. First.
2. **G (doctests) + F (diagnostics)** — cheapest runnable capital; guaranteed-truthful few-shot signal. [E-strong].
3. **B (typed builders) + C (contracts)** — convert hallucinations to compile/assert failures at seams. [E-mid].
4. **D (differential oracles)** — the modification-specific safety net; the class most worth validating for our actual task. [E-mid].
5. **A (generators) + H (simulators)** — highest ceiling, highest cost. [E-strong].
6. **I (codemods)** — potentially decisive for the swarm, but [E-hyp]; prototype and measure before the guide commits to it.

## 3. The scaffold-reality checklist (all four must hold)
- [ ] **Runs/checks:** emits pass/fail or typed signal, not prose.
- [ ] **Carries cognition:** encodes a decision a weak reader would otherwise re-derive.
- [ ] **Fast enough:** signal returns inside the per-cell loop budget (<~60s).
- [ ] **Cannot silently lie:** if it drifts from reality it FAILS (compile error / assert / red test), never misleads. (The doctest-vs-comment distinction.)

## 4. The build/use boundary (a pilot hypothesis worth stating)
There may be a sharp capability line between *building* a scaffold (Classes A, I — emit/parameterize) and *using* one (Classes G, H — consume). Evidence: Haiku did not improve even with the executable library, suggesting the barrier is not the scaffold's presence but the ability to wield it. If true, the weakest swarm tier should receive consume-only scaffolds (G/H) and invoke-only operations (I as a fixed command), never build-it-yourself scaffolds. Prime pilot question (R4).
