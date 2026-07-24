# DRIFT-003 — campaign phase is hardcoded "A" in the progress adapter {#root}

<status stage="impl" state="done" ref="DRIFT-003"/>

**Status:** done — executed by Opus 2026-07-24, reviewed and accepted by
Fable the same day (diff read in full; five new tests incl. the
torn-tail and unknown-kind laws; §6 manual acceptance run live: phase
event backfilled, `campaign.json` and RESUME render "B";
`self-check` all green, exit 0). Accepted addition beyond §4's
minimum: the `journal_tolerates_unknown_event_kinds` test locking the
forward-compat requirement, with `read_journal` reworked to a
two-stage parse (torn tail = incomplete JSON stops; a complete line
of an unmodeled kind is skipped, never truncates).
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (progress adapter / journal)
**Unit-stability check (release precondition):** every anchor cited in §2 has
no open obligation in the findings ledger and no `unknown` marker.

## 1. Goal {#goal}

`campaign.json` (and therefore the dashboard's phase lane and `RESUME.md`)
reports the campaign's actual phase, derived from the campaign's own
journal, instead of a compiled-in `"A"`.

## 2. Contract {#contract}

> `campaign.json` (wave, stage-of-campaign, gates, counters, `updated_at`) …
> The dashboard reads **only** these; it computes nothing and parses no
> Markdown ever.
> — `spec://vibevm/modules/vibe-progress/PROP-043#state`

> all writes are atomic (tmp + rename); the journal is append-only JSONL
> (a torn tail line is discarded on read)
> — `spec://vibevm/modules/vibe-progress/PROP-043#data`

## 3. Current state {#current}

- `crates/vibe-cli/src/commands/progress.rs:114` passes a literal `"A"`
  into `state::write_state(...)` on every refresh, so `campaign.json`
  says `"phase": "A"` while the campaign runs Phase B (observed live
  2026-07-24; `RESUME.md` renders the same stale phase).
- The journal (`run/journal.jsonl`) carries only `step-start` /
  `step-done` events; nothing records a phase transition
  machine-readably — the plan's LOG section does, but the tool must not
  parse Markdown (§2 contract).

## 4. Change {#change}

1. Teach the journal reader in `progress-core` a third event kind:
   `{"kind":"phase","value":"B","ts":"…"}` — append-only, last one wins;
   absent ⇒ `"A"` (the campaign's opening phase).
2. `refresh_state` in the vibe-cli adapter derives the phase from the
   journal (when a campaign zone is present) and passes it to
   `write_state`; the `resume` renderer prints the same derived value.
3. Backfill is out of scope for the code: the campaign executor appends
   the `phase` event for the already-open Phase B by hand once this
   lands (the journal is the executor's file).
4. Unit test in `progress-core`: journal with no phase event ⇒ `"A"`;
   with two phase events ⇒ the later value; torn tail still tolerated.
5. Adapter test (vibe-cli): fixture campaign zone with a `phase` event ⇒
   `campaign.json` carries it.

## 5. Stop-rule {#stop}

Stop and return the task if deriving the phase seems to require reading
any Markdown file (plan, RESUME) — that violates the §2 contract and
means the journal-event design needs Fable review instead.

## 6. Acceptance {#acceptance}

- `cargo test -p progress-core -p vibe-cli` green, including the new
  tests of §4.
- Manual: append `{"kind":"phase","value":"B"}` to this campaign's
  journal, run `vibe progress scan` — `campaign.json` says `"phase": "B"`
  and `RESUME.md` renders `**Phase:** B`.
- `bash tools/self-check.sh` green (real exit code).
