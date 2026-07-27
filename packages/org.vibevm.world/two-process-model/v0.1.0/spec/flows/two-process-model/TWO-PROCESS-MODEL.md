# The Two-Process Model {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines the mental model the
whole redbook collection rests on: *what* the human and the AI each
are as computational processes, *why* the two dominant metaphors for
working with an AI fail on real projects, *how* a productive cycle
between the two processes looks, and *what* standing consequences
follow. @impl/done

##every-other-flow-is-a-consequence Every other flow in the collection is a consequence of this
model. @spec/done

## The two wrong metaphors {#wrong-metaphors}

##everyone-begins-with-one-of-two-mental-models Everyone who starts working with a language model on a production
project begins with one of two mental models. @spec/done

##BOTH-FAIL-AND-BOTH-FAIL-THE-SAME-WAY Both fail, and both fail the same way. @spec/done

### Boss and subordinate {#boss}

##the-boss-and-subordinate-loop The human formulates the task, the AI executes, the human inspects,
the AI fixes. @spec/done

##the-loop-is-familiar Familiar — most of the industry runs on it. @spec/done

##THE-AI-IS-A-BAD-SUBORDINATE But the AI
is a *bad subordinate*: it remembers nothing between sessions, it
does not ask clarifying questions when the task is vague — it
guesses, confidently and often wrongly — and it does not learn from
last week's corrections, because for it there was no last week. @spec/done

##THE-DEEPER-FAILURE-IS-LOAD-DISTRIBUTION The deeper failure is load distribution. @spec/done

##THE-HUMAN-CARRIES-ALL-OF-THE-THINKING In this model the human
carries all of the thinking: planning, decomposition, verification,
the big picture. @spec/done

##the-ai-contributes-fast-hands The AI contributes fast hands. @spec/done

##fast-hands-are-an-expensive-goal If fast hands were
the goal, the strongest model on the market would be an expensive
way to buy them. @spec/done

### Human and tool {#tool}

##the-ai-as-a-very-good-autocomplete The AI as a very good autocomplete: start a function, it finishes;
describe a component, it generates. @spec/done

##A-TOOL-HAS-NO-NOTION-OF-THE-PROJECT Fine for small tasks, corrosive
for a months-long project — a tool has no notion of the project. @spec/done

##A-TOOL-OPTIMIZES-LOCALLY-AND-DESTROYS-GLOBAL-CONSISTENCY It
optimizes locally (this file, this function) and can quietly destroy
global consistency doing so. @spec/done

##EVERY-INVOCATION-IS-A-DISCONNECTED-ACT Every invocation is a disconnected act. @spec/done

### The shared mistake {#shared-mistake}

##THE-HUMAN-TAKES-ON-ONE-HUNDRED-PERCENT-OF-THE-COGNITIVE-LOAD In both models the human takes on **one hundred percent of the
cognitive load**, and the human is not built for sustained 100%
load on non-routine work. @spec/done

##BOTH-MODELS-WASTE-THE-ACTUAL-OPPORTUNITY Both models also waste the actual
opportunity: distributing *thinking* — not typing — across two
processes with different architectures. @spec/done

## Coprocessors {#coprocessors}

##picture-a-cpu-and-a-gpu Picture a system with two processors of *different architectures*
working one task — a CPU and a GPU. @spec/done

##the-cpu-and-the-gpu-each-win-elsewhere The CPU wins on sequential logic
with deep dependencies, the GPU on massive parallelism of simple
operations. @spec/done

##NEITHER-IS-BETTER-THE-POWER-IS-THE-SPLIT Neither is "better"; the system's power is the split. @spec/done

| | Strong at | Weak at |
|---|---|---|
| ##ROW-HUMAN **Human** @spec/done | persistent memory (weeks, years); intent and the "spirit" of a decision; intuition ahead of formalization; deep verification across the whole project; decisions under uncertainty; taste — aesthetic, ethical, UX @spec/done | throughput (reads and writes slowly); mechanical consistency (typos, forgotten twin files); holding many details at once (7±2); routine repetition; work under fatigue @spec/done |
| ##ROW-AI **AI** @spec/done | throughput (thousands of consistent lines per minute); mechanical consistency within a session; broad shallow erudition (syntaxes, APIs); routine transforms and boilerplate; formal structure (parse, transform, generate); tirelessness within the session budget @spec/done | persistent memory (none across sessions); the spirit of a decision (follows the letter); long-range coherence; decisions needing context beyond the window; catching its own errors; volition — it cannot want the project to succeed @spec/done |

##THE-TWO-COLUMNS-ARE-COMPLEMENTARY The key observation: **the two columns are complementary**. @spec/done

##THE-WEAKNESSES-OF-EACH-ARE-THE-STRENGTHS-OF-THE-OTHER The weaknesses of each process are the strengths of the other. @spec/done

