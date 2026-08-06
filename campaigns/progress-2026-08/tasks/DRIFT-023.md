# DRIFT-023 — the baseline gets a writer, and §6 becomes runnable {#root}

<status stage="impl" state="plan" ref="DRIFT-023"/>

**Status:** ready — owner approved the build direction 2026-07-26
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** progress-core + cli (the campaign's own tooling)
**Unit-stability check:** `PROP-043#baseline` is the anchor this task makes
true; it currently carries a false `confirmed` (see §3). No other anchor moves.

## 1. Goal {#goal}

`campaigns/<id>/baseline.json` can be produced by a command, and feeding it
straight back to `vibe progress rescan` reports every unit as
**carried-forward**. The monthly recurrence stops being a document.

## 2. Contract {#contract}

> @fact:BASELINE-RECORD `baseline.json` — per unit: URI#anchor, unit content-hash
> at verdict time, verdict, evidence refs, date, named crates, marker
> snapshot. **Shipped:** `baseline.rs`'s `BaselineUnit` carries exactly these
> fields, with `Baseline::load` / `store` and the `rescan` CLI live.
> — `spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#baseline`

> The one artifact worth keeping anyway is `baseline.json`: not knowledge but
> **acceleration** — its loss returns the next run's cost from O(delta) to
> O(corpus).
> — `PROP-043` `##BASELINE-ACCELERATION`

Finding realised: **F-065**. Campaign plan §5 Close-out ("`baseline.json`
written") and §6 Recurrence both depend on this.

## 3. Current state {#current}

Measured 2026-07-26 — do not re-discover, but do contradict me if a number
is wrong:

- **`Baseline::store` does not exist.** The only `impl Baseline` in
  `crates/progress-core/src/baseline.rs` is `load` (`:172-179`). Nothing in
  the workspace constructs or serialises a `Baseline`. The `store` the spec
  cites is `Cache::store` (`cache.rs:106`) — a different type in the same
  crate.
- **There is no `baseline` subcommand.** `ProgressSubcommand`
  (`crates/vibe-cli/src/cli/progress.rs:20-51`) is exactly
  `Scan · Check · Report · Mirror · Weave · Rescan · Resume · Gate`.
- So `rescan --baseline <file>` — the entire §6 recurrence entry point —
  consumes a file nothing in the tree can produce. **The loop has never been
  run end to end.**
- The quoted §7.3 status line is therefore a false `Shipped:` claim, and
  `BASELINE-RECORD` was flipped drift → confirmed on it by commit `0064fd4a`.
  This task makes the line true; the reviewer re-seals the verdict after.

**The granularity gap — this is the crux, and the decision is already made
for you (§4.1). Do not redesign it.** The campaign's knowledge is
**per fact anchor**; the baseline contract is **per unit**:

- `run/cache.json` verdicts are keyed by `##FACT-ID` — 4 492 of them. The
  cache says so itself: every file's `campaign.coverage_note` opens
  *"verdicts keyed by fact anchors"*.
- `rescan` walks `doc.units` and addresses them `unit_addr(doc, i)` →
  `path#<unit-anchor>` or `path#L<line>` (`baseline.rs:212-218`). There are
  **920 units** across the 58 observed files.
- Example: `PROP-002-decentralized-registry.md` — 35 units, 359 facts,
  288 verdicts.

## 4. Required behavior {#behavior}

### 4.1 The projection: facts roll up to units {#projection}

Write the baseline at **unit** granularity, exactly the §7.3 schema, with no
schema extension. Fact-level verdicts stay where they already live —
`run/cache.json`, which is in git.

*(Considered and declined: making the baseline fact-grained. It would need a
content hash per fact, a rewrite of `rescan` and its six tests, and an edit
to a shipped §7.3 contract — to buy resolution §7.3 deliberately does not
want. §7.3 calls code-side invalidation "deliberately coarse" and pairs it
with a random control sample precisely because of this. If you become
convinced this is wrong, that is a §8 stop, not a redesign.)*

Per unit, in `doc.units` order:

1. **`addr`** — `unit_addr(doc, i)`. Use the existing function; do not
   re-derive the format.
2. **`unit_hash`** — the unit's `content_hash`. It must be the same field
   `rescan` compares at `baseline.rs:250`, or nothing ever carries forward.
3. **`verdict`** — the **worst** verdict among the unit's judged facts, on
   the order `drift` > `unverifiable` > `confirmed`. Worst-wins, because a
   unit carrying one drifting fact is not a unit that may skip re-verification.
4. **`evidence`** — the union of those facts' evidence strings, deduplicated,
   order-stable. Construct **through `BaselineUnit::new`**, never as a
   literal: `new` is what derives `crates`, and its own doc-comment records
   that a hand-built literal forgetting `crates` leaves the named-crate
   invalidation rule unable to fire — the exact state DRIFT-009 found.
5. **`marker`** — resolved **by the same code path `rescan` uses**
   (`baseline.rs:260-270`): the first `Granularity::Section` marker whose
   line falls inside `[u.line_start, u.line_end]`, else the document marker,
   formatted `"{stage}/{state}"`. **Extract that resolution into one shared
   function and call it from both sides.** Re-implementing it is the single
   most likely way to ship a baseline where every unit reports
   `marker_diverged` forever.
6. **`verified_at`** — the file's `campaign.verified_at`.

**A unit with zero judged facts is omitted from the baseline, and the count
of omitted units is reported on stdout.** Do not invent a verdict for it.
An absent unit reads as `new` on the next rescan, which costs one
re-verification; a fabricated verdict carries forward a judgment nobody made.
This artifact must fail toward re-verifying, never toward false confidence.

### 4.2 `Baseline::store` {#store}

Mirror `Cache::store` (`cache.rs:106`) in shape: atomic write, and the
no-op-write skip DRIFT-017 established (identical content ⇒ no write, and say
so). `schema = BASELINE_SCHEMA`, `written_at` RFC-3339 UTC, `campaign_id`
from the campaign zone. Stable ordering throughout — `units` is a
`BTreeMap`, so two runs over an unchanged tree must produce a byte-identical
file.

### 4.3 The subcommand {#cmd}

`vibe progress baseline` beside the other seven, taking `ProgressCommonArgs`
plus `--out <file>` (default `campaigns/<id>/baseline.json`). It reads the
campaign cache — it does **not** re-verify anything and must never invent a
verdict. Journal the step like the other campaign verbs. Print a summary:
units written, units omitted for want of a verdict, and the verdict
breakdown.

Edge cases: no campaign zone ⇒ clean error naming what is missing, non-zero
exit. An empty verdict map ⇒ write an empty baseline and say so plainly
rather than failing. A unit anchor that collides with another in the same
file ⇒ surface it; do not silently let one overwrite the other in the map.

Error paths: an unreadable cache is an error, never a silently empty
baseline — a truncated baseline is worse than none, because it reads as
knowledge.

## 5. Boundaries {#boundaries}

- **Do not touch `rescan`'s behaviour.** You may extract the marker
  resolution into a shared function — that is a refactor with identical
  semantics, and `rescan`'s six existing tests must pass untouched. Anything
  that changes what `rescan` classifies is out of scope.
- Do not change the §7.3 schema. No new `BaselineUnit` fields.
- **Never edit spec text.** `PROP-043` §7.3's status line is wrong today and
  this task makes it true; the reviewer lands the spec side under
  sync-from-code. Record in §9 the exact text you made true.
- Do not touch `run/cache.json`'s verdicts. You read them; the reviewer
  writes them.

## 6. Acceptance {#acceptance}

```bash
cargo test --workspace
bash tools/self-check.sh
cargo run -q -p vibe-cli --bin vibe -- progress baseline
cargo run -q -p vibe-cli --bin vibe -- progress rescan --baseline campaigns/progress-2026-08/baseline.json --control-rate 0
```

- **The round-trip control, and it is the whole point of the task:** write
  the baseline, then rescan against it with `--control-rate 0` on an
  unchanged tree. **Every row must be `carried-forward`.** A single `new` or
  `changed` row means the projection disagrees with the reader — most likely
  the marker resolution or the hash field — and the task is not done. Report
  the row counts by class verbatim in §9.
- **The negative control:** change one unit's text, rescan, confirm exactly
  that row turns `changed` and its neighbours do not; revert. A gate never
  seen to fire is not known to work.
- **The determinism control:** run `progress baseline` twice; the second run
  must report no write, and `git diff` must be empty.
- `marker_diverged` must be **false everywhere** on the round-trip. If it is
  true anywhere, §4.1.5 was re-implemented rather than shared.
- Unit test: worst-wins rollup — a unit whose facts are
  `confirmed`+`drift` records `drift`.
- Unit test: a unit with no judged facts is omitted, not fabricated.
- Discipline: `cargo fmt --all`, clippy clean, atomic commits, no AI
  attribution.

## 7. Analogies {#analogies}

`Cache::store` (`crates/progress-core/src/cache.rs:106`) is the writer shape,
including the changed-only write. `crates/vibe-cli/src/commands/progress/rescan.rs`
is the subcommand shape and shows how the campaign zone is located.
`BaselineUnit::new`'s doc-comment carries the rule about `crates` derivation.

## 8. Stop rule {#stop}

If the round-trip cannot be made clean — if some units insist on reading
`new` or `changed` against a baseline just written from the same tree —
**STOP and report the classes and a sample of addresses.** Do not add a
fudge, a tolerance, or an exception list to make the control go green. A
baseline that needs a tolerance to round-trip is measuring something other
than what `rescan` reads, and that is a finding, not a bug to paper over.

If §4.1's rollup turns out to be impossible because units and fact anchors do
not nest the way §3 says: STOP, show the counter-example, return.

Budget signal: past ~6 files, stop and return.

## 9. Log {#log}

- queued 2026-07-26 (Fable), on the owner's ruling («Сделай первое»). Filed
  as F-065 an hour earlier: the spec claimed a writer that was never built,
  and the claim was authored by this campaign's own Phase D — the `store` in
  view belonged to a different type in the same crate.

- built 2026-07-26. `§3`'s numbers all held: `Baseline::store` did not exist,
  the eight subcommands were the eight named, and nothing in the tree could
  produce the file `rescan --baseline` consumes. **The loop now runs end to
  end.**

  **Shipped.** `progress-core`: `baseline::governing_marker` — the marker
  resolution lifted out of `rescan` verbatim and now called from both sides
  (§4.1.5); `baseline::project` (new file) — the fact → unit projection and
  `Baseline::store`; `cache::stamp_span` widened to recognise the baseline's
  `written_at` beside `updated_at`, so DRIFT-017's no-op-write skip covers
  this artifact too. `vibe-cli`: `progress baseline [--out]`, and `Ground`
  now keeps the cache's load warning so this one verb can **refuse** rather
  than write a truncated baseline. `rescan`'s six tests are untouched and
  green.

  **The projection, as implemented.** A judged fact rolls up into *every*
  unit whose body span carries it, nested subsections included — a unit's
  identity is the hash of its whole span, so a verdict riding on that hash
  has to answer for everything inside it. Worst wins on
  `drift > unverifiable > confirmed`, and a verdict string this code does
  not model outranks all three rather than being swallowed by a neighbouring
  `confirmed`. Evidence is the union in document order, deduplicated, built
  through `BaselineUnit::new` so `crates` is derived. Nothing else is
  invented: a unit no judged fact reaches is omitted and counted, a file
  with no `verified_at` has its units omitted and is named, an address two
  units claim is reported, and a verdict key that names no fact anchor is
  reported.

  **The spec text this makes true** — `PROP-043` §7.3, for the reviewer's
  sync pass, quoted verbatim as it stands today:

  > - @fact:BASELINE-RECORD `baseline.json` — per unit: URI#anchor, unit content-hash at verdict time,
  >   verdict, evidence refs, date, named crates, marker snapshot. **Shipped:**
  >   `baseline.rs`'s `BaselineUnit` carries exactly these fields, with
  >   `Baseline::load` / `store` and the `rescan` CLI live. @status:impl/done

  The `store` half of that sentence is now true of `Baseline` rather than of
  `Cache`. The line still says nothing about the `baseline` subcommand or
  about where the verdicts come from — §5's CMD list is one command short of
  the shipped tool.

  **The round trip (§6), verbatim.** `progress baseline` → 920 units
  written, **0 omitted**, 914 confirmed / 2 drift / 4 unverifiable; 58
  verdict keys unresolved (53 per-file `_elements` bundles + 5 anchors that
  sit mid-block, all five `confirmed`, and each of their units is judged by
  the block's own first anchor, so no unit lost its coverage).
  `rescan --baseline campaigns/progress-2026-08/baseline.json
  --control-rate 0` on the unchanged tree:

  > 0 new, 1 changed (suspect), 919 carried-forward, 0 control-sample
  > Changed spec/boot/00-core.md#L1  [crate `vibe-cli` moved after the verdict]

  `marker_diverged` is **false on all 920 rows**. The one non-carried row is
  not a writer/reader disagreement: `crate_moved` is only ever set on the
  branch where the hashes already matched, so that unit's hash and marker
  both agreed — it is invalidation rule 2 firing correctly, because the unit
  cites `crates/vibe-cli/Cargo.toml` and `crates/vibe-cli` last moved
  2026-07-25T22:56Z, after that file's `verified_at` of 2026-07-25T12:44Z.
  No tolerance was added for it (§8); the row is the rule working.

  **Negative control.** One word edited inside `PROP-043`'s §7.3 body: that
  unit turns `changed`, and with it exactly its two ancestors (`#data`,
  `#root`) whose hashes cover it — `#cache`, `#state`, `#campaign-zone` and
  `#erasure` all carry forward. Restored byte-for-byte (sha256 verified) and
  the run goes back to 919/1.

  **Determinism control.** A second `progress baseline` over the unchanged
  campaign reports `unchanged, nothing written`; the file's mtime (ns) and
  sha256 are identical across the two runs.

  **Not absorbed, deliberately.** (a) §4.3's "journal the step like the
  other campaign verbs" — *no* verb journals today (`start_step` /
  `done_step` have no caller outside their own tests), `journal.jsonl` is
  tracked, and appending on every run would dirty `git diff` and break §6's
  own determinism control. The campaign's steps stay journaled by whoever
  runs them. (b) Three files (`MT-02`, `PROP-026`, `PROP-002`) moved after
  their verdicts were formed — their `processed_hash` is not what this run
  parsed — so their units carry forward a judgment formed against different
  text. §4.1.2 fixes the hash as the current one, so they are projected as
  written and **reported** (a warning per file, and `stale` in `--json`)
  rather than filtered out: filtering would be a rule §4.1 does not have.
  Re-verify those three, or refresh their maps, before close-out seals this
  baseline.
