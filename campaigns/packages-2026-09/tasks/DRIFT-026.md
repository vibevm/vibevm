# DRIFT-026 — sealing a verdict stops depending on memory {#root}

<status stage="impl" state="plan" ref="DRIFT-026"/>

**Status:** ready — owner picked option (a), 2026-07-26
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** progress-core + cli (the campaign's own tooling)
**Unit-stability check:** PROP-043 §4/§7 gain one anchor, written by the
reviewer after this lands (§5). No existing anchor moves.

## 1. Goal {#goal}

A session that re-derives a file's verdicts against its current text can say so
in one command, and the staleness warning stops firing on the freshest files in
the corpus.

## 2. Contract {#contract}

> Gates are **recorded, never computed here** — the caller runs the real gate
> and reports what it found.
> — `record_gate`, `crates/progress-core/src/state.rs`

Finding realised: **F-067**.

## 3. Current state {#current}

Measured 2026-07-26 — contradict me if a number is wrong:

- `campaign.processed_hash` records the text a file's verdicts were computed
  against. `progress baseline` warns when it differs from the hash just parsed.
- **Only a real verify batch writes it.** This campaign seals verdicts *by
  hand* throughout — Phase D's sync-from-code waves, and every seal this
  session — and hand-sealing sets `verified_at` and leaves `processed_hash`
  pointing at superseded text.
- **So the signal inverts.** On 2026-07-26 the warning fired on `PROP-002` and
  `PROP-043` — the two files whose verdicts had been re-derived against code
  hours earlier — and would have sent the next campaign to re-verify precisely
  the work just done.
- **A second bug, found by migrating the zones:** `content_hash` is refreshed
  only by a `scan`, so between scans the detector compares one cached value
  with another and **cannot see the disk at all**. `00-core.md` and
  `90-user.md` read as fresh minutes after being edited; only a rescan moved
  the count from 3 stale files to 5.
- The live zone is `campaigns/packages-2026-09/` (F-073); wave 1's is archival.

## 4. Required behavior {#behavior}

### 4.1 The verb is `seal`, and it records rather than verifies {#verb}

`vibe progress seal <path>…`, beside the other nine. **It is the same shape as
`gate`** — the caller did the real work and reports the result; the command
computes no verdict, changes no verdict, and never invents one. Do **not** call
it `verify`: it verifies nothing, and a name that claims otherwise is how the
next person mis-reads it.

For each named path it sets **both** `content_hash` and
`campaign.processed_hash` to the file's digest **recomputed from disk**, plus
`verified_at`. Recomputing from disk is the whole point — trusting the cached
`content_hash` reproduces the second bug in §3, and it is the exact mistake
made by hand earlier that day.

### 4.2 What sealing claims, and the refusal that keeps it honest {#claim}

**Sealing a file asserts that *every* verdict in it is valid for its current
text.** That is a strong claim and the command must not let it be made
casually:

1. **Refuse** if any marker in the file lacks a verdict — you cannot vouch for
   verdicts that do not exist. Name the count and the first few anchors.
2. **Refuse** if the path is not in the campaign cache at all.
3. **Print what is being vouched for** before doing it: the file, the verdict
   count, and the digest transition. A seal that silently touches 300 verdicts
   reads like a no-op in a diff.
4. **Refuse if the file's digest already matches** — nothing to seal, say so
   and exit 0. Re-sealing must be a no-op, not a fresh timestamp.

*(Considered and rejected: per-anchor sealing. `processed_hash` is a per-file
field, so anchor granularity needs a schema change — and the honest per-file
claim is more useful than a granular one nobody will maintain. The consequence
is deliberate: a file where only 4 of ~300 anchors were re-verified **must not
be sealed**, and stays flagged. PROP-043 is exactly that case today.)*

Edge cases: several paths at once ⇒ each independently, one refusal does not
abort the rest, exit non-zero if any refused. A path outside the observed
scope ⇒ refusal 2.

Error paths: an unreadable file is an error naming it, never a silent skip.

## 5. Boundaries {#boundaries}

- **Never edit `spec/**`.** PROP-043 gains an anchor for the verb; the
  **reviewer** writes it under sync-from-code. Record your proposed wording in
  §9.
- **Never touch a verdict's `v` or `ev`.** This command moves hashes and a
  timestamp, nothing else. A diff showing a changed verdict is a failed task.
- Do not point any command at `campaigns/progress-2026-08` — that zone is
  archival, and a stray run drags 286 package files into it.
- Do not change what `baseline`'s warning says; make it stop firing falsely by
  making the data true, not by softening the message.

## 6. Acceptance {#acceptance}

```bash
cargo test --workspace
bash tools/self-check.sh
cargo run -q -p vibe-cli --bin vibe -- progress baseline --campaign campaigns/packages-2026-09
```

- **Before:** `progress baseline` names **5** stale files (`00-core.md`,
  `90-user.md`, `MT-02-vibe-tree-tui.md`, `PROP-026-tcg-tool-family.md`,
  `PROP-043-progress-markup.md`). Report the list you actually get.
- Seal `spec/boot/00-core.md` and `spec/boot/90-user.md` — both were edited
  today and their markers all carry verdicts. **After: 3 stale files.** The
  other three must still be named: MT-02 and PROP-026 were edited by Phase D
  and nobody re-verified them, and PROP-043 had 4 of ~300 anchors re-derived.
  **If sealing makes all five go quiet, the refusal in §4.2 is not working.**
- Unit test: sealing a file with an unjudged marker refuses and names it.
- Unit test: sealing twice is a no-op the second time, and does not move
  `verified_at`.
- **The disk test, which is the one that matters:** edit a sealed file, do
  **not** rescan, and confirm `seal` sees the new digest anyway. That is §3's
  second bug and the reason this command may not read `content_hash`.
- Ledger unchanged at **4 489 confirmed / 0 drift / 3 unverifiable**. A moved
  verdict is a failed task.
- Discipline: `cargo fmt --all`, clippy clean, no AI attribution.

## 7. Analogies {#analogies}

`record_gate` (`crates/progress-core/src/state.rs`) is the shape: the caller
ran the real thing, the command records it, and the doc-comment says so in as
many words. `commands/progress/baseline.rs` is the subcommand shape;
`progress.rs` was split by DRIFT-025 precisely so this verb has room.

## 8. Stop rule {#stop}

If sealing cannot be made to refuse a partially-judged file — if there is no
way to tell "every marker carries a verdict" from the cache alone — **STOP and
say so.** Without that refusal the command is a rubber stamp, and a rubber
stamp on a staleness signal is worse than the inverted signal it replaces.

Budget signal: past ~4 files, stop and return.

## 9. Log {#log}

- queued 2026-07-26 (Fable), owner picked (a). Amendment A4 requires this
  before Phase C, which will hand-seal across 294 files — unfixed, the signal
  is noise from the first row.
- implemented 2026-07-26. `progress-core::seal` carries the decision, a new
  `commands/progress/seal.rs` carries the verb, and the two boot files are
  sealed: `progress baseline` names 5 stale files before and 3 after
  (MT-02, PROP-026, PROP-043 still flagged). Ledger unmoved at 4 489
  confirmed / 0 drift / 3 unverifiable; the cache diff is four lines, two
  `processed_hash` and two `verified_at`, and no verdict's `v` or `ev`.

### 9.1 Measured against §4.2: the refusal fires, and PROP-043 is not one of
its cases {#refusal-reach}

§4.2's parenthetical says PROP-043 is a file the marker/verdict refusal keeps
out. **It is not, and the difference is worth §5's anchor.** Measured on the
live cache:

- PROP-043 carries 148 verdict entries covering **every one** of its 147
  addressable fact anchors, plus the `_elements` bundle. The 14-marker gap
  against its 162 parsed markers is exactly the 14 table cells that inherit a
  row anchor and the one document-level `<status>` — the two kinds of marker a
  verdict map has no key for, which is why the anchored-when-marked law
  exempts cells and why campaigns file the document marker under `_elements`.
  Sealing it succeeds.
- What actually makes PROP-043 unsealable is **recency, not coverage**: 4 of
  its ~148 anchors were re-derived against today's text and the other 144 were
  formed on 2026-07-25. The cache cannot see that. All 4 492 verdict entries
  in the corpus carry exactly two keys, `v` and `ev`; the only date is one
  `verified_at` per **file**. There is no per-anchor date, batch tag or hash
  to key a recency refusal on, and inventing one is a schema change this task
  is not.
- So the refusal built here is the one §8 asks for and it does fire — one
  live corpus file trips it today
  (`packages/org.vibevm.ai-native/rust-ai-native/v0.7.0/README.md`, 6 markers,
  0 verdicts) — but it is a **coverage** gate, not a recency gate. Between
  scans the command's honesty rests where `gate`'s does: on the caller having
  done the work it reports. PROP-043 stays flagged because nobody sealed it,
  not because the command would refuse.

Left for the owner, deliberately unabsorbed: whether a per-anchor date (or a
`verify_batch` stamped per entry rather than per file) is worth the schema
bump. It is the only thing that would let a machine, rather than the
operator's discipline, keep a 4-of-148 seal from being made.

### 9.2 Proposed PROP-043 wording, for the reviewer's sync pass {#proposed}

The code scopes to `spec://vibevm/modules/vibe-progress/PROP-043#seal`, which
does not exist yet. Two edits, both under sync-from-code:

**§5 (`#tool`)** — one line in the verb list, after `gate`:

> `seal <path>…` — record that a file's verdicts hold for its current text.
> Like `gate`, it records rather than measures: the caller re-derived the
> verdicts and reports that it did, and the command computes no verdict,
> changes none and invents none.

**§7 (`#data`)** — a new subsection after §7.5, carrying the anchor:

> ### 7.6 Sealing — the record that a file's verdicts hold {#seal}
>
> A verdict is formed against a specific text. `campaign.processed_hash`
> records which one, and a campaign that hand-seals verdicts — every
> sync-from-code wave does — must write it, or the staleness warning inverts
> and fires hardest on the freshest files in the corpus.
>
> Sealing sets `content_hash` and `processed_hash` to the file's digest
> **recomputed from disk**, plus `verified_at`. Recomputing is normative: the
> cached `content_hash` is refreshed only by a scan, so a seal that read it
> would compare one stale number against another and never see the file.
>
> A seal is a per-**file** claim — that *every* verdict in the file is valid
> for its current text — because `processed_hash` is a per-file field. It is
> refused when any marker the verdict map can address carries no verdict
> (naming the count and the anchors), and when the path has no record in the
> campaign cache. A file whose digest is already recorded is a no-op that says
> so and leaves `verified_at` standing: a date that advances while nothing was
> re-verified is a forged re-verification. The claim's *recency* is not
> machine-checkable — the map carries one date per file and none per anchor —
> so a file only partly re-derived must simply not be sealed.
