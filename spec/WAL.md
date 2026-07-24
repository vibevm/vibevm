# WAL — Project Continuation State

_Updated: 2026-07-24 (evening — Phase B open, fact amendment landed)_

## Current phase

**Progress Control (PROP-043) — Phase B is OPEN and running at FACT
grain.** This session: `progress.toml` narrowed wave 1 to `spec/` (97
files, `8d5ccc8`); **B0** converted 73 legacy status lines into document
markers (one reviewed diff, 0 deletions, `2c98a1e`); **B1** marked all 12
`spec/common` files paragraph-exhaustively (`91274c8`); then the owner
issued the **FACT-GRAIN DIRECTIVE** (verbatim in the plan LOG): every
list item and table body cell is a unit, multi-fact prose is
deconstructed into lists, `##<ID>` fact anchors are mandatory on marked
units (anchored-when-marked), table rows are addressed by a first-cell
id. Ratified into PROP-043 (§3.8 items 4–6, §3.9, §8 — `cd2688f`);
the fact-grain scanner landed (`b67fa97`, 31 tests, cache schema 2);
PROP-029 re-piloted the grammar: 9 paragraphs → 30 anchored facts, 0
issues (`6714876`). Scale: wave 1 = **8 219 facts** (was 3 684
paragraphs); `check` carries **435 expected MissingAnchor** on the
pre-amendment markup — burns down as files re-mark. Ledger: 14 findings
(F-001…F-014); first Opus task DRIFT-001 (cache prune) queued
(`910d545`). Campaign journal is live (`run/journal.jsonl`, step per
file; a real power cut mid-B1 recovered by the §4 rule — verified, no
redo needed). Key laws in force: FACT-exhaustive campaign granularity
(supersedes paragraph grain), anchored-when-marked, separable core,
**no fractality for this campaign** (Fable = markup/verification/review,
Opus = DRIFT coding tasks), verdicts live in cache/baseline never in
markup, campaign zone excluded from scans/packaging, the surface is
called "dashboard". Two OPEN owner review points in the plan LOG:
anchor-naming convention (UPPER vs kebab mix), generated
`spec/boot/STATIC.md`+`INDEX.md` inside scope. Plan:
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`. Owner manual
(RU): `spec/modules/vibe-progress/OWNER-GUIDE.md`.

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
Next step, 2026-07-24 evening):

1. Re-mark the remaining 11 `spec/common` files under the fact grammar —
   deconstruction into anchored fact lists per the PROP-029 re-pilot
   pattern; the cluster's 435 MissingAnchor burn to 0.
2. Then B2… batches (`spec/modules/**`, design, research, terraforms) —
   fact-grain from the start; journal step per file, batch commits,
   ledger findings in passing; semantic edits forbidden.
3. Every campaign session starts by reading
   `campaigns/progress-2026-08/run/RESUME.md` (or `vibe progress resume`),
   then the plan LOG (§9) for the next step and the two OPEN owner
   review points.

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
