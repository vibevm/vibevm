# The cognitive load split {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file is the operational
responsibility table derived from the two-process model: *which* work
belongs to which process, *where* both participate and in what
shape, and *what follows* from the one asymmetry that dominates
everything — the AI has no memory between sessions. @status:impl/done

## The three zones {#zones}

### Only the human (the AI can, but badly) {#human-only}

- @fact:HUMAN-ONLY-MEANING-AND-ITS-CONNECTION-TO-REALITY Thinking about the project at the level of meaning and its
  connection to reality. @status:impl/done
- @fact:HUMAN-ONLY-ARCHITECTURE-AT-THE-LEVEL-OF-INTENT Architectural decisions at the level of intent. @status:impl/done
- @fact:HUMAN-ONLY-TASK-PRIORITIES Task priorities. @status:impl/done
- @fact:HUMAN-ONLY-SENSING-WRONG-BEHAVIOUR-BEFORE-A-TEST Sensing that "the system behaves wrong" before any test says so. @status:impl/done
- @fact:HUMAN-ONLY-CARRYING-CONTEXT-BETWEEN-SESSIONS Carrying context between sessions. @status:impl/done
- @fact:HUMAN-ONLY-MAINTAINING-GLOBAL-COHERENCE Maintaining global coherence. @status:impl/done
- @fact:HUMAN-ONLY-DECIDING-WHEN-A-SPECIFICATION-IS-STALE Deciding when a specification is stale. @status:impl/done
- @fact:HUMAN-ONLY-TALKING-TO-USERS Talking to users; understanding what they need. @status:impl/done
- @fact:HUMAN-ONLY-ETHICAL-CALLS Ethical calls. @status:impl/done

### Only the AI (the human can, but wastefully) {#ai-only}

- @fact:AI-ONLY-ARCHITECTURE-AT-THE-LEVEL-OF-SMALL-DETAIL Architectural decisions at the level of small detail. @status:impl/done
- @fact:AI-ONLY-GENERATING-LARGE-CONSISTENT-VOLUMES Generating large volumes of mutually consistent code in minutes. @status:impl/done
- @fact:AI-ONLY-RECALLING-THE-EXACT-SYNTAX-OF-A-THOUSAND-APIS Recalling the exact syntax of a thousand APIs. @status:impl/done
- @fact:AI-ONLY-MECHANICAL-REFACTORS-ACROSS-THE-CODEBASE Mechanical refactors across the whole codebase where the IDE gives
  up. @status:impl/done
- @fact:AI-ONLY-BOILERPLATE Boilerplate: tests, templates, configs. @status:impl/done
- @fact:AI-ONLY-FORMAL-CHECKS Formal checks: does it build, does it lint, is it formatted. @status:impl/done
- @fact:AI-ONLY-HOLDING-EVERY-DETAIL-OF-A-FILE Holding every detail of a file simultaneously (within the context
  window). @status:impl/done

### Both, differently {#both}

| Activity | Human contributes | AI contributes |
|---|---|---|
| @fact:ROW-WRITING-SPECS Writing specs @status:impl/done | the idea and the decision; approves the final text @status:impl/done | structure, formalization, gap-finding @status:impl/done |
| @fact:ROW-CODE-REVIEW Code review @status:impl/done | semantics — does it do what was *meant* @status:impl/done | formal properties — builds, tests pass, lint is clean @status:impl/done |
| @fact:ROW-DEBUGGING Debugging @status:impl/done | the hypothesis @status:impl/done | evidence collection (logs, traces, live probing) and hypothesis testing @status:impl/done |
| @fact:ROW-DOCUMENTATION Documentation @status:impl/done | checks it reflects reality @status:impl/done | generates the update from the diff @status:impl/done |

## The dominating asymmetry: memory {#memory}

@fact:THE-AI-HAS-NO-MEMORY-BETWEEN-SESSIONS **The AI has no memory between sessions. None.** @status:spec/done

@fact:EVERY-SESSION-IS-A-NEW-PROCESS Every session is a
new process that knows nothing of the previous ones. @status:spec/done

@fact:imagine-a-developer-who-never-returns Imagine a
brilliant developer who arrives every morning, works, leaves in the
evening — and never returns. @status:spec/done

@fact:tomorrow-a-different-one-arrives Tomorrow a different one arrives, equally
brilliant, with zero recollection of your project. @status:spec/done

@fact:WHATEVER-THE-DOCUMENTATION-SAYS-IS-ALL-THE-NEW-ARRIVAL-KNOWS Whatever
the documentation says is all the new arrival knows. @status:spec/done

@fact:a-stale-doc-yields-the-old-hash-function If you switched
hash functions yesterday and did not update the docs, today's
developer uses the old one. @status:spec/done

