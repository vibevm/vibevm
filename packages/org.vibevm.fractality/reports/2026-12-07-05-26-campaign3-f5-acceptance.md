# Campaign 3 · Stage B · Ф5 — Acceptance / PP-002 fold-in (FD-9) — phase report

_Written 2026-07-12 ~05:26. Owner-facing; plan §9 ledger is the commit map.
Phase **COMPLETE** (2 slices)._

## TL;DR

Acceptance is now a first-class notion. A packet can declare itself an
acceptance **verifier** (`output.verifier`) over the work in its
`context_from`; mission-control refuses a **cold** verifier (one over an
empty/workless tree) at the door, and a verifier's acceptance verdict is
surfaced as a **verifier-accept** (ACCEPTED / REJECTED) over that work.
Both halves of FD-9 landed:

| slice | what | commit |
|---|---|---|
| Ф5.1 | `output.verifier` marker + cold-verifier suppression at admission | `85ac2a7` |
| Ф5.2 | `RunRecord.verifier` denorm + verifier-accept verdict in `run`/`show` | `af977a4` |

Floor green (test-gate 213). Real `~/.fractality` untouched.

## What was done

- **The marker (Ф5.1).** `OutputSpec.verifier: bool` rides the packet next
  to `merge`. A verifier reads only named results (RD-11: clean context by
  design — the fold law already forbids parent/sibling transcripts), so
  `context_from` IS the set of work under review.
- **Cold-verifier suppression (Ф5.1).** `admission::check_verifier_has_work`
  refuses (400) a verifier whose `context_from` names no run that produced
  a result — no cold verification over an empty tree (§10.2). Applied in
  `register_run` right after packet validation, unconditionally (a packet-
  validity check, like `validate()`), so a cold verifier never lands in the
  registry. Two integration tests: a verifier over a completed run with a
  result is admitted; empty and resultless `context_from` are refused 400.
- **Verifier-accept surfaced (Ф5.2).** `RunRecord.verifier` denormalizes
  the packet flag so `ps`/`show` mark verifier runs without a packet read.
  `verifier_verdict` renders the acceptance verdict as
  `ACCEPTED (n/m checks passed)` / `REJECTED` / `inconclusive` / `pending`
  in the `run` summary and the `show` detail — the verifier-accept signal,
  distinct from a plain run's self-test acceptance line.

## Decisions taken

1. **Cold-verifier check is unconditional (not spawn-gated).** It is a
   packet-validity check (a verifier over nothing is a malformed request),
   so it applies like `validate()`, not like the spawn-only admission
   checks. This also keeps it testable with `spawn=false`.
2. **"Has work" = `context_from` names a run with a produced result**
   (`collected.result_source != "none"`). Conservative and mechanical; a
   terminal-but-resultless run does not count as work to verify.
3. **Verifier-accept is a READABLE verdict, not a hard completion gate.**
   §10.8 conservative: the verifier's acceptance verdict is first-class on
   its own record and rendered as ACCEPTED/REJECTED; it does not block the
   tree's runs from going terminal. A hard gate that suspends tree
   completion until acceptance would be intrusive and is not what v1 needs.

## What is left undone / висяки

- **No query "given a work run, find its verifier."** The verdict is
  first-class on the verifier's record, but resolving *the tree → its
  acceptance* needs either a `context_from` denorm on `RunRecord` or a
  server-side scan. Deferred — an `on_acceptance`/tree-status surface is a
  natural Phase-5 delegation-rules concern, not v1.
- **Acceptance does not yet feed routing.** FD-5 names the journal's
  acceptance data as the future soft-label training table; wiring
  per-worker × task-class acceptance into the router is later work (the
  D-C3-8 soft-label table is the seam).
- **The verifier's own result is not re-verified.** As with the Ф3 retry,
  one verifier verdict stands.

## Next

**Ф6 — trial (D-C3-9):** pre-register MT-C3-01 (committed FIRST — §10.7
pre-reg-first is BINDING), then fire the budget-matched paid arms (RP-C3-2
PRE-AUTHORIZED 2026-07-11; no second word needed once the pre-registration
lands), score, and record fatigue + uncertainty facts. GLM cold boss; an
orchestration-collapse probe. Then Ф7 (close) → PP-003 (Option C advisor).
