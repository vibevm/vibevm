# The cognitive load split {#root}

**Scope of this document.** This file is the operational
responsibility table derived from the two-process model: *which* work
belongs to which process, *where* both participate and in what
shape, and *what follows* from the one asymmetry that dominates
everything — the AI has no memory between sessions.

## The three zones {#zones}

### Only the human (the AI can, but badly) {#human-only}

- Thinking about the project at the level of meaning and its
  connection to reality.
- Architectural decisions at the level of intent.
- Task priorities.
- Sensing that "the system behaves wrong" before any test says so.
- Carrying context between sessions.
- Maintaining global coherence.
- Deciding when a specification is stale.
- Talking to users; understanding what they need.
- Ethical calls.

### Only the AI (the human can, but wastefully) {#ai-only}

- Architectural decisions at the level of small detail.
- Generating large volumes of mutually consistent code in minutes.
- Recalling the exact syntax of a thousand APIs.
- Mechanical refactors across the whole codebase where the IDE gives
  up.
- Boilerplate: tests, templates, configs.
- Formal checks: does it build, does it lint, is it formatted.
- Holding every detail of a file simultaneously (within the context
  window).

### Both, differently {#both}

| Activity | Human contributes | AI contributes |
|---|---|---|
| Writing specs | the idea and the decision; approves the final text | structure, formalization, gap-finding |
| Code review | semantics — does it do what was *meant* | formal properties — builds, tests pass, lint is clean |
| Debugging | the hypothesis | evidence collection (logs, traces, live probing) and hypothesis testing |
| Documentation | checks it reflects reality | generates the update from the diff |

## The dominating asymmetry: memory {#memory}

**The AI has no memory between sessions. None.** Every session is a
new process that knows nothing of the previous ones. Imagine a
brilliant developer who arrives every morning, works, leaves in the
evening — and never returns. Tomorrow a different one arrives,
equally brilliant, with zero recollection of your project. Whatever
the documentation says is all the new arrival knows. If you switched
hash functions yesterday and did not update the docs, today's
developer uses the old one.

Four consequences, each load-bearing:

### Record decisions, not facts {#decisions-not-facts}

"We use blake3" is a fact — recoverable from the code in a second.
"We use blake3 because SHA-256 drags in a dependency we cannot
afford on edge hardware" is a decision — unrecoverable from
anywhere once forgotten. Record the second kind; the first kind
records itself. The full practice is flow:decision-records.

### Unwritten knowledge does not exist {#unwritten}

Human teams survive on tribal knowledge because Vasya can be asked.
The AI cannot ask Vasya. Knowledge that is not in a file the AI can
read does not exist for the AI — and the AI will decide *without*
it, confidently. Every time a decision is made, write it down; not
because you will forget (though you will), but because the AI never
knew.

### The context window is finite working memory {#window}

The window holds the specs, the code, the conversation — everything.
Long sessions push early content into the zone the attention
mechanism effectively ignores ("lost in the middle"): technically
present, statistically unread. Consequences: **short sessions beat
long ones** (five sessions of thirty minutes outperform one of two
and a half hours), and critical constraints belong at the start or
end of any document, never buried in its middle (see
flow:addressable-specs).

### Write for the whole system {#whole-system}

"Write specs for the AI" is half the truth. In two months *you*
will not remember why the timeout is 600 seconds; the new teammate
never knew. Write every load-bearing file for three readers at
once — the AI's next session, your future self, the next human —
from one source. If the text works for the AI, it works for the
other two for free.

## Delegation rule of thumb {#delegation}

Before assigning any piece of work, ask: *is this bounded,
mechanical, and verifiable by a formal check?* If yes, it goes to
the AI whole. *Does it require memory of why, taste, or a decision
that outlives the session?* If yes, the human does the deciding —
and may still delegate the typing.

## Summary {#summary}

- Three zones: human-only (meaning, coherence, decisions), AI-only
  (throughput, mechanics, formal checks), and shared work split by
  nature, not by halves.
- The AI's zero cross-session memory dominates the design: record
  decisions, treat unwritten knowledge as nonexistent, keep sessions
  short, write every file for AI + future-you + the next human at
  once.
- Delegate bounded mechanical work whole; keep decisions human even
  when delegating their typing.
