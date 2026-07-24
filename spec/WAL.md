# WAL — Project Continuation State

_Updated: 2026-07-25 (early night — B2 at 14/35; fact-links commission
complete; DRIFT loop 5/5)_

## Current phase

**Progress Control (PROP-043) — Phase B at FACT grain.** `spec/common`
is DONE (1 009 facts, 0 issues); **B2 stands at 14/35 modules files**
(~166+345 facts marked across batches 1–7: progress templates, modules
README, PROP-042/025/026/021/023/020/022/041/039, OWNER-GUIDE).
Scope rulings all landed: generated boot pair OUT, **`spec/WAL.md`
OUT** (checkpoint mortality), authored `00-core`/`90-user` stay —
**94 files, ~8 2xx facts**; wave residue = 40 expected errors in the
two pilot files (SHRINK-PLAN, design/README), theirs at their B2+
batches. **The DRIFT execute-review loop closed 5/5 with no returned
round-trip** (§5-E prediction green): DRIFT-001 cache prune,
DRIFT-002 parse split, DRIFT-003 journal-derived phase (dashboard
lane honest, `{"kind":"phase"}` events), DRIFT-004 specmap fact
units (core v0.8.0 mdspec + `is_valid_fact_id`; heading kebab law
intact), DRIFT-005 fact inheritance (vibe-spec: `NodeKind::Fact` IR
leaves, per-fact override under `:add`, `CompileError::DuplicateId`
merged-view gate, fact-addressed `#embed`). **The owner's fact-links
commission is complete**: contract (PROP-014 §2.1, PROP-035 §5/§7.3
+ heading-repeat precision) → engine → compiler; code can cite
`spec://…#<FACT-ID>` per statement (same URI form as headings, one
id space per doc); F-022 RESOLVED. Ledger: 22 findings (F-015
blockquote anchors, F-016 stale modules index, F-017 aiui scrollbar
code-ahead, F-018/F-019/F-021 stale status vs shipped code, F-020
owner guide lags the fact grammar). **Coder-tier engine pin**:
`claude-opus-5` — committed agent type `.claude/agents/opus5.md`
(selective) + machine-local subagent env pin; both bind at session
start, so they are LIVE from the next session (verified empirically;
the id itself verified live). Key laws in force: FACT-exhaustive
granularity, anchored-when-marked, two anchor registers, separable
core, **no fractality for this campaign** (Fable =
markup/verification/review, Opus = DRIFT coding), verdicts in
cache/baseline never in markup, campaign zone excluded, the surface
is "dashboard". Plan:
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`. Owner manual
(RU): `spec/modules/vibe-progress/OWNER-GUIDE.md` (lags the fact
grammar — F-020).

Previously landed (2026-07-23): the **default registry migration** — the
vibespecs GitHub + GitVerse `[[registry]]` pair moved from per-project
`vibe.toml` templates into the machine-global `~/.vibe/registry.toml`,
seeded by `ensure_default_global_registry()` at the CLI composition root.

## Constraints — do not violate

- **mtime unit in the vvm manifest.** The TS port stores `mtime_ms`
  (milliseconds, integer-floored); the Rust twin stores `mtime_nanos`.
  Both compare equal-on-equal-API (PROP-019 §2.15), but a tool reading
  both manifests MUST account for the unit difference. Documented in
  `vibevm-term/.../common/v0.1.0/vvm/placer.mjs`.
- **electron-packager temp cache.** Two concurrent `<product> self install`
  runs race on the shared `os.tmpdir()` `win32-x64-template` rename. Run
  installs sequentially, not in parallel. Documented in the vibevm-term WAL.
- **CI-off gate split.** `CI` / `VIBE_NO_DEFAULT_REGISTRY` suppresses
  vibe-embedded but NOT project-local (it is portable). Do not broaden
  the gate — see PROP-030 §5 + §3.3.
- **conform R-001 gate.** `crates/vibe-cli/src/registry.rs` is the only
  site sanctioned to construct `EmbeddedProvider` / `LocalCompositeProvider`.
  New providers land there.

## Done (collapsed — see `git log` for detail)

- PROP-030 §3.3 **project-packages auto-discovery** — `LocalCompositeProvider`,
  `SourceKind::Local`, `--prefer-local` / `--no-prefer-local`, the spec
  amendment. 12 commits, `dc45b24`.
- **vibevm-term Phase 2b** — Rust vvm ported to TS (`common/v0.1.0/vvm/`),
  product self CLIs (vibeterm/vibeframe/launcher), 3 PROP-self-install
  contracts. Real-build verified (all 3 products install end-to-end).
- **vibevm-term layout move** — `org.vibevm.term/` → `packages/org.vibevm.term/`,
  `~/.vibe/registry.toml` hack removed. `f2f73e9`.
- **Phase 2a host tear-down** — vibe builds the `vibe` binary only; terminal
  apps resolve via `$VIBEVM_<APP>` → packaged `<instance>/<app>/` → `PATH`,
  with an in-place fallback for `vibe tree`. vibe-launcher crate removed.

## In progress

Nothing mid-flight — Phase A closed cleanly; every Phase A commit is on
`main` and mirrored. The campaign journal is empty by design (journal
steps begin with Phase B batches).

## Next

**Phase B continues at fact grain, executor Fable** (per the plan LOG's
Next step):

1. B2 remainder — 21 modules files by size: PROP-015 (48) → PROP-034
   (50) → PROP-027 (53) → PROP-036 (54) → PROP-030/011/012/010/040/038
   → PROP-008/001/009/017/043/035/037/007 → PROP-005/003/002; then
   `spec/design` / `spec/research` / `spec/terraforms` incl. the two
   pilot files' re-mark. Journal step per file; batch commits of ~2–4
   files (fact density); ledger findings in passing; semantic edits
   forbidden.
2. The Opus queue is EMPTY (DRIFT-001…005 all done, 5/5 no-return).
   New DRIFT tasks spawn via `subagent_type: opus5` (the engine pin —
   live from the next session).
3. Every campaign session starts by reading
   `campaigns/progress-2026-08/run/RESUME.md` (or `vibe progress resume`),
   then the plan LOG (§9) for the next step. `CONTINUE.md` carries the
   cold-resume snapshot of 2026-07-25.

Parked follow-ups from earlier work (unchanged): vibe-vvm/term-vvm
conformance-golden; Linux/macOS install smoke; arbitrary user-repos
design-doc; `vibe doctor` project-local row.

## Known issues

- **GitVerse mirror diverged or degraded (2026-07-25, session end).** The
  final fan-out reported non-fast-forward on `gitverse:main` +
  `gitverse:tags` (a direct write to the host, or a degraded link — a
  follow-up `git fetch origin` timed out, so it could not be inspected).
  **GitHub carries everything** (synced through the session-end commits).
  Next session: `git fetch origin` → inspect `main..origin/main` → if the
  host truly carries foreign commits, reconcile INTO mainline
  (`cargo xtask mirror --from gitverse` for an accepted merge) and re-fan;
  if it was transient, a plain `cargo xtask mirror` re-fan suffices.
  NEVER `--force` (the standing red line).
- **vibespecs 401 on this machine** — `redbook` + `rust-ai-native` resolve
  via vibe-embedded (host `packages/`) here, not via the network registries.
  The `vibe.lock` for any project consuming them carries
  `source_kind = "embedded"` and trips the reproducibility guard. Production
  resolution needs vibespecs credentials (or vendoring).
- **specmap ratchet** — 34 gated orphans in `vibe-spec` (provisional +
  `EmbeddedPrecedence` baseline). Pre-existing, not this work.

## Session context

Open `spec/modules/vibe-registry/PROP-030-embedded-registry.md` §3.3 for the
project-local contract; `crates/vibe-resolver/src/local_composite_provider.rs`
for the composite cell; `vibevm-term/packages/org.vibevm.term/common/v0.1.0/vvm/`
for the ported version-manager. Run `bash tools/self-check.sh` for the floor;
`cargo xtask mirror` to push.
