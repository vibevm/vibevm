//! Unit tests for the `vibe prefs` plumbing cell. Split out of `mod.rs` to
//! honour the ≤600-line AI-Native file budget. Non-`#[test]` helpers carry
//! `#[cfg(test)]` so file-grain scanners (the conform frontend) scope their
//! `unwrap`s as test code.

use super::*;
use crate::schema::{Deprecation, KeyMeta, KeyType, Scope};

#[cfg(test)]
fn schema_with(deprecated: &[(&str, Option<&str>)]) -> Schema {
    let mut s = Schema::new();
    for (path, repl) in deprecated {
        let mut k = KeyMeta::new(*path, KeyType::String, Scope::User, "a test key").unwrap();
        if let Some(r) = repl {
            k = k.with_deprecation(Deprecation::with_replacement("migrate", *r));
        }
        s.register(k).unwrap();
    }
    s
}

#[test]
fn get_unset_key_returns_none() {
    let out = run_prefs(
        PrefsOp::Get { key: "nope" },
        &Schema::new(),
        &LayeredRaw::default(),
        &toml::Table::new(),
        &toml::Table::new(),
    )
    .unwrap();
    assert!(matches!(out, PrefsOutcome::Value(None)));
}

#[test]
fn set_unknown_key_is_allowed_and_targeted() {
    // Empty schema → unknown key is allowed (warns at `check`, not refused).
    let raw = LayeredRaw::default();
    let out = run_prefs(
        PrefsOp::Set {
            key: "tree.palette",
            value: toml::Value::String("rosé-pine".into()),
            layer: Layer::L3,
        },
        &Schema::new(),
        &raw,
        &toml::Table::new(),
        &toml::Table::new(),
    )
    .unwrap();
    let table = match out {
        PrefsOutcome::LayerWritten { layer, table } => {
            assert_eq!(layer, Layer::L3);
            table
        }
        _ => panic!("expected LayerWritten"),
    };
    assert_eq!(
        table
            .get("tree")
            .and_then(|t| t.as_table())
            .and_then(|t| t.get("palette"))
            .and_then(|v| v.as_str()),
        Some("rosé-pine")
    );
}

#[test]
fn set_declared_key_refuses_wrong_layer() {
    // TeamOnly is L2-only (§7 #scope-matrix); writing it to L3 is refused.
    let mut s = Schema::new();
    s.register(
        KeyMeta::new(
            "team.palette",
            KeyType::String,
            Scope::TeamOnly,
            "shared palette",
        )
        .unwrap(),
    )
    .unwrap();
    let err = run_prefs(
        PrefsOp::Set {
            key: "team.palette",
            value: toml::Value::String("x".into()),
            layer: Layer::L3,
        },
        &s,
        &LayeredRaw::default(),
        &toml::Table::new(),
        &toml::Table::new(),
    )
    .unwrap_err();
    match err {
        PrefsError::WrongLayer {
            key,
            scope,
            layer,
            allowed,
        } => {
            assert_eq!(key, "team.palette");
            assert_eq!(scope, "team-only");
            assert_eq!(layer, Layer::L3);
            assert_eq!(allowed, "L2");
        }
    }
}

#[test]
fn list_reports_resolved_leaves_with_origin() {
    let l2: toml::Table = toml::from_str("a = 1\n[b]\nc = 2\n").unwrap();
    let raw = LayeredRaw {
        l1: toml::Table::new(),
        l2,
        l3: toml::Table::new(),
    };
    let out = run_prefs(
        PrefsOp::List,
        &Schema::new(),
        &raw,
        &toml::Table::new(),
        &toml::Table::new(),
    )
    .unwrap();
    let keys = match out {
        PrefsOutcome::Keys(k) => k,
        _ => panic!("expected Keys"),
    };
    let by_path: Vec<(&str, Origin)> = keys.iter().map(|k| (k.path.as_str(), k.origin)).collect();
    assert_eq!(by_path, vec![("a", Origin::L2), ("b.c", Origin::L2)]);
}

#[test]
fn check_surfaces_unknown_and_deprecated_per_layer() {
    let s = schema_with(&[("node.sort", Some("tree.sort"))]);
    let l2: toml::Table = toml::from_str("node.sort = \"name\"\ntypo = true\n").unwrap();
    let raw = LayeredRaw {
        l1: toml::Table::new(),
        l2,
        l3: toml::Table::new(),
    };
    let diags = match run_prefs(
        PrefsOp::Check,
        &s,
        &raw,
        &toml::Table::new(),
        &toml::Table::new(),
    )
    .unwrap()
    {
        PrefsOutcome::Diagnostics(d) => d,
        _ => panic!("expected Diagnostics"),
    };
    let joined = diags.join("\n");
    assert!(
        joined.contains("L2"),
        "diagnostic names its layer: {joined}"
    );
    assert!(joined.contains("deprecated"));
    assert!(joined.contains("unknown"));
}

#[test]
fn migrate_with_no_deprecated_is_empty() {
    let raw = LayeredRaw::default();
    let out = run_prefs(
        PrefsOp::Migrate,
        &Schema::new(),
        &raw,
        &toml::Table::new(),
        &toml::Table::new(),
    )
    .unwrap();
    assert!(matches!(out, PrefsOutcome::Migrated(m) if m.is_empty()));
}

#[test]
fn show_origins_all_returns_one_per_leaf() {
    let l2: toml::Table = toml::from_str("a = 1\n[b]\nc = 2\n").unwrap();
    let raw = LayeredRaw {
        l1: toml::Table::new(),
        l2,
        l3: toml::Table::new(),
    };
    let out = run_prefs(
        PrefsOp::ShowOrigins { key: None },
        &Schema::new(),
        &raw,
        &toml::Table::new(),
        &toml::Table::new(),
    )
    .unwrap();
    let origins = match out {
        PrefsOutcome::Origins(o) => o,
        _ => panic!("expected Origins"),
    };
    // Two leaves → two breakdowns, each naming L2 as the origin.
    assert_eq!(origins.len(), 2);
    assert!(origins.iter().all(|e| e.value.origin == Origin::L2));
    // Paths are carried on the entry.
    let paths: Vec<&str> = origins.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, vec!["a", "b.c"]);
}
