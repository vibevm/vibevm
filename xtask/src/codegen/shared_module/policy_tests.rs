//! Effective-policy joins through the real registry loader and both emitters.

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::Result;

use super::super::super::strictness::{Strictness, apply_strictness};
use super::super::super::vocabulary::Resolved;
use super::super::{apply_shared_strictness, guard_shared_strictness};

const SOURCE: &str = "#[derive(Serialize, Deserialize)]\npub struct SharedRow {\n    pub value: String,\n}\n\n#[derive(Serialize, Deserialize)]\npub struct SharedRowEmpty {}\n";

/// Each tuple is schema stem, parser population and an optional override.
/// No effective-policy computation occurs in the fixture: the owner loads it.
fn registry(
    rows: &[(&str, &str, Option<&str>)],
) -> Result<(tempfile::TempDir, Vec<(PathBuf, Resolved)>)> {
    let root = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join("formats"))?;
    std::fs::create_dir(root.path().join("schemas"))?;
    let mut text = String::new();
    let mut resolved = Vec::new();
    for (index, (stem, population, policy)) in rows.iter().enumerate() {
        let relative = format!("schemas/{stem}.jtd.json");
        text.push_str(&format!("[format.owner-row{index}]\nepoch=1\nschema=\"{relative}\"\nrecoverable=true\nforeign_parsers=\"{population}\"\ncorpus=\"none\"\n"));
        if let Some(policy) = policy {
            text.push_str(&format!("unknown_fields=\"{policy}\"\n"));
        }
        let path = root.path().join(relative);
        std::fs::write(&path, "{}\n")?;
        let closure = BTreeSet::from(["shared_row".to_string()]);
        resolved.push((
            path.clone(),
            Resolved {
                doc: path,
                vocabularies: closure.clone(),
                ordinary_vocabularies: closure,
                projections: Vec::new(),
            },
        ));
    }
    std::fs::write(root.path().join("formats/REGISTRY.toml"), text)?;
    Ok((root, resolved))
}

fn matching_emission(rows: &[(&str, &str, Option<&str>)], strict: bool) -> Result<()> {
    let (root, resolved) = registry(rows)?;
    let shared = guard_shared_strictness(root.path(), &resolved)?;
    let shared_output = apply_shared_strictness(SOURCE, "shared/mod.rs", &shared)?;
    assert_eq!(
        shared_output.matches("deny_unknown_fields").count(),
        if strict { 2 } else { 0 }
    );
    let consumer = Strictness::load(root.path())?;
    for (schema, _) in &resolved {
        let output = apply_strictness(SOURCE, "consumer/mod.rs", schema, &consumer)?;
        assert_eq!(
            output, shared_output,
            "shared and consumer attributes must stitch"
        );
        assert_eq!(
            apply_strictness(&output, "consumer/mod.rs", schema, &consumer)?,
            output
        );
    }
    Ok(())
}

#[test]
fn explicit_ours_and_many_deny_match_consumer_structs() -> Result<()> {
    matching_emission(
        &[
            ("request", "ours", Some("deny")),
            ("report", "many", Some("deny")),
        ],
        true,
    )
}

#[test]
fn explicit_none_allow_matches_permissive_consumers() -> Result<()> {
    matching_emission(
        &[
            ("override", "none", Some("allow")),
            ("default", "many", None),
        ],
        false,
    )
}

#[test]
fn legacy_defaults_preserve_none_and_foreign_readers() -> Result<()> {
    for (population, strict) in [("none", true), ("ours", false), ("many", false)] {
        matching_emission(&[("default", population, None)], strict)?;
    }
    Ok(())
}

#[test]
fn shared_mixed_effective_policies_refuse() -> Result<()> {
    let (root, resolved) = registry(&[
        ("strict", "many", Some("deny")),
        ("permissive", "none", Some("allow")),
    ])?;
    let error = guard_shared_strictness(root.path(), &resolved)
        .unwrap_err()
        .to_string();
    for expected in [
        "shared_row",
        "strict.jtd.json",
        "permissive.jtd.json",
        "unknown_fields",
        "deny",
        "allow",
    ] {
        assert!(error.contains(expected), "missing {expected}: {error}");
    }
    Ok(())
}

#[test]
fn duplicate_schema_effective_conflicts_refuse() -> Result<()> {
    let (root, resolved) = registry(&[
        ("duplicate", "ours", Some("deny")),
        ("duplicate", "ours", Some("allow")),
    ])?;
    let error = guard_shared_strictness(root.path(), &resolved)
        .unwrap_err()
        .to_string();
    for expected in [
        "owner-row0",
        "owner-row1",
        "duplicate.jtd.json",
        "conflicting unknown_fields",
    ] {
        assert!(error.contains(expected), "missing {expected}: {error}");
    }
    Ok(())
}

#[test]
fn duplicate_equal_policies_ignore_parser_population() -> Result<()> {
    matching_emission(
        &[
            ("duplicate", "ours", Some("deny")),
            ("duplicate", "many", Some("deny")),
        ],
        true,
    )
}

#[test]
fn explicit_non_string_policy_refuses_without_legacy_default() -> Result<()> {
    use super::super::super::format_id::load_format_registry;

    for (population, fallback) in [("none", "deny"), ("ours", "allow"), ("many", "allow")] {
        for policy in [None, Some("allow"), Some("deny")] {
            let (root, _) = registry(&[("request", population, policy)])?;
            let entries = load_format_registry(root.path())?;
            assert_eq!(entries[0].unknown_fields, policy.unwrap_or(fallback));
        }
        for raw in ["true", "1", "[\"deny\"]", "{}"] {
            let (root, _) = registry(&[("request", population, Some("deny"))])?;
            let path = root.path().join("formats/REGISTRY.toml");
            let text = std::fs::read_to_string(&path)?;
            assert_eq!(text.matches("unknown_fields=\"deny\"").count(), 1);
            std::fs::write(
                &path,
                text.replace("unknown_fields=\"deny\"", &format!("unknown_fields={raw}")),
            )?;
            let error = load_format_registry(root.path())
                .map(|_| ())
                .expect_err("present non-string cannot select a legacy default")
                .to_string();
            assert!(
                error.contains("[format.owner-row0].unknown_fields"),
                "{error}"
            );
            assert!(error.contains("must be a string when present"), "{error}");
        }
    }
    Ok(())
}
