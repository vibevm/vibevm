# Files as IPC {#root}

**Scope of this document.** This file reframes the project's text
files — specs, the checkpoint file, the boot file — from
"documentation" into what they actually are in a human-AI system:
the **inter-process communication channel**. It defines the three
planes of that channel, the budget each plane runs on, and the four
requirements the channel must satisfy (each delivered by a sibling
flow in this collection).

## Documentation is the wrong word {#wrong-word}

Documentation, traditionally, is text written by people for people:
written after the fact, updated irregularly, frequently stale — and
*optional*. A project survives bad documentation, because human
teams have side channels: Vasya walks over to Petya and asks why
`new_handler.rs` exists, and the knowledge moves through the air.

Between the human and the AI there is no air. No hallway, no chat
archive the next session reads, no "remember what we discussed
yesterday". **There are only files.** If the file does not say that
`new_handler.rs` exists because the old handler mishandles
reconnection, the next session may "deduplicate" it — or write a
third handler solving the same problem a third way.

So the spec tree is not documentation. It is **IPC** — the single
channel through which two processes exchange state. A broken channel
(stale specs, an unupdated checkpoint, dead anchors) does not make
the system worse; it makes the system stop.

> Sociology has a precise term for artifacts that mediate between
> fundamentally different actors: a **boundary object** (Star &
> Griesemer, 1989) — "plastic enough to adapt to the needs of the
> parties, robust enough to maintain a common identity". A spec is
> exactly that: the human reads it as *intent* ("I want 600 seconds,
> VPN users need the slack"), the AI reads the same lines as
> *instruction* (`const TIMEOUT: u64 = 600`). One file, two
> readings, one identity.

## The three planes {#planes}

| Plane | Contents | Direction | Budget and rules |
|---|---|---|---|
| **Control** | the boot file (entry instructions), the checkpoint/WAL file, the specs | mostly human → AI | loaded every session, so every token is a recurring tax: boot ≤ ~500 tokens, checkpoint ≤ ~3000, one module spec ≤ ~5000 — split when over |
| **Data** | code, tests, spec updates proposed by the AI | AI → human, verifiable | artifacts are *regenerable*: losing code is an inconvenience, losing a spec is a catastrophe. The spec is the source; the code is the binary. Nobody mourns a binary at recompile |
| **Signals** | git diff, REVIEW markers, changelog lines, broken tests, the end-of-session report | both ways | every signal is minimal: a diff, not the file; one marker line with a reason; one changelog line. Bandwidth is human attention |

The end-of-session report deserves one emphasis: it is not a status
memo for politeness. It is structured input for the next decision
cycle — read it with your eyes, every time.

## The four requirements {#requirements}

Any IPC mechanism — pipes, sockets, shared memory — has to solve the
same four problems. The file channel is no exception. Each
requirement is delivered by its own flow in this collection:

| Requirement | What it demands | Delivered by |
|---|---|---|
| **Addressability** | every statement in every file is precisely citable, so a correction costs twenty tokens, not a re-derivation | flow:addressable-specs |
| **Atomicity** | every update to the shared state is one logical step, visible and verifiable in one diff | flow:atomic-commits |
| **Conflict protocol** | two writers *will* contradict each other; explicit priorities and a loud escalation path resolve it without a race | flow:conflict-protocol (and flow:sync-from-code for the sanctioned reverse flow) |
| **Visibility** | a change one process made must be *seen* by the other: session-start reads are cache invalidation, the morning routine re-syncs the human, the diff is the notification | flow:wal |

## The private buffer {#private-buffer}

Not everything belongs on the channel. Notes the AI should never
read — copy-paste snippets, personal reminders, drafts of
corrections — live in a directory excluded from the agent's view
(`.human/` or equivalent, plus the agent's ignore file). Physical
invisibility beats a logical prohibition for the same reason a
firewall beats a "keep out" sign: the excluded file costs zero
tokens and cannot be "helpfully" acted upon.

## Failure smell {#failure-smell}

When the channel degrades, the symptoms are always the same: the AI
re-asks what was settled last week; "fixes" undo deliberate choices;
the human starts re-reading whole files instead of diffs. Treat any
of these as a channel outage — stop feature work and repair the
files first. The repair is never mysterious: some plane is over
budget, some fact has two homes, or some change was never made
visible.

## Summary {#summary}

- Spec files are not documentation; they are the only channel two
  processes share. Optional is the one thing they are not.
- Three planes: control (budgeted, loaded every session), data
  (regenerable artifacts; spec is source, code is binary), signals
  (minimal, attention-bounded).
- Four requirements — addressability, atomicity, conflict rules,
  visibility — each a sibling flow.
- Keep a private human buffer physically invisible to the agent.
- When the AI seems to forget or undo things, suspect the channel
  before the model.
