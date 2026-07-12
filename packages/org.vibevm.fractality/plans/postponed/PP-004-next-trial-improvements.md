# PP-004 — next-trial improvements (make MT-C3-01's blind spots measurable)

_Filed: 2026-07-12 · Status: **POSTPONED** (deliberately not done
mid-frozen-protocol at Ф6) · Origin: Campaign 3 Ф6 trial (MT-C3-01
Recorded runs; `reports/2026-12-07-06-44-campaign3-f6-trial.md`)._

## The task

The Ф6 gated trial confirmed the big things — fractality runs end to end,
and the RLM patterns lift delegation to 44.4% (vs 16.7% naive). But three
of the four predictions could not be scored cleanly because the harness +
menu did not exercise them. A future trial should fix four small things,
none of which is worth touching a frozen protocol mid-run:

1. **Raise the worker turn cap.** Workers hit their 30-turn cap mid-task
   (multi-file tasks like "6 fixtures → 6 JSON files" need more); bosses hit
   50. `delegated` was measured honestly, but `delegated-AND-collected`
   (3 across 3 runs) trailed it purely because of the caps. Raise the worker
   cap (profile/packet config) and give the boss a larger budget.
2. **Add a structured-output task to the menu (test P-C3-b).** No task set
   an `output_schema`, so the schema gate (Ф1.2b, built + unit-tested) never
   fired in the trial. One explicit "emit JSON conforming to this schema"
   task would make P-C3-b measurable.
3. **Add a genuine Silo task to the menu (test P-C3-d).** The menu has no
   whole-repo reasoning task that a split would destroy, so nothing
   escalated. A task like "reconcile the error-handling strategy across ALL
   modules and justify one design" is chunk-destroying and should escalate,
   not fan out.
4. **Add a `fractality decisions` read verb** (mirror of `escalations`) so
   the need-gate decision journal (`/v0/decisions`, written by
   `gate --record`) can be read back. Then P-C3-a becomes a hard number
   (route/spawn/inline/escalate counts) instead of a transcript-grep
   direction.

## Why postponed

Changing the worker cap, the menu, or adding a CLI verb mid-trial would
break the frozen MT-C3-01 pre-registration (§10.7 pre-reg-first). The right
move was to run the frozen protocol, record the blind spots honestly, and
schedule the fixes for a *next* pre-registered trial. Items 1–3 are also
just not blocking Stage B's mandate (build the machinery + prove it runs —
done). Item 4 is a small feat but was not needed to close Ф6.

## Unblock / first steps

The owner commissions a follow-up trial (its own MT-C3-02 pre-registration),
or item 4 (the `decisions` verb) as a standalone slice. First steps:
- item 4: add `Cmd::Decisions { json }` in `fractality-cli` + a
  `boss::decisions` thin client over `GET /v0/decisions` (the endpoint +
  `DecisionListResponse` already exist, Ф3.2b); mirror `escalations`.
- items 1–3: a new menu file + preamble under `spec/manual-tests/trial/`,
  a fresh MT-C3-02, and re-run `run-arm.sh g`.

## Canonical pointers

- Trial verdicts + caveats: MT-C3-01 Recorded runs
  ([`spec/manual-tests/MT-C3-01-rlm-gated-trial.md`](../../fractality/v0.1.0/spec/manual-tests/MT-C3-01-rlm-gated-trial.md)).
- Narrative: `reports/2026-12-07-06-44-campaign3-f6-trial.md`.
- Predictions + verdicts: plan §7.
