# WAL — Project Continuation State

_Updated: 2026-07-24_

## Current phase

**Progress Control (PROP-043) + the spec-actualization campaign — Phase A
(scaffold) begins.** The 2026-07-24 owner design session ratified the design
of an inline `<status>` markup system (stage/state/action/audience), the
`vibe progress` tool (scan/check/report/mirror/weave/rescan/resume), the
`campaigns/` zone with crash-safe journal+RESUME protocol, and a phased
campaign (A–G) to mark, verify, stitch, and de-drift the whole spec tree.
Contract: `spec/modules/vibe-progress/PROP-043-progress-markup.md` (DRAFT,
awaiting owner ratification). Plan:
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`. Owner manual (RU):
`spec/modules/vibe-progress/OWNER-GUIDE.md`. Key laws: paragraph-exhaustive
campaign granularity, anchored-unit maintenance granularity, separable core
(no vibevm deps), **no fractality for this campaign** (Fable = high-level,
Opus = IMPL tasks, SPEC tasks by budget), verdicts live in cache/baseline
never in markup, campaign zone excluded from scans/packaging.

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

**Spec-actualization campaign, Phase A (scaffold).** The document package
landed this session (PROP-043 + campaign plan + owner guide + task
templates). Not started yet: owner ratification of PROP-043, the core crate
+ `vibe progress` adapter, the `campaigns/progress-2026-08/` skeleton, the
zero-dep dashboard (`tools/progress-dashboard/`), the 2–3-file pilot markup.
Phase A exit gate: floor green with the new crate, `check --exhaustive`
correct on the pilot, dashboard renders, RESUME.md generates.

## Next

1. **Owner ratifies PROP-043** (or amends — amendments land before code).
2. **Phase A build** per SPEC-ACTUALIZATION-CAMPAIGN §5: core crate,
   `vibe progress`, campaign zone, dashboard, pilot.
3. Then Phase B (paragraph-exhaustive markup of host `spec/`, 91 files,
   Fable, batches of ~8–12).

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
