# The Two-Process Model {#root}

**Scope of this document.** This file defines the mental model the
whole redbook collection rests on: *what* the human and the AI each
are as computational processes, *why* the two dominant metaphors for
working with an AI fail on real projects, *how* a productive cycle
between the two processes looks, and *what* standing consequences
follow. Every other flow in the collection is a consequence of this
model.

## The two wrong metaphors {#wrong-metaphors}

Everyone who starts working with a language model on a production
project begins with one of two mental models. Both fail, and both
fail the same way.

### Boss and subordinate {#boss}

The human formulates the task, the AI executes, the human inspects,
the AI fixes. Familiar — most of the industry runs on it. But the AI
is a *bad subordinate*: it remembers nothing between sessions, it
does not ask clarifying questions when the task is vague — it
guesses, confidently and often wrongly — and it does not learn from
last week's corrections, because for it there was no last week.

The deeper failure is load distribution. In this model the human
carries all of the thinking: planning, decomposition, verification,
the big picture. The AI contributes fast hands. If fast hands were
the goal, the strongest model on the market would be an expensive
way to buy them.

### Human and tool {#tool}

The AI as a very good autocomplete: start a function, it finishes;
describe a component, it generates. Fine for small tasks, corrosive
for a months-long project — a tool has no notion of the project. It
optimizes locally (this file, this function) and can quietly destroy
global consistency doing so. Every invocation is a disconnected act.

### The shared mistake {#shared-mistake}

In both models the human takes on **one hundred percent of the
cognitive load**, and the human is not built for sustained 100%
load on non-routine work. Both models also waste the actual
opportunity: distributing *thinking* — not typing — across two
processes with different architectures.

## Coprocessors {#coprocessors}

Picture a system with two processors of *different architectures*
working one task — a CPU and a GPU. The CPU wins on sequential logic
with deep dependencies, the GPU on massive parallelism of simple
operations. Neither is "better"; the system's power is the split.

| | Strong at | Weak at |
|---|---|---|
| **Human** | persistent memory (weeks, years); intent and the "spirit" of a decision; intuition ahead of formalization; deep verification across the whole project; decisions under uncertainty; taste — aesthetic, ethical, UX | throughput (reads and writes slowly); mechanical consistency (typos, forgotten twin files); holding many details at once (7±2); routine repetition; work under fatigue |
| **AI** | throughput (thousands of consistent lines per minute); mechanical consistency within a session; broad shallow erudition (syntaxes, APIs); routine transforms and boilerplate; formal structure (parse, transform, generate); tirelessness within the session budget | persistent memory (none across sessions); the spirit of a decision (follows the letter); long-range coherence; decisions needing context beyond the window; catching its own errors; volition — it cannot want the project to succeed |

The key observation: **the two columns are complementary**. The
weaknesses of each process are the strengths of the other. The human
is slow but remembers everything and holds the picture; the AI is
fast but forgets everything and holds the details. A system that
routes work with the grain of this table gets more than either
participant alone. A system that routes against it gets an exhausted
human supervising a drifting machine.

## The boundary moves; one thing does not {#boundary}

Where exactly the line sits between "human work" and "AI work"
shifts with model capability, with the project's criticality, even
with the hour of the day. Do not freeze the table into dogma —
recalibrate it per project and per year.

One assignment never moves: **the human owns coherence** — the
agreement between iterations, between modules, between what the
system does and what it is for. The AI cannot own it, because
coherence lives across sessions and the AI does not. Everything else
in this collection — checkpoint files, addressable specs, conflict
rules — exists to make that one human job cheap enough to actually
perform.

## What a productive cycle looks like {#cycle}

A working session is a loop in which each side does what the other
cannot:

1. **The human decides.** Reads the current state (the checkpoint
   file), makes the one pending decision (a timeout value, an
   approach), updates the spec. Minutes, not hours.
2. **The AI generates.** Receives a precise task with an address
   into the spec ("implement §5.3; the timeout changed yesterday —
   re-read it; do not touch the matcher"). Produces code, tests,
   and the updated shared state.
3. **The human verifies.** Reads the diff, not the codebase. Checks
   that the tests cover what the spec says. Commits.
4. **The state carries over.** The checkpoint file and the spec —
   not anyone's memory — carry the result into the next session.

The same task given as "finish the verification module" produces the
unproductive mirror image: the AI re-reads everything, guesses the
intent, "improves" adjacent code, and the human spends two hours
partitioning a diff that mixes the asked-for change with three
unasked ones. Same model, same tooling — the difference is whether
the human did steps 1 and 3.

## The model is dated, deliberately {#dated}

This is the model for the current generation of AI: strong enough to
be a real partner, not yet reliable enough to be autonomous. It will
change. Re-derive the split when the underlying capabilities move —
that is what the revisit discipline (flow:decision-records) is for.

## Re-derive for your project {#re-derive}

Copy the prompt-task, not the prompt-implementation. Paste this to
your agent in a fresh session:

```
Read spec/flows/two-process-model/ end to end. Then look at THIS
project: its criticality, team size, test maturity, and how capable
the models we use actually are. Produce a one-page project-specific
responsibility split: which decisions are human-only here, which
work is delegated to the AI by default, and where the boundary is
deliberately different from the generic table (say why). Propose it
as a draft for the project's boot file. Do not apply until I
approve.
```

## Summary {#summary}

- Boss/subordinate and human/tool both fail the same way: the human
  carries 100% of the thinking.
- Human and AI are coprocessors with complementary profiles; route
  work with the grain.
- The boundary between zones moves; human ownership of coherence
  does not.
- A productive cycle is decide → generate → verify → carry over, and
  the human's steps are the cheap ones — if the shared state is
  maintained.
- Everything else in this collection exists to keep that cycle cheap.
