# Files as IPC {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file reframes the project's text
files — specs, the checkpoint file, the boot file — from
"documentation" into what they actually are in a human-AI system:
the **inter-process communication channel**. @impl/done

##planes-budgets-and-requirements-pointer It defines the three
planes of that channel, the budget each plane runs on, and the four
requirements the channel must satisfy (each delivered by a sibling
flow in this collection). @impl/done

## Documentation is the wrong word {#wrong-word}

##traditional-documentation-is-optional Documentation, traditionally, is text written by people for people:
written after the fact, updated irregularly, frequently stale — and
*optional*. @spec/done

##human-teams-survive-bad-docs-on-side-channels A project survives bad documentation, because human
teams have side channels: Vasya walks over to Petya and asks why
`new_handler.rs` exists, and the knowledge moves through the air. @spec/done

##BETWEEN-THE-HUMAN-AND-THE-AI-THERE-IS-NO-AIR Between the human and the AI there is no air. @spec/done

##no-hallway-no-archive-no-remembering No hallway, no chat
archive the next session reads, no "remember what we discussed
yesterday". @spec/done

##THERE-ARE-ONLY-FILES **There are only files.** @spec/done

##an-unwritten-reason-invites-a-third-handler If the file does not say that
`new_handler.rs` exists because the old handler mishandles
reconnection, the next session may "deduplicate" it — or write a
third handler solving the same problem a third way. @spec/done

##THE-SPEC-TREE-IS-NOT-DOCUMENTATION So the spec tree is not documentation. @impl/done

##THE-SPEC-TREE-IS-IPC It is **IPC** — the single
channel through which two processes exchange state. @impl/done

##A-BROKEN-CHANNEL-MAKES-THE-SYSTEM-STOP A broken channel
(stale specs, an unupdated checkpoint, dead anchors) does not make
the system worse; it makes the system stop. @impl/done

