# PROP-006: Operating modes — codeword-triggered work postures {#root}

**Status:** accepted 2026-05-06.
**Related:** [`CLAUDE.md`](../../CLAUDE.md) (the four rules + session-end codeword), [PROP-000](PROP-000.md) (foundation).

---

## 1. Motivation {#motivation}

A vibevm session has a default operating posture, set by [`CLAUDE.md`](../../CLAUDE.md):

- Routine large work proceeds without asking.
- Non-routine work (history rewrites, force-push, large blobs, CI / signing / secrets changes, irreversible ops) stops and asks first.
- The owner is generally available to answer mid-session.

This default optimises for safety: the agent prefers a clarifying question to an irreversible mistake. It is the right default for ~90% of sessions.

It is the **wrong** posture for sustained heads-down work where the owner has pre-authorised a substantial body of work, has thought through the trade-offs, has made the decision, and now wants the agent to execute end-to-end — without midstream confirmation handshakes interrupting the flow. In those sessions, every "shall I proceed?" question is overhead the owner has already paid for upstream by approving the activity.

**This PROP defines codewords** — short trigger phrases the owner can use to switch the session into an alternate posture for the remainder of the work cycle. The default is unchanged; codewords are explicit opt-ins, recognised when invoked, in force until the work cycle they cover ends.

The session-end codeword (`ЗАВЕРШИ СЕССИЮ` / `END SESSION`, defined in [`CLAUDE.md`](../../CLAUDE.md)) is the existing precedent: a phrase, invoked when needed, that triggers a structured behaviour. PROP-006 generalises that pattern to other operating postures.

---

## 2. Codeword: «move fast and break things» {#mfbt}

**Codename / trigger phrase:** `move fast and break things` (case-insensitive; partial matches counted as the same intent — `MFBT`, `move fast`, `MFAT` all recognised when context makes the intent unambiguous).

**Authoritative description (verbatim, owner-supplied 2026-05-06):**

> Целься сразу в максимальную версию. Работай этапами, каждый этап должен быть проверяем тестами. Покрывай всё тестами. Сама активность одобрена, начинай. Не прерывайся на согласования, делай пока не сделаешь. Время не экономь. Токены не экономь. Всё это слишком важно, чтобы экономить. Работай с максимальным использованием возможностей /effort max, ultrathink, superthink, think-like-mythos.

**Operative interpretation.** When the codeword is active:

