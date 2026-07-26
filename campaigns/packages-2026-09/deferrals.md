# Deferrals — campaign `packages-2026-09`

Open tails at close-out land here: obligations ruled deferred, unexecuted
tasks, postponed doc chapters. A tail earns a line **only** if someone decided
to leave it — anything still being worked belongs in `tasks/INDEX.md`.

## Inherited from wave 1 at ratification {#inherited}

Wave 1 closed out on 2026-07-26 and handed this campaign work rather than
merely a method. These are not this campaign's own tails yet — they are its
**inbox**, and they should move out of this section into phases as they are
picked up. The authoritative statement of each is
[wave 1's deferrals](../progress-2026-08/deferrals.md).

- **The judgment-marking pass** — *wave 1's* Phase F (amendment A3.i). **Not this
  campaign's Phase F, which is the credibility report, and not Phase T.** The
  collision of labels is why this line now names the work instead of a letter. Wave 1 marked what
  4 917 facts *are* and was never asked what should *happen* to them, so every
  forward-looking view came out empty. This campaign marks judgment as it marks
  state, in one sweep over both corpora, so the three owner plans have an input.
- **The harvest pass and the two doc trees** — *wave 1's* Phase G (A3.ii). The User Guide
  and the Package Author Guide, the latter documenting the `packages/` corpus
  that is this campaign's own subject.
- **`FACT-GRAIN-EVIDENCE`** — wave 1's single surviving drift row, which no
  work in the host repository could close. It closes at **Phase A step 2**,
  when `rust-ai-native-lang` is re-minted at v0.8.0 with the fact-aware specmap
  engine — **now deferred by owner ruling 2026-07-26** («не перевыпускай
  пакет, сделаем это потом»).

- **F-067 — CLOSED 2026-07-26 by `e9fc7b44`, and amendment A4 is discharged
  with it.** *(Was: `processed_hash` is written only by a real verify batch, so
  a campaign that hand-seals leaves it pointing at superseded text and the
  staleness warning names the freshest files in the corpus.)* `vibe progress
  seal` has written `processed_hash` since DRIFT-026 landed — it is the file's
  own sha256, from the single `content_hash` the parser already computes, so a
  hand seal and a verify batch record the same digest. Verified by running a
  seal on a scratch copy and comparing to `sha256sum`. **F-075 asked for exactly
  this and needed no code**; DRIFT-033 added the test that was missing.
  *This line stood in the present tense for a day after the fix shipped, and two
  task files inherited the stale claim from it — which is the case for
  re-measuring a ledger entry before quoting it, not merely re-reading it.*
- **Two files need re-verifying first**: `MT-02-vibe-tree-tui.md` and
  `PROP-026-tcg-tool-family.md` carry wave-1 verdicts formed against text
  Phase D changed afterwards.

## The engine re-mint, deferred — and what it blocks {#engine}

**Phase C cannot open until this is done.** Its evidence join needs fact
anchors and the engine the host consumes cannot see them; Phase A's exit gate
is partially unreachable for the same reason (the `specmap.json` clause).
Phase B markup is *not* blocked and proceeds.

Confirmed before deferring, so nobody re-derives it: `is_valid_fact_id` exists
**only** in `core-ai-native/v0.8.0`; `vibe.lock` pins `core-ai-native@=0.7.0`
and `rust-ai-native-lang@=0.7.0`; `cargo xtask sync-engines --check` is green
across 33 pairs in 6 sync sets — **nothing has drifted, the gap is a version**.

Three things must be settled when it is taken up, all of them the owner's:

1. **Publishing is a Rule 4 red line** and stops for him regardless.
2. **The host resolves these packages from a second, stale working copy** —
   `file:///C:/Users/olegc/gits/vibevm/…`, last commit `c112f6f`, weeks behind
   this one. A re-mint in *this* copy is invisible to the host until the
   resolve is repointed or that copy is synced. This is an environmental fact
   about the machine, not about the repository.
3. **The network registries 401 here**, so publishing may be impossible even
   if authorised.

*Most likely resolution, recorded so it is not re-reasoned: repoint the resolve
at this copy's `packages/` and bump the lockfile locally. That closes the gap
with no publication at all — publishing is only needed for external consumers.*
