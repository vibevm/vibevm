# Flow: Operating Modes {#root}

This project uses **codeword-triggered operating modes**. The session
has one safe default posture; the owner can flip it into an alternate
posture for one work cycle by speaking a catalogued codeword. Modes
change *confirmation behaviour* — they never change what is
off-limits.

## The default posture {#default}

Unless a codeword is active: routine work proceeds normally, and
anything **non-routine** — history rewrites, force-pushes, large
binary imports, CI/signing/secrets changes, any operation whose
reversal would cost work — stops for the owner's explicit
confirmation. When uncertain whether something is routine, ask.

## Codewords {#codewords}

The project's codewords are catalogued in
[`OPERATING-MODES-PROTOCOL.md`](../flows/operating-modes/OPERATING-MODES-PROTOCOL.md).
Recognise a codeword by intent, not exact wording. When one fires:

1. Acknowledge which mode is now active and what it changes.
2. Apply its operative rules for the current work cycle.
3. Drop back to the default posture when the cycle ends — a mode
   **never** persists into a new session uninvoked.

The catalogue ships one worked mode:
[`mfbt-mode.md`](../flows/operating-modes/mfbt-mode.md) — heads-down
pre-authorised execution («move fast and break things»).

## The red-lines law {#red-lines}

A small set of red lines survives **every** mode: rewriting published
history, force-pushing, importing large binaries, touching
CI/signing/secrets configuration, and any operation whose reversal
costs work. A codeword removes the "may I proceed with routine work?"
handshake; it does **not** remove the "may I cross an irreversible
threshold?" handshake. No codeword can be defined that erodes this.

## Reporting cadence {#cadence}

Under any accelerated mode, report **status, not requests**: "phase N
landed, tests green, moving to N+1" is right; "phase N landed — shall
I proceed?" is the exact overhead the codeword was spoken to remove.

## Never {#never}

- Never cross a red line under any mode — the handshake for
  irreversible operations is unconditional.
- Never carry an active mode across a session boundary; every session
  starts in the default posture.
- Never act on a codeword that is not in the catalogue — propose
  adding it first.
- Never reply to your own completed work with a permission question
  when a mode pre-authorised it — report status and continue.
