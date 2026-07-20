# Settings-Home Consolidation & Global Registry Config — Design Plan v0.1

**status: EXECUTING (2026-07-20) · owner-directed · decisions LOCKED (§2) · two coupled deliverables in one arc — (1) a machine-global registry config at `~/.vibe/registry.toml`, (2) a single `~/.vibe` settings home with a `$VIBE_SETTINGS` override, folding the legacy `~/.vibevm` and the XDG `~/.config/vibe` in · acceptance = one settings chokepoint every read routes through, a working global registry merge (project-first), a refined `--offline` (local repos yes, remote no), all crates rebuilt, floor GREEN**

> **Read-first / boot.** Self-contained. This captures the research (§1), the owner's locked decisions (§2), the architecture (§3–§4), the migration & backward-compat contract (§5), the exact change surface (§6), the AI-native test plan (§7), the phased sequence with the delegation split (§8), and the risks (§9). A cold reader resumes from here plus `spec/WAL.md`.

---

## 0. Why this exists {#why}

Two owner-stated problems, one settings-layer arc:

1. **No machine-global registry config.** Registry/source settings live *only* in a project's `vibe.toml`. A team shares one `vibe.toml`, but each developer's *local* repositories sit at different paths on different machines — so today those paths get hard-coded into a shared file, or not shared at all. The owner wants a per-user `~/.vibe/registry.toml` that supplements the project config with machine-local registries; **project config keeps priority**.

2. **The settings home split across two dirs.** Config has drifted across `~/.vibevm` (publish tokens, `aiui/` discovery, the fractality z.ai token *by reference*), `~/.vibe` (PROP-040 `settings.toml`, registry & search caches), and — a third the owner also folds in — the XDG `~/.config/vibe/config.toml`. The owner declares **`~/.vibe` canonical**, wants it the default, and wants a `$VIBE_SETTINGS` env override in case another app contends for `~/.vibe`. Everything is to be consolidated, every read pointed at the one dir (fractality included), then rebuilt.

---

## 1. Findings — current state (verified 2026-07-20) {#findings}

**Registry config (Part 1): the feature does not exist.**
- Consumer-side source model is `RegistrySection` / `MirrorSection` / `OverrideSection` in `crates/vibe-core/src/manifest/project.rs`, assembled onto `Manifest.{registries,mirrors,overrides}` in `manifest/document.rs` (fields ~`:166–175`, accessors `:537–556`).
- The resolver `MultiRegistryResolver::{open,from_manifest}` (`crates/vibe-registry/src/multi_registry_resolver/mod.rs`) is built **straight from the in-memory `Manifest`**; ordering **is** the `Vec` order (first = highest priority; walk at `walk.rs:202`).
- `vibe registry add|remove|set-mirror` mutate the **project** `vibe.toml` only. **No code reads any user-global registry file.** `~/.vibe/settings.toml` (PROP-040 prefs) is a typed scalar/enum/array store with no `[[registry]]` and an empty production schema — not a registry home.
- Multiple registries already supported (a `Vec`, ordered). Local registries today = `file://` URLs as ordinary `[[registry]]`, plus the single `--registry <path>` and the PROP-030 embedded registry.

**Settings home (Part 2): the change surface is dispersed — no chokepoint.**
- **8 independent home-dir resolvers**; the dir name is a raw literal at each site. `$VIBE_SETTINGS` does not exist. `DOT_VIBE = ".vibe"` is *duplicated* (`vibe-settings/src/loader.rs:35`, `vibe-cli/src/commands/tree/tui/settings.rs:56`), never shared.
- `~/.vibevm` touch points in code: publish tokens `crates/vibe-publish/src/token.rs:236,242` (+ test `:363`, error text `lib.rs:141,143`); aiui discovery `crates/vibe-cli/src/commands/aiui/control.rs:178` (writer is the JS side `apps/vibeframe/renderer.js:89`).
- `~/.vibe` touch points: `registry_cache.rs:32`, `search/cache.rs:87`, `prefs/mod.rs:100`, `tree/tui/settings.rs:581/586`, `settings/loader.rs` classifier `:167/:287`.
- XDG config: `crates/vibe-core/src/user_config.rs:118` resolves `VIBEVM_USER_CONFIG` → `$XDG_CONFIG_HOME/vibe/config.toml` → `%APPDATA%\vibe\config.toml` → `~/.config/vibe/config.toml`.
- **fractality** keeps its own home `~/.fractality` (`FRACTALITY_HOME` → `<USERPROFILE>/.fractality`, `fractality-mc-client/src/home.rs`), **separate** and untouched. Its only `~/.vibevm` coupling is the *string value* of `token_file` in the user's `~/.fractality/profiles.toml`, tilde-expanded against `USERPROFILE` at spawn (`fractality-pod/src/main.rs:116`). Sample template: `.../fractality/v0.1.0/spec/examples/profiles.sample.toml:11`. **Config-data, not code** — moving it is a file move + one edited line.
- Out of scope (not settings): `VIBEVM_HOME` (launcher binary instance), `VIBEVM_INSTALL_ROOT` (`<root>/vibevm` VVM store).

