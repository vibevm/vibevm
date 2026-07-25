# WAL — Project Continuation State

_Updated: 2026-07-25 (session end — Phase L closed; Phase C in flight:
4 of 5 clusters verified, spec/common closed 12/12; modules cluster
handed to the next session)_

## Current phase

**Progress Control (PROP-043) — Phase L CLOSED; Phase C (verification)
IN FLIGHT.** The verify loop is proven and its conventions are durable
in the plan LOG §9 (cache campaign maps `{verify_batch, processed_hash,
verdicts{anchor→{v,ev[]}}, summary}`; verdicts never in markup; stage
semantics: impl/done ⇒ presence, spec/done ⇒ absence, doc/done ⇒
no-contradiction; assert-gated coverage; `_elements` for status tags).
**Tally: 1 398 / 4 944 units — 1 353 confirmed / 42 drift / 3
unverifiable; findings 44** (F-035…F-044 new). Verified: boot (c0),
manual-tests (c1), design (c2), common 12/12 (c3a–c3d) = 23 of 58
files. Drift profile settled: "the spec aged behind its own success" —
fired triggers (UPL relicense → F-043; the §2.6 engine split → F-044),
proposed-era headers over shipped systems (F-005/F-006/F-013), the MT
keymap era (F-037/F-038), one code-side stale clap help (F-036, Phase
E DRIFT candidate). Honest design proposals (031/032/033) pass clean —
every absence claim grep-verified. **Remaining: the modules cluster
(35 files, 3 300 markers)** — batch map + loop recipe in
`CONTINUE.md`. Phase L record: 26 gate-binding sites repointed in 4
batches; 35 files moved to `legacy-spec/` (plan carve-out honoured);
specmap regenerated (`f311f429`). Key laws unchanged: no fractality
(Fable = verification, Opus = DRIFT coding), engine pin
`claude-opus-5`. Plan: `spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`.

## Constraints — do not violate

- **mtime unit in the vvm manifest.** TS port stores `mtime_ms`; Rust
  twin stores `mtime_nanos` (PROP-019 §2.15) — account for the unit
  difference when reading both.
- **electron-packager temp cache.** Concurrent `<product> self install`
  runs race on the shared tmpdir template rename — run sequentially.
- **CI-off gate split.** `CI` / `VIBE_NO_DEFAULT_REGISTRY` suppresses
  vibe-embedded but NOT project-local (PROP-030 §5 + §3.3).
- **conform R-001 gate.** `crates/vibe-cli/src/registry.rs` is the only
  sanctioned constructor site for embedded/local-composite providers.
- **Boot pair marking.** `spec/boot/00-core.md` / `90-user.md` are
  user-owned: mark ADDITIVELY only; never re-form their prose.
- **legacy-spec/ is an archive.** Nothing in the living corpus or
  crates may cite into it as a normative source — archive-provenance
  pointers only; the campaign plan in `spec/terraforms/` is the one
  live file still inside a legacy-named path (owner carve-out).
- **Cache campaign maps are load-bearing.** `run/cache.json` carries
  the C-phase verdicts; mutate it by load-and-merge only (scan
  preserves the maps; a from-scratch rewrite would erase them).

## Done (collapsed — see `git log`)

- **Phase C, clusters 1–4 (2026-07-25).** c0 boot 64u (F-035
  tests/-path drift); c1 manual-tests 67u (MT keymap era F-037/F-038 +
  clap-help F-036); c2 design 306u (aged-tense family F-039/040/041 +
  F-034); c3a common-small 150u (family roster F-042); c3b PROP-000
  162u (six aged spots F-043); c3c 018+019 322u (headers F-013/F-005);
  c3d common tail 327u (deferral-fired F-044; spec/common closed).
- **Phase L (full, 2026-07-25).** Inventory → no-port verdict → 4
  repoint batches → `git mv` 35 files → `legacy-spec/`; exit gate
  green; specmap regen.
- **Phase B (full).** 58 files, 4 880/4 880 facts marked; grammar
  precedents complete. DRIFT-001…005 5/5 no-return.
- Earlier: default registry migration; PROP-030 §3.3; vibevm-term
  Phase 2b; M1.17/M1.18/M1.19.

## In progress

Phase C mid-flight at a clean batch boundary — journal closed (last
step `c3d-common-tail` done), no step open. The next batch is `c4a`
(vibe-progress family) per the `CONTINUE.md` batch map.

## Next

1. **Phase C, modules cluster** — batches c4a…c4f per `CONTINUE.md`
   (35 files, 3 300 markers; specmap join gives 626 edges — PROP-002
   alone carries 110). Loop, conventions, and pitfalls are recorded
   there and in LOG §9. Exit gate: 100 % markers carry verdicts;
   X/Y/Z in the LOG; **ask the owner before opening Phase D**.
2. Then Phase D (stitching) over the 44-row ledger — the F-024…F-044
   families are sweep-shaped (one fix per family).
3. Opus queue EMPTY; DRIFT candidates: F-036 (stale clap help on
   `--plain`), the stale-header sweep, F-016 modules README, F-020
   OWNER-GUIDE, F-017 aiui scrollbar, engine-family minting.

Parked follow-ups (unchanged): vibe-vvm/term-vvm conformance-golden;
Linux/macOS install smoke; arbitrary user-repos design-doc; `vibe
doctor` project-local row.

## Known issues

- **GitVerse SSH link DOWN (2026-07-25).** Banner-exchange timeout —
  network-level, not divergence (HTTPS `ls-remote`: strict ancestor).
  GitHub carries everything through `9baa7fa6`. Recovery: plain
  `cargo xtask mirror`; NEVER `--force`.
- **vibespecs 401 on this machine** — redbook + rust-ai-native resolve
  via vibe-embedded; consuming lockfiles carry
  `source_kind = "embedded"` and trip the reproducibility guard.
- **specmap ratchet** — 37 gated orphans host-side (re-based at the
  2026-07-25 regen `f311f429`; gate passes, 0 suspects).

## Session context

One session executed Phase L wholesale and drove Phase C through four
of five clusters. The session boundary is a clean batch boundary: the
journal is closed, all verdicts committed, GitHub synced. The next
session opens with the phrase in `CONTINUE.md` §"How to start" and
continues at c4a; the phase lane already reads C (no new phase event).
`progress check` must stay 0; the cache's campaign maps must survive
every write (load-and-merge only).
