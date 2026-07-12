# Flow: redbook {#root}

This project follows the **redbook** — a collection of AI-native
development practices, each installed as its own flow with its own
boot snippet and protocol documents. This snippet only names the
collection; the members carry the actual rules.

## The source of the spirit

The practices are distilled from the book *AI-native development*.
The book itself ships in this package at `spec/book/ru/` (currently
in Russian — see `spec/book/README.md` for the edition plan). **The
general spirit of the process comes from the book**: two processes
working one task, files as the only shared memory, decisions
recorded with their reasons, one commit per thought.

Do **not** read the book at session boot — it is reference depth,
not standing instructions. Open a chapter when a philosophy question
actually arises; the members' boot snippets carry everything a
session needs.

## The members (edition 0.2.0)

The book's core — the four IPC requirements and the memory model:

- `two-process-model` — the foundation: human and AI as
  coprocessors; the human owns coherence; files are the IPC.
- `wal` — the checkpoint file and cold-resume snapshot; session
  wind-down and resume rituals.
- `sync-from-code` — the sanctioned reverse path when code changed
  before the spec.
- `atomic-commits` — one commit, one idea; Conventional Commits;
  pushed history is frozen.
- `addressable-specs` — `spec://` URIs and stable anchors; correct
  the agent in twenty tokens.
- `decision-records` — record decisions with reasons and revisit
  triggers, at the anchor they govern.
- `conflict-protocol` — Human > Spec > Tests > Code; REVIEW markers;
  the conservative-default uncertainty path.
- `campaign-plans` — cold-executable plans with phase gates,
  falsifiable predictions, and an execution ledger.
- `discovery-prompt` — the structured collaborative-research prompt
  for open-ended sessions.
- `attribution-policy` — the deliberate authorship posture
  (human-authored surface by default).

The project-practice wave — running a project over the long haul:

- `operating-modes` — codeword-triggered postures; red lines that
  survive every mode.
- `health-audit` — the periodic judgment sweep over what the gate
  cannot see.
- `manual-tests` — human-runnable walkthroughs for the integration
  surfaces automation cannot prove.
- `secrets-hygiene` — surface-secrets never printed or persisted;
  scope discipline; third-party-code consent.
- `licensing` — a deliberate licence posture; permissive-only
  dependencies; the EULA-to-open path.
- `source-mirrors` — single-writer multi-homing; fail-loud
  fast-forward-only fan-out.
- `spec-genres` — contract vs lore vs research vs plans; what goes
  where and who wins.
- `comparative-research` — evergreen competitor studies with two-way
  gap analysis and roadmap deltas.
- `managed-blocks` — how a tool writes into files it does not own
  (for tool authors).
- `qualified-naming` — namespaces for package ecosystems (for
  ecosystem designers).
- `tool-design-lessons` — paid-for lessons for self-updating tools
  and package systems.

An **edition** is a tested set: the umbrella pins every member
exactly, and the umbrella's version is the edition number. Individual
members may move ahead on their own lines between editions.
