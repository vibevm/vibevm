# Operating Modes Protocol {#root}

**Scope of this document.** This file defines *what* a codeword-
triggered operating mode is, *why* a project wants explicit postures
instead of one fixed stance, *how* a codeword is shaped so it is
unambiguous and safe, and the one law every mode obeys — the red
lines that no codeword can erode. The catalogue of a project's actual
codewords lives at the end; this collection ships one worked mode,
[`mfbt-mode.md`](mfbt-mode.md).

## What a mode is {#what}

A session has a **default operating posture**: routine work proceeds,
anything non-routine stops for the owner's confirmation, and the
agent prefers a clarifying question to an irreversible mistake. That
default is right for most sessions.

It is the *wrong* posture when the owner has already thought a large
body of work through, made the decision, and wants it executed end to
end. There, every "shall I proceed?" is overhead the owner already
paid for by approving the activity.

A **codeword** is a short trigger phrase that flips the session into
an alternate posture for one work cycle. The default is unchanged;
codewords are explicit opt-ins, recognised when spoken, in force only
until the work they cover ends. A mode changes *confirmation
behaviour* — never what is permitted.

## Why codewords rather than a settings flag {#why}

- **Opt-in per work cycle.** The posture shift is scoped to what the
  owner is describing right now, not a persistent global that a later
  session inherits by surprise.
- **Recognised by intent.** A spoken phrase carries the owner's
  framing; the agent reads the intent, not a rigid string.
- **Auditable.** The trigger is visible in the transcript. Anyone
  reading the session sees exactly when the posture changed and back.
- **Bounded by law.** Because a codeword only ever relaxes the
  *routine* handshake, the dangerous handshakes (§red-lines) stay put
  by construction — there is no flag that can be left on to erode
  them.

## The five-part codeword shape {#shape}

Every codeword in a project's catalogue is specified with the same
five parts, so none is ambiguous and each states its own limits:

| Part | What it fixes |
|------|---------------|
| **1. Trigger phrase** | the words (and recognisable variants) that activate it; case-insensitive; matched by intent |
| **2. Authoritative description** | the owner's own framing of what the mode is for, recorded verbatim |
| **3. Operative interpretation** | a numbered list of concrete behavioural rules the agent follows while it is active |
| **4. Changes / does NOT change** | what confirmation behaviour it relaxes, and an explicit restatement that the red lines survive it |
| **5. Lifecycle + cadence** | activation, persistence within a session, non-persistence across sessions, the abort words, and the reporting cadence |

A phrase that does not carry all five parts is not yet a codeword —
it is a proposal. Do not act on it until the owner has fixed the five.

## The red-lines law {#red-lines}

This is the invariant the whole practice is built to protect. A small
set of operations survives **every** mode and always requires the
owner's explicit confirmation:

- rewriting published history;
- force-pushing;
- importing large binary blobs;
- changing CI, signing, or secrets configuration;
- **anything whose reversal would cost work.**

A codeword removes the *"may I proceed with routine work?"*
handshake. It does **not** remove the *"may I cross an irreversible
threshold?"* handshake. The two are different questions, and only the
first is ever waived.

Consequences that follow directly:

- No codeword may be defined that relaxes a red line. A proposed mode
  that would is malformed — refuse it and say why.
- If work under an active mode cannot land without crossing a red
  line, the agent stops at that boundary, reports, and asks. This is
  **not** the interruption the mode forbids — it is the mode's own
  escape hatch firing.
- Every other standing invariant of the project (its attribution
  posture, secrets hygiene, licence constraints, output conventions)
  is a red line for this purpose too: modes never touch them.

## Reporting cadence {#cadence}

Even with routine confirmations suspended, an active mode still
reports — as **status, not requests**. "Phase N landed, tests green,
moving to N+1" is the right cadence; "Phase N landed — shall I
proceed?" is the exact overhead the codeword was spoken to remove.
The owner reads these passively and need not answer.

## The catalogue {#catalogue}

A project keeps its codewords catalogued in one place (this document,
in a consuming project). Ship the ones the project actually uses;
recognise a spoken codeword only if it is catalogued, and propose
adding any new one through the five-part shape before acting on it.
This collection ships one worked mode as a reference:

- [`mfbt-mode.md`](mfbt-mode.md) — «move fast and break things»:
  pre-authorised heads-down execution.

## Re-derive for your project {#re-derive}

Copy the prompt-task, not the prompt-implementation. Paste this to
your agent in a fresh session:

```
Read spec/flows/operating-modes/ end to end. Then define THIS
project's operating modes: (1) confirm the default posture and the
project's exact red-line list — the operations that must survive
every mode. (2) For each accelerated posture we actually want, write
a codeword using the five-part shape (trigger, verbatim description,
operative rules, changes/does-not-change with the red lines
restated, lifecycle + cadence). Keep the mfbt reference mode only if
we want it. Show me the catalogue as a draft for the boot file; do
not treat any phrase as active until I approve its five parts.
```

## Summary {#summary}

- A codeword flips the session into an alternate posture for one work
  cycle; the default is unchanged and modes are explicit opt-ins.
- Every codeword is fixed by the five-part shape, or it is a proposal,
  not a mode.
- The red lines survive every mode: a codeword waives the routine
  handshake, never the irreversible-threshold handshake.
- Report status, not requests, while a mode is active.
- Catalogue the modes in one place; propose new ones through the five
  parts before acting.
