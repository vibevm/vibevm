# WAL — Project Continuation State

_Updated: 2026-07-25 (session end — **PHASE C COMPLETE, exit gate
green**: 58/58 files, 4 944/4 944 markers carry verdicts; the corpus
is measured at 93.0 % confirmed; Phase D awaits the owner's call)_

## Current phase

**Progress Control (PROP-043) — Phase C (verification) COMPLETE; the
phase lane stays C until the owner opens Phase D (§5 entry law).**
**Final tally: 4 455 units judged — 4 141 confirmed / 311 drift / 3
unverifiable = 93.0 % / 7.0 % / 0.07 % — the first measured actuality
level of the spec tree. Findings 55** (F-001…F-054; this session
minted F-045…F-054 and extended F-018/F-024/F-047/F-048 across ten
batches c4a…c4f3). The §5-C prediction confirmed **mirrored**: drift
concentrates in Status lines that promised *less* than the tree
delivers — the shipped-under-proposed families (PROP-030 63 rows;
the bridge four PROP-020/021/022/023 = 137; PROP-000's F-043 six;
the DRAFT-era cli/actions headers F-052) — while honestly-updated
files confirm nearly wholesale (**eight zero-drift files**:
PROP-011/012/025/042/015/026/027/005). Notable code-side rows for
Phase E: F-036 (--plain clap lie), F-047 (three stale deviates
denying the shipped ResolvoDepSolver). One verdict was corrected
mid-phase (c4b SOLVER-IDENTITY-FIELD confirm → drift in c4c; F-048f).
The 311-row drift ledger is sweep-shaped for Phase D: ~15 family rows
cover ~80 % of it. **Next: the owner reads the measurement and rules
on opening Phase D (stitching)**; the Opus DRIFT queue stays EMPTY.
Key laws unchanged: no fractality (Fable = verification, Opus = DRIFT
coding), engine pin `claude-opus-5`. Plan:
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md` (LOG §9 carries
the full ten-batch record and the exit-gate boundary entry).

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

- **Phase C, modules cluster c4a…c4f3 (2026-07-25, this session).**
  Ten batches, 3 057 units: vibe-progress self-verification (F-045
  self-status, F-046 parity family), registry core (F-047 stale
  deviates, F-048 promised-absent, resolvo confirmed as production
  default), resolver pair (F-050 un-marked supersession tail; c4b
  verdict corrected), workspace 10/10 (F-018 mass families PROP-020/
  022; F-029/F-026/F-028 land; PROP-011/012/025 zero-drift),
  cli/actions (F-052 DRAFT-era headers; PROP-042 clean), registry
  rest (F-053 PROP-030 63 rows — the densest; F-054 nits; bridge
  family completed), mcp+settings (three zero-drift; F-010/F-019
  land), index PROP-005 all-confirmed. Exit gate green.
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

Nothing open — Phase C closed at its exit gate; the journal's last
step (`c4f3-index`) is done and the boundary entry is in LOG §9. The
campaign holds at the C→D boundary for the owner's ruling.

## Next

1. **Phase C is COMPLETE** — exit gate green (58/58 files, 4 944/4 944
   markers, final 4 141C/311D/3U = 93.0 %). **Ask the owner to open
   Phase D (stitching)** over the 54-row ledger — ~15 family rows
   cover ~80 % of the 311 drift verdicts.
2. Phase D shape when opened: the shipped-under-proposed re-mark
   sweeps (F-018 four-spec bridge family, F-053 PROP-030, F-052
   cli/actions, F-043 PROP-000, F-046 PROP-043 parity) close most of
   the ledger; reality-mismatch rows route through sync-from-code.
3. Opus queue EMPTY; Phase E DRIFT candidates grew: F-036 (`--plain`
   clap lie), F-047 (three stale deviates denying the shipped
   ResolvoDepSolver), F-048 (--trust-mirror / `list --overrides`
   promised-absent), F-016 modules README, F-020 OWNER-GUIDE, F-017
   aiui scrollbar.

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

One session drove the entire modules cluster (ten batches, 3 057
units) and closed Phase C at its exit gate. The journal is closed, all
verdicts committed batch-by-batch, GitHub synced throughout (GitVerse
still SSH-down; plain re-fan on recovery). The next session opens by
reporting the measurement to the owner and asking whether to open
Phase D — a `phase` journal event lands only on that ruling.
`progress check` must stay 0; the cache's campaign maps must survive
every write (load-and-merge only). Method note kept durable: coverage
counts come from `progress mirror`'s ParsedDoc, never the raw-grep
extractor (code-span shorthands over-count); `run/mirror/` is
ephemeral and safe to delete.
