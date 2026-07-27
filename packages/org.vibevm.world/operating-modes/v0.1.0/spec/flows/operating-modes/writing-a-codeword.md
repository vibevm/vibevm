# Writing a codeword {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** The procedure for adding a new operating
mode to a project's catalogue: how to take a posture the owner keeps
describing informally and turn it into a catalogued codeword that is
unambiguous, safe, and recognised the same way every time. @impl/done

## When a new codeword is warranted {#when}

##WRITE-ONE-WHEN-THE-ASKING-ITSELF-HAS-BECOME-OVERHEAD Write one when the owner **repeatedly** asks for the same non-default
posture — and the asking itself has become overhead. Signs: @impl/done

- ##SIGN-THE-OWNER-KEEPS-RE-EXPLAINING-THE-SAME-STYLE The owner keeps re-explaining the same working style ("just push
  through, don't check with me on each step"). @spec/done
- ##SIGN-THE-AGENT-KEEPS-MIS-CALIBRATING The agent keeps mis-calibrating because the posture was implicit. @spec/done
- ##SIGN-A-RECOGNISABLE-PHRASE-IS-ALREADY-EMERGING A recognisable phrase is already emerging in how the owner asks. @spec/done

##ONE-OFF-REQUESTS-DO-NOT-NEED-A-CODEWORD One-off requests do not need a codeword. @impl/done

##A-CODEWORD-IS-FOR-A-POSTURE-WORTH-NAMING-BECAUSE-IT-RECURS A codeword is for a posture
worth naming because it recurs. @impl/done

## The procedure {#procedure}

### 1. Name the posture and the trigger {#step-name}

##PICK-THE-TRIGGER-FROM-HOW-THE-OWNER-ALREADY-SPEAKS Pick the trigger phrase from how the owner already speaks. @impl/done

##RECORD-THE-RECOGNISABLE-VARIANTS Record the
recognisable variants. @impl/done

##CASE-INSENSITIVE-AND-MATCHED-BY-INTENT Case-insensitive, matched by intent — the
agent recognises the meaning, not a literal string. @impl/done

### 2. Record the description verbatim {#step-describe}

##CAPTURE-THE-OWNERS-OWN-FRAMING-IN-THEIR-WORDS-DATED Capture the owner's own framing of what the mode is for, in their
words, dated. @impl/done

##THE-DESCRIPTION-IS-THE-AUTHORISATION This is part 2 of the five-part shape and it is
load-bearing: the description *is* the authorisation, so a paraphrase
would be a different authorisation. @impl/done

##DO-NOT-TIDY-THE-DESCRIPTION-INTO-YOUR-OWN-PROSE Do not tidy it into your own
prose. @impl/done

### 3. Derive the operative rules {#step-rules}

##TRANSLATE-THE-DESCRIPTION-INTO-A-NUMBERED-LIST-OF-BEHAVIOURS Translate the description into a numbered list of concrete behaviours
the agent follows while the mode is active. @impl/done

##EACH-RULE-IS-TESTABLE-BY-THE-AGENT-ITSELF Each rule is testable in
the sense that the agent can tell whether it is obeying it. @impl/done

##vague-rules-are-useless-specific-ones-are-the-value Vague rules ("be thorough") are useless; specific ones ("each phase
lands with green tests before the next begins") are the whole value. @spec/done

### 4. State changes and non-changes — restate the red lines {#step-limits}

##write-explicitly-lead Write, explicitly: @impl/done

- ##STATE-WHAT-CONFIRMATION-BEHAVIOUR-THE-MODE-RELAXES **what confirmation behaviour the mode relaxes** (almost always:
  the routine-work handshake), and @impl/done
- ##STATE-THAT-THE-RED-LINES-SURVIVE-IT **that the red lines survive it** — reproduce the project's
  red-line list here, do not merely reference it. The restatement is
  deliberate friction: a mode whose author had to re-type the red
  lines is a mode whose author confirmed the mode respects them. @impl/done

