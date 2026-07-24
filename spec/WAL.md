# WAL — Project Continuation State

_Updated: 2026-07-25 (session end — the B2 modules sweep: 18 files in
batches 8–18, B2 at 32/35)_

## Current phase

**Progress Control (PROP-043) — Phase B at FACT grain.** `spec/common` is
DONE (1 009 facts); **B2 stands at 32/35 modules files** after this
session's sweep: PROP-015/034 (batch 8), 027/036 (9), 011/012 (10),
010/040 (11), 038 (12), 008/001 (13), 009/017 (14), **043 — the campaign
contract marks itself** (15), 035 (16), 037 (17), 007 (18); commits
`b27336ae`…`1e7dff01`. ~1 540 units → ~1 770 anchored facts; corpus now
94 files, **5 225/8 589 facts unmarked**; every marked file reads 0
unmarked / 0 issues; `check` carries only the **40 expected pilot-file
errors** (SHRINK-PLAN 28, design/README 12). Ledger grew to **29
findings**: F-023 (PROP-036 §2.13 cites PROP-043 as the GUI-launcher
home — dangling ref), and the **stale-header family** F-024 (PROP-038
§2.7 vs §5), F-025 (PROP-008), F-026 (PROP-009), F-027 (PROP-017),
F-028 (PROP-035), F-029 (PROP-007 vs PROP-009 shipping) — one Phase C/D
sweep can fix all seven. Grammar precedents set: the
Decision-paragraph idiom (lead sentence + deconstructed items),
`##req-*`/`##design-*` for req-lines, `##self-uri`, checkbox anchors
before `[x]`, `@impl/plan` for unexecuted phase plans, superseded-arc
spec/done-vs-impl/done, em-dash cells count as units. The Opus/DRIFT
queue is EMPTY (5/5 no-return). Key laws unchanged: fact-exhaustive
granularity, anchored-when-marked, two registers, verdicts never in
markup, **no fractality for this campaign**, engine pin `claude-opus-5`.
Plan: `spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`.

## Constraints — do not violate

- **mtime unit in the vvm manifest.** TS port stores `mtime_ms`; Rust twin
  stores `mtime_nanos`. Equal-on-equal-API (PROP-019 §2.15); a tool
  reading both MUST account for the unit difference.
- **electron-packager temp cache.** Concurrent `<product> self install`
  runs race on the shared tmpdir template rename — run sequentially.
- **CI-off gate split.** `CI` / `VIBE_NO_DEFAULT_REGISTRY` suppresses
  vibe-embedded but NOT project-local (PROP-030 §5 + §3.3).
- **conform R-001 gate.** `crates/vibe-cli/src/registry.rs` is the only
  sanctioned constructor site for embedded/local-composite providers.

## Done (collapsed — see `git log`)

- **B2 batches 1–18** — 32/35 modules files fact-marked; PROP-043 §3.8/3.9
  grammar exercised end to end (tables, checkboxes, blockquote re-forms,
  REQ grains, superseded arcs).
- DRIFT-001…005 through the loop 5/5 no-return; fact-links commission
  complete (PROP-014 §2.1, PROP-035 §5/§7.3, core v0.8.0, vibe-spec).
- Earlier: default registry migration; PROP-030 §3.3 project-packages;
  vibevm-term Phase 2b; M1.18/M1.19 stacks.

## In progress

Nothing mid-flight — every journal step closed (counts written after the
verifying scan). The B2 tail is queued, not started.

## Next

1. **B2 remainder (3 files, then clusters):** PROP-005 (928 lines) →
   PROP-003 (881) → PROP-002 (736); then `spec/design` → `spec/research`
   → `spec/terraforms` incl. the two pilot re-marks (burns the 40
   expected errors). Per file: journal step-start → mark (scratchpad
   python replace-scripts, exact-match + count==1 asserts) → `check` +
   `scan`, corpus row 0/0 → step-done with post-scan counts. Batch
   commits ~1–2 files at this density; `vibe progress resume` before
   each commit; `cargo xtask mirror` at checkpoints.
2. **Opus queue EMPTY**; new DRIFT tasks spawn as `subagent_type: opus5`.
   Candidate material (owner to prioritise): the F-023…F-029
   stale-header sweep (Phase C/D), F-016 modules README, F-020
   OWNER-GUIDE, F-017 aiui scrollbar, engine-family minting for the
   fact-aware mdspec.
3. Every campaign session starts from
   `campaigns/progress-2026-08/run/RESUME.md` + the plan LOG §9;
   `CONTINUE.md` carries the cold-resume snapshot of 2026-07-25.

Parked follow-ups (unchanged): vibe-vvm/term-vvm conformance-golden;
Linux/macOS install smoke; arbitrary user-repos design-doc; `vibe doctor`
project-local row.

## Known issues

- **GitVerse SSH link DOWN (all session, 2026-07-25).** Banner-exchange
  timeout on `git@gitverse.ru` — network-level, not auth, not
  divergence. Verified via HTTPS `ls-remote` at session start: GitVerse
  `main` = `78322dac`, a **strict ancestor** of local main; tags
  identical; **no foreign commits**. GitHub carries everything through
  the session-end commits. Recovery: when SSH works again, a plain
  `cargo xtask mirror` re-fan suffices (fast-forward). NEVER `--force`.
  Trick: `GIT_SSH_COMMAND="ssh -o ConnectTimeout=15 -o BatchMode=yes"`
  bounds the gitverse attempts; the fan-out still pushes github
  (per-target failure collection, verified in `xtask/src/mirror.rs`
  `fan_out`).
- **vibespecs 401 on this machine** — redbook + rust-ai-native resolve
  via vibe-embedded here; consuming lockfiles carry
  `source_kind = "embedded"` and trip the reproducibility guard.
- **specmap ratchet** — 34 gated orphans in `vibe-spec` (pre-existing).

## Session context

Next session: open `campaigns/progress-2026-08/run/RESUME.md`, then mark
`spec/modules/vibe-index/PROP-005-package-index.md` first (the largest
remaining file — read in two passes, script in 3–4 parts). The marking
conventions and trap list live in `CONTINUE.md` §"Non-obvious findings"
and §"Resume recipe". `cargo run -q -p vibe-cli --bin vibe -- progress
check` must stay at 40 expected errors until the pilot re-marks.
