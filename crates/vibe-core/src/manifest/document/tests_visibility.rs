//! `[visibility]` / `[override]` manifest-table tests, out-of-line per the
//! file-length budget — the same `#[cfg(test)] mod` convention as `tests.rs`.

use super::*;
use crate::manifest::OverrideTarget;
#[test]
fn visibility_and_override_tables_parse_and_roundtrip() {
    let text = r#"
[project]
name = "demo"
version = "0.1.0"
[visibility]
friends = ["org.x/friend"]
unfriend = ["org.x/blocked"]
allow-friends = []
ignore-concept-warnings = ["legacy"]
[override]
"org.x/a -> org.x/b" = { access = "friends-only", friend = false, exclude = true }
"org.x/sealed" = { allow-friends = ["org.x/friend"] }
"#;
    let manifest = Manifest::parse_str(text).unwrap();
    let visibility = manifest.visibility.as_ref().unwrap();
    assert_eq!(
        (
            &visibility.friends,
            &visibility.unfriend,
            &visibility.allow_friends
        ),
        (
            &vec!["org.x/friend".into()],
            &vec!["org.x/blocked".into()],
            &Some(Vec::new())
        )
    );
    assert_eq!(visibility.ignore_concept_warnings, ["legacy"]);
    let targets = manifest.override_table.as_ref().unwrap().targets().unwrap();
    assert!(
        matches!(targets[0].0, OverrideTarget::Edge { .. })
            && matches!(targets[1].0, OverrideTarget::Node(_))
    );
    let back = Manifest::parse_str(&toml::to_string_pretty(&manifest).unwrap()).unwrap();
    assert_eq!(back, manifest);
    assert_eq!(back.visibility.unwrap().allow_friends, Some(Vec::new()));
}

#[test]
fn invalid_override_keys_and_field_combinations_fail_manifest_parse() {
    let bad_tables = [
        "\"not-a-coordinate\" = { allow-friends = \"*\" }",
        "\"org.x/node\" = { access = \"public\" }",
        "\"org.x/a -> org.x/b\" = { allow-friends = [] }",
        "\"org.x/node\" = { allow-friends = \"somebody\" }",
        "\"org.x/a -> org.x/b -> org.x/c\" = { access = \"public\" }",
    ];
    for table in bad_tables {
        let text =
            format!("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n[override]\n{table}\n");
        let error = Manifest::parse_str(&text).unwrap_err().to_string();
        assert!(
            error.contains("override") || error.contains("allow-friends"),
            "{error}"
        );
    }
}
