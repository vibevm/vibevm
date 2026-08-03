# DRIFT-017 — a run that changes nothing writes nothing {#root}

<status stage="impl" state="plan" ref="DRIFT-017"/>

**Status:** queued (blocked on DRIFT-016 — same files)
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (progress-core state + cache writes)
**Unit-stability check:** no spec anchor moves.

## 1. Goal {#goal}

`vibe progress scan` over an unchanged tree stops rewriting `cache.json`
and the five state files — so the campaign's real cost stops being the
JSON it emits when it has nothing new to say.

## 2. Contract {#contract}

> All formats are schema-versioned (`"schema": 1`); all writes are atomic.
> — `spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#data`

> Everything else can be erased at any moment — no knowledge is lost.
> — `spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#erasure`

**Owner's ruling, 2026-07-25:** take the lever DRIFT-010 identified and
declined to pull — «не переписывать state-файлы при отсутствии изменений».

## 3. Current state {#current}

From DRIFT-010's own measurement — do not re-discover:

- Parsing was never the expensive part. In release, over this corpus, the
  parse is **10.3 ms**. What dominates a run is the JSON of `cache.json` /
  `corpus.json` and their fsync'd atomic writes.
- `refresh_state` writes cache + all five state files unconditionally on
  every `scan`, whether or not a single byte of the corpus moved.
- DRIFT-010 named this as the real lever and deliberately left it: §5 of
  that task fixed the state shapes, and `updated_at` semantics move if a
  write is skipped. That is the design question below, and it is why this
  is its own task.

## 4. Required behavior {#behavior}

1. Before writing any of the six files, serialise the candidate content and
   compare it with what is on disk. Identical ⇒ skip the write entirely, do
   not touch the file, do not fsync.
2. **The `updated_at` question, which is the whole task.** A naive
   implementation stamps `updated_at = now()` first and then compares, so
   nothing is ever identical and nothing is ever skipped. Decide and state
   the semantics in §9, choosing between:
   - **(a) `updated_at` means "when the content last changed."** Compare
     everything *except* `updated_at`; if the rest is identical, skip the
     write and the old stamp stands. Simple, and it makes the field mean
     what its name says.
   - **(b) `updated_at` means "when the tool last looked."** Then a skip is
     a lie and the field must move, which defeats the task.
   **(a) is the reviewer's reading** — a freshness plaque that advances
   while nothing changed is not freshness. But if a dashboard screen or the
   RESUME generator depends on (b), say so and stop rather than silently
   changing what a reader is told.
3. The skip must be observable: `--json` output gains a per-file or
   per-artifact `written: bool`, and the human line says how many files were
   skipped. A silent optimisation is one nobody can debug.
4. `--no-cache` keeps forcing the parse; it does **not** force a write of
   identical content — those are different questions and conflating them
   would make the flag mean two things.

Edge cases: a file absent on disk is always written. A file present but
unreadable is written (a corrupt projection should be replaced). The journal
is append-only and out of scope — never skip a journal append.

Error paths: unchanged. A comparison that fails for any reason falls back to
writing, never to skipping — the safe direction is the one that costs
milliseconds, not the one that loses state.

## 5. Boundaries {#boundaries}

- Do not change any file's *shape*. This is about whether a write happens,
  not what it contains.
- Do not touch the journal.
- Never skip a write of `cache.json` whose `campaign` maps differ — the
  verdicts are the one thing worth an unconditional fsync. If the
  comparison cannot prove they are identical, write.
- Never edit spec text.

## 6. Acceptance {#acceptance}

```bash
cargo test -p progress-core -p vibe-cli
cargo run -q -p vibe-cli --bin vibe -- progress scan   # twice
bash tools/self-check.sh
```

- New test: `second_scan_writes_nothing` — scan a fixture twice, capture
  every mtime after the first, assert none moved after the second.
- New test: `edited_file_forces_the_write` — touch one source file, assert
  cache and the affected projections are rewritten.
- New test: `verdict_change_always_writes`.
- New test: `absent_file_is_always_written`.
- **Measured, reported in §9:** wall time of a second consecutive `scan` on
  this repository, before and after, in **release** — debug numbers were
  what made DRIFT-010's headline misleading, and this task exists because of
  that lesson.
- Discipline: `cargo fmt --all`, clippy clean, atomic commits, no AI
  attribution.

## 7. Analogies {#analogies}

`crates/vibe-install`'s freshness skip (PROP-011) — this project's existing
"do nothing when nothing changed", including the fact that it is observable
rather than silent. And the managed-blocks flow's law, which is the same
idea one layer up: *never rewrite a file when the result is byte-identical.*

## 8. Stop rule {#stop}

If any consumer — the dashboard, `resume`, the freshness plaque — depends on
`updated_at` advancing on every run: STOP, name the consumer in §9, and
return. Changing what a reader is told about freshness is a contract
question, not an optimisation.

Budget signal: past ~5 files or ~350 lines, stop and return.

## 9. Log {#log}

