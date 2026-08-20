# Writing a codeword {#root}

**Scope of this document.** The procedure for adding a new operating
mode to a project's catalogue: how to take a posture the owner keeps
describing informally and turn it into a catalogued codeword that is
unambiguous, safe, and recognised the same way every time.

## When a new codeword is warranted {#when}

Write one when the owner **repeatedly** asks for the same non-default
posture — and the asking itself has become overhead. Signs:

- The owner keeps re-explaining the same working style ("just push
  through, don't check with me on each step").
- The agent keeps mis-calibrating because the posture was implicit.
- A recognisable phrase is already emerging in how the owner asks.

One-off requests do not need a codeword. A codeword is for a posture
worth naming because it recurs.

## The procedure {#procedure}

### 1. Name the posture and the trigger {#step-name}

Pick the trigger phrase from how the owner already speaks. Record the
recognisable variants. Case-insensitive, matched by intent — the
agent recognises the meaning, not a literal string.

### 2. Record the description verbatim {#step-describe}

Capture the owner's own framing of what the mode is for, in their
words, dated. This is part 2 of the five-part shape and it is
load-bearing: the description *is* the authorisation, so a paraphrase
would be a different authorisation. Do not tidy it into your own
prose.

### 3. Derive the operative rules {#step-rules}

Translate the description into a numbered list of concrete behaviours
the agent follows while the mode is active. Each rule is testable in
the sense that the agent can tell whether it is obeying it. Vague
rules ("be thorough") are useless; specific ones ("each phase lands
with green tests before the next begins") are the whole value.

### 4. State changes and non-changes — restate the red lines {#step-limits}

Write, explicitly:

- **what confirmation behaviour the mode relaxes** (almost always:
  the routine-work handshake), and
- **that the red lines survive it** — reproduce the project's
  red-line list here, do not merely reference it. The restatement is
  deliberate friction: a mode whose author had to re-type the red
  lines is a mode whose author confirmed the mode respects them.

A proposed mode that would relax a red line is malformed. Do not
catalogue it; tell the owner why.

### 5. Fix the lifecycle and cadence {#step-lifecycle}

Specify activation scope, persistence within a session, non-
persistence across sessions, the abort signals (owner-side and
agent-side), and the reporting cadence (status, not requests). Modes
default to *not* crossing a session boundary; say so explicitly.

### 6. Catalogue it {#step-catalogue}

Add the mode to the project's operating-modes document and, if the
project loads a boot file, note its existence there so a fresh
session recognises the trigger. Until it is catalogued, the phrase is
a proposal and the agent does not act on it.

## Composing modes {#composing}

Modes can combine when it makes sense — "wrap up + move fast" runs a
finish-up phase at accelerated cadence. Composition never sums the
relaxations past the red lines: two modes together still cannot waive
an irreversible-threshold handshake. When in doubt about a
combination, treat the stricter posture as winning.

## Anti-patterns {#anti-patterns}

| Smell | Why it fails | Fix |
|-------|--------------|-----|
| A codeword with no verbatim description | the authorisation is the agent's paraphrase, not the owner's intent | record the owner's words |
| "be careful mode" as a codeword | the default posture is already careful; a mode *relaxes*, it does not re-assert the default | drop it; the default covers it |
| A mode that lists no red lines | nothing confirms it respects them | restate the full list in part 4 |
| A phrase acted on before cataloguing | non-repeatable, unauditable behaviour | catalogue first, act second |
| A mode that persists across sessions silently | a later session inherits a posture nobody re-authorised | make non-persistence explicit |

## Summary {#summary}

- Write a codeword when a non-default posture recurs enough that
  asking for it is itself overhead.
- Follow the six steps; the verbatim description and the restated red
  lines are the two non-negotiable parts.
- A mode relaxes confirmation behaviour only; it never touches a red
  line, alone or composed.
- Catalogue before acting — an uncatalogued phrase is a proposal, not
  a mode.