---

## 2. Owner decisions — LOCKED (2026-07-20) {#decisions}

1. **Merge model:** *additive, project-first, dedupe by `name`.* Effective registry list = project registries, then global; a `name` collision resolves to the project entry. Global `[[mirror]]`/`[[override]]` are merged too. **`--offline` refinement:** offline still searches **local** repositories (including machine-local ones from `~/.vibe/registry.toml`) but disables resolution of **remote** registries (github/gitverse and any http(s)/ssh source).
2. **Scope of consolidation:** fold **both** `~/.vibevm` **and** `~/.config/vibe/config.toml` into `~/.vibe`. One true settings dir.
3. **Live-file migration:** move the owner's live files immediately (`~/.vibevm/*.publish.token`, `~/.vibevm/zai.api.token`, any `~/.vibevm/aiui/`, `~/.config/vibe/config.toml`) and edit `~/.fractality/profiles.toml`. Done surgically (code ships new-with-fallback first, so no window where a running spawn breaks; verify a fractality spawn after the token move — Rule 4).

---

## 3. Architecture A — the settings-dir chokepoint {#chokepoint}

New module **`crates/vibe-core/src/settings.rs`** (`vibe-core` is the foundation crate every other depends on; no new external dep — reuse the manual `HOME → USERPROFILE` home resolver already used by `user_config.rs`, so the whole workspace resolves home *one* way).

```rust
pub const SETTINGS_DIR_ENV: &str = "VIBE_SETTINGS";

/// Canonical per-user settings dir. `$VIBE_SETTINGS` (verbatim) → `<home>/.vibe`.
pub fn settings_dir() -> Option<PathBuf>;

/// Pre-consolidation dir `<home>/.vibevm`. Backward-compatible *reads* only; never written.
pub fn legacy_settings_dir() -> Option<PathBuf>;

// Typed accessors (canonical location):
pub fn registry_config_path() -> Option<PathBuf>;  // settings_dir()/registry.toml
pub fn user_config_path()    -> Option<PathBuf>;    // settings_dir()/config.toml
pub fn settings_toml_path()  -> Option<PathBuf>;    // settings_dir()/settings.toml (PROP-040 L1)
pub fn registries_cache_dir()-> Option<PathBuf>;    // settings_dir()/registries
pub fn search_cache_dir()    -> Option<PathBuf>;    // settings_dir()/search-cache
pub fn aiui_dir()            -> Option<PathBuf>;     // settings_dir()/aiui
```

Every dispersed resolver (§6) is rewired to route through this module. Consequences:
- `$VIBE_SETTINGS` moves the whole settings tree atomically and correctly, everywhere, because there is exactly one dir authority.
- Home is resolved one way (`HOME`, then `USERPROFILE` on Windows) — the prior `dirs::home_dir()` vs manual inconsistency is erased. On the owner's box `HOME == USERPROFILE`, so nothing relocates.
- `vibe-settings` deliberately avoids a `vibe-core` dep (PROP-040 §12 crate-boundary): its *classifier* (`parent_is_dot_vibe`) stays structural; the **caller** (`vibe-cli`, which already deps `vibe-core`) passes the resolved L1 dir in. No new crate edge.

## 4. Architecture B — global registry.toml, merge, offline {#global-registry}

**Model reuse.** `~/.vibe/registry.toml` uses the *same* section shapes as `vibe.toml`:

