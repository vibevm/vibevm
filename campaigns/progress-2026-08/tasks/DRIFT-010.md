# DRIFT-010 — the subcommands take the incremental path {#root}

<status stage="impl" state="plan" ref="DRIFT-010"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (progress adapter / progress-core cache)
**Unit-stability check:** `TOOL-INCREMENTAL` carries the owner's 2026-07-25
ruling («сделай wire и реализуй»).

## 1. Goal {#goal}

`vibe progress` stops re-parsing every file on every run, so wave 2 (the
`packages/` corpus, roughly five times this one) stays usable.

## 2. Contract {#contract}

> All subcommands are incremental over the content-hash cache (§7.1).
> — `spec://vibevm/modules/vibe-progress/PROP-043#tool`

> Per observed file: path, content-hash, extracted markers with positions …
> — `spec://vibevm/modules/vibe-progress/PROP-043#cache`

Anchor realised: `TOOL-INCREMENTAL`.

## 3. Current state {#current}

From Phase C verification evidence — do not re-discover:

- `crates/progress-core/src/cache.rs:80` — `Cache::is_current(path, hash)`
  exists and is correct. Its **only** callers are its own unit test and a
  doctest; no subcommand consults it.
- `crates/vibe-cli/src/commands/progress.rs:48` — the adapter's `ground()`
  is commented "Parse without touching any cache" and re-parses every
  observed file on every invocation.
- So the cache is written and never read for its stated purpose: today it is
  a verdict store that happens to hold hashes.

## 4. Required behavior {#behavior}

1. `ground()` grows an incremental mode: for each observed file, hash the
   bytes; if `cache.is_current(path, &hash)` **and** the cached record
   carries a parse result usable by the caller, reuse it; otherwise parse and
   upsert. The observed-set pruning that DRIFT-001 added must keep working —
   a file that left the scope is still dropped.
2. This requires the cache to hold enough to rebuild a `ParsedDoc` without
   re-reading the file, or the reuse is a lie. Two honest options, and the
   executor must pick and say which in §9: (a) store the parsed markers and
   segment offsets in the record so a `ParsedDoc` can be reconstructed; or
   (b) keep the cache as-is and skip only the *downstream* work (rollup,
   state projection) for unchanged files while still parsing. **(a) is the
   contract's reading** — "incremental over the content-hash cache" — but if
   the record cannot faithfully round-trip a `ParsedDoc`, (b) plus a recorded
   REVIEW is the honest answer, not a silent half-reuse.
3. A `--no-cache` flag on the affected subcommands forces the full path, and
   the campaign's own runs may use it. Verification runs must be able to
   distrust the cache by construction.
4. Correctness bar, non-negotiable: for every subcommand, the output with a
   warm cache must be **byte-identical** to the output with a cold one. That
   equality is what the tests below assert, and it is the whole safety
   argument for the feature.

Edge cases: a cache record whose schema predates this change is treated as a
miss (parse it). A file whose hash matches but whose record is missing markers
is a miss. A hash collision is out of scope — sha256 is the contract's own
identity story.

Error paths: an unreadable cache is a warning and a cold run, never a failure
— `spec://…#erasure` makes the cache erasable acceleration by law.

## 5. Boundaries {#boundaries}

- **Never** let an incremental run change a campaign verdict map. The
  campaign field is load-bearing; `upsert` already preserves it and must
  keep doing so.
- Do not change the marker grammar, the segmentation rules, or the state
  file shapes.
- Never edit spec text or golden tests.

## 6. Acceptance {#acceptance}

```bash
cargo test -p progress-core -p vibe-cli
cargo run -q -p vibe-cli --bin vibe -- progress scan     # 58 files, 4975 markers, 0 errors
cargo run -q -p vibe-cli --bin vibe -- progress check    # must stay 0
bash tools/self-check.sh
```

- New test (vibe-cli): `warm_and_cold_agree` — run scan twice over a fixture
  tree and assert `corpus.json`, `campaign.json` and the report output are
  byte-identical between the cold and warm runs.
- New test (vibe-cli): `edited_file_is_reparsed` — touch one file's content,
  rescan, assert only that file's record changed and its new markers appear.
- New test (vibe-cli): `campaign_map_survives_incremental` — a record
  carrying verdicts keeps them across a warm run.
- New test: `no_cache_flag_forces_full_parse`.
- Measured: report the warm-run wall time against the cold one on this
  repository in §9. A speed-up that cannot be measured is not a feature.
- Discipline: `#[spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#tool")]`,
  `cargo fmt --all`, clippy clean, atomic commits, no AI attribution.

## 7. Analogies {#analogies}

