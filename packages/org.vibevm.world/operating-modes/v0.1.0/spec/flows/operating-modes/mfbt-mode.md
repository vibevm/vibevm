# Codeword: «move fast and break things» {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** A worked reference codeword, specified in
the five-part shape from
[`OPERATING-MODES-PROTOCOL.md`](OPERATING-MODES-PROTOCOL.md#shape). @status:impl/done

@fact:THIS-IS-THE-CANONICAL-PRE-AUTHORISED-HEADS-DOWN-POSTURE It
is the canonical example of a pre-authorised heads-down execution
posture — adopt it, adapt it, or read it as the template for writing
your own. @status:impl/done

## 1. Trigger phrase {#trigger}

@fact:THE-TRIGGER-IS-CASE-INSENSITIVE-AND-MATCHED-BY-INTENT `move fast and break things` — case-insensitive, matched by intent. @status:impl/done

@fact:RECOGNISABLE-VARIANTS-COUNT-AS-THE-SAME-TRIGGER Recognisable variants (`MFBT`, `move fast`) count as the same trigger
when context makes the intent unambiguous. @status:impl/done

## 2. Authoritative description {#description}

@fact:the-owners-framing-lead The owner's framing, given 2026-05-06 and rendered here in
English rather than the Russian it was spoken in: @status:impl/done

> @fact:THE-OWNERS-RECORDED-FRAMING Aim straight at the maximum version. Work in phases, each phase
> verifiable by tests. Cover everything with tests. The activity is
> approved — begin. Do not pause for confirmations; keep going until
> it is done. Do not economise on time. Do not economise on tokens.
> All of this is too important to economise on. Work at full
> reasoning depth. @status:impl/done

@fact:THE-DESCRIPTION-IS-RECORDED-VERBATIM A description is recorded verbatim, in the owner's own words and
language, because the mode is the owner's authorisation, and a
paraphrase is a different authorisation. @status:impl/done

## 3. Operative interpretation {#interpretation}

@fact:while-the-codeword-is-active-lead While the codeword is active: @status:impl/done

1. @fact:OPERATIVE-AIM-AT-THE-MAXIMUM-VERSION **Aim at the maximum version of the deliverable, not the minimum
   viable.** Do not pre-emptively trim scope to fit a "small first
   commit". If the planned scope is N slices, walk all N. @status:impl/done
2. @fact:OPERATIVE-PHASE-THE-WORK-INTO-TESTABLE-ITERATIONS **Phase the work into testable iterations.** Each phase is one
   logical unit closed by its tests passing — the phasing is for
   verifiability and clean commit grouping, not for confirmation
   handshakes. @status:impl/done
3. @fact:OPERATIVE-COVER-EVERYTHING-WITH-TESTS **Cover everything with tests.** Each new unit lands with hermetic
   tests; each cross-component flow lands with at least one
   integration test. The bar: *if it shipped, a test fails when it
   regresses.* @status:impl/done
4. @fact:OPERATIVE-THE-ACTIVITY-IS-PRE-APPROVED **The activity is pre-approved. Begin.** Do not ask "shall I
   start?" — the codeword's invocation is the green light. @status:impl/done
5. @fact:OPERATIVE-DO-NOT-PAUSE-FOR-CONFIRMATIONS **Do not pause for confirmations. Drive to completion.** Mid-work
   questions are reserved for genuine ambiguity (the spec is silent,
   two paths equally defensible), not for "may I proceed?". On real
   ambiguity, take the conservative reading, mark it for review,
   proceed, and surface it in the closing report. @status:impl/done
6. @fact:OPERATIVE-SPEND-WHATEVER-TIME-AND-TOKENS-THE-WORK-REQUIRES **Spend whatever time and tokens the work requires.** No artificial
   trimming for speed or context economy — read fully, reason fully,
   write fully. @status:impl/done
7. @fact:OPERATIVE-WORK-AT-FULL-REASONING-DEPTH **Work at full reasoning depth.** Depth is the explicit ask, not
   an optional optimisation. @status:impl/done

## 4. What it changes / does NOT change {#limits}

@fact:THE-ROUTINE-LARGE-CHANGE-HANDSHAKE-IS-SUSPENDED **Changes.** The default "stop and ask before routine large changes"
handshake is suspended: implementing a planned milestone, finishing a
slice, touching many files for one coherent reason all proceed
without confirmation. @status:impl/done

@fact:PER-PHASE-MESSAGES-BECOME-STATUS-NOT-REQUESTS Per-phase messages become short **status**, not
requests. @status:impl/done

@fact:TEST-ALONGSIDE-CADENCE-BECOMES-MANDATORY Test-alongside cadence becomes mandatory — no "tests
later". @status:impl/done

@fact:every-standing-invariant-survives-unchanged-lead **Does NOT change.** Every standing invariant survives unchanged: @status:impl/done

- @fact:SURVIVES-THE-PROJECTS-COMMIT-DISCIPLINE The project's commit discipline (one logical unit per commit, the
  project's message format, its attribution posture). @status:impl/done
- @fact:SURVIVES-SECRETS-LICENCE-AND-OUTPUT-CONVENTIONS Secrets hygiene, licence constraints, output conventions — all in
  force. @status:impl/done
- @fact:SURVIVES-THE-RED-LINES **The red lines** (see
  [`OPERATING-MODES-PROTOCOL.md` §red-lines](OPERATING-MODES-PROTOCOL.md#red-lines)):
  rewriting published history, force-pushing, large binary blobs,
  CI/signing/secrets changes, and anything whose reversal costs work
  STILL require explicit owner confirmation. The codeword removes the
  routine-work handshake; it does not remove the irreversible-
  threshold handshake. @status:impl/done

@fact:A-RED-LINE-STOP-IS-THE-MODES-OWN-ESCAPE-HATCH If a phase cannot land without crossing a red line, stop at that
boundary, report, and ask — that is the mode's own escape hatch, not
a forbidden interruption. @status:impl/done

## 5. Lifecycle + cadence {#lifecycle}

- @fact:LIFECYCLE-ACTIVATION **Activation.** The owner speaks the codeword in a turn. It covers
  the work described in that turn plus the obvious follow-up phases
  that complete the same deliverable. @status:impl/done
- @fact:LIFECYCLE-PERSISTENCE-WITHIN-A-SESSION **Persistence within a session.** In force for that work cycle; it
  does not bleed into unrelated later requests unless the owner
  re-affirms. @status:impl/done
- @fact:LIFECYCLE-PERSISTENCE-ACROSS-SESSIONS **Persistence across sessions.** None. A fresh session starts in the
  default posture; the owner re-invokes if they want it again. @status:impl/done
- @fact:LIFECYCLE-OWNER-SIDE-ABORT **Owner-side abort.** Any message signalling "stop", "wait",
  "slow down", "careful" (by intent, in any language) suspends the
  mode immediately: finish the in-flight tool call, report state,
  revert to default pending direction. @status:impl/done
- @fact:LIFECYCLE-AGENT-SIDE-ABORT **Agent-side abort.** The agent reverts on its own if it lands on a
  red-line situation or hits systematic failures it cannot diagnose
  within the phase. Report and ask. @status:impl/done
- @fact:LIFECYCLE-CADENCE **Cadence.** Phase entry (one sentence: what and scope), phase
  landing (one sentence: what landed, test count, commit subject),
  hard pivot (one sentence: why the direction changed), and the
  standard closing summary at the end of the cycle. Status, not
  requests. @status:impl/done

## Summary {#summary}

- @fact:SUM-MFBT-PRE-AUTHORISES-HEADS-DOWN-EXECUTION «move fast and break things» pre-authorises heads-down, maximum-
  version, test-covered execution with the routine handshake
  suspended. @status:impl/done
- @fact:SUM-IT-WAIVES-CONFIRMATIONS-FOR-ROUTINE-WORK-ONLY It waives confirmations for routine work only; every invariant and
  every red line survives it. @status:impl/done
- @fact:SUM-IT-IS-SCOPED-TO-ONE-WORK-CYCLE It is scoped to one work cycle, never crosses a session boundary,
  and aborts on the owner's word or a red-line situation. @status:impl/done
- @fact:SUM-IT-IS-THE-REFERENCE-INSTANCE-OF-THE-FIVE-PART-SHAPE It is the reference instance of the five-part codeword shape — copy
  it to author your own modes. @status:impl/done
