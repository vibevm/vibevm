# WAL — Project Continuation State

_Updated: 2026-07-25 (session end, part 2 — scope narrowed by owner
ruling; PROP-005 marked; 14 files / ~1 280 facts left in Phase B)_

## Current phase

**Progress Control (PROP-043) — Phase B at FACT grain, closing.** The
owner ruled `spec/terraforms`, `spec/research`, `spec/neworder` **out of
scope** («рефакторинги и исследования давным-давно» — verbatim in the
plan LOG §9; `progress.toml` narrowed, commit `8901cd05`): scope is now
**59 files / 4 894 facts**, expected `check` residue **12** (the
design/README pilot — burns at its re-mark). B2 modules stand at
**33/35** after PROP-005 (273→278 facts, batch 19 `e9d330f8`) and the
late PROP-030 commit (`ff4ca088`); earlier the same day batches 8–18
swept 18 modules files. **Remaining: PROP-003 (310) → PROP-002 (359) →
design ×6 (README re-mark burns the 12 errors; structural-loader,
loading-and-boot-model, action-system, workspace-and-qualified-naming,
tui-visual-language) → authored boot pair (additive-only — user-owned)
→ manual-tests ×3 → discipline/README.** Then the Phase B exit:
`check --exhaustive` green over 59 files + floor + the §4 boundary
ritual in the plan LOG; Phase C opening is the owner's call. Ledger: 29
findings (the F-024…F-029 stale-header family + F-023 — one C/D sweep
fixes all). Opus/DRIFT queue EMPTY. Journal fully closed (b2-prop-003
opened-and-closed NOT STARTED). Key laws unchanged: fact-exhaustive
granularity, anchored-when-marked, two registers, verdicts never in
markup, **no fractality** (Fable = markup, Opus = DRIFT coding), engine
pin `claude-opus-5`. Plan:
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`. The per-file
loop, marking conventions, and trap list live in `CONTINUE.md`.

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
  user-owned: mark ADDITIVELY only (anchors + markers), never re-form
  or deconstruct their prose.

## Done (collapsed — see `git log`)

- **B2 batches 1–19 + scope ruling** — 33/35 modules files fact-marked;
  scope 94→59; the campaign contract PROP-043 marks itself; grammar
  precedents complete (tables, checkboxes, blockquote re-forms, REQ
  grains, superseded arcs, `@impl/plan`).
- DRIFT-001…005 through the loop 5/5 no-return; fact-links commission
  complete (PROP-014 §2.1, PROP-035 §5/§7.3, core v0.8.0, vibe-spec).
- Earlier: default registry migration; PROP-030 §3.3 project-packages;
  vibevm-term Phase 2b; M1.17/M1.18/M1.19 stacks.

## In progress

Nothing mid-flight — every journal step closed. The B2 tail (14 files)
is queued, not started; PROP-003 is the first file of the next session.

## Next

1. **Finish Phase B (14 files, ~1 280 facts)** in the order recorded in
   `CONTINUE.md` §"How to start the next session": PROP-003 (§2.2
   carries a SUPERSEDED-by-PROP-017 blockquote — F-015 re-form;
   superseded libsolv sections spec/done per the PROP-001 arc pattern)
   → PROP-002 → design cluster (README re-mark first burns the last 12
   expected errors) → boot pair (additive-only) → manual-tests →
   discipline/README.
2. **Phase B exit gate:** `progress check --exhaustive` green over the
   59-file scope + `self-check` with the REAL exit code + the §4
   boundary entry in the plan LOG (commit map, predictions per §5) +
   plan status-line refresh. **Ask the owner before opening Phase C.**
3. Opus queue EMPTY; candidate DRIFT material: the F-023…F-029
   stale-header sweep (Phase C/D), F-016 modules README, F-020
   OWNER-GUIDE, F-017 aiui scrollbar, engine-family minting.

Parked follow-ups (unchanged): vibe-vvm/term-vvm conformance-golden;
Linux/macOS install smoke; arbitrary user-repos design-doc; `vibe
doctor` project-local row.

## Known issues

- **GitVerse SSH link DOWN (all of 2026-07-25).** Banner-exchange
  timeout — network-level, not divergence: HTTPS `ls-remote` shows a
  strict ancestor, no foreign commits. GitHub carries everything.
  Recovery: plain `cargo xtask mirror` when SSH returns; NEVER
  `--force`. Bound the attempts with
  `GIT_SSH_COMMAND="ssh -o ConnectTimeout=15 -o BatchMode=yes"` — the
  fan-out collects per-target failures and still pushes github.
- **vibespecs 401 on this machine** — redbook + rust-ai-native resolve
  via vibe-embedded; consuming lockfiles carry
  `source_kind = "embedded"` and trip the reproducibility guard.
- **specmap ratchet** — 34 gated orphans in `vibe-spec` (pre-existing).

## Session context

Next session opens with the resume phrase recorded in `CONTINUE.md`
§"How to start the next session", boots per `CLAUDE.md`, reads
`campaigns/progress-2026-08/run/RESUME.md` + the plan LOG §9, then
re-opens `b2-prop-003` and works the per-file loop. `progress check`
must stay at 12 expected errors until the design/README re-mark, then 0.
