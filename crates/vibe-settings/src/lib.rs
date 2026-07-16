//! `vibe-settings` — application/user preferences for vibevm (PROP-040).
//!
//! A three-level, schema-first, introspectable store for how vibevm's surfaces
//! look and behave for a user (the Vibe Tree UI — palette/glyphs/rendering
//! tier/mode/sort/tree-shape/fold; future vibe-app prefs) — **not** project
//! config (`vibe.toml`, which is the vibe-PROJECT manifest, the `pom.xml`
//! analogue). Levels (lowest → highest precedence):
//!
//! - **L1** `~/.vibe/` — user-machine global defaults;
//! - **L2** `<repo>/.vibe/settings.toml` — repo-shared (committed);
//! - **L3** `<repo>/.vibe/settings.local.toml` — user-project (gitignored).
//!
//! L3 wins among file layers; CLI flags and `VIBE_*` env vars override every
//! file layer. Frontend-agnostic (zero rendering dependencies) so the TUI, a
//! future GUI, and the headless AIUI all read through one resolver.
//!
//! Implementation phases (each cell carries its own `specmark::scope!` citing
//! the PROP-040 anchor it implements; public fns carry
//! `#[spec(implements = "spec://…")]` per the AI-Native Rust discipline):
//!
//! - Phase 2.2 — `loader`: L1/L2/L3 TOML parse + path-classifier (PROP-040 §3, §9).
//! - Phase 2.3 — `resolver`: `ResolvedPrefs` + deep-merge + `inspect` (§4, §5).
//! - Phase 2.4 — `schema`: `KeyMeta`, `Scope`, registry, validation (§6, §7).
//! - Phase 2.5 — `events`: change-events + `applies` + file-watch (§10).
//! - Phase 2.6 — `cli`: `vibe prefs` plumbing (§8).
//! - Phase 2.7 — `persist`: diff-from-default + `.gitignore` gen (§6, §9).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#root");

pub mod error;
pub mod loader;
pub mod resolver;
pub mod schema;