##A-MODE-THAT-WOULD-RELAX-A-RED-LINE-IS-MALFORMED A proposed mode that would relax a red line is malformed. @impl/done

##DO-NOT-CATALOGUE-IT-TELL-THE-OWNER-WHY Do not
catalogue it; tell the owner why. @impl/done

### 5. Fix the lifecycle and cadence {#step-lifecycle}

##SPECIFY-ACTIVATION-PERSISTENCE-ABORTS-AND-CADENCE Specify activation scope, persistence within a session, non-
persistence across sessions, the abort signals (owner-side and
agent-side), and the reporting cadence (status, not requests). @impl/done

##MODES-DEFAULT-TO-NOT-CROSSING-A-SESSION-BOUNDARY Modes
default to *not* crossing a session boundary; say so explicitly. @impl/done

### 6. Catalogue it {#step-catalogue}

##ADD-THE-MODE-TO-THE-CATALOGUE-AND-NOTE-IT-IN-THE-BOOT-FILE Add the mode to the project's operating-modes document and, if the
project loads a boot file, note its existence there so a fresh
session recognises the trigger. @impl/done

##AN-UNCATALOGUED-PHRASE-IS-A-PROPOSAL Until it is catalogued, the phrase is
a proposal and the agent does not act on it. @impl/done

## Composing modes {#composing}

##MODES-CAN-COMBINE-WHEN-IT-MAKES-SENSE Modes can combine when it makes sense — "wrap up + move fast" runs a
finish-up phase at accelerated cadence. @impl/done

##COMPOSITION-NEVER-SUMS-RELAXATIONS-PAST-THE-RED-LINES Composition never sums the
relaxations past the red lines: two modes together still cannot waive
an irreversible-threshold handshake. @impl/done

##WHEN-IN-DOUBT-THE-STRICTER-POSTURE-WINS When in doubt about a
combination, treat the stricter posture as winning. @impl/done

## Anti-patterns {#anti-patterns}

| Smell | Why it fails | Fix |
|-------|--------------|-----|
| ##ROW-SMELL-NO-VERBATIM-DESCRIPTION A codeword with no verbatim description @spec/done | the authorisation is the agent's paraphrase, not the owner's intent @spec/done | record the owner's words @spec/done |
| ##ROW-SMELL-BE-CAREFUL-MODE "be careful mode" as a codeword @spec/done | the default posture is already careful; a mode *relaxes*, it does not re-assert the default @spec/done | drop it; the default covers it @spec/done |
| ##ROW-SMELL-NO-RED-LINES-LISTED A mode that lists no red lines @spec/done | nothing confirms it respects them @spec/done | restate the full list in part 4 @spec/done |
| ##ROW-SMELL-ACTED-ON-BEFORE-CATALOGUING A phrase acted on before cataloguing @spec/done | non-repeatable, unauditable behaviour @spec/done | catalogue first, act second @spec/done |
| ##ROW-SMELL-SILENT-CROSS-SESSION-PERSISTENCE A mode that persists across sessions silently @spec/done | a later session inherits a posture nobody re-authorised @spec/done | make non-persistence explicit @spec/done |

## Summary {#summary}

- ##SUM-WRITE-A-CODEWORD-WHEN-ASKING-IS-ITSELF-OVERHEAD Write a codeword when a non-default posture recurs enough that
  asking for it is itself overhead. @impl/done
- ##SUM-THE-TWO-NON-NEGOTIABLE-PARTS Follow the six steps; the verbatim description and the restated red
  lines are the two non-negotiable parts. @impl/done
- ##SUM-A-MODE-NEVER-TOUCHES-A-RED-LINE A mode relaxes confirmation behaviour only; it never touches a red
  line, alone or composed. @impl/done
- ##SUM-CATALOGUE-BEFORE-ACTING Catalogue before acting — an uncatalogued phrase is a proposal, not
  a mode. @impl/done
