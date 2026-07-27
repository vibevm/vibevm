# The cognitive load split {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file is the operational
responsibility table derived from the two-process model: *which* work
belongs to which process, *where* both participate and in what
shape, and *what follows* from the one asymmetry that dominates
everything — the AI has no memory between sessions. @impl/done

## The three zones {#zones}

### Only the human (the AI can, but badly) {#human-only}

- ##HUMAN-ONLY-MEANING-AND-ITS-CONNECTION-TO-REALITY Thinking about the project at the level of meaning and its
  connection to reality. @impl/done
- ##HUMAN-ONLY-ARCHITECTURE-AT-THE-LEVEL-OF-INTENT Architectural decisions at the level of intent. @impl/done
- ##HUMAN-ONLY-TASK-PRIORITIES Task priorities. @impl/done
- ##HUMAN-ONLY-SENSING-WRONG-BEHAVIOUR-BEFORE-A-TEST Sensing that "the system behaves wrong" before any test says so. @impl/done
- ##HUMAN-ONLY-CARRYING-CONTEXT-BETWEEN-SESSIONS Carrying context between sessions. @impl/done
- ##HUMAN-ONLY-MAINTAINING-GLOBAL-COHERENCE Maintaining global coherence. @impl/done
- ##HUMAN-ONLY-DECIDING-WHEN-A-SPECIFICATION-IS-STALE Deciding when a specification is stale. @impl/done
- ##HUMAN-ONLY-TALKING-TO-USERS Talking to users; understanding what they need. @impl/done
- ##HUMAN-ONLY-ETHICAL-CALLS Ethical calls. @impl/done

### Only the AI (the human can, but wastefully) {#ai-only}

- ##AI-ONLY-ARCHITECTURE-AT-THE-LEVEL-OF-SMALL-DETAIL Architectural decisions at the level of small detail. @impl/done
- ##AI-ONLY-GENERATING-LARGE-CONSISTENT-VOLUMES Generating large volumes of mutually consistent code in minutes. @impl/done
- ##AI-ONLY-RECALLING-THE-EXACT-SYNTAX-OF-A-THOUSAND-APIS Recalling the exact syntax of a thousand APIs. @impl/done
- ##AI-ONLY-MECHANICAL-REFACTORS-ACROSS-THE-CODEBASE Mechanical refactors across the whole codebase where the IDE gives
  up. @impl/done
- ##AI-ONLY-BOILERPLATE Boilerplate: tests, templates, configs. @impl/done
- ##AI-ONLY-FORMAL-CHECKS Formal checks: does it build, does it lint, is it formatted. @impl/done
- ##AI-ONLY-HOLDING-EVERY-DETAIL-OF-A-FILE Holding every detail of a file simultaneously (within the context
  window). @impl/done

### Both, differently {#both}

| Activity | Human contributes | AI contributes |
|---|---|---|
| ##ROW-WRITING-SPECS Writing specs @impl/done | the idea and the decision; approves the final text @impl/done | structure, formalization, gap-finding @impl/done |
| ##ROW-CODE-REVIEW Code review @impl/done | semantics — does it do what was *meant* @impl/done | formal properties — builds, tests pass, lint is clean @impl/done |
| ##ROW-DEBUGGING Debugging @impl/done | the hypothesis @impl/done | evidence collection (logs, traces, live probing) and hypothesis testing @impl/done |
| ##ROW-DOCUMENTATION Documentation @impl/done | checks it reflects reality @impl/done | generates the update from the diff @impl/done |

## The dominating asymmetry: memory {#memory}

##THE-AI-HAS-NO-MEMORY-BETWEEN-SESSIONS **The AI has no memory between sessions. None.** @spec/done

##EVERY-SESSION-IS-A-NEW-PROCESS Every session is a
new process that knows nothing of the previous ones. @spec/done

##imagine-a-developer-who-never-returns Imagine a
brilliant developer who arrives every morning, works, leaves in the
evening — and never returns. @spec/done

##tomorrow-a-different-one-arrives Tomorrow a different one arrives, equally
brilliant, with zero recollection of your project. @spec/done

##WHATEVER-THE-DOCUMENTATION-SAYS-IS-ALL-THE-NEW-ARRIVAL-KNOWS Whatever
the documentation says is all the new arrival knows. @spec/done

