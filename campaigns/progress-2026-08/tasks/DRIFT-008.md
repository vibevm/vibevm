# DRIFT-008 — `campaign.json` carries the gate panel {#root}

<status stage="impl" state="done" ref="DRIFT-008"/>

**Status:** done — executed by Opus 2026-07-25, reviewed and accepted by Fable
the same day (diff read in full; the no-gate shape is byte-compared against the
pre-panel bytes, the panel survives a scan, the error names both the file and
the command that writes it, and the core spawns nothing).
**Reviewer ruling on the surfaced residual:** `write_state`'s tolerant read
means a *corrupt* `campaign.json` silently loses the reported panel. Accepted:
`progress-core` is a pure library with no logging seam, inventing one for this
corner would cost more than the data — which is re-reportable by re-running the
gate — and the erasure law (§7.5) makes the projection expendable by design.
Revisit if the panel ever carries something that is not re-derivable.
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
> — `spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#state`

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
- Discipline: `#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#state")]`,
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
- implemented 2026-07-25. §8 stop rule checked first: PROP-043 §7.2
  (`STATE-FILES`) names `gates` in the `campaign.json` field list and
  specifies no shape, so §4's design stands unconflicted — no `REVIEW`
  marker raised. Two points §4 left open, decided against in-repo
  precedent and recorded for the reviewer: (a) `detail` is
  `skip_serializing_if = "Option::is_none"`, mirroring `journal.rs`'s
  `StepDone::result` (§7's named analogy); (b) `write_state` reads the
  previous `campaign.json` *tolerantly* — an unreadable projection
  degrades to an empty panel instead of wedging every scan, since the
  file is a derived artifact (§7.5) and `record_gate` is the path that
  fails loudly. `unknown` is representable in `GateStatus` but not
  offered by the CLI, exactly as §4.2/§4.4 are written.
- §6 CLI scenario was run against a **copy** of this campaign's
  `run/state/campaign.json` in a scratch zone (`--campaign <scratch>`),
  not against the live file: the executor was instructed not to write
  under `campaigns/progress-2026-08/run/`. Output identical in kind —
  the `gates` array appears with `floor`/`red`/the detail string, and a
  following full-corpus `vibe progress scan` (58 files, 4911 facts)
  preserved it.
