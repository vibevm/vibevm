# WAL — Project Continuation State

_Updated: 2026-07-25 (Phase L executed and closed in session; Phase C
awaits the owner's opening call)_

## Current phase

**Progress Control (PROP-043) — Phases A, B, and L CLOSED; Phase C
(verification) awaits the owner's opening call.** Phase L (legacy
relocation, owner amendment 2026-07-25) executed start-to-finish this
session: L1 reference inventory (gate-binding set: 26 sites in 13
corpus files + the `outdated.rs` doc comment; neworder/discipline had
zero corpus inbound), L2 fact discovery with the verdict **every
cited fact was already corpus-resident** — L3 ported nothing, the
owner's new-spec grant went unused, citations dissolved into
archive-provenance form; L4 repoints in four batches (`83346e78`
`f8f347d8` `9514e8fb` `1ec6a27c`, incl. both `spec://vibevm/research`
URI retirements and four word-level sites only a lookbehind/word
sweep catches); L5 relocation `70f3cbdd` — 35 files `git mv`'d to
root **`legacy-spec/`** (terraforms 25 / research 8 / neworder 1 /
discipline 1), **the campaign plan carve-out honoured** (it stays at
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md` per the owner's
reversal), live out-of-gate pointers (ROADMAP, docs/, terraform
links, findings.json, discipline.lock, progress.toml) updated in the
same commit, historical records left verbatim. `f311f429`
regenerated the stale host specmap (absorbed B-phase drift + the
move; ratchet green). Exit gate: reference greps zero into the four
dirs from corpus + crates (plan carve-out aside); `check
--exhaustive` clean (58 files, 4 880 facts, 0 errors); floor
`self-check` all green, real exit 0. Boundary entry in the plan LOG
§9. Key laws unchanged: fact-exhaustive granularity,
anchored-when-marked, two registers, verdicts never in markup, **no
fractality** (Fable = markup/porting/review, Opus = DRIFT coding),
engine pin `claude-opus-5`. Plan:
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`.

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
- **legacy-spec/ is an archive.** Nothing in the living corpus or
  crates may cite into it as a normative source — archive-provenance
  pointers only; the campaign plan in `spec/terraforms/` is the one
  live file still inside a legacy-named path (owner carve-out).

## Done (collapsed — see `git log`)

- **Phase L (full, 2026-07-25).** Inventory → discovery (nothing to
  port) → four repoint batches → `git mv` of 35 files to
  `legacy-spec/` with the plan carve-out → specmap regen. Corpus and
  crates are reference-free into the archive.
- **Phase B (full).** 58 files, 4 880/4 880 facts marked; `check
  --exhaustive` 0; grammar precedents complete; two scope narrowings
  (94→59→58). Ledger: 35 findings. DRIFT-001…005 5/5 no-return.
- Earlier: default registry migration; PROP-030 §3.3 project-packages;
  vibevm-term Phase 2b; M1.17/M1.18/M1.19 stacks.

## In progress

Nothing mid-flight — every journal step closed. Phases A/B/L closed;
Phase C opens only on the owner's word.

## Next

1. **Phase C (verification)** per the plan §5 — every marker gets an
   evidence-backed verdict (machine first, judgment where machines
   are silent; honesty enforced: not found ⇒ `unverifiable`). The
   F-024…F-034 stale-header family is prime drift material. **Awaits
   the owner's explicit opening call.**
2. Then Phase D (stitching) over the ledger fixpoint.
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
- **specmap ratchet** — 37 gated orphans host-side after the
  2026-07-25 regen (`f311f429`; previously recorded as 34 — the regen
  was the first since before Phase B and re-based the count). Gate
  passes within the recorded allowance; 0 suspects.

## Session context

Phase L was executed wholesale in the 2026-07-25 session on the
owner's recorded opening phrase. The corpus is fully marked AND
reference-clean of the archive; the tree layout now is: living spec
(`spec/boot common design modules manual-tests` + `WAL.md` + the
campaign plan in `spec/terraforms/`), archive (`legacy-spec/`).
`progress check` must stay at 0. The next session (or this one, on
the owner's word) opens Phase C with a journal phase event per the
DRIFT-003 lane.
