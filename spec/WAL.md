# WAL — Project Continuation State

_Updated: 2026-07-25 (mid-session checkpoint — Phase C modules cluster:
12 of 35 module files verified in five batches c4a…c4d2; workspace
cluster closed; cli/actions (c4e) and the mcp/index/settings/registry
tail (c4f) remain)_

## Current phase

**Progress Control (PROP-043) — Phase C (verification) IN FLIGHT, the
modules cluster past half.** Conventions stay per LOG §9 (verdicts in
cache campaign maps only; impl/done ⇒ presence, spec/done ⇒ absence,
doc/done ⇒ no-contradiction; coverage from `progress mirror`'s
ParsedDoc — the raw-grep extractor over-counts code-span shorthands;
assert-gated; `_elements` for document markers). **Tally: 3 233 /
4 944 — 3 062 confirmed / 168 drift / 3 unverifiable; findings 51**
(F-045…F-051 new this session; F-018/F-024/F-047/F-048 extended).
Verified: boot, manual-tests, design, common 12/12, vibe-progress
(c4a), registry core 002+001 (c4b), resolver 003+017 (c4c), workspace
10/10 (c4d1+c4d2) = 35 of 58 files. Session findings of note: F-046
(PROP-043's own §5–§7 marker-vs-implementation parity, both
directions), F-047 (stale in-code deviates denying the shipped
ResolvoDepSolver — production default `unwrap_or("resolvo")`), F-048
(--trust-mirror and `list --overrides` promised-absent; SOLVER-IDENTITY
corrected from a wrong c4b confirm), F-050 (PROP-003's solver tail
outside its §2.2 supersede marker), F-018-extended (PROP-020/022:
whole shipped systems — hooks.rs runner, Materialization modes — under
proposed headers; 78 drift rows, one re-mark sweep each). First
zero-drift files: PROP-011/012/025 (227/227). **Remaining: c4e
(vibe-cli+vibe-actions: 037/036/042/039 — 327 markers) and c4f
(vibe-mcp ×3, index 005, settings ×2, registry rest — 957)** per the
`CONTINUE.md` batch map. Exit gate: 100 % markers carry verdicts;
ask the owner before Phase D. Key laws unchanged: no fractality
(Fable = verification, Opus = DRIFT coding, queue EMPTY), engine pin
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
