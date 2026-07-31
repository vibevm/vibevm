# Operating Modes Protocol {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines *what* a codeword-
triggered operating mode is, *why* a project wants explicit postures
instead of one fixed stance, *how* a codeword is shaped so it is
unambiguous and safe, and the one law every mode obeys — the red
lines that no codeword can erode. @impl/done

##sibling-document-pointers The catalogue of a project's actual
codewords lives at the end; this collection ships one worked mode,
[`mfbt-mode.md`](mfbt-mode.md). @impl/done

## What a mode is {#what}

##a-session-has-a-default-operating-posture-lead A session has a **default operating posture**: @impl/done

- ##DEFAULT-ROUTINE-WORK-PROCEEDS routine work proceeds, @impl/done
- ##DEFAULT-NON-ROUTINE-STOPS-FOR-CONFIRMATION
  anything non-routine stops for the owner's confirmation, @impl/done
- ##DEFAULT-A-QUESTION-IS-PREFERRED-TO-A-MISTAKE and the
  agent prefers a clarifying question to an irreversible mistake. @impl/done

##THE-DEFAULT-IS-RIGHT-FOR-MOST-SESSIONS That default is right for most sessions. @impl/done

##THE-DEFAULT-IS-WRONG-FOR-PRE-APPROVED-WORK It is the *wrong* posture when the owner has already thought a large
body of work through, made the decision, and wants it executed end to
end. @impl/done

##the-handshake-is-overhead-already-paid-for There, every "shall I proceed?" is overhead the owner already
paid for by approving the activity. @spec/done

##A-CODEWORD-FLIPS-THE-SESSION-FOR-ONE-WORK-CYCLE A **codeword** is a short trigger phrase that flips the session into
an alternate posture for one work cycle. @impl/done

##CODEWORDS-ARE-EXPLICIT-OPT-INS The default is unchanged;
codewords are explicit opt-ins, recognised when spoken, in force only
until the work they cover ends. @impl/done

##A-MODE-CHANGES-CONFIRMATION-BEHAVIOUR-NEVER-WHAT-IS-PERMITTED A mode changes *confirmation
behaviour* — never what is permitted. @impl/done

## Why codewords rather than a settings flag {#why}

- ##WHY-OPT-IN-PER-WORK-CYCLE **Opt-in per work cycle.** The posture shift is scoped to what the
  owner is describing right now, not a persistent global that a later
  session inherits by surprise. @impl/done
- ##WHY-RECOGNISED-BY-INTENT **Recognised by intent.** A spoken phrase carries the owner's
  framing; the agent reads the intent, not a rigid string. @impl/done
- ##WHY-AUDITABLE **Auditable.** The trigger is visible in the transcript. Anyone
  reading the session sees exactly when the posture changed and back. @impl/done
- ##WHY-BOUNDED-BY-LAW **Bounded by law.** Because a codeword only ever relaxes the
  *routine* handshake, the dangerous handshakes (§red-lines) stay put
  by construction — there is no flag that can be left on to erode
  them. @impl/done

## The five-part codeword shape {#shape}

##every-codeword-is-specified-with-the-same-five-parts-lead Every codeword in a project's catalogue is specified with the same
five parts, so none is ambiguous and each states its own limits: @impl/done

| Part | What it fixes |
|------|---------------|
| ##ROW-PART-TRIGGER-PHRASE **1. Trigger phrase** @impl/done | the words (and recognisable variants) that activate it; case-insensitive; matched by intent @impl/done |
| ##ROW-PART-AUTHORITATIVE-DESCRIPTION **2. Authoritative description** @impl/done | the owner's own framing of what the mode is for, recorded verbatim @impl/done |
| ##ROW-PART-OPERATIVE-INTERPRETATION **3. Operative interpretation** @impl/done | a numbered list of concrete behavioural rules the agent follows while it is active @impl/done |
| ##ROW-PART-CHANGES-AND-DOES-NOT-CHANGE **4. Changes / does NOT change** @impl/done | what confirmation behaviour it relaxes, and an explicit restatement that the red lines survive it @impl/done |
| ##ROW-PART-LIFECYCLE-AND-CADENCE **5. Lifecycle + cadence** @impl/done | activation, persistence within a session, non-persistence across sessions, the abort words, and the reporting cadence @impl/done |

##A-PHRASE-MISSING-A-PART-IS-A-PROPOSAL-NOT-A-CODEWORD A phrase that does not carry all five parts is not yet a codeword —
it is a proposal. @impl/done

##DO-NOT-ACT-UNTIL-THE-OWNER-HAS-FIXED-THE-FIVE Do not act on it until the owner has fixed the five. @impl/done

## The red-lines law {#red-lines}

##THE-RED-LINES-ARE-THE-INVARIANT-THE-PRACTICE-PROTECTS This is the invariant the whole practice is built to protect. @impl/done