- queued 2026-07-25 (Fable), on the owner's ruling. Blocked on DRIFT-016:
  both tasks rewrite the same write paths, and landing them concurrently
  would produce a diff nobody can review.
- implemented 2026-07-25 (Opus), on the cache DRIFT-016 left at 2.68 MB
  with a one-line steady-state diff. That one line is gone.
- **§4.2, the whole task: reading (a).** `updated_at` records when the
  content last **changed**. The comparison ignores the stamp on both
  sides, so a run may hold its own clock in the candidate while the file
  keeps the one it was last changed at, and a run that changes nothing
  leaves the old stamp standing. The naive shape §4.2 warns about —
  stamp first, compare after — is not merely avoided, it is the *first*
  mutation the tests are checked against below, because it is the failure
  that looks exactly like success.
- **§8 did not fire, and the one consumer is named.** The only reader of
  `updated_at` anywhere outside this crate's own tests is the dashboard's
  freshness plaque, `tools/progress-dashboard/index.html:101-104`: it
  renders `campaign.updated_at` as "state updated …" and adds the `stale`
  class past 24 hours. Nothing *depends* on the stamp advancing — the
  dashboard re-fetches every 5 s and reads the files themselves, so it
  notices a change whether or not the clock moved. What does change is
  what the amber means: it used to say "nobody has scanned in a day" and
  now says "nothing has moved in a day", which is §4.2's own argument
  — a plaque that advances while nothing changed is not freshness. The
  file is untouched; the reading is recorded here so it can be overturned
  in one line if the owner wants (b). `resume` was checked and does not
  read the field at all: `render_resume` builds from the journal and the
  counters. The spec names `updated_at` as a field of `campaign.json`
  (PROP-043 §7.2) and defines no semantics for it, so nothing there moves.
- **What changed.** One primitive, three call sites, one report:
  - `crates/progress-core/src/cache.rs:238` `write_if_changed` — read,
    compare, write only on a difference; `:251` `same_but_for_stamp`;
    `:270` `stamp_span`. The span is found on the bytes, not by parsing:
    the needle is a newline, two spaces and the key, which `serde_json`'s
    pretty printer can only produce for a key at depth 1 (a newline inside
    a string value is escaped, so it cannot occur there). That matters on
    the live corpus — `corpus.json` carries the word `updated_at` inside a
    verdict's own text at line 15 361, and a looser search finds it.
  - `crates/progress-core/src/cache.rs:106` `Cache::store` → `Result<bool>`.
  - `crates/progress-core/src/state.rs:77` `write_state` →
    `Result<BTreeMap<String, bool>>`; `:112` corpus, `:143` campaign. The
    three passthroughs keep their existing seed-when-absent rule and
    report `false` when present — see the edge case below.
  - `crates/vibe-cli/src/commands/progress.rs:197` `Refresh` + `:204`
    `tally`, `:226` `refresh_state` → `Refresh`, `:253` the writes.
  - `crates/progress-core/src/sidecar.rs:245` `Payloads::store` → `bool`.
- **§4.3, both surfaces.** `--json` gains `written` (artifact → bool) and
  `skipped`; `state_written` keeps its old meaning — whether there was a
  campaign zone at all — and now says so in a comment, because next to
  `written` it reads like something it is not. The human line is
  `state refreshed under <dir> — 0 written, 7 unchanged and skipped`.
  Verified in both directions on the live corpus: an untouched tree
  reports all seven `false`; with `corpus.json` clobbered, exactly
  `"corpus.json": true` and `"skipped": 6`.
- **§5 discharged on the bytes.** Identity is decided on the serialised
  document the verdict maps are *part of*, so byte-equality outside the
  stamp is proof that every `campaign` map is identical — `BTreeMap`
  makes the serialisation deterministic, which is why this is proof and
  not a guess. Everything short of proof writes: absent, unreadable, not
  UTF-8, a length that differs, a document with no stamp where this build
  puts one. The one direction that would be dangerous — a comparison that
  returns "same" when it does not know — cannot be reached.
- **An edge case §4 states and the code declines, deliberately.** "A file
  present but unreadable is written" holds for the four artifacts a run
  *derives* (cache, corpus, campaign, payloads) and not for
  `findings.json` / `tasks.json` / `docdebt.json`, which are seeded when
  absent and never rewritten — here and before this task. Replacing
  another subsystem's torn ledger with an empty seed is data loss wearing
  repair's clothes. My own test asserted the general rule first and this
  is where it failed; the behaviour is right and the test now says so.
- **Measured (§6), release profile**, on this repository's 58 files
  against a scratch copy of the live campaign zone (`--campaign` outside
  the repo, `VIBE_SETTINGS` relocated, so no live file was written by the
  measurement). Second consecutive `scan`, before and after **paired** —
  the two binaries alternate sample by sample and swap order each pair,
  which cancels the load drift a busy box adds to both; 60 pairs:

  | | min | p25 | median |
  | --- | --- | --- | --- |
  | before (unconditional writes) | 131 ms | 147 ms | 154 ms |
  | after (skip when unchanged) | **119 ms** | **124 ms** | **142 ms** |
  | paired difference | — | 4 ms | **12 ms** |

  Against a 44 ms fixed process cost (`vibe --version`, same box), that
  is ~87 ms of work before and ~75 ms after: **the writes were ~14 % of a
  warm run, not the bulk of it.**
