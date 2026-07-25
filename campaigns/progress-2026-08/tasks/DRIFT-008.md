# DRIFT-008 — `campaign.json` carries the gate panel {#root}

<status stage="impl" state="plan" ref="DRIFT-008"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (progress-core state / progress adapter)
**Unit-stability check:** `STATE-FILES` carries the owner's 2026-07-25 ruling
— «а что если она будет гоняться не руками?? если это важно для
автоматизации — wire».

## 1. Goal {#goal}

The dashboard and any automation can read the project's gate-panel state out
of `campaign.json` instead of a human remembering to run the floor and
report it in prose.

## 2. Contract {#contract}

> `campaign.json` (phase lane, wave, counters, **gates**) … The dashboard
> reads **only** these; it computes nothing and parses no Markdown ever.
> — `spec://vibevm/modules/vibe-progress/PROP-043#state`

Anchor realised: `STATE-FILES`.

## 3. Current state {#current}

From Phase C verification evidence — do not re-discover:

- `crates/progress-core/src/state.rs:17` — `CampaignState` has
  `schema`, `updated_at`, `campaign_id`, `phase` (line 22), `counters`
  (line 24). `grep gates state.rs` = **0 hits**.
- `write_state(state_dir, campaign_id, phase, cache)` (line 30) builds the
  struct and the `counters` json (line 68).
- The five state files themselves exist and feed the dashboard, exactly as
  the contract says. Only the `gates` field is missing.

## 4. Required behavior {#behavior}

1. `CampaignState` gains `gates: Vec<GateRecord>`, serialised as an array and
   omitted when empty (`skip_serializing_if`) so existing consumers of the
   file see no change until a gate is recorded.
2. `GateRecord { name: String, status: GateStatus, ran_at: String, detail:
   Option<String> }` where `GateStatus` is `green | red | stale | unknown`,
   serialised lowercase.
3. Gates are **recorded, never computed by progress-core** — the core must
   not shell out, and must not know what a floor is. Add
   `progress_core::state::record_gate(state_dir, GateRecord) -> Result<()>`,
   which loads `campaign.json`, replaces the entry with the same `name` (or
   appends), and writes atomically. `write_state` **preserves** any gates
   already on disk: a scan must never erase the panel.
4. The adapter grows `vibe progress gate <name> --status <green|red|stale>
   [--detail <text>]`, which calls `record_gate` with the current UTC
   timestamp. This is the automation seam: a CI step or a local script runs
   the real gate and reports the verdict here.
5. `stale` is the status a gate takes when it has not been re-run since the
   corpus changed. `record_gate` does not decide that; a caller does.

Edge cases: a `campaign.json` with no `gates` key loads as an empty vec. Two
records with the same `name` never coexist — the later replaces the earlier.
An unknown `--status` value is a clap error, not a silently stored string.

Error paths: `record_gate` on a missing `campaign.json` is an error naming
the file and suggesting `vibe progress scan` — the state dir must exist first.

## 5. Boundaries {#boundaries}

- progress-core must not run any command, spawn any process, or read any
  file outside the state dir. The gate panel is data the project reports in,
  never something the core measures — that is `spec://…#separability`.
- Do not change the shape of `counters` or any other existing field.
- Never edit spec text or golden tests.

## 6. Acceptance {#acceptance}

```bash
cargo test -p progress-core -p vibe-cli
bash tools/self-check.sh
```

- New test: `gates_absent_serialises_to_no_key` — a state with no gates
  produces json without a `gates` key (byte-compare against the current
  fixture).
- New test: `record_gate_appends_then_replaces` — recording `floor=green`
  then `floor=red` leaves exactly one entry, red.
- New test: `write_state_preserves_gates` — record a gate, run
  `write_state`, assert the gate survives.
- New test (vibe-cli): `progress_gate_cli_records` — the subcommand writes
  the record and a following `progress scan` keeps it.
- CLI scenario: `vibe progress gate floor --status red --detail
  "cli_pkg_cycle::install_from_git_registry (F-055, environmental)"` then
  reading `campaigns/progress-2026-08/run/state/campaign.json` shows it.
- Discipline: `#[spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#state")]`,
  `cargo fmt --all`, clippy clean, atomic commits, no AI attribution.

## 7. Analogies {#analogies}

`crates/progress-core/src/journal.rs` is the closest shape for an
append-and-derive file with atomic writes; `state.rs`'s existing
`write_state` shows the serialisation and atomic-write idiom to reuse.

## 8. Stop rule {#stop}

If PROP-043 §7.2 turns out to specify a gate shape different from §4's:
STOP, mark `<!-- REVIEW: … -->`, record the question here, set status
`returned`. The contract's word is `gates` and nothing more, so §4 is a
reviewer's design — surface any conflict rather than absorbing it.

Budget signal: past ~5 files or ~350 lines, stop and return.

## 9. Log {#log}

- queued 2026-07-25 (Fable), on the owner's ruling.