> ##A-SPEC-IS-A-BOUNDARY-OBJECT Sociology has a precise term for artifacts that mediate between
> fundamentally different actors: a **boundary object** (Star &
> Griesemer, 1989) — "plastic enough to adapt to the needs of the
> parties, robust enough to maintain a common identity". A spec is
> exactly that: the human reads it as *intent* ("I want 600 seconds,
> VPN users need the slack"), the AI reads the same lines as
> *instruction* (`const TIMEOUT: u64 = 600`). One file, two
> readings, one identity. @spec/done

## The three planes {#planes}

| Plane | Contents | Direction | Budget and rules |
|---|---|---|---|
| ##ROW-PLANE-CONTROL **Control** @impl/done | the boot file (entry instructions), the checkpoint/WAL file, the specs @impl/done | mostly human → AI @impl/done | loaded every session, so every token is a recurring tax: boot ≤ ~500 tokens, checkpoint ≤ ~3000, one module spec ≤ ~5000 — split when over @impl/done |
| ##ROW-PLANE-DATA **Data** @impl/done | code, tests, spec updates proposed by the AI @impl/done | AI → human, verifiable @impl/done | artifacts are *regenerable*: losing code is an inconvenience, losing a spec is a catastrophe. The spec is the source; the code is the binary. Nobody mourns a binary at recompile @impl/done |
| ##ROW-PLANE-SIGNALS **Signals** @impl/done | git diff, REVIEW markers, changelog lines, broken tests, the end-of-session report @impl/done | both ways @impl/done | every signal is minimal: a diff, not the file; one marker line with a reason; one changelog line. Bandwidth is human attention @impl/done |

##the-report-is-not-a-status-memo The end-of-session report deserves one emphasis: it is not a status
memo for politeness. @impl/done

##READ-THE-END-OF-SESSION-REPORT-EVERY-TIME It is structured input for the next decision
cycle — read it with your eyes, every time. @impl/done

## The four requirements {#requirements}

##any-ipc-mechanism-solves-the-same-four-problems Any IPC mechanism — pipes, sockets, shared memory — has to solve the
same four problems. @spec/done

##THE-FILE-CHANNEL-IS-NO-EXCEPTION The file channel is no exception. @impl/done

##each-requirement-has-its-own-flow Each
requirement is delivered by its own flow in this collection: @impl/done

| Requirement | What it demands | Delivered by |
|---|---|---|
| ##ROW-REQ-ADDRESSABILITY **Addressability** @impl/done | every statement in every file is precisely citable, so a correction costs twenty tokens, not a re-derivation @impl/done | flow:addressable-specs @impl/done |
| ##ROW-REQ-ATOMICITY **Atomicity** @impl/done | every update to the shared state is one logical step, visible and verifiable in one diff @impl/done | flow:git-atomic-commits @impl/done |
| ##ROW-REQ-CONFLICT-PROTOCOL **Conflict protocol** @impl/done | two writers *will* contradict each other; explicit priorities and a loud escalation path resolve it without a race @impl/done | flow:conflict-protocol (and flow:sync-from-code for the sanctioned reverse flow) @impl/done |
| ##ROW-REQ-VISIBILITY **Visibility** @impl/done | a change one process made must be *seen* by the other: session-start reads are cache invalidation, the morning routine re-syncs the human, the diff is the notification @impl/done | flow:wal @impl/done |

## The private buffer {#private-buffer}

##NOT-EVERYTHING-BELONGS-ON-THE-CHANNEL Not everything belongs on the channel. @impl/done

##PRIVATE-NOTES-LIVE-IN-A-DIRECTORY-EXCLUDED-FROM-THE-AGENTS-VIEW Notes the AI should never
read — copy-paste snippets, personal reminders, drafts of
corrections — live in a directory excluded from the agent's view
(`.human/` or equivalent, plus the agent's ignore file). @impl/done

##physical-invisibility-beats-a-logical-prohibition Physical
invisibility beats a logical prohibition for the same reason a
firewall beats a "keep out" sign: the excluded file costs zero
tokens and cannot be "helpfully" acted upon. @spec/done

## Failure smell {#failure-smell}

##the-symptoms-of-a-degraded-channel-lead When the channel degrades, the symptoms are always the same: @spec/done

- ##SYMPTOM-THE-AI-RE-ASKS-WHAT-WAS-SETTLED the AI re-asks what was settled last week; @spec/done
- ##SYMPTOM-FIXES-UNDO-DELIBERATE-CHOICES "fixes" undo deliberate choices; @spec/done
- ##SYMPTOM-THE-HUMAN-RE-READS-WHOLE-FILES the human starts re-reading whole files instead of diffs. @spec/done

##TREAT-ANY-OF-THESE-AS-A-CHANNEL-OUTAGE Treat any of these as a channel outage — stop feature work and
repair the files first. @impl/done

##the-repair-is-never-mysterious-lead The repair is never mysterious: @spec/done

- ##CAUSE-A-PLANE-IS-OVER-BUDGET some plane is over budget, @spec/done
- ##CAUSE-A-FACT-HAS-TWO-HOMES some fact has two homes, @spec/done
- ##CAUSE-A-CHANGE-WAS-NEVER-MADE-VISIBLE or some change was never made visible. @spec/done

## Summary {#summary}

- ##SUM-SPEC-FILES-ARE-THE-ONLY-CHANNEL Spec files are not documentation; they are the only channel two
  processes share. Optional is the one thing they are not. @impl/done
- ##SUM-THREE-PLANES Three planes: control (budgeted, loaded every session), data
  (regenerable artifacts; spec is source, code is binary), signals
  (minimal, attention-bounded). @impl/done
- ##SUM-FOUR-REQUIREMENTS-EACH-A-SIBLING-FLOW Four requirements — addressability, atomicity, conflict rules,
  visibility — each a sibling flow. @impl/done
- ##SUM-KEEP-A-PRIVATE-HUMAN-BUFFER Keep a private human buffer physically invisible to the agent. @impl/done
- ##SUM-SUSPECT-THE-CHANNEL-BEFORE-THE-MODEL When the AI seems to forget or undo things, suspect the channel
  before the model. @impl/done