- **REVIEW — DRIFT-010 §9 overstated this lever, and the number above is
  why.** That task's REVIEW said "what actually dominates a run is the
  JSON of the cache and `corpus.json` plus their fsync'd atomic writes".
  Half of that is right and half is not, and the half that is right is
  the half this task cannot collect: §4.1 requires serialising the
  candidate *in order to compare it*, so all ~6.5 MB of JSON generation
  still happens on every run — only the write, fsync and rename are
  saved, and this task additionally pays ~6.5 MB of reads to decide that.
  On this box those two nearly cancel; 12 ms is what is left. Anyone
  reaching for the next lever should aim at the **serialisation**, not
  the IO: not writing is now free, and not *rendering* is where the
  remaining ~75 ms lives.
- **The win that is not wall-clock, and is probably the real one.** Two
  live scans of this repository now leave `git status` byte-identical —
  the campaign zone is not touched at all. Before this change every scan
  dirtied `cache.json`, `corpus.json` and `campaign.json`; DRIFT-016 §9
  measured that steady-state diff as "one line, the timestamp", and it is
  now zero files. Demonstrated side by side on the scratch zone: the old
  binary rewrites all three on a second scan, the new one leaves all
  three mtimes untouched.
- **One artifact beyond the six §4.1 names, deliberately.** The payload
  sidecar (`payloads.json`, 1.11 MB, its own fsync every run) is skipped
  on the same rule — it carries no clock, so its test is plain byte
  equality. §1's title is "a run that changes nothing writes nothing",
  and leaving the seventh file rewriting would have made that false. It
  is one line in `sidecar.rs` and one entry in the reported map; back it
  out there if the reviewer wants the six exactly.
- **Tests added, each mutation-checked rather than merely green** — the
  mutation, then what caught it:
  - naive (b), the stamp compared like content ⇒
    `second_scan_writes_nothing` and `verdict_change_always_writes` fail.
  - content never compared, everything skips ⇒
    `edited_file_forces_the_write` and `verdict_change_always_writes` fail.
  - an absent or unreadable file counts as identical ⇒ all four fail.
  - `progress-core`: `cache::tests::write_if_changed_skips_the_stamp_and_nothing_else`
    — absent ⇒ written, identical ⇒ skipped, stamp-only ⇒ skipped *and the
    old stamp still on disk byte for byte*, content-only ⇒ written, the
    `corpus.json` trap (the field named inside a value) ⇒ written, not
    UTF-8 ⇒ written, and the no-stamp fallback the sidecar takes.
  - `vibe-cli`: §6's four, in a cell of their own,
    `src/commands/progress/tests/writes.rs` — `second_scan_writes_nothing`
    (every artifact's mtime after the first scan, none moved after the
    second, and the reported tally agrees), `edited_file_forces_the_write`,
    `verdict_change_always_writes`, `absent_file_is_always_written`. The
    instrument is the mtime, not the tally: the tally is this code's own
    opinion of what it did, and the point is to check it.
  - `verdict_change_always_writes` also pinned something worth keeping: a
    verdict written into `cache.json` out of band leaves the cache
    unwritten on the next scan (it already says that) while `corpus.json`
    *is* rewritten, because the projection carries the campaign map. The
    projection is never left behind a verdict just because the cache was
    not.
- The floor caught the same defect for the third running task —
  `.expect()` in a test helper (`settled`), which DRIFT-010 hit on
  `incremental_fixture` and DRIFT-016 on `payload_for`. It now returns
  `Result<()>` and each `#[test]` decides to panic. `cargo xtask conform
  check`: 0 findings. The new cell also keeps
  `commands/progress/tests.rs` at 549 lines, under the 600 budget, which
  is why it is a cell rather than an append.
- Concurrency note for the reviewer: this tree was **not** exclusive,
  though the task brief said it was. While this task ran, another agent
  modified `crates/vibe-cli/tests/cli_init.rs`,
  `crates/vibe-cli/tests/cli_search.rs` and
  `crates/vibe-cli/tests/common/mod.rs` (a `UserScratch` migration citing
  F-056), between 23:01 and 23:04. None of it was touched here and all of
  it is in the working tree unstaged — the `cargo test`, `cargo clippy`
  and `bash tools/self-check.sh` results reported above were green
  *including* that work.
- Verified per §6: `cargo test -p progress-core -p vibe-cli` green,
  `progress scan` twice on this repository (58 files, 4 979 markers, 0
  errors, `0 written, 7 unchanged and skipped` both times),
  `bash tools/self-check.sh` → `self-check: all green`, exit 0.