1. **Aim at the maximum version of the deliverable, not the minimum viable.** Do not pre-emptively trim scope to fit a perceived "small first commit". The owner has approved the maximum. If the planned scope is N slices, walk through all N.
2. **Phase the work into testable iterations.** Each phase = one logical unit (PROP/FEAT slice, milestone, etc.) closed by all tests passing. The phasing is for *verifiability* and for clean Conventional-Commits grouping (Rule 3) — not for confirmation handshakes.
3. **Cover everything with tests.** Each new module / function / route / CLI subcommand lands with hermetic unit tests; each cross-component flow lands with at least one integration test; the test bar is "if it shipped, a test fails when it regresses."
4. **The activity is pre-approved. Begin.** Do not ask "shall I start?" — start. The codeword's invocation is itself the green light.
5. **Do not pause for confirmations. Drive to completion.** Mid-work clarifying questions are overhead the owner has already paid for at activation time. If genuine ambiguity surfaces (the spec is silent, two paths are equally defensible), apply the [`spec/boot/00-core.md` uncertainty protocol](../boot/00-core.md#uncertainty-protocol): pick the conservative interpretation, mark `<!-- REVIEW: … -->`, proceed, surface in the end-of-session report.
6. **Spend whatever time is required.** No artificial trimming for "speed".
7. **Spend whatever tokens are required.** No artificial trimming for context-window economy. Read fully, think fully, write fully.
8. **Lean fully into deep-reasoning capability.** The owner explicitly invokes `/effort max`, `ultrathink`, `superthink`, `think-like-mythos` (and any future deep-reasoning levers the harness exposes). Reasoning depth is the explicit ask, not an optional optimisation.

### 2.1 What the codeword changes {#mfbt-changes}

Specifically, when «move fast and break things» is active:

- **Rule 4's "stop and ask first for routine large changes" no longer applies.** Implementing a planned milestone, finishing a PROP slice, touching many files for one coherent reason — all proceed without confirmation. Each phase still commits as one logical unit (Rule 3) with a Conventional Commit (Rule 2).
- **Mid-work questions to the owner are reserved for genuine spec ambiguity, not for "may I proceed".** If a fork in the road has clear PROP / FEAT / repo-precedent guidance, follow it. If two paths are equally defensible, pick the more conservative one and flag `<!-- REVIEW: ... -->`.
- **Test-first or test-alongside cadence is mandatory.** No "I'll add tests later" — every phase commits with green tests for what it landed.
- **Per-phase progress messages are short status, not requests for input.** "Slice N landed: X tests added, all green" is the right cadence; "Slice N landed; shall I proceed to N+1?" is wrong.

### 2.2 What the codeword does NOT change {#mfbt-limits}

The four non-negotiable rules from [`CLAUDE.md`](../../CLAUDE.md) and [PROP-000 §12](PROP-000.md#commits) survive «move fast and break things» unchanged:

- **Rule 1 (human-only attribution).** No machine-authorship marker on any commit, branch, comment, or artefact, ever.
- **Rule 2 (Conventional Commits).** Subject ≤ 60 chars (hard limit 72), body explains *why*, types from the canonical list.
- **Rule 3 (group commits by meaning).** Each phase = its own logical commit; mixed concerns split.
- **Rule 4's red-line subclause.** The escape-hatch list — rewriting published history, `git push --force` / `--force-with-lease`, large binary blobs, CI / signing / secrets configuration changes, **anything whose reversal would cost work** — STILL requires explicit owner confirmation when active. The codeword removes the "may I proceed with routine work" handshake; it does NOT remove the "may I cross an irreversible threshold" handshake.

The token-secrecy invariant ([PROP-000 §20](PROP-000.md#token-secrecy)), the licence-permissive invariant ([PROP-000 §3](PROP-000.md#licensing)), the language-Russian-output invariant for chat communication, and every other foundation directive in this repository remain in force.

If a phase under «move fast and break things» discovers it cannot land without crossing one of the red lines (e.g. needs to rewrite published history to land cleanly), the agent stops at that phase boundary, reports the situation, and asks. This is not "interrupting for confirmation" in the sense the codeword forbids — it is the codeword's own §2.2 escape hatch firing.

### 2.3 Activation lifecycle {#mfbt-lifecycle}

- **Activation:** the owner says the codeword (verbatim or recognisably) inside a chat turn. The activation covers the work the owner is describing in that turn (and any obvious follow-up phases that complete the same deliverable).
- **Persistence within a session:** the activation persists for the duration of the work cycle the owner described. It does not bleed into unrelated subsequent requests in the same session unless the owner re-affirms.
- **Persistence across sessions:** does NOT persist by default. A fresh session starts in default posture; the owner re-invokes if they want MFBT for the new session.
- **Owner-side abort:** any owner message containing "stop", "пауза", "wait", "halt", "осторожнее", "не торопись", or similar suspends MFBT immediately. The agent finishes the in-flight tool call, reports state, and reverts to default posture pending owner direction.
- **Agent-side abort:** the agent itself reverts to default posture if it lands on a red-line situation (§2.2) or detects systematic test failures it cannot diagnose within the current phase. Report and ask.

### 2.4 Reporting cadence under MFBT {#mfbt-reports}

Even with confirmations suspended, the agent posts brief progress messages so the owner can stay aware of the work:

- **Phase entry:** one sentence — what this phase implements, scope.
- **Phase landing:** one sentence — what landed, test count, commit subject.
- **Hard pivot:** one sentence — if the phase changed direction mid-way, why.
- **End of work cycle:** standard end-of-session summary (TL;DR + commits + push status).

This is *status*, not *request*. The owner reads these passively; they do not need to respond.

---

## 3. Future codewords {#future}

Other operating postures may need codewords later (research-mode, archaeology-mode, dry-run-mode, …). They land in this PROP as additional `## 3.x Codeword: <name>` sections, following the same five-part shape:

1. Trigger phrase.
2. Authoritative description.
3. Operative interpretation (numbered list of behavioural rules).
4. What the codeword changes / does not change (with explicit reference to the four non-negotiable rules).
5. Activation lifecycle + reporting cadence.

Each codeword stands alone. Owner can mix codewords (e.g. "wrap up + move fast" — finish-up phase under MFBT cadence) when the combinations make sense.

---

## 4. Version history {#history}

- **2026-05-06 — accepted.** Initial codeword: «move fast and break things». Authoritative description recorded verbatim from owner.
