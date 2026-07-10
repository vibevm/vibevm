# Codeword: «move fast and break things» {#root}

**Scope of this document.** A worked reference codeword, specified in
the five-part shape from
[`OPERATING-MODES-PROTOCOL.md`](OPERATING-MODES-PROTOCOL.md#shape). It
is the canonical example of a pre-authorised heads-down execution
posture — adopt it, adapt it, or read it as the template for writing
your own.

## 1. Trigger phrase {#trigger}

`move fast and break things` — case-insensitive, matched by intent.
Recognisable variants (`MFBT`, `move fast`) count as the same trigger
when context makes the intent unambiguous.

## 2. Authoritative description {#description}

The owner's framing, recorded as spoken:

> Aim straight at the maximum version. Work in phases, each phase
> verifiable by tests. Cover everything with tests. The activity is
> approved — begin. Do not pause for confirmations; keep going until
> it is done. Do not economise on time. Do not economise on tokens.
> All of this is too important to economise on. Work at full
> reasoning depth.

The description is recorded verbatim because the mode is the owner's
authorisation, and a paraphrase is a different authorisation.

## 3. Operative interpretation {#interpretation}

While the codeword is active:

1. **Aim at the maximum version of the deliverable, not the minimum
   viable.** Do not pre-emptively trim scope to fit a "small first
   commit". If the planned scope is N slices, walk all N.
2. **Phase the work into testable iterations.** Each phase is one
   logical unit closed by its tests passing — the phasing is for
   verifiability and clean commit grouping, not for confirmation
   handshakes.
3. **Cover everything with tests.** Each new unit lands with hermetic
   tests; each cross-component flow lands with at least one
   integration test. The bar: *if it shipped, a test fails when it
   regresses.*
4. **The activity is pre-approved. Begin.** Do not ask "shall I
   start?" — the codeword's invocation is the green light.
5. **Do not pause for confirmations. Drive to completion.** Mid-work
   questions are reserved for genuine ambiguity (the spec is silent,
   two paths equally defensible), not for "may I proceed?". On real
   ambiguity, take the conservative reading, mark it for review,
   proceed, and surface it in the closing report.
6. **Spend whatever time and tokens the work requires.** No artificial
   trimming for speed or context economy — read fully, reason fully,
   write fully.
7. **Work at full reasoning depth.** Depth is the explicit ask, not
   an optional optimisation.

## 4. What it changes / does NOT change {#limits}

**Changes.** The default "stop and ask before routine large changes"
handshake is suspended: implementing a planned milestone, finishing a
slice, touching many files for one coherent reason all proceed
without confirmation. Per-phase messages become short **status**, not
requests. Test-alongside cadence becomes mandatory — no "tests
later".

**Does NOT change.** Every standing invariant survives unchanged:

- The project's commit discipline (one logical unit per commit, the
  project's message format, its attribution posture).
- Secrets hygiene, licence constraints, output conventions — all in
  force.
- **The red lines** (see
  [`OPERATING-MODES-PROTOCOL.md` §red-lines](OPERATING-MODES-PROTOCOL.md#red-lines)):
  rewriting published history, force-pushing, large binary blobs,
  CI/signing/secrets changes, and anything whose reversal costs work
  STILL require explicit owner confirmation. The codeword removes the
  routine-work handshake; it does not remove the irreversible-
  threshold handshake.

If a phase cannot land without crossing a red line, stop at that
boundary, report, and ask — that is the mode's own escape hatch, not
a forbidden interruption.

## 5. Lifecycle + cadence {#lifecycle}

- **Activation.** The owner speaks the codeword in a turn. It covers
  the work described in that turn plus the obvious follow-up phases
  that complete the same deliverable.
- **Persistence within a session.** In force for that work cycle; it
  does not bleed into unrelated later requests unless the owner
  re-affirms.
- **Persistence across sessions.** None. A fresh session starts in the
  default posture; the owner re-invokes if they want it again.
- **Owner-side abort.** Any message signalling "stop", "wait",
  "slow down", "careful" (by intent, in any language) suspends the
  mode immediately: finish the in-flight tool call, report state,
  revert to default pending direction.
- **Agent-side abort.** The agent reverts on its own if it lands on a
  red-line situation or hits systematic failures it cannot diagnose
  within the phase. Report and ask.
- **Cadence.** Phase entry (one sentence: what and scope), phase
  landing (one sentence: what landed, test count, commit subject),
  hard pivot (one sentence: why the direction changed), and the
  standard closing summary at the end of the cycle. Status, not
  requests.

## Summary {#summary}

- «move fast and break things» pre-authorises heads-down, maximum-
  version, test-covered execution with the routine handshake
  suspended.
- It waives confirmations for routine work only; every invariant and
  every red line survives it.
- It is scoped to one work cycle, never crosses a session boundary,
  and aborts on the owner's word or a red-line situation.
- It is the reference instance of the five-part codeword shape — copy
  it to author your own modes.
