# DRIFT-033 — campaign state stops being believed on its own word {#root}

```
<status stage="impl" state="plan" ref="DRIFT-033"/>
```

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** common (`progress-core`)
**Findings:** F-075 (owner chose option **d**) and F-077 (owner chose option
**a**), both 2026-07-26.

Two findings, one theme: a stored value that nothing forces to stay true. They
travel together because they are the same class in the same crate, and they land
as **two commits**.

## 1. Goal {#goal}

`seal` writes the staleness evidence it already has a field for, and the
campaign's counts stop being stored beside the data they are derived from.

## 2. Contract {#contract}

```
> `processed_hash` records the text a file's verdicts were computed against
> — campaign amendment A4, spec://vibevm/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1#amendments
```

Amendment **A4** already requires the hand-seal staleness gap to be closed
**before Phase C**, and names F-067 as the reason. F-075 is the same mechanism
seen from the other end: `seal` can check *coverage* and not *recency*. One
change closes both.

## 3. Current state {#current}

Measured 2026-07-26 by reading `campaigns/packages-2026-09/run/`. **Do not
re-discover.**

**F-075.** A verdict entry carries exactly `{"v": <verdict>, "ev": [<refs>]}` —
**no timestamp**. Recency lives one level up, per file:
`{processed_hash, summary, verdicts, verified_at, verify_batch}`. So one date
covers every verdict in a file — `spec/boot/00-core.md` has **33** verdicts
under a single `verified_at`. `processed_hash` is the field that would make
staleness checkable, and per F-067 it is written **only by a real verify batch**,
never by a hand `seal`. This campaign hand-seals across hundreds of files.

**F-077.** The top-level `campaign.summary` the finding was written about is
**gone** — `campaign.json` now carries `{campaign_id, counters, phase, schema,
updated_at, wave}` and `summary` reads `null`. The class did not go away, it
moved: `counters` = `{files: 264, facts: 10825, unmarked: 5478, issues: 0}` is
now the stored projection. Per-file `summary` fields also survive in the cache —
**58 of them, and all 58 currently agree** with a recount over their own
`verdicts` map. Zero drift today; nothing prevents it tomorrow.

## 4. Required behavior {#behavior}

```
F-075 (commit 1):
1. `vibe progress seal` writes `processed_hash` for each file it seals,
   computed from the same bytes a real verify batch would use. Find that
   computation and CALL it — do not write a second implementation, or
   the two hashes become the next thing nothing keeps honest.
2. A file sealed by hand and a file verified by a batch must end up
   with the same `processed_hash` for the same content. Prove it.
3. Nothing else about `seal`'s refusal changes: it remains a coverage
   gate. Recency becomes *checkable*, which is what was missing.

F-077 (commit 2):
4. Delete the stored per-file `summary` field and compute it on read.
5. Delete `counters` from campaign.json and compute on read, from the
   same source. If a consumer needs it in the file, stop and report -
   that is a design question, not a code one.
6. Reading a cache written by the previous version must still work:
   an existing `summary`/`counters` is ignored, not an error.
```

Edge cases: sealing a file whose verdicts are empty; a file whose content
changed between the seal and the write; a cache from before this change (both
fields present) and one from after (both absent) must both load.

Error paths: none new.

## 5. Boundaries {#boundaries}

- **Do not edit `spec/**`.** PROP-043 §7 describes the data contracts and will
  want an edit — **the reviewer writes it.** A spec doubt is a §8 stop.
- **Do not hand-write a timestamp or a hash into campaign state**, and do not
  hand-edit `run/cache.json` or `run/state/*.json`. The tool writes them. A
  hand-written `verified_at` already landed 2 and 8.5 hours in the *future*
  once in this campaign and silently disabled an invalidation rule.
- **Do not touch** `packages/**`; `campaigns/**` only in §9 of this file and by
  running the tool.

## 6. Acceptance {#acceptance}

```bash
cargo fmt --all
cargo test -p progress-core
bash tools/self-check.sh ; echo "EXIT=$?"
```

Read the floor's **real** exit code.

Then, with `--no-cache` on the first verification run after the change:

```bash
cargo run -q -p vibe-cli --bin vibe -- progress check --no-cache --campaign campaigns/packages-2026-09
```

- `check` → clean, 264 files, 0 warnings;
- after a `seal`, the sealed file's record carries a **non-empty
  `processed_hash`** — show the before/after JSON for one file in §9;
- `campaign.json` no longer carries `counters`, and `cache.json` no longer
  carries a per-file `summary`;
- a cache file saved **before** your change still loads without error — keep a
  copy and prove it, do not reason about it.

New tests: one asserting `seal` writes `processed_hash`; one asserting a
hand-sealed and a batch-verified file agree on it for identical content; one
asserting a legacy cache carrying `summary`/`counters` loads and ignores them.

Discipline: `cargo fmt --all`, clippy clean, **two commits**, **no AI
attribution anywhere**.

## 7. Analogies {#analogies}

Whatever the verify batch already calls to compute `processed_hash` is the
function `seal` must call. The bug this task is most likely to introduce is a
second hash implementation that agrees today and diverges later.

## 8. Stop rule {#stop}

- If a consumer genuinely needs `counters` present in `campaign.json` (a
  dashboard, a report, an external reader): **STOP and report** with file:line.
  Removing a field something reads is a design question.
- If `processed_hash` turns out to be computed from something a hand seal
  cannot reach: **STOP** — that changes F-075's answer from (d) to (b) or (c),
  and that is the owner's call, not yours.
- **Budget signal:** past **8 files / 250 lines**, stop and return.

## 9. Log {#log}

*(appended by executor / reviewer)*
