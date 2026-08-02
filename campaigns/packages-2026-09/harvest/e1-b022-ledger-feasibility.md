# B-022 · LEDGER-INTENT's five cache mechanisms — the feasibility study {#root}

**Date:** 2026-08-03 · **HEAD at synthesis:** `b6e4eb2d` (the evidence commit) ·
**Owner directive:** `BACKLOG.md` B-022 — «давай положим в бэклог исследование»
(2026-08-01), modelled on B-012.

**What this document is.** The boss synthesis over one evidence pass —
[`e1-b022-evidence.md`](e1-b022-evidence.md) (worker-gathered, evidence-only,
boss-reviewed: spot-checks re-run against the tree at `BACKLOG.md:559`,
`BACKLOG.md:647`, `ledger.rs:132-136`, `sync-engines.toml`). The **verdicts and
recommendations below are the boss's**, marked as proposals: nothing is
scheduled until the owner rules. Genre: campaign harvest, non-binding.

**The one-line answer to «можно ли реализовать»:** yes for all five — nothing
is architecturally impossible — but only **one slice is unblocked today**
(M-E's enum, plus the structural half of M-A it drags in), and the other four
sit behind exactly **two keys**: the LLM producer that does not exist (B-020,
`vibe-llm` is a 9-line stub), and the parked security programme (B-015 —
owner's notice only). The registry's five F-159 anchors stay `deferred` until
the owner rules on this study.

## Verdict table {#verdicts}

| # | mechanism | feasible? | effort | blocked on | recommendation (proposal) |
|---|---|---|---|---|---|
| M-A | entry carries provenance fields | yes | S struct / M with key change | half: nothing; half: B-020 (no producer has `model_id`/`prompt_rev`/`cost`) | **build layer 1 with M-E** (entry wrapper `{producer, epoch, inputs-hash, created_at}` + structured key); **layer 2 annotated → B-020** |
| M-B | GC: LRU + pin set + size budget | yes | M (needs index + entry metadata) | M-A layer 1 (metadata), M-D (pin source), and *in practice* B-020 — the deterministic producer creates no eviction pressure (recompute ≈ 0; live store: 1 object) | **do not build now**; annotate in place naming B-020 as the pressure trigger and M-D as the pin source |
| M-C | telemetry: four measures | yes | S fields / gated data | cost halves on B-020; per-kind on M-E + a second kind; rot-rate on a draft-input path no producer has | **build nothing new now**; the two live measures stand; annotate the two absent ones → B-020; fix the false host line `terraform/REPORT.md:41` («cost field is plumbed» — no such field) |
| M-D | release slice: exported, signed, shipped | yes | L, cross-crate | **B-015, parked by owner's word**; the spec itself forbids unsigned exposure (`LEDGER-INTENT-v0.1.md:87`), so no unsigned interim exists | **no work until the owner's B-015 notice**; annotation names B-015 explicitly |
| M-E | closed query-kind enum, reviewed key schemas | yes | S | nothing («дёшевый и независимый» — the entry's own coupling note) | **build in Phase E** — the one unblocked slice; carries M-A layer 1; prepares M-C's per-kind split |

## The two keys that gate everything else {#keys}

1. **B-020 — the external-LLM client** (`BACKLOG.md` B-020, `planned`,
   owner 2026-08-01). Until a second producer exists, provenance fields
   beyond `created_at` have no writer (M-A layer 2), cost metrics have no
   cost (M-C), and the store has no growth to evict (M-B). The evidence's
   sharpest structural fact: the one shipped producer is a deterministic
   template whose recompute is ≈ free, so the cache's economic half is
   dormant *by design* until an expensive producer lands.
2. **B-015 — the parked security programme** (owner's parking is explicit:
   «НЕ строить до его специального уведомления», `BACKLOG.md:559`). M-D is
   a strict subset: the spec's own §7 forbids unsigned remote exposure, so
   there is no honest unsigned interim — the mechanism waits whole.

## The Phase-E-ready slice, precisely {#phase-e-ready}

**One build: `QueryKind` enum + structured key + minimal entry wrapper**, all
inside `core-ai-native-specmap/src/ledger.rs` (303 lines today):

- the enum replaces the in-function string `const PRODUCER` (`ledger.rs:132`);
- the cache key becomes a structured tuple (kind, producer, epoch,
  subject-hash) rather than one opaque sha256 (`ledger.rs:136`) — which also
  creates the «invalidate a bad producer by predicate» capability §8's
  cache-poisoning row promises and today lacks;
- the stored object gains a thin serialised wrapper
  `{producer, epoch, inputs-hash, created_at, body}` — the four fields a
  deterministic producer *can* populate; the LLM-only fields wait for B-020.

**Cost note the acceptance must carry:** `ledger.rs` is vendored into six
consumer packages (`sync-engines.toml`; the evidence enumerates all six), so
this edit is `cargo xtask sync-engines` in the same pass **and a release event
for six packages — owner sign-off before publication, per the Phase E mandate.**

## Owner decision points, exactly three {#owner-points}

1. **Approve/decline the M-E(+M-A-layer-1) build** as Phase E work (the only
   scheduling decision this study asks for now).
2. **The annotation set** for M-B, M-C's absent halves, M-A layer 2, M-D —
   each an in-place «Specified, not built (→ B-nnn)» naming its key
   (BUILD-FIRST form; the ready texts follow the ruling, applied
   verdict-first: re-judge F-159's anchors → registry mints → edit → re-seal).
3. **The host prose fix** — `terraform/REPORT.md:41`'s false «the ledger's
   cost field is plumbed and zero-valued» (no cost field exists,
   `ledger.rs:82-88`): a one-line host-side correction, no package involved.

## Method and honesty {#method}

The evidence half is the worker's (`e1-b022-evidence.md`, run archived at
`cache/agents/sorted/E1-B022-SWEEP/`); every verdict, effort class and
recommendation here is the boss's judgment over it, per the campaign's
delegation law (the decision is never delegated). Figures are properties of
HEAD `779b3aaa`/`b6e4eb2d`; anyone re-running takes HEAD's own measurements.
The five F-159 registry anchors (all `@impl/done` over unbuilt mechanisms)
re-judge only after the owner's ruling, through the standing seal path
(mirror → merge-verdicts → seal, never chained).
