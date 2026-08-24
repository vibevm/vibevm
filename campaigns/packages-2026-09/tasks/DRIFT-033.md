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
> — campaign amendment A4, spec://org.vibevm.core/vibevm/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1#amendments
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
covers every verdict in a file — `vibevm/vibespecs/boot/00-core.xml` has **33** verdicts
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

### Executor, 2026-07-26 — one finding was already closed, the other splits

**F-075 needed no code. It shipped with DRIFT-026.** §3 says `processed_hash`
"is written **only** by a real verify batch, never by a hand `seal`". That was
true when F-067 was written and stopped being true at `e9fc7b44`
(*feat(progress): sealing a verdict stops depending on memory*). Today
`progress-core/src/seal.rs:154-157` writes it on every `Seal::Recorded`, and the
number it writes is `doc.content_hash` — produced by
`progress-core/src/parse/mod.rs:58` (`content_hash`, sha256 of the bytes), from
the parse the CLI adapter performs on the file it just read
(`vibe-cli/src/commands/progress/seal.rs:114`). There is **one** hash
implementation in the crate and `seal` already calls it; the spec says so too
(`##CMD-SEAL`, PROP-043 §5). Measured, not read:

```
$ vibe progress seal --campaign <scratch copy> vibevm/vibespecs/boot/00-core.xml
progress seal: `vibevm/vibespecs/boot/00-core.xml` — vouching for 33 verdict(s) against the text on disk
  78d18746e702 → 27697df5871b
  sealed at 2026-07-26T14:45:23Z

# before                                    # after
processed_hash 78d18746e702bfbb…6d3a312f    processed_hash 27697df5871b…fee9c511
verified_at    2026-07-26T10:23:56Z         verified_at    2026-07-26T14:45:23Z

$ sha256sum vibevm/vibespecs/boot/00-core.xml
27697df5871b7e4831d1ae9db525ff6a93c0b124fcc90afa3d02047dfee9c511
```

The seal recorded the file's own sha256 — the digest a batch records. Run
against a **copy** of the campaign zone, not the live one: `vibevm/vibespecs/boot/00-core.xml`
has moved since it was judged, so sealing it here would stamp a fresh
`verified_at` on verdicts nobody re-derived, which is the forgery §5 exists to
prevent. What F-075 lacked was a test, and that is commit 1: one asserting the
field is written and non-empty, one asserting a hand-sealed and a batch-verified
file agree on it *and* that the value is `parse::content_hash` by name, so a
second implementation growing in either path breaks the build.

**F-077 splits.** Step 4 and step 6 landed (commit 2). Step 5 is a §8 **STOP**.

The per-file `summary` is read by nothing — not the crate, not `vibe-cli`, not
the dashboard, not `spec/**`, not this campaign's own documents; and §7.1's
`##CACHE-RECORD` lists the campaign fields without it. So it is deleted from the
record and computed by `FileRecord::campaign_view` on every projection write.
`corpus.json` still carries the key, with the same numbers, derived rather than
remembered.

`counters` is not the same case, on two independent grounds, both in `spec/**`:

- `vibevm/vibespecs/modules/vibe-progress/PROP-043-progress-markup.xml:478` —
  `##STATE-FILES` names `counters` as a field **of `campaign.json`**, `@impl/done`.
- `vibevm/vibespecs/modules/vibe-progress/PROP-043-progress-markup.xml:482-483` —
  `##DASHBOARD-READS-ONLY`: "The dashboard reads **only** these; **it computes
  nothing**". So "compute on read" is closed to the one consumer that reads it:
  `tools/progress-dashboard/index.html:106` (`const c = camp.counters ?? {};`),
  rendering files/facts/unmarked/issues at `:107-110`.

The numbers *are* derivable from `corpus.json`, which that page already fetches
in the same `Promise.all` (`:96-98`) — so this is decidable, but it is decidable
by whoever may edit `##DASHBOARD-READS-ONLY`, and §5 says that is not the
executor. One consequence for whoever rules: without `counters`, `campaign.json`
stops moving when the corpus moves, and
`vibe-cli/src/commands/progress/tests/writes.rs:132-157` asserts that it does.

**Two §3 claims re-measured, both hold.** All 58 per-file summaries agree with a
recount over their own `verdicts` map (0 disagreements); the 4 498 verdict
entries are uniformly `{"v", "ev"}` with `v ∈ {confirmed, unverifiable}`.

**A pre-change cache loads — proven, not reasoned.** A byte-for-byte copy of
`891f59df`'s `run/cache.json` (sha256 `4f21b791915ce8f5…c9e59586`) run through
the new binary: `progress check: clean (264 files, 0 warning(s))`, exit 0; 4 498
verdicts, 58 `processed_hash`, 58 `verified_at`, 58 `verify_batch` all still
there; 58 `summary` gone; and the 58 computed summaries in `corpus.json`
**identical** to the 58 the old file stored. The live `run/cache.json` in commit
2 is that same rewrite: 1 insertion, 177 deletions, every one of them a `summary`
line or the cache's own `updated_at`.

**Acceptance.** `cargo fmt --all` → 0. `cargo test -p progress-core` → 95 + 14 +
10 pass, 0 fail. `bash tools/self-check.sh` → `self-check: all green`, **EXIT=0**.
`vibe progress check --no-cache --campaign campaigns/packages-2026-09` →
`progress check: clean (264 files, 0 warning(s))`, exit 0. Not met, by stop rule:
"`campaign.json` no longer carries `counters`".

`cache.rs` crossed the 600-line budget on the way and the floor caught it
(`conform: NEW file-length … 817 lines`); its tests moved to a file-backed
submodule, the split `baseline/project` already carries.

Also noticed, not changed: `campaigns/packages-2026-09/deferrals.md:29` still
states F-067 in the present tense, and `e9fc7b44` closed it.
