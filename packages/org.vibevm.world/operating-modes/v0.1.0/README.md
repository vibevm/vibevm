# `flow:operating-modes` — codeword-triggered work postures

A vibevm `flow` package that installs **operating modes**: a project
keeps one safe default posture and lets the owner flip into an
alternate posture for one work cycle by speaking a catalogued
codeword. A mode changes *confirmation behaviour* — it never changes
what is off-limits. A small set of red lines survives every mode.

The default posture prefers a clarifying question to an irreversible
mistake — right for most sessions, wrong for sustained heads-down
work the owner has already thought through and approved. There, every
"shall I proceed?" is overhead already paid for. A codeword removes
that overhead without removing the guardrails that matter.

This package ships three pieces of content plus a boot snippet:

- `spec/flows/operating-modes/OPERATING-MODES-PROTOCOL.md` — what a
  mode is, the five-part codeword shape, and the red-lines law that
  no codeword can erode.
- `spec/flows/operating-modes/mfbt-mode.md` — a worked reference
  codeword, «move fast and break things»: pre-authorised heads-down
  execution, fully specified in the five-part shape.
- `spec/flows/operating-modes/writing-a-codeword.md` — the procedure
  for adding a new mode to a project's catalogue, with anti-patterns.
- `spec/boot/45-flow-operating-modes.md` — boot snippet: the default
  posture, how codewords fire, the red-lines law, and the never-do
  list.

## Install

```bash
vibe install flow:operating-modes
```

## Uninstall

```bash
vibe uninstall flow:operating-modes
```

Uninstalling removes every file the package wrote, including the boot
snippet. User-owned files are never touched.

## Composition

- `flow:conflict-protocol` — a mode never overrides the
  uncertainty protocol; genuine ambiguity still takes the
  conservative default and a REVIEW marker, even at full speed.
- `flow:atomic-commits` — an accelerated mode still commits one
  logical unit at a time; its frozen-history rule is one of the red
  lines every mode preserves.
- `flow:secrets-hygiene` and `flow:attribution-policy` — standing
  invariants that are red lines for mode purposes: no posture relaxes
  them.
- `flow:campaign-plans` — a campaign is often run under an
  accelerated mode; the plan's phase gates supply the "status, not
  requests" cadence.

## Philosophical background

Extracted from the origin project's operating-modes law, which
generalized its own session-end codeword into a family of postures.
The collection's spirit is the book *AI-native development*, shipped
in Russian inside `flow:redbook` at `spec/book/ru/`.

## License

UPL-1.0. See `LICENSE.md`.
