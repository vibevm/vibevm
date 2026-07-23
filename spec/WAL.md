# WAL — Project Continuation State

_Updated: 2026-07-23_

## Current phase

The **default registry migration** is landing: the vibespecs GitHub +
GitVerse `[[registry]]` pair moves from per-project `vibe.toml` templates
into the machine-global `~/.vibe/registry.toml`, seeded automatically on
first use. `vibe init` now produces a clean project manifest (no
`[[registry]]` boilerplate); `~/.vibe/registry.toml` is created by
`ensure_default_global_registry()` at the CLI composition root on any
registry-needing command if it does not yet exist.

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

Nothing open. The extraction + auto-discovery campaigns are complete; both
repos are synced with their remotes and the floors are green.

## Next

Candidate follow-ups (no default picked — owner steers):

1. **Conformance-golden** between Rust `vibe-vvm` and TS `term-vvm` — a
   formal cross-floor golden asserting the two twins agree on the instance
   layout + manifest shape. Out of scope today; the contracts agree by
   construction.
2. **Linux `.desktop` / macOS `.app` smoke** for the install-machinery —
   Windows is verified; the other two platforms are spec'd but not yet
   run end-to-end.
3. **Arbitrary user-repos** — the next expansion after `--prefer-local`
   (PROP-030 §9 D2 "future expansion under a different name"). Needs a
   design-doc first.
4. **vibe doctor reporting project-local** — the §3.3 feature does not yet
   surface in `vibe self doctor` (which is install-scoped); a project-scoped
   doctor or a `vibe check` row would close the discoverability gap.

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
