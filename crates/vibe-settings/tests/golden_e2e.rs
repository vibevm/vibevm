//! End-to-end golden test for the settings system (PROP-040, SETTINGS-SYSTEM
//! IMPL-PLAN §11 acceptance): the full three-level flow — build a schema, write
//! three layer files, load → resolve → inspect → scope-refusal → deprecation
//! migrate — over a real temp dir. This is the integration guard above the
//! per-cell unit tests; it fails if any cell regresses the whole.
//!
//! Spec: [PROP-040](../../../spec/modules/vibe-settings/PROP-040-settings.md).

use tempfile::tempdir;

use vibe_settings::cli::{PrefsOp, PrefsOutcome, run_prefs};
use vibe_settings::loader::{Layer, LayeredRaw, load_all};
use vibe_settings::resolver::{Origin, resolve};
use vibe_settings::schema::{KeyMeta, KeyType, MergeStrategy, Schema, Scope};

/// A small schema exercising every scope + an array merge strategy + a
/// deprecation, the shape a real consumer (the `vibe tree` TUI) will register.
fn schema() -> Schema {
    let mut s = Schema::new();
    s.register(
        KeyMeta::new(
            "tree.palette",
            KeyType::String,
            Scope::User,
            "the TUI palette",
        )
        .unwrap(),
    )
    .unwrap();
    s.register(
        KeyMeta::new(
            "tree.mode",
            KeyType::String,
            Scope::User,
            "the display mode",
        )
        .unwrap()
        .with_default(toml::Value::String("tree".into())),
    )
    .unwrap();
    s.register(
        KeyMeta::new(
            "tree.fold",
            KeyType::Bool,
            Scope::User,
            "fold subtrees by default",
        )
        .unwrap()
        .with_default(toml::Value::Boolean(true)),
    )
    .unwrap();
    s.register(
        KeyMeta::new(
            "tree.search.path",
            KeyType::Array,
            Scope::User,
            "search-path roots",
        )
        .unwrap()
        .with_merge(MergeStrategy::Append),
    )
    .unwrap();
    // Team-only: a team preference a user may not override in L3 (§7 #scope-matrix).
    s.register(
        KeyMeta::new(
            "team.canonical_palette",
            KeyType::String,
            Scope::TeamOnly,
            "the team palette",
        )
        .unwrap(),
    )
    .unwrap();
    // Deprecated key with a replacement target (§6 #deprecation).
    s.register(
        KeyMeta::new(
            "legacy.sort",
            KeyType::String,
            Scope::User,
            "retired — use tree.sort",
        )
        .unwrap()
        .with_deprecation(vibe_settings::schema::Deprecation::with_replacement(
            "use `tree.sort`",
            "tree.sort",
        )),
    )
    .unwrap();
    s
}

#[test]
fn three_level_resolve_inspect_and_scope_refusal() {
    let dir = tempdir().unwrap();
    let l1 = dir.path().join("settings.toml"); // L1 shape (user-machine)
    let l2 = dir.path().join("repo").join(".vibe").join("settings.toml");
    let l3 = dir
        .path()
        .join("repo")
        .join(".vibe")
        .join("settings.local.toml");
    std::fs::create_dir_all(l2.parent().unwrap()).unwrap();

    // L1: user prefers Catppuccin; L2: team sets the mode + the canonical palette;
    // L3: this user fine-tunes the palette to Frappé for this project.
    std::fs::write(
        &l1,
        "tree.palette = \"catppuccin\"\nlegacy.sort = \"alpha\"\n",
    )
    .unwrap();
    std::fs::write(
        &l2,
        "tree.mode = \"tabs\"\nteam.canonical_palette = \"rosé-pine\"\n",
    )
    .unwrap();
    std::fs::write(&l3, "tree.palette = \"frappé\"\n").unwrap();

    let schema = schema();
    let LayeredRaw { l1, l2, l3 } = load_all(&l1, &l2, &l3).unwrap();
    let raw = LayeredRaw { l1, l2, l3 };
    let rp = resolve(raw, &schema, toml::Table::new(), toml::Table::new());

    // L3 wins over L1 for the palette (§2 #precedence-law).
    assert_eq!(
        rp.get("tree.palette").and_then(|v| v.as_str()),
        Some("frappé"),
    );
    assert_eq!(rp.origin("tree.palette"), Some(Origin::L3));

    // L2 sets the mode; L3/L1 don't touch it.
    assert_eq!(rp.get("tree.mode").and_then(|v| v.as_str()), Some("tabs"));
    assert_eq!(rp.origin("tree.mode"), Some(Origin::L2));

    // A key nobody sets falls back to its built-in default (§4 #merge-algorithm).
    assert_eq!(rp.get("tree.fold").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(rp.origin("tree.fold"), Some(Origin::Default));

    // inspect carries each layer's contribution + the winning origin (§5 #inspect).
    let inspect = rp.inspect("tree.palette").unwrap();
    assert_eq!(
        inspect.l1.as_ref().and_then(|v| v.as_str()),
        Some("catppuccin")
    );
    assert_eq!(inspect.l3.as_ref().and_then(|v| v.as_str()), Some("frappé"));
    assert_eq!(inspect.origin, Origin::L3);

    // get_section returns the namespace subtree (§5 #get-section).
    let tree = rp.get_section("tree").unwrap();
    assert!(tree.contains_key("palette"));
    assert!(tree.contains_key("mode"));

    // Scope-refusal (§7 #scope-matrix): TeamOnly may be written to L2 only.
    let raw_for_set = LayeredRaw::default();
    let res = run_prefs(
        PrefsOp::Set {
            key: "team.canonical_palette",
            value: toml::Value::String("mine".into()),
            layer: Layer::L3,
        },
        &schema,
        &raw_for_set,
        &toml::Table::new(),
        &toml::Table::new(),
    );
    assert!(res.is_err(), "TeamOnly scope must refuse an L3 write");
}

#[test]
fn deprecation_migrate_rewrites_to_replaced_by() {
    let dir = tempdir().unwrap();
    let l1 = dir.path().join("settings.toml");
    let l2 = dir.path().join(".vibe").join("settings.toml");
    let l3 = dir.path().join(".vibe").join("settings.local.toml");
    std::fs::create_dir_all(l2.parent().unwrap()).unwrap();
    // L1 carries the retired key.
    std::fs::write(&l1, "legacy.sort = \"alpha\"\n").unwrap();
    std::fs::write(&l2, "").unwrap();
    std::fs::write(&l3, "").unwrap();

    let schema = schema();
    let LayeredRaw { l1, l2, l3 } = load_all(&l1, &l2, &l3).unwrap();
    let raw = LayeredRaw { l1, l2, l3 };

    let outcome = run_prefs(
        PrefsOp::Migrate,
        &schema,
        &raw,
        &toml::Table::new(),
        &toml::Table::new(),
    )
    .expect("migrate is infallible over a well-formed schema");
    let migrated = match outcome {
        PrefsOutcome::Migrated(m) => m,
        other => panic!("expected Migrated, got {other:?}"),
    };
    // At least one layer rewrote the deprecated `legacy.sort`.
    let rewrote: Vec<&String> = migrated.iter().flat_map(|m| m.rewrote.iter()).collect();
    assert!(
        rewrote.iter().any(|line| line.contains("legacy.sort")),
        "expected a rewrite of legacy.sort, got {rewrote:?}",
    );
}
