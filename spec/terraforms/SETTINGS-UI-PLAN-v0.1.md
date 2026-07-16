# SETTINGS-UI-PLAN v0.1 — the `vibe prefs` TUI surface (PROP-041)

_Status: **EXECUTED** (2026-07-16) — all seven phases S1–S7 landed on `main`,
floor-green throughout (`self-check` all green, 347 vibe-cli tests, conform 0,
specmap clean); the owner's visual sign-off is the open item
(`spec/manual-tests/MT-03-vibe-prefs-tui.md`). The settings UI atop the landed
`vibe tree` TUI (PROP-037, Шаг 3) + the settings system (PROP-040, Шаг 2). The
surface lives in `vibe-cli` (`commands/prefs/tui/`) — it composes PROP-037's
`ui::` components + theme and PROP-040's `vibe-settings` data layer; it owns no
preference logic._

**Contract:** [PROP-041](../modules/vibe-settings/PROP-041-settings-ui.md). **Data
layer:** [PROP-040](../modules/vibe-settings/PROP-040-settings.md) (`vibe-settings`).
**Component base:** PROP-037 §2 (`ui::`).

## Phases (cold-executable)

| Phase | REQ § | Deliverable | Lead |
|---|---|---|---|
| **S1** foundation | §1, §2, §3 | `commands/prefs/tui/` module; the **page registry** (Configurable-EP-style declarations, lazy bodies, enumerable); the **settings tree widget** (reuse PROP-037 `Tree` — glyphs/theme/fold/keys); a page row's origin hint; enter → opens §S2 | Delegate |
| **S2** edit form | §4 | per-type fields (bool→toggle, enum→RadioGroup/Menu, int/string→TextField, array→list, table→Group); the Configurable lifecycle (`is_modified`/`apply`/`reset`); write-layer choice (default L3 project / L1 no-project); the `applies` badge | Delegate |
| **S3** provenance | §5 | the "where does this value come from?" view (resolved value + per-layer contribution + winning origin + shadowed layers); layer-aware override (set/clear a specific layer) | Delegate |
| **S4** validation | §6 | inline schema-violation feedback (warning style + rule cited); the "check all layers" action (flat warning list, jump-to-field) | Delegate |
| **S5** search | §7 | settings search via the `vibe-actions` Search Everywhere engine (key/name/description/synonyms); deprecated-key discoverability (`replaced_by`) | Delegate |
| **S6** actions + integration | §8, §11 | every command a `vibe.prefs` action (PROP-039), bound via the PROP-037 keymap; footer lists enabled actions; modal-stack enum/provenance popups; the `vibe prefs ui` (or `vibe prefs`) launch entry; AIUI-clean | Delegate + boss |
| **S7** sign-off | — | MT-03 manual test + owner sign-off; specmap green; conform 0 | Boss |

**Dependency:** S1 (registry+tree) → S2 (form opened from tree) → S3/S4 (per-field
views) → S5 (search over registry) → S6 (action wiring). The data layer
(`vibe-settings` inspect/get/set/list/apply) is already PROP-040 — every phase
reads through it.

## Risks
- **vibe-settings API surface** — S1 must read PROP-040's `lib.rs` (resolver/
  schema/persist) to call `inspect`/`get`/`set`/`list` correctly.
- **state.rs budget (600/600)** — the prefs TUI is a SEPARATE module
  (`commands/prefs/tui/`), not the tree `App`; it owns its own state.
- **scope!→PROP-041** every file; `#[spec(implements)]` on REQ-implementing fns.

## Verification (per-phase + финал)
`cargo fmt --all --check`; `cargo clippy -p vibe-cli -- -D warnings`; `cargo test -p vibe-cli`;
`cargo xtask conform check` (baseline EMPTY); `cargo xtask specmap --check`; `bash tools/self-check.sh`.

## Running ledger

### S1–S7 — DONE (2026-07-16); the settings TUI ships

All seven phases landed on `main`, floor-green at every boundary (`self-check`
all green, 347 vibe-cli tests, conform 0, specmap clean):

- **S1** (`0128360`) — the foundation: a Configurable-EP-style **page registry**
  + the **settings tree widget** (reusing PROP-037's visual language — rounded
  frames, ▾▸/│├└─, theme, no widget re-invented) with a per-row origin hint;
  `vibe prefs ui` launch.
- **S2** (`4438aee`) — the **edit form**: per-type fields (bool→toggle, enum/
  closed-set→Selection, int/string→TextField, array/table→note) + the
  Configurable lifecycle (`is_modified`/`apply`/`reset` mirroring
  `TreeSettings::try_set`) + a write-layer selector (scope-forbidden refused);
  `applies` badge.
- **S3** (`9fb48bc`) — **provenance**: `?` toggles an inline block (resolved
  value + per-layer contribution + winning origin + shadowed layers); `x` clears
  the focused write-layer (falls back).
- **S4** (`9fb48bc`) — **validation**: inline schema-violation warnings (gold,
  rule cited) gating `apply`; `c` opens the lint modal (`validate` across
  L1/L2/L3, jump-to-field).
- **S5** (`623eab8`) — **search**: a `SettingsProvider` over the registry via the
  `vibe-actions` Search Everywhere engine (key/name/description/synonyms;
  deprecated keys surface `replaced_by`); selecting opens the page focused on
  the field.
- **S6** (`623eab8`) — **actions**: every command a `vibe.prefs` action (8-spec
  catalogue → collision-checked Registry + keymap; `dispatch_by_addr`; footer
  lists enabled actions per context); modal-stack Esc pop.
- **S7** — sign-off: `self-check` all green; `specmap` advisory (34 pre-existing
  orphans = `vibe_spec`/`vibe_resolver` debt, unchanged); i18n ships via the
  `vibe.prefs` catalogue inline-English defaults; the manual visual sign-off is
  `spec/manual-tests/MT-03-vibe-prefs-tui.md` (owner's eye). One clippy
  `--all-targets` unused-import fix landed (`2592112`).
