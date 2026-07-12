# Flow: Two-Process Model {#root}

This project runs on the **two-process model**: the human and the AI
are two coprocessors with radically different architectures working
one task. Neither is the boss's subordinate; neither is a tool. The
system's strength is the combination, and each side is assigned only
the work it is structurally good at.

## The architecture

- The **human process** is optimized for: persistent memory across
  sessions, semantic understanding (the intent behind words),
  intuition ("something is wrong" before it can be formalized),
  slow but deep verification, decisions under uncertainty, taste.
- The **AI process** is optimized for: throughput (thousands of
  consistent lines per minute), mechanical consistency within a
  session, broad shallow erudition (syntax of dozens of languages,
  APIs of hundreds of libraries), routine transformations, formal
  structure, tirelessness within the session budget.
- These profiles are **complementary**: the weaknesses of one are the
  strengths of the other. Assign work with the grain, never against
  it.

## Standing consequences

1. **The human owns coherence.** Consistency across sessions, global
   architecture, priorities, and the sense that "the system behaves
   wrong" are human work. Never assume them; losing coherence is this
   system's worst failure mode.
2. **Files are the only shared memory.** Nothing said in a session
   survives it. Whatever must survive goes into the repository. Specs
   are not documentation — they are the inter-process channel; see
   [`files-as-ipc.md`](../flows/two-process-model/files-as-ipc.md).
3. **Precise tasks beat broad ones.** A task that cites the exact
   spec section costs twenty tokens to act on; "finish the module"
   costs a re-derivation of the whole context and invites drift.
4. **Verification is asymmetric.** The AI checks formal properties
   (builds, tests, lint); the human checks semantics (does it do what
   was *meant*). Do not spend human attention on what a machine
   checks, and do not let a machine sign off on meaning.

## Never

- Never take an architectural decision that outlives the session
  without surfacing it to the human — decisions are the human's zone.
- Never optimize locally (one file, one function) at the cost of
  global consistency; when the two conflict, stop and ask.
- Never leave load-bearing knowledge only in the conversation. If it
  matters tomorrow, it lands in a file today.
- Never treat the human as a code generator or the AI as an oracle:
  wrong process, wrong work.

Full model: [`TWO-PROCESS-MODEL.md`](../flows/two-process-model/TWO-PROCESS-MODEL.md).
Responsibility table: [`cognitive-load-split.md`](../flows/two-process-model/cognitive-load-split.md).
The file channel: [`files-as-ipc.md`](../flows/two-process-model/files-as-ipc.md).