`crates/vibe-install`'s freshness skip (PROP-011) is this project's existing
"trust the cache when the hash matches" precedent — imitate its structure and
especially its escape hatch.

## 8. Stop rule {#stop}

If option (a) of §4.2 requires a cache schema bump that would invalidate the
live campaign's verdict maps: **STOP immediately** and return. The campaign
cache is irreplaceable — 4 486 verdicts, three phases of work. Surface the
migration question; do not attempt it.

Budget signal: past ~8 files or ~600 lines, stop and return.

## 9. Log {#log}

- queued 2026-07-25 (Fable), on the owner's ruling.
- implemented 2026-07-25 (Opus). **Option (a)** — the record stores the
  parse. §8 did not fire: the payload is a new `FileRecord.parsed:
  Option<ParsedDoc>` carrying `#[serde(default,
  skip_serializing_if = "Option::is_none")]`, which is additive in both
  directions. A record written before it loads unchanged and reads as a
  miss (§4's own edge case); a reader that predates it ignores the key. No
  record is re-keyed, so `CACHE_SCHEMA` stays **2** and there is no
  migration for the live verdict maps to survive. Verified against the
  real thing rather than argued: the live `run/cache.json` was copied to a
  scratch zone and scanned with the new binary — **4 486 verdicts in,
  4 486 verdicts out, 58/58 campaign maps intact**, and the live campaign
  files were never written to.
- Why (a) and not (b): the record *can* round-trip a `ParsedDoc`. The only
  two fields that do not survive the JSON are `Block::scan_text` and
  `Fact::span`, both already `#[serde(skip)]` in the type itself — the
  marker scanner's scratch, written and read inside `parse` and referenced
  by nothing downstream (`run/mirror/` has been shipping `ParsedDoc`
  without them since Phase B). That residue is named, not glossed:
  `cache::tests::cached_doc_round_trips_the_parse` clears exactly those two
  fields on a freshly parsed document and then asserts **whole-struct**
  equality against what came back out of the cache, so the day someone
  reaches for `scan_text` downstream, that test is what stops them.
- Measured (§6), debug profile — the profile the acceptance commands use —
  on this repository's 58 files, warm and cold **interleaved**, 12 samples
  each, minimum reported (the box was carrying two other campaign agents;
  the minimum is the noise-robust statistic, and interleaving cancels
  drift):

  | command | warm | cold (`--no-cache`) | speed-up |
  | --- | --- | --- | --- |
  | `progress scan` | 319.5 ms | 460.1 ms | ×1.44 |
  | `progress check` | 312.3 ms | 468.3 ms | ×1.50 |
  | `progress report --md` | 139.2 ms | 286.7 ms | ×2.06 |

  The saving is a flat ~140–155 ms in all three — the parse, exactly
  (component measurement: 58 files parse in 136.7 ms debug). `scan` wins
  least because it pays the payload's extra JSON on both sides of the
  comparison; `report` wins most because it only reads.
- REVIEW — the honest ceiling on this feature. Parsing was never the
  expensive part. Component costs on this corpus, release profile: parse
  **10.3 ms**, while the payload's own serialize + deserialize + clone is
  **7.5 ms**. So in release the reuse very nearly pays for itself and no
  more; the ×1.4–×2 above is a debug-profile result. The cost side is
  concrete: `run/cache.json` grows **2.68 MB → 5.14 MB (+92 %)** on this
  corpus, and that file is git-tracked, so every scan commit carries the
  larger blob (and wave 2, at ~5×, scales it the same way). What actually
  dominates a run is the JSON of the cache and `corpus.json` plus their
  fsync'd atomic writes, not the Markdown. If wave 2 turns out slow, the
  next lever is **not writing** `cache.json`/`corpus.json` when nothing
  changed — deliberately out of scope here (§5 fixes the state shapes, and
  `updated_at` semantics would move), so it is surfaced rather than taken.
- Also fixed in passing, because the reuse path made it reachable:
  `Cache::default()` used to yield `schema: 0`, a value no reader means and
  the `Ground` of a campaign-less run would have carried. It now yields
  `CACHE_SCHEMA`, and `load`/`load_tolerant` say so once instead of three
  times.
- Concurrency note for the reviewer: this tree was **not** exclusive. While
  this task ran, other agents edited `campaigns/…/DRIFT-012.md`,
  `DRIFT-014.md`, `spec/modules/vibe-cli/PROP-042-aiui-observation.md`,
  `specmap.json`, `crates/vibe-resolver/**` and `crates/vibe-cli/tests/**`.
  That is why `progress scan` reports 4 979 markers where §6 predicted
  4 975: the whole +4 is PROP-042 (58 → 62 markers), which this task never
  touched. File count (58) and errors (0) are as §6 states.
