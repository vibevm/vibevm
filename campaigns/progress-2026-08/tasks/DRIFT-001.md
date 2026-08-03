# DRIFT-001 — cache prunes records that leave the observed scope {#root}

<status stage="impl" state="done" ref="DRIFT-001"/>

**Status:** done — executed by Opus 2026-07-24, reviewed and accepted by
Fable the same day (diff read in full; retain_paths preserves survivors'
campaign maps and returns dropped-verdict paths for a loud warning;
adapter test proves corpus.json == observed set after narrowing;
self-check all green, exit 0). Reviewer ruling on the surfaced §5
residual: drop-with-loud-warning is CORRECT per the erasure law
(PROP-043 §7.5 — the cache is erasable acceleration; baseline.json §7.3
is the durable verdict home); if a mid-campaign narrowing ever
threatens live verdicts, land them into the baseline first.
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (progress adapter / progress-core cache)
**Unit-stability check (release precondition):** every anchor cited in §2 has
no open obligation in the findings ledger and no `unknown` marker.

## 1. Goal {#goal}

`vibe progress scan` stops carrying cache/state records for files that are no
longer in the observed scope, so the dashboard and counters always describe
exactly the configured corpus.

## 2. Contract {#contract}

> All subcommands are incremental over the content-hash cache (§7.1).
> — `spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#tool`

> Per observed file: path, content-hash, extracted markers with positions …
> — `spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#cache`

> `corpus.json` (per-file rollups and counts) … The dashboard reads **only**
> these; it computes nothing and parses no Markdown ever.
> — `spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#state`

The cache is *per observed file*; a file outside the observed set has no
contract right to a record.

## 3. Current state {#current}

- `crates/progress-core/src/cache.rs` — `Cache::upsert` inserts/updates by
  path; nothing ever removes a record.
- `crates/vibe-cli/src/commands/progress.rs::refresh_state` — upserts every
  parsed doc, then writes state from **the whole cache**, so records for
  files that left the scope (e.g. after `progress.toml` narrowed the globs)
  survive forever and inflate `corpus.json` / `campaign.json` counters.
- Observed live 2026-07-24: after the wave-1 narrowing the cache carried 497
  records vs 97 observed files; the dashboard showed 8 625 unmarked vs the
  true 3 638. (The schema-2 bump rebuilt the cache and hid the symptom; the
  defect — no pruning on scope change — remains.)

## 4. Change {#change}

1. Add `Cache::retain_paths(observed: &BTreeSet<String>)` (or equivalent)
   in `progress-core` that drops records whose path is not in the observed
   set — preserving campaign fields of the records that stay.
2. Call it from `refresh_state` in the vibe-cli adapter with the paths of
   the docs just parsed, before `store` / `write_state`.
3. Unit test in `progress-core`: upsert two files, retain one, assert the
   other's record is gone and the survivor's `campaign` map is intact.
4. Adapter test (vibe-cli): scan a fixture tree, narrow the scope config,
   rescan, assert `corpus.json` rows equal the observed set.

## 5. Stop-rule {#stop}

Stop and return the task if: pruning would delete a record that carries a
non-empty `campaign` field for a file that still exists on disk but left the
scope (verdict data loss) — surface the question instead of choosing.

## 6. Acceptance {#acceptance}

- `cargo test -p progress-core -p vibe-cli` green, including the two new
  tests of §4.
- Manual: `vibe progress scan` on this repo, then narrow `progress.toml` to
  a single subdirectory, rescan — `corpus.json` contains exactly the
  narrowed set; counters match the scan line.
- `bash tools/self-check.sh` green.