##a-small-set-of-operations-survives-every-mode-lead A small
set of operations survives **every** mode and always requires the
owner's explicit confirmation: @impl/done

- ##RED-LINE-REWRITING-PUBLISHED-HISTORY rewriting published history; @impl/done
- ##RED-LINE-FORCE-PUSHING force-pushing; @impl/done
- ##RED-LINE-IMPORTING-LARGE-BINARY-BLOBS importing large binary blobs; @impl/done
- ##RED-LINE-CHANGING-CI-SIGNING-OR-SECRETS changing CI, signing, or secrets configuration; @impl/done
- ##RED-LINE-ANYTHING-WHOSE-REVERSAL-WOULD-COST-WORK **anything whose reversal would cost work.** @impl/done

##A-CODEWORD-REMOVES-THE-ROUTINE-WORK-HANDSHAKE A codeword removes the *"may I proceed with routine work?"*
handshake. @impl/done

##A-CODEWORD-DOES-NOT-REMOVE-THE-IRREVERSIBLE-THRESHOLD-HANDSHAKE It does **not** remove the *"may I cross an irreversible
threshold?"* handshake. @impl/done

##ONLY-THE-FIRST-QUESTION-IS-EVER-WAIVED The two are different questions, and only the
first is ever waived. @impl/done

##consequences-that-follow-directly-lead Consequences that follow directly: @impl/done

- ##CONSEQUENCE-NO-CODEWORD-MAY-RELAX-A-RED-LINE No codeword may be defined that relaxes a red line. A proposed mode
  that would is malformed — refuse it and say why. @impl/done
- ##CONSEQUENCE-THE-MODES-OWN-ESCAPE-HATCH If work under an active mode cannot land without crossing a red
  line, the agent stops at that boundary, reports, and asks. This is
  **not** the interruption the mode forbids — it is the mode's own
  escape hatch firing. @impl/done
- ##CONSEQUENCE-EVERY-STANDING-INVARIANT-IS-A-RED-LINE-TOO Every other standing invariant of the project (its attribution
  posture, secrets hygiene, licence constraints, output conventions)
  is a red line for this purpose too: modes never touch them. @impl/done

## Reporting cadence {#cadence}

##AN-ACTIVE-MODE-STILL-REPORTS-AS-STATUS-NOT-REQUESTS Even with routine confirmations suspended, an active mode still
reports — as **status, not requests**. @impl/done

##the-right-cadence-versus-the-overhead "Phase N landed, tests green,
moving to N+1" is the right cadence; "Phase N landed — shall I
proceed?" is the exact overhead the codeword was spoken to remove. @impl/done

##THE-OWNER-READS-THESE-PASSIVELY The owner reads these passively and need not answer. @impl/done

## The catalogue {#catalogue}

##A-PROJECT-KEEPS-ITS-CODEWORDS-CATALOGUED-IN-ONE-PLACE A project keeps its codewords catalogued in one place (this document,
in a consuming project). @impl/done

##SHIP-RECOGNISE-AND-PROPOSE-THROUGH-THE-FIVE-PART-SHAPE Ship the ones the project actually uses;
recognise a spoken codeword only if it is catalogued, and propose
adding any new one through the five-part shape before acting on it. @impl/done

##one-worked-mode-lead This collection ships one worked mode as a reference: @impl/done

- ##CATALOGUE-MFBT-MODE [`mfbt-mode.md`](mfbt-mode.md) — «move fast and break things»:
  pre-authorised heads-down execution. @impl/done

## Re-derive for your project {#re-derive}

##COPY-THE-PROMPT-TASK-NOT-THE-PROMPT-IMPLEMENTATION Copy the prompt-task, not the prompt-implementation. @impl/done

##re-derive-lead Paste this to
your agent in a fresh session: @impl/done

```
Read this flow's documents (your project installed them — typically `vibedeps/flow-operating-modes/<version>/spec/flows/operating-modes/`, check `vibe.lock`) end to end. Then define THIS
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

- ##SUM-A-CODEWORD-FLIPS-THE-POSTURE-FOR-ONE-WORK-CYCLE A codeword flips the session into an alternate posture for one work
  cycle; the default is unchanged and modes are explicit opt-ins. @impl/done
- ##SUM-THE-FIVE-PART-SHAPE-OR-IT-IS-A-PROPOSAL Every codeword is fixed by the five-part shape, or it is a proposal,
  not a mode. @impl/done
- ##SUM-THE-RED-LINES-SURVIVE-EVERY-MODE The red lines survive every mode: a codeword waives the routine
  handshake, never the irreversible-threshold handshake. @impl/done
- ##SUM-REPORT-STATUS-NOT-REQUESTS Report status, not requests, while a mode is active. @impl/done
- ##SUM-CATALOGUE-BEFORE-ACTING Catalogue the modes in one place; propose new ones through the five
  parts before acting. @impl/done