##a-stale-doc-yields-the-old-hash-function If you switched
hash functions yesterday and did not update the docs, today's
developer uses the old one. @spec/done

##four-consequences-lead Four consequences, each load-bearing: @impl/done

### Record decisions, not facts {#decisions-not-facts}

##BLAKE3-ALONE-IS-A-FACT "We use blake3" is a fact — recoverable from the code in a second. @spec/done

##BLAKE3-WITH-ITS-REASON-IS-A-DECISION
"We use blake3 because SHA-256 drags in a dependency we cannot
afford on edge hardware" is a decision — unrecoverable from
anywhere once forgotten. @spec/done

##RECORD-DECISIONS-THE-FIRST-KIND-RECORDS-ITSELF Record the second kind; the first kind
records itself. @impl/done

##full-practice-pointer The full practice is flow:decision-records. @impl/done

### Unwritten knowledge does not exist {#unwritten}

##human-teams-survive-on-tribal-knowledge Human teams survive on tribal knowledge because Vasya can be asked. @spec/done

##THE-AI-CANNOT-ASK-VASYA The AI cannot ask Vasya. @spec/done

##KNOWLEDGE-NOT-IN-A-FILE-DOES-NOT-EXIST-FOR-THE-AI Knowledge that is not in a file the AI can
read does not exist for the AI — and the AI will decide *without*
it, confidently. @spec/done

##WRITE-EVERY-DECISION-DOWN Every time a decision is made, write it down; not
because you will forget (though you will), but because the AI never
knew. @impl/done

### The context window is finite working memory {#window}

##THE-WINDOW-HOLDS-EVERYTHING The window holds the specs, the code, the conversation — everything. @spec/done

##LONG-SESSIONS-PUSH-EARLY-CONTENT-INTO-THE-IGNORED-ZONE Long sessions push early content into the zone the attention
mechanism effectively ignores ("lost in the middle"): technically
present, statistically unread. @spec/done

##SHORT-SESSIONS-BEAT-LONG-ONES-AND-CONSTRAINTS-GO-AT-THE-EDGES Consequences: **short sessions beat long ones**
(five sessions of thirty minutes outperform one of two and a half
hours), and critical constraints belong at the start or end of any
document, never buried in its middle (see
flow:addressable-specs). @impl/done

### Write for the whole system {#whole-system}

##write-specs-for-the-ai-is-half-the-truth "Write specs for the AI" is half the truth. @spec/done

##you-will-not-remember-and-the-teammate-never-knew In two months *you*
will not remember why the timeout is 600 seconds; the new teammate
never knew. @spec/done

##WRITE-EVERY-LOAD-BEARING-FILE-FOR-THREE-READERS Write every load-bearing file for three readers at
once — the AI's next session, your future self, the next human —
from one source. @impl/done

##TEXT-THAT-WORKS-FOR-THE-AI-WORKS-FOR-THE-OTHER-TWO If the text works for the AI, it works for the
other two for free. @spec/done

## Delegation rule of thumb {#delegation}

##ASK-WHETHER-THE-WORK-IS-BOUNDED-MECHANICAL-AND-CHECKABLE Before assigning any piece of work, ask: *is this bounded,
mechanical, and verifiable by a formal check?* @impl/done

##IF-YES-IT-GOES-TO-THE-AI-WHOLE If yes, it goes to the AI whole. @impl/done

##ASK-WHETHER-IT-NEEDS-MEMORY-TASTE-OR-A-LASTING-DECISION *Does it require memory of why, taste, or a decision
that outlives the session?* @impl/done

##IF-YES-THE-HUMAN-DECIDES-AND-MAY-DELEGATE-THE-TYPING If yes, the human does the deciding —
and may still delegate the typing. @impl/done

## Summary {#summary}

- ##SUM-THREE-ZONES Three zones: human-only (meaning, coherence, decisions), AI-only
  (throughput, mechanics, formal checks), and shared work split by
  nature, not by halves. @impl/done
- ##SUM-ZERO-CROSS-SESSION-MEMORY-DOMINATES The AI's zero cross-session memory dominates the design: record
  decisions, treat unwritten knowledge as nonexistent, keep sessions
  short, write every file for AI + future-you + the next human at
  once. @impl/done
- ##SUM-DELEGATE-MECHANICAL-WORK-KEEP-DECISIONS-HUMAN Delegate bounded mechanical work whole; keep decisions human even
  when delegating their typing. @impl/done
