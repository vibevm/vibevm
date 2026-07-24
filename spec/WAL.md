# WAL — Project Continuation State

_Updated: 2026-07-25 (Phase B CLOSED mid-session — corpus fully marked,
exit gate green; awaiting the owner's call to open Phase L)_

## Current phase

**Progress Control (PROP-043) — Phase B CLOSED; Phase L awaits the
owner's opening call.** The markup pass is complete: **58 files,
4 880/4 880 facts marked, `progress check --exhaustive` clean (0
errors), floor `self-check` all green (real exit 0)** — the §4
boundary entry is in the plan LOG §9 with the commit map and the §8
predictions check. Two owner rulings landed in session: **(1) Phase L
(legacy relocation)** inserted between B and C — inventory every
reference into `spec/terraforms|research|neworder|discipline` from the
living corpus AND code (specmark, doctests), port the referenced facts
into `common/design/modules/manual-tests` (new specs allowed), mark
them, then `git mv` the four dirs to root `legacy-spec/`; ordering law:
before Phase C so verification covers the ported facts; **(2)
spec/discipline left the markup corpus** (the Discipline lives in the
ai-native packages now) — scope went 59 → 58 files. B2 tail landed as
batches 20–26: PROP-003 (superseded-arc split #3), PROP-002 (modules
35/35), design cluster 6/6 (README re-mark burned the last 12 expected
errors — the gate has read 0 since `cb6e55b0`), boot pair
(additive-only), manual-tests ×3 (genre precedent: steps + Expected
paragraphs are separate units). Ledger: **35 findings** (F-030…F-034
new). Opus/DRIFT queue EMPTY. Key laws unchanged: fact-exhaustive
granularity, anchored-when-marked, two registers, verdicts never in
markup, **no fractality** (Fable = markup, Opus = DRIFT coding), engine
pin `claude-opus-5`. Plan:
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`. Open review
point for the owner: the plan file itself lives in `spec/terraforms/` —
relocate mid-campaign or at close-out (before L5).

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
  or deconstruct their prose. (Their B2 markup honoured this —
  line-for-line in-place edits only.)

## Done (collapsed — see `git log`)

- **Phase B (full).** B0 conversion; B1/B1f common at fact grain; B2
  batches 1–26 — modules 35/35, design 6/6, boot pair, manual-tests;
  two scope narrowings (94→59→58); the campaign contract PROP-043
  marks itself; grammar precedents complete (tables per cell, checkbox
  anchors, blockquote re-forms per F-015, REQ/design register lines,
  superseded arcs spec/done vs impl/done, `@impl/plan` phases,
  manual-test step/Expected units, additive-only user-owned files).
- DRIFT-001…005 through the loop 5/5 no-return; fact-links commission
  complete (PROP-014 §2.1, PROP-035 §5/§7.3, core v0.8.0, vibe-spec).
- Earlier: default registry migration; PROP-030 §3.3 project-packages;
  vibevm-term Phase 2b; M1.17/M1.18/M1.19 stacks.

## In progress

Nothing mid-flight — every journal step closed. Phase B is closed;
the next phase (L) opens only on the owner's word.

## Next

1. **Owner call: open Phase L (legacy relocation)** — the §5 Phase L
   section in the plan carries the owner-ordered steps L1–L5
   (reference inventory incl. specmark/doctests → fact discovery →
   relocation into the living corpus, new specs allowed → fact-grain
   markup + scope growth → `git mv` to `legacy-spec/`). Exit before
   Phase C opens. Resolve in passing: does the campaign plan itself
   move with `spec/terraforms/`, or at close-out?
2. **Then Phase C (verification)** per the plan §5 — every marker gets
   an evidence-backed verdict; the F-024…F-034 stale-header family is
   prime drift material for C/D.
3. Opus queue EMPTY; candidate DRIFT material unchanged: the
   stale-header sweep (C/D), F-016 modules README, F-020 OWNER-GUIDE,
   F-017 aiui scrollbar, engine-family minting.

Parked follow-ups (unchanged): vibe-vvm/term-vvm conformance-golden;
Linux/macOS install smoke; arbitrary user-repos design-doc; `vibe
doctor` project-local row.

## Known issues

- **GitVerse SSH link DOWN (2026-07-25).** Banner-exchange timeout —
  network-level, not divergence: HTTPS `ls-remote` shows a strict
  ancestor, no foreign commits. GitHub carries everything. Recovery:
  plain `cargo xtask mirror` when SSH returns; NEVER `--force`. Bound
  attempts with `GIT_SSH_COMMAND="ssh -o ConnectTimeout=15 -o
  BatchMode=yes"`.
- **vibespecs 401 on this machine** — redbook + rust-ai-native resolve
  via vibe-embedded; consuming lockfiles carry
  `source_kind = "embedded"` and trip the reproducibility guard.
- **specmap ratchet** — 34 gated orphans in `vibe-spec` (pre-existing).

## Session context

Phase B closed mid-session with the boundary entry in the plan LOG §9;
the session continues awaiting the owner's Phase L call. If this
session ends here, the next one boots per `CLAUDE.md`, reads this WAL +
the plan LOG §9 tail, and reports — Phase L does not open without the
owner.