```toml
[[registry]]   # name / url / ref / naming / auth / token_env
[[mirror]]     # of / url / priority
[[override]]   # pkgref / source_url / ref / reason
```

New in `vibe-core` (`settings/registry.rs` or `global_registry.rs`):

```rust
#[serde(deny_unknown_fields)]
pub struct GlobalRegistryConfig {
    #[serde(default, rename = "registry")] pub registries: Vec<RegistrySection>,
    #[serde(default, rename = "mirror")]   pub mirrors:    Vec<MirrorSection>,
    #[serde(default, rename = "override")] pub overrides:  Vec<OverrideSection>,
}
impl GlobalRegistryConfig {
    pub fn load() -> Result<Self, _>;          // settings::registry_config_path(); missing = default
    pub fn load_from(&Path) -> Result<Self, _>;
}

/// Pure, unit-tested. project-first + dedupe registries by name (project wins);
/// mirrors concatenated; overrides project-wins by pkgref key.
pub fn merge_effective(project: &Manifest, global: &GlobalRegistryConfig) -> EffectiveRegistryConfig;

pub struct EffectiveRegistryConfig { registries, mirrors, overrides }
impl EffectiveRegistryConfig {
    /// `--offline`: keep only sources with a local URL (file:// or a bare path);
    /// drop http(s)/ssh/git remotes.
    pub fn local_only(self) -> Self;
}

impl RegistrySection { pub fn is_local(&self) -> bool; } // file: or no "://" and not scp-form
```

**Integration** at the CLI composition root `crates/vibe-cli/src/commands/install/resolver.rs`:
- `open_multi(manifest, args)` becomes: `let eff = merge_effective(manifest, &GlobalRegistryConfig::load()?); let eff = if args.offline { eff.local_only() } else { eff }; MultiRegistryResolver::open(&eff.registries, &eff.mirrors, &eff.overrides)…`.
- The current blanket `|| args.offline ⇒ declared = None` (`resolver.rs:427`) is replaced: under offline the declared walk is the **local-only** effective set; it is `None` only when that set is empty. So a machine-local `file://` repo (from `registry.toml` or the project) resolves offline; a github registry is simply absent, no credential prompt.
- Emptiness/bail checks read the **effective** set, not `manifest.registries`.

**Scoped out (documented limitation):** offline filtering applies to the registry walk + `[[override]]` `source_url` + `[[mirror]]` url. `[requires.packages]` git-sources (a separate mechanism) are left as-is this pass; offline+git-source is a follow-up.

---

## 5. Migration & backward-compat {#migration}

**Reads are new-then-old; writes go new.** For each relocated file, the canonical `~/.vibe` path is authoritative; if absent, fall back to the legacy path so a teammate who has not migrated (or a partial local move) keeps working:
- publish tokens: `~/.vibe/<prefix>.publish.token` → fallback `~/.vibevm/<prefix>.publish.token` (same for the legacy `git.publish.token`).
- aiui discovery: read `~/.vibe/aiui/` → fallback `~/.vibevm/aiui/`; the JS writer moves to `~/.vibe/aiui/`.
- user config: `VIBEVM_USER_CONFIG` → `~/.vibe/config.toml` → fallback old XDG/`%APPDATA%` path.
- caches (`registries/`, `search-cache/`) are already `.vibe` and regenerable — just routed through the chokepoint (no fallback needed).

