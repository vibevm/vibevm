# Flow: Operating Modes {#root}

<status stage="impl" state="done"/>

##THIS-PROJECT-USES-CODEWORD-TRIGGERED-OPERATING-MODES This project uses **codeword-triggered operating modes**. @impl/done

##ONE-DEFAULT-POSTURE-FLIPPED-BY-A-CATALOGUED-CODEWORD The session
has one safe default posture; the owner can flip it into an alternate
posture for one work cycle by speaking a catalogued codeword. @impl/done

##MODES-CHANGE-CONFIRMATION-BEHAVIOUR-NOT-WHAT-IS-OFF-LIMITS Modes
change *confirmation behaviour* — they never change what is
off-limits. @impl/done

## The default posture {#default}

##NON-ROUTINE-WORK-STOPS-FOR-EXPLICIT-CONFIRMATION Unless a codeword is active: routine work proceeds normally, and
anything **non-routine** — history rewrites, force-pushes, large
binary imports, CI/signing/secrets changes, any operation whose
reversal would cost work — stops for the owner's explicit
confirmation. @impl/done

##WHEN-UNCERTAIN-WHETHER-SOMETHING-IS-ROUTINE-ASK When uncertain whether something is routine, ask. @impl/done

## Codewords {#codewords}

##codeword-catalogue-pointer The project's codewords are catalogued in
@spec://org.vibevm.world/operating-modes/flows/operating-modes/OPERATING-MODES-PROTOCOL#root. @impl/done

##RECOGNISE-A-CODEWORD-BY-INTENT-NOT-EXACT-WORDING
Recognise a codeword by intent, not exact wording. When one fires: @impl/done

1. ##ON-FIRING-ACKNOWLEDGE-THE-ACTIVE-MODE Acknowledge which mode is now active and what it changes. @impl/done
2. ##ON-FIRING-APPLY-THE-OPERATIVE-RULES Apply its operative rules for the current work cycle. @impl/done
3. ##ON-FIRING-DROP-BACK-WHEN-THE-CYCLE-ENDS Drop back to the default posture when the cycle ends — a mode
   **never** persists into a new session uninvoked. @impl/done

##worked-mode-pointer The catalogue ships one worked mode:
@spec://org.vibevm.world/operating-modes/flows/operating-modes/mfbt-mode#root — heads-down
pre-authorised execution («move fast and break things»). @impl/done

## The red-lines law {#red-lines}

##red-lines-survive-every-mode-lead A small set of red lines survives **every** mode: @impl/done

- ##RED-LINE-REWRITING-PUBLISHED-HISTORY rewriting published
  history, @impl/done
- ##RED-LINE-FORCE-PUSHING force-pushing, @impl/done
- ##RED-LINE-IMPORTING-LARGE-BINARIES importing large binaries, @impl/done
- ##RED-LINE-CI-SIGNING-SECRETS-CONFIGURATION touching
  CI/signing/secrets configuration, @impl/done
- ##RED-LINE-ANY-OPERATION-WHOSE-REVERSAL-COSTS-WORK and any operation whose reversal
  costs work. @impl/done

##A-CODEWORD-WAIVES-THE-ROUTINE-HANDSHAKE-ONLY A codeword removes the "may I proceed with routine work?"
handshake; it does **not** remove the "may I cross an irreversible
threshold?" handshake. @impl/done

##NO-CODEWORD-CAN-BE-DEFINED-THAT-ERODES-THIS No codeword can be defined that erodes this. @impl/done

## Reporting cadence {#cadence}

##REPORT-STATUS-NOT-REQUESTS-UNDER-AN-ACCELERATED-MODE Under any accelerated mode, report **status, not requests**: "phase N
landed, tests green, moving to N+1" is right; "phase N landed — shall
I proceed?" is the exact overhead the codeword was spoken to remove. @impl/done

## Never {#never}

- ##NEVER-CROSS-A-RED-LINE-UNDER-ANY-MODE Never cross a red line under any mode — the handshake for
  irreversible operations is unconditional. @impl/done
- ##NEVER-CARRY-AN-ACTIVE-MODE-ACROSS-A-SESSION-BOUNDARY Never carry an active mode across a session boundary; every session
  starts in the default posture. @impl/done
- ##NEVER-ACT-ON-AN-UNCATALOGUED-CODEWORD Never act on a codeword that is not in the catalogue — propose
  adding it first. @impl/done
- ##NEVER-ASK-PERMISSION-FOR-WORK-A-MODE-PRE-AUTHORISED Never reply to your own completed work with a permission question
  when a mode pre-authorised it — report status and continue. @impl/done