##the-human-holds-the-picture-the-ai-holds-the-details The human
is slow but remembers everything and holds the picture; the AI is
fast but forgets everything and holds the details. @spec/done

##ROUTING-WITH-THE-GRAIN-BEATS-EITHER-PARTICIPANT-ALONE A system that
routes work with the grain of this table gets more than either
participant alone. @spec/done

##ROUTING-AGAINST-THE-GRAIN-EXHAUSTS-THE-HUMAN A system that routes against it gets an exhausted
human supervising a drifting machine. @spec/done

## The boundary moves; one thing does not {#boundary}

##the-line-shifts-with-capability-criticality-and-the-hour Where exactly the line sits between "human work" and "AI work"
shifts with model capability, with the project's criticality, even
with the hour of the day. @spec/done

##DO-NOT-FREEZE-THE-TABLE-INTO-DOGMA Do not freeze the table into dogma —
recalibrate it per project and per year. @impl/done

##ONE-ASSIGNMENT-NEVER-MOVES-THE-HUMAN-OWNS-COHERENCE One assignment never moves: **the human owns coherence** — the
agreement between iterations, between modules, between what the
system does and what it is for. @impl/done

##the-ai-cannot-own-coherence The AI cannot own it, because
coherence lives across sessions and the AI does not. @spec/done

##EVERYTHING-ELSE-EXISTS-TO-MAKE-THAT-JOB-CHEAP Everything else
in this collection — checkpoint files, addressable specs, conflict
rules — exists to make that one human job cheap enough to actually
perform. @impl/done

## What a productive cycle looks like {#cycle}

##the-cycle-lead A working session is a loop in which each side does what the other
cannot: @impl/done

1. ##CYCLE-THE-HUMAN-DECIDES **The human decides.** Reads the current state (the checkpoint
   file), makes the one pending decision (a timeout value, an
   approach), updates the spec. Minutes, not hours. @impl/done
2. ##CYCLE-THE-AI-GENERATES **The AI generates.** Receives a precise task with an address
   into the spec ("implement §5.3; the timeout changed yesterday —
   re-read it; do not touch the matcher"). Produces code, tests,
   and the updated shared state. @impl/done
3. ##CYCLE-THE-HUMAN-VERIFIES **The human verifies.** Reads the diff, not the codebase. Checks
   that the tests cover what the spec says. Commits. @impl/done
4. ##CYCLE-THE-STATE-CARRIES-OVER **The state carries over.** The checkpoint file and the spec —
   not anyone's memory — carry the result into the next session. @impl/done

##the-unproductive-mirror-image The same task given as "finish the verification module" produces the
unproductive mirror image: the AI re-reads everything, guesses the
intent, "improves" adjacent code, and the human spends two hours
partitioning a diff that mixes the asked-for change with three
unasked ones. @spec/done

##THE-DIFFERENCE-IS-WHETHER-THE-HUMAN-DID-STEPS-ONE-AND-THREE Same model, same tooling — the difference is whether the human did
steps 1 and 3. @spec/done

## The model is dated, deliberately {#dated}

##the-model-fits-the-current-generation This is the model for the current generation of AI: strong enough to
be a real partner, not yet reliable enough to be autonomous. @spec/done

##the-model-will-change It will
change. @spec/done

##RE-DERIVE-THE-SPLIT-WHEN-CAPABILITIES-MOVE Re-derive the split when the underlying capabilities move —
that is what the revisit discipline (flow:decision-records) is for. @impl/done

## Re-derive for your project {#re-derive}

##COPY-THE-PROMPT-TASK-NOT-THE-PROMPT-IMPLEMENTATION Copy the prompt-task, not the prompt-implementation. @impl/done

##re-derive-lead Paste this to
your agent in a fresh session: @impl/done

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

- ##SUM-BOTH-METAPHORS-FAIL-THE-SAME-WAY Boss/subordinate and human/tool both fail the same way: the human
  carries 100% of the thinking. @spec/done
- ##SUM-COPROCESSORS-WITH-COMPLEMENTARY-PROFILES Human and AI are coprocessors with complementary profiles; route
  work with the grain. @spec/done
- ##SUM-THE-BOUNDARY-MOVES-BUT-COHERENCE-DOES-NOT The boundary between zones moves; human ownership of coherence
  does not. @impl/done
- ##SUM-THE-PRODUCTIVE-CYCLE A productive cycle is decide → generate → verify → carry over, and
  the human's steps are the cheap ones — if the shared state is
  maintained. @impl/done
- ##SUM-EVERYTHING-ELSE-KEEPS-THAT-CYCLE-CHEAP Everything else in this collection exists to keep that cycle cheap. @impl/done
