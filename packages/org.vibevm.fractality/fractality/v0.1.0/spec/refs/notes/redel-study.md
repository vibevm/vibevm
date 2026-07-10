# Study note — ReDel (recursive delegation toolkit) {#root}

_T2 note (boss-authored) for INVENTORY S12 — repo pin `79eb988`
(MIT + Commons Clause; study-only) + arXiv 2408.02248 (EMNLP 2024
demo; Zhu, Dugan, Callison-Burch, UPenn). First-pass survey
delegated to GLM-5.2 over a sandboxed copy; boss spot-checked the
load-bearing mechanisms verbatim (depth-cap-by-tool-removal at
kanis.py:106, rapidfuzz>80 guard, root_has_tools=False default) —
all held. Decisions and facts only._

## What it is {#what}

The only selected system whose recursion is **agent delegation,
not context slicing**: a root agent the human addresses may
delegate subtasks to spawned children (function-calling tool), who
may delegate further — a runtime tree, fully event-sourced, with a
live/replay web UI. ~2.8k LOC Python + a Vue/d3 viz.

## Facts that matter to us {#facts}

- **Delegation is an ordinary tool** the model chooses to call;
  two shipped schemes: **DelegateOne** (foreground — parent blocks
  in WAITING until the child finishes; parallel children fall out
  of parallel function calling) and **DelegateWait** (background —
  spawn returns immediately with a named helper; a separate
  `wait(name|"next"|"all")` blocks on FIRST_COMPLETED /
  ALL_COMPLETED). Await-any/await-all semantics existed here in
  2024 — the exact shape PROP-001 §7 plans for MC.
- **Depth cap by capability removal:** at `max_delegation_depth`
  (default 8) the child is simply built WITHOUT the delegate tool
  — the cleanest possible enforcement (can't misuse what you don't
  have), vs erroring at call time.
- **Anti-overdelegation guard:** if the delegation instruction is
  >80% fuzzy-similar to the parent's own task, the spawn is
  refused with a corrective string ("do not delegate your whole
  task"). A one-line defense against the pass-the-buck failure.
- **Root keeps no tools by default** (`root_has_tools=False`) —
  the root plans and delegates; workers work. Their experiments
  doc calls removing root tools important. (Independent
  confirmation of our boss-does-judgment posture, from the other
  end.)
- **Event-sourced everything:** every spawn / delegation edge /
  state change / message / token count is one pydantic event on an
  async queue, fanned to listeners, appended as JSONL AOF per
  session + periodic `state.json` snapshot with an event-count
  checksum. The web UI's replay is a **client-side reducer over
  the event log with undo functions** — scrub back and forth
  through a run. Live view: d3 tree, node color = run state, click
  a node → its full chat history.
- **Child→parent contract is a plain string** (joined assistant
  text; exceptions returned as "encountered an exception: …" so
  the parent ALWAYS gets a tool result); DelegateWait truncates at
  6 000 chars. Same weakness as everywhere: unstructured returns.
- Failure modes handled: over-delegation (guard), runaway
  recursion (depth cap), transient API errors (retry_attempts=10,
  then ERRORED state), child verbosity (round caps, truncation).

## Decisions we take {#decisions}

1. **Depth enforcement = capability removal.** A packet spawned at
   its depth ceiling gets a profile/toolset WITHOUT spawn rights
   (MC refuses to grant the surface), not a runtime error. Compose
   with recursive-llm's soft-refusal for the boundary case.
2. **The pass-the-buck guard becomes a fabric check:** MC (or the
   pod) compares a child packet's goal against the parent's; near-
   duplicates are refused with a corrective reason. Cheap,
   mechanical, catches the classic delegation pathology the Ф6
   trial's inverse showed (bosses keeping everything is one
   failure; bosses re-posting their task is the other).
3. **Await-any/await-all are v1 verbs, not horizons:** ReDel had
   both in 2024 as *the* two delegation schemes; Stage B should
   treat `wait(next|all|name)` as core MC client surface.
4. **Event-sourced replay is the observability bar:** our journal
   (I3) already appends events; the deltas to steal as *ideas*:
   (a) delegation EDGES as first-class events (parent id, child
   id, instruction), (b) periodic state snapshot with event-count
   checksum for fast cold loads, (c) a reducer-with-undo client
   over the journal → scrubber replay of any run tree (candidate
   for the V5/GUI horizon and MT evidence capture).
5. **Root-without-tools maps to promotion policy:** granting the
   boss surface should *reduce* a node's work-tools by default
   (plan/delegate/review), aligning incentives with the two-
   process split — worth a Stage B decision record.

**Non-adoptions:** kani/function-calling substrate (we are
process-level); plain-string child returns (structured packets
stay); Commons-Clause'd code stays uncopied per the standing law.
