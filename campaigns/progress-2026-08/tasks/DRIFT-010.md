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
