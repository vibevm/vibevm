//! Tests for the `vibe.tree.*` settings bridge (PROP-037 §9).
//!
//! File-backed submodule of [`super`] — split out so the cell stays under the
//! 600-line AI-Native file budget. Every test builds a [`super::TreeSettings`]
//! against a tempdir so persistence round-trips never touch the operator's real
//! `~/.vibe/`.

#![allow(clippy::unwrap_used)]

use std::path::Path;

use super::*;
use tempfile::tempdir;
use vibe_settings::loader::classify_with_home;

/// Classify a path's layer using the home resolved at launch (parity with
/// the loader's classifier — exercised so the wiring stays consistent).
fn classify(path: &Path) -> Layer {
    classify_with_home(path, super::home_dot_vibe().parent())
}

/// Build a `TreeSettings` whose L1 sits at `dir/settings.toml` and whose L2/L3
/// point at absent files in the same tempdir, so only L1 contributes.
fn settings_in(dir: &Path) -> TreeSettings {
    TreeSettings::with_paths(
        dir.join("settings.toml"),
        dir.join("missing-l2.toml"),
        dir.join("missing-l3.toml"),
    )
}

#[test]
fn schema_declares_the_six_vibe_tree_keys() {
    let s = TreeSettings::new();
    let schema = s.schema();
    for path in [
        KEY_PALETTE,
        KEY_TIER,
        KEY_MODE,
        KEY_SORT,
        KEY_SHAPE,
        KEY_STATIC_FIRST,
    ] {
        assert!(schema.contains(path), "schema declares {path}");
    }
    // palette + mode + sort + shape + static-first carry defaults; tier does not.
    assert_eq!(
        schema.get(KEY_PALETTE).unwrap().scope,
        Scope::User,
        "palette is User-scoped"
    );
    assert!(
        schema.get(KEY_TIER).unwrap().default.is_none(),
        "tier has no default (auto-detect)"
    );
}

#[test]
fn loading_palette_mocha_builds_a_mocha_theme() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.toml"),
        "[vibe.tree]\npalette = \"catppuccin-mocha\"\n",
    )
    .unwrap();
    let s = settings_in(dir.path());
    let rp = s.load();
    let theme = s.theme(&rp);
    assert_eq!(theme.palette_name(), "catppuccin-mocha");
}

#[test]
fn tier_setting_overrides_detection() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("settings.toml"), "[vibe.tree]\ntier = 1\n").unwrap();
    let s = settings_in(dir.path());
    let rp = s.load();
    let theme = s.theme(&rp);
    assert_eq!(theme.tier(), Tier::T1);
    // And the snapshot carries the override.
    assert_eq!(s.snapshot(&rp).tier_override, Some(Tier::T1));
}

#[test]
fn missing_settings_yield_defaults() {
    let dir = tempdir().unwrap();
    let s = settings_in(dir.path());
    let rp = s.load();
    let theme = s.theme(&rp);
    assert_eq!(theme.palette_name(), "rose-pine");
    let snap = s.snapshot(&rp);
    assert_eq!(snap.mode, DisplayMode::All);
    assert_eq!(snap.sort, Ordering::Topological);
    assert_eq!(snap.shape, TreeShape::MembersAsRoots);
    assert!(snap.static_first);
    assert!(snap.tier_override.is_none());
}

#[test]
fn corrupt_settings_are_swallowed_to_defaults() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("settings.toml"), "not = valid = toml\n").unwrap();
    let s = settings_in(dir.path());
    // A parse error short-circuits load_all → the resolver sees an empty
    // triple → every key resolves to its default. The launch never breaks.
    let rp = s.load();
    let theme = s.theme(&rp);
    assert_eq!(theme.palette_name(), "rose-pine");
}

#[test]
fn mode_round_trips_through_persist_and_reload() {
    let dir = tempdir().unwrap();
    let s = settings_in(dir.path());

    // Initially absent → default (All).
    assert_eq!(s.snapshot(&s.load()).mode, DisplayMode::All);

    // Persist mode = "tabs".
    s.set(
        KEY_MODE,
        toml::Value::String(mode_label(DisplayMode::Tabs).into()),
    );

    // Reload → restored.
    assert_eq!(s.snapshot(&s.load()).mode, DisplayMode::Tabs);

    // The file on disk carries the value under [vibe.tree].
    let on_disk = std::fs::read_to_string(dir.path().join("settings.toml")).unwrap();
    assert!(on_disk.contains("[vibe.tree]"));
    assert!(on_disk.contains("mode = \"tabs\""));
}

#[test]
fn setting_a_key_to_its_default_drops_it_from_the_file() {
    let dir = tempdir().unwrap();
    let s = settings_in(dir.path());
    s.set(
        KEY_MODE,
        toml::Value::String(mode_label(DisplayMode::Tabs).into()),
    );
    s.set(
        KEY_MODE,
        toml::Value::String(mode_label(DisplayMode::All).into()),
    );
    // Back to default → diff-from-default removes it → the file has no
    // `mode` key (and the now-empty `[vibe.tree]` table collapses away).
    let on_disk = std::fs::read_to_string(dir.path().join("settings.toml")).unwrap();
    assert!(
        !on_disk.contains("mode"),
        "default-valued key dropped: {on_disk}"
    );
}

#[test]
fn persist_preserves_a_sibling_key_and_comments() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    std::fs::write(&path, "# my note\n[vibe.tree]\nshape = \"pruned-tree\"\n").unwrap();

    let s = settings_in(dir.path());
    s.set(
        KEY_MODE,
        toml::Value::String(mode_label(DisplayMode::Tabs).into()),
    );

    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("my note"), "operator comment preserved");
    assert!(after.contains("mode = \"tabs\""), "new key written");
    assert!(
        after.contains("shape = \"pruned-tree\""),
        "sibling key preserved"
    );
}

#[test]
fn classify_helper_is_total() {
    // Smoke: the classifier never panics on the conventional paths.
    let _ = classify(std::path::Path::new("/x/.vibe/settings.toml"));
}

#[test]
fn parse_palette_falls_back_on_unknown() {
    assert_eq!(parse_palette(Some("nonsense")), PaletteName::RosePine);
    assert_eq!(parse_palette(None), PaletteName::RosePine);
    assert_eq!(parse_palette(Some("catppuccin-latte")), PaletteName::Latte);
}
