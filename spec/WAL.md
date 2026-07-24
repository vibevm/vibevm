# WAL — Project Continuation State

_Updated: 2026-07-24 (night — B1f landed: spec/common fact-grain clean)_

## Current phase

**Progress Control (PROP-043) — Phase B at FACT grain; the spec/common
cluster is DONE.** This session: the two owner review points RESOLVED
(anchor registers: **both stay** — UPPER=normative / kebab=service,
decision recorded at PROP-043 §3.8; generated `spec/boot/STATIC.md` +
`INDEX.md`: **excluded from scope** by include-enumeration in
`progress.toml` — scope 97 → **95 files**, 8 219 → **7 872 facts**,
unblocks the B exit gate; `b15d3d9`). The stale `b-fact-scanner`
journal step was closed retroactively (`4ee4899`). **B1f re-marked all
11 remaining `spec/common` files**: 386 paragraph-grain units → 979
anchored facts (`83bed35` / `4aed13f` / `d639bcf`; batch-1 message
counts corrected in the plan LOG); cluster now **1 009 facts, 0
unmarked, 0 issues** — MissingAnchor 386 → 0. Wave residue: **40
expected errors** in the two pilot files (SHRINK-PLAN 28,
design/README 12), theirs at B2+. Grammar traps ledgered: blockquotes
cannot carry `##` anchors (**F-015**); a wrapped line opening `+ `
parses as a phantom list item. **The standing floor is RED on one
conform finding**: `progress-core/src/parse.rs` 809 > 600-line budget
(from the scanner landing) — queued as **DRIFT-002** (HIGH, Opus);
**DRIFT-003** queued for the hardcoded `"A"` phase in `campaign.json`
(dashboard/RESUME show a stale phase lane until it lands). Ledger: 15
findings (F-001…F-015); tasks DRIFT-001…003 queued. Key laws in force:
FACT-exhaustive granularity, anchored-when-marked, two anchor
registers (owner-ruled), separable core, **no fractality for this
campaign** (Fable = markup/verification/review, Opus = DRIFT coding),
verdicts live in cache/baseline never in markup, campaign zone
excluded from scans/packaging, the surface is called "dashboard".
Plan: `spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`. Owner
manual (RU): `spec/modules/vibe-progress/OWNER-GUIDE.md`.

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
Next step, 2026-07-24 night):

1. B2… batches: `spec/modules/**` at fact grain (largest cluster), then
   `spec/design` / `spec/research` / `spec/terraforms` incl. the two
   pilot files' re-mark. Journal step per file; batch commits of ~3–6
   files; ledger findings in passing; semantic edits forbidden.
2. DRIFT-002 and DRIFT-003 are DONE (Opus executed, Fable accepted;
   floor all green; the phase lane renders the journal-derived "B").
   Last queued Opus task: DRIFT-001 (cache prune).
3. Every campaign session starts by reading
   `campaigns/progress-2026-08/run/RESUME.md` (or `vibe progress resume`),
   then the plan LOG (§9) for the next step.

Parked follow-ups from earlier work (unchanged): vibe-vvm/term-vvm
conformance-golden; Linux/macOS install smoke; arbitrary user-repos
design-doc; `vibe doctor` project-local row.

## Known issues

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