**Live move (owner's box, Phase E):** copy tokens + `zai.api.token` + `config.toml` into `~/.vibe/`, edit `~/.fractality/profiles.toml` `token_file` → `~/.vibe/zai.api.token`, verify a fractality spawn reads it, then remove the old copies. Fallback code means no breakage window.

---

## 6. Change surface {#surface}

| Site | File:line | Change |
| --- | --- | --- |
| cache root | `vibe-registry/src/registry_cache.rs:32` | route via `settings::registries_cache_dir()` (keep `VIBE_REGISTRY_CACHE`) |
| search cache | `vibe-registry/src/search/cache.rs:87` | route via `settings::search_cache_dir()` |
| prefs L1 | `vibe-cli/.../prefs/mod.rs:100` | route via `settings::settings_toml_path()` |
| tree TUI L1 | `vibe-cli/.../tree/tui/settings.rs:581/586` | route via chokepoint; drop dup `DOT_VIBE` |
| L1 classifier | `vibe-settings/src/loader.rs:167/287` | classify against caller-passed L1 dir |
| publish token | `vibe-publish/src/token.rs:236/242` | `~/.vibe` canonical + `~/.vibevm` fallback read; fix test `:363`, error text `lib.rs:141/143` |
| aiui | `vibe-cli/.../aiui/control.rs:178` | `~/.vibe/aiui` + fallback; JS `apps/vibeframe/renderer.js:89` writes new |
| XDG config | `vibe-core/src/user_config.rs:118` | default `~/.vibe/config.toml` + old-XDG fallback |
| resolver | `vibe-cli/.../install/resolver.rs:365,427,460` | merge global + offline local-only |
| model/loader | `vibe-core` (new `settings.rs`, `global_registry.rs`) | chokepoint + `GlobalRegistryConfig` + `merge_effective` |
| docs | RUNTIME-GUIDE / DEV-GUIDE / troubleshooting / registry-redirect / ROADMAP / manual-tests / CLAUDE·AGENTS·GEMINI:68 / fractality sample+MT / conform.toml | path updates (delegable) |

## 7. Test plan — AI-native cards {#tests}

- **B typed-builders / closed vocab:** `RegistrySection::is_local()` as a typed predicate; `EffectiveRegistryConfig` a named type, not a tuple.
- **C runnable-contracts (doctests, Card G):** `settings_dir()` env precedence, each accessor's suffix, `GlobalRegistryConfig::default`, `merge_effective` project-wins — all as doctests.
- **D differential/characterization oracle:** merge — project+global with a name collision yields project entry, correct order; `local_only()` drops a remote and keeps a `file://`; `is_local` truth table (file://, bare path, https, ssh, `git@host:` scp).
- **F structured diagnostics:** `GlobalRegistryConfig` parse errors cite the REQ + path (mirror `UserConfigError`'s form).
- **`#[verifies]`/`#[spec]` markers** on every new test/error, traced by specmap.
- **Regression:** existing `resolver.rs` offline tests updated to the local-only semantics; token path test updated.

## 8. Phases & delegation split {#phases}

- **A. Design doc** (this file) — Claude. ✅
- **B. Settings chokepoint** — Claude (new core module + discipline; rewire the 8 sites incl. XDG fold, with fallbacks). Gate + commit.
- **C. Global registry + merge + offline** — Claude (judgment: precedence, offline classification, tests). Gate + commit.
- **D. Docs/spec sweep** — **delegate to GLM via fractality** (many files, pure text, exact refs in §6; verification = read the diff). Includes the CLAUDE/AGENTS/GEMINI:68 token-path line + the in-place operating-facts ledger token line.
- **E. Live-file migration** — Claude, careful, Rule 4 (move + profiles.toml edit + verify spawn).
- **F. Rebuild + floor** — Claude (`cargo build --workspace`; fmt/test/clippy/conform/specmap/`vibe check`).
- **G. apps/vibeframe JS aiui path** — Claude (one-liner) or folded into D.

Rationale for the split: the core module + merge/offline logic are safety-critical (a precedence/fallback bug is expensive; verification ≥ generation) → Claude. The doc sweep is low-error, high-verifiability, multi-file → GLM.

## 9. Risks & open items {#risks}

- **Home resolver change** (`dirs` → manual `HOME`/`USERPROFILE`): on Windows where `HOME ≠ USERPROFILE`, caches/settings could appear to relocate. Mitigated: caches are regenerable; settings.toml gets a fallback read; on the owner's box the two are equal.
- **`vibe-settings` crate-boundary:** keep it `vibe-core`-free by passing the L1 dir from the caller; do **not** add a dep edge just for one function.
- **VIBEVM-SPEC.md path docs:** owner-frozen in the licensing sense; path-reference updates there are factual, but treat conservatively — prefer the operational guides (RUNTIME-GUIDE etc.) and note SPEC deltas rather than rewriting frozen prose.
- **offline + git-source deps:** out of scope this pass (documented in §4).
- **fractality is a specspace:** editing its sample template + MT docs crosses into it; the task explicitly asked to check fractality, so this is in-bounds — noted, not silent.
