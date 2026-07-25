# WAL — Project Continuation State

_Updated: 2026-07-26 (session end — **Phases D and E driven end to end**:
the drift ledger fell from 311 rows to 1, twelve DRIFT tasks executed,
and the floor is green with no workaround for the first time)_

## Current phase

**Progress Control (PROP-043) — Phase E; the queue is drained.** The spec
tree measured 93.0 % true at the Phase C gate and now measures **4 486
confirmed / 1 drift / 3 unverifiable of 4 490 = 99.9 %**. Findings **61
of 64 resolved**. The single remaining drift row
(`FACT-GRAIN-EVIDENCE`) cannot close here: it waits on
`rust-ai-native-lang` v0.8.0 re-vendoring the fact-aware specmap engine,
which is wave 2's Phase A2.

**`bash tools/self-check.sh` exits 0 against the real `~/.vibe/` with no
`VIBE_SETTINGS` override** — F-055 is genuinely fixed rather than worked
around. Gate panel in `campaign.json`: floor / check / specmap / conform
all green.

**Parked on the owner, nothing else blocked:** (1) DRIFT-022 needs a
letter — (a) allowlist `VIBE_*`/`VIBEVM_*` for the `[env]` promotion, or
(b) remove it; (2) DRIFT-020 is ready to run and needs only a go;
(3) F-063 needs a sync-from-code diff plus one edit only the owner may
make. **Phases F and G were never opened, and close-out has not started** —
`baseline.json`, deferrals, and the REPORT against §8's six predictions.
Wave 2 is planned and unratified:
`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md`.

Key laws unchanged: no fractality (Fable = judgment and ALL review,
Opus = DRIFT execution), engine pin `claude-opus-5`, `reality-mismatch`
closes only through sync-from-code with owner approval.

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
- **The parse payload lives outside the repository** since 2026-07-26:
  `~/.vibe/progress-cache/<repo-id>/<branch-slug>/<campaign>/`. It is
  pure acceleration — deleting it is silent and harmless. Never put a
  verdict there.
- **Never trust a substring match about a data file.** `"parsed"`,
  `"verdicts"` and `updated_at` each read as present when they were not,
  in one day. Walk the structure or anchor on bytes. **It struck a third
  time in code**, and inside the campaign's own correction: PROP-043 §7.3
  was made to claim `Baseline::load / store` because a `store` existed —
  on `Cache`, a different type in the same crate (F-065).