@fact:four-consequences-lead Four consequences, each load-bearing: @status:impl/done

### Record decisions, not facts {#decisions-not-facts}

@fact:BLAKE3-ALONE-IS-A-FACT "We use blake3" is a fact — recoverable from the code in a second. @status:spec/done

@fact:BLAKE3-WITH-ITS-REASON-IS-A-DECISION
"We use blake3 because SHA-256 drags in a dependency we cannot
afford on edge hardware" is a decision — unrecoverable from
anywhere once forgotten. @status:spec/done

@fact:RECORD-DECISIONS-THE-FIRST-KIND-RECORDS-ITSELF Record the second kind; the first kind
records itself. @status:impl/done

@fact:full-practice-pointer The full practice is flow:decision-records. @status:impl/done

### Unwritten knowledge does not exist {#unwritten}

@fact:human-teams-survive-on-tribal-knowledge Human teams survive on tribal knowledge because Vasya can be asked. @status:spec/done

@fact:THE-AI-CANNOT-ASK-VASYA The AI cannot ask Vasya. @status:spec/done

@fact:KNOWLEDGE-NOT-IN-A-FILE-DOES-NOT-EXIST-FOR-THE-AI Knowledge that is not in a file the AI can
read does not exist for the AI — and the AI will decide *without*
it, confidently. @status:spec/done

@fact:WRITE-EVERY-DECISION-DOWN Every time a decision is made, write it down; not
because you will forget (though you will), but because the AI never
knew. @status:impl/done

### The context window is finite working memory {#window}

@fact:THE-WINDOW-HOLDS-EVERYTHING The window holds the specs, the code, the conversation — everything. @status:spec/done

@fact:LONG-SESSIONS-PUSH-EARLY-CONTENT-INTO-THE-IGNORED-ZONE Long sessions push early content into the zone the attention
mechanism effectively ignores ("lost in the middle"): technically
present, statistically unread. @status:spec/done

@fact:SHORT-SESSIONS-BEAT-LONG-ONES-AND-CONSTRAINTS-GO-AT-THE-EDGES Consequences: **short sessions beat long ones**
(five sessions of thirty minutes outperform one of two and a half
hours), and critical constraints belong at the start or end of any
document, never buried in its middle (see
flow:addressable-specs). @status:impl/done

### Write for the whole system {#whole-system}

@fact:write-specs-for-the-ai-is-half-the-truth "Write specs for the AI" is half the truth. @status:spec/done

@fact:you-will-not-remember-and-the-teammate-never-knew In two months *you*
will not remember why the timeout is 600 seconds; the new teammate
never knew. @status:spec/done

@fact:WRITE-EVERY-LOAD-BEARING-FILE-FOR-THREE-READERS Write every load-bearing file for three readers at
once — the AI's next session, your future self, the next human —
from one source. @status:impl/done

@fact:TEXT-THAT-WORKS-FOR-THE-AI-WORKS-FOR-THE-OTHER-TWO Text that works for the AI carries the
other two most of the way, though not for free: where a reader needs
a different shape — the human's end-of-session scan against the
checkpoint the next session reads — the cost is a second rendering
of the one source, never a second source. @status:spec/done

## Delegation rule of thumb {#delegation}

@fact:ASK-WHETHER-THE-WORK-IS-BOUNDED-MECHANICAL-AND-CHECKABLE Before assigning any piece of work, ask: *is this bounded,
mechanical, and verifiable by a formal check?* @status:impl/done

@fact:IF-YES-IT-GOES-TO-THE-AI-WHOLE If yes, it goes to the AI whole. @status:impl/done

@fact:ASK-WHETHER-IT-NEEDS-MEMORY-TASTE-OR-A-LASTING-DECISION *Does it require memory of why, taste, or a decision
that outlives the session?* @status:impl/done

@fact:IF-YES-THE-HUMAN-DECIDES-AND-MAY-DELEGATE-THE-TYPING If yes, the human does the deciding —
and may still delegate the typing. @status:impl/done

## Summary {#summary}

- @fact:SUM-THREE-ZONES Three zones: human-only (meaning, coherence, decisions), AI-only
  (throughput, mechanics, formal checks), and shared work split by
  nature, not by halves. @status:impl/done
- @fact:SUM-ZERO-CROSS-SESSION-MEMORY-DOMINATES The AI's zero cross-session memory dominates the design: record
  decisions, treat unwritten knowledge as nonexistent, keep sessions
  short, write every file for AI + future-you + the next human at
  once. @status:impl/done
- @fact:SUM-DELEGATE-MECHANICAL-WORK-KEEP-DECISIONS-HUMAN Delegate bounded mechanical work whole; keep decisions human even
  when delegating their typing. @status:impl/done