- **Do not run a real `vibe` command while `tools/self-check.sh` is
  running.** The floor now snapshots the real `~/.vibe` before it builds
  and compares after the test steps (DRIFT-020's tripwire). `vibe progress
  scan` writes into `~/.vibe/progress-cache/`, so a concurrent scan turns
  the floor red — correctly, by the gate's own definition, but confusingly.
  Sequence them: scan first, then the floor.
- **Commit delegated work on the completion notification**, never on a
  filled-in task journal — executors write §9 as they go.
- **Outstanding manual runs (owner sign-off pending):** MT-02
  (`vibe tree` TUI) and MT-03 (`vibe prefs ui`). An agent may pre-run;
  only a person signs off.

## Done (collapsed — see `git log`)

- **Phase D — stitching, complete (2026-07-25/26).** Waves d1 and
  d2a–d2h closed **310 of 311 drift rows** across 36 files. d1 took the
  shipped-under-proposed families in one sweep (F-053 PROP-030's 63 rows;
  F-018's bridge four, 137; F-043's PROP-000 twelve) — 191 of those were
  scripted straight off the C-phase verdict map, since a deterministic
  transform beats a re-reading. d2 took the stale headers (22 rows,
  13 files), the design-doc tense family, PROP-003's solver tail
  corrected clause-by-clause, the module index completed to the live tree
  (26 rows added), the MT keymap re-authoring, and the archive's status
  lines. Every row ran through sync-from-code with owner approval.
- **Phase E — coding, queue drained (2026-07-25/26).** DRIFT-006…021
  executed by Opus, each reviewed diff-by-diff; DRIFT-015 superseded
  before it ran. Landed: the specmap evidence join with its report
  column, the lossless-fold check (warning severity — `EXPLICIT-BEATS`
  blesses the divergence a document cannot distinguish from a lying
  fold), the gate panel in `campaign.json`, baseline invalidation's two
  missing rules, blockquote fact anchors, the incremental parse path, the
  `--plain` and resolver-doc corrections, two `deviates` that turned out
  never to have been deviations, the cache split, the no-op-write skip,
  and the removal of the legacy `~/.vibevm` read leg.
- **The test suite stopped reading the developer's home.** F-055, F-056
  and F-057 were one forgotten discipline caught three times by accident.
  Six e2e files now route through a `UserScratch` helper that isolates
  settings, registry cache and search cache together. DRIFT-021 then
  removed the leg no isolation could reach — and found a third read path
  nobody had measured, carrying the vibeterm control-server token.
- Earlier: Phase C (2026-07-25, 93.0 % measured), Phase L, Phase B,
  M1.17/M1.18/M1.19.

## In progress

Nothing running. Two tasks are queued: **DRIFT-020** (test isolation as a
guarantee — ready, needs only a go) and **DRIFT-022** (the `[env]`
promotion — refuses to start until the owner picks (a) allowlist or
(b) remove).

## Next

1. **Answer DRIFT-022's letter** and **release DRIFT-020.** Both close
   F-061 between them.
2. **F-063 — a sync-from-code diff for the token precedence.** Both
   `PROP-002` `##PUB-TOKEN-LOADING` and `spec/boot/90-user.md`
   `##TOKEN-FILE-CONVENTION` name `VIBEVM_PUBLISH_TOKEN` as the highest
   precedence; `VIBEVM_PUBLISH_TOKEN_<HOST>` sits above it. The
   `90-user.md` edit is the owner's — that file is user-owned.
3. **Close wave 1 out.** `baseline.json`, the deferrals file, and the
   REPORT against §8's six predictions — four are already checkable, and
   prediction 3 (≥60 % of IMPLEMENTED claims confirm; ≤10 % unverifiable)
   is comfortably met at 99.9 % / 0.07 %.
4. **Phases F and G were never opened.** F: three owner plans generated
   from views (release/productization, improvement, global idea ledger).
   G: the User Guide and Package Author Guide, written from proven
   behaviour rather than spec prose.
5. **Wave 2 awaits ratification** —
   `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md`, 37 packages
   / 294 files. Its Phase A2 (re-mint `rust-ai-native-lang` v0.8.0) is
   what unblocks wave 1's last drift row.

Parked follow-ups (unchanged): vibe-vvm/term-vvm conformance-golden;
Linux/macOS install smoke; arbitrary user-repos design-doc; `vibe doctor`
project-local row.

## Known issues

- **GitVerse SSH link DOWN all of 2026-07-25/26.** Banner-exchange
  timeout — network-level, not divergence. GitHub carries everything
  through `97c26bf6`. Recovery: plain `cargo xtask mirror`; NEVER
  `--force`.
- **F-063 — the documented token precedence is wrong** in a
  security-relevant way (see Next 2).
- **F-064 — a second config home** (`legacy_xdg_config_path()`,
  `user_config.rs:285`) that `$VIBE_SETTINGS` does not relocate. Same
  shape as the leg DRIFT-021 removed, one severity lower.
- **vibespecs 401 on this machine** — redbook + rust-ai-native resolve
  via vibe-embedded; consuming lockfiles carry `source_kind = "embedded"`.
- **specmap ratchet** — 37 gated orphans host-side, unmoved.

## Session context

One session ran Phase D and Phase E end to end: twelve DRIFT tasks
dispatched to Opus, every diff reviewed before it was committed, and the
ledger taken from 311 open rows to 1. The lasting lessons are in the
Constraints above — the three substring false positives, the two
measurements that corrected each other, and the reviewer error of
committing a worker's output on journal evidence rather than on its
completion. The floor is green plainly for the first time, which is the
one fact a cold reader should verify first.
