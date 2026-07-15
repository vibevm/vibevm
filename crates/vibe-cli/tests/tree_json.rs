//! Golden test for `vibe tree --json` (PROP-036 §2.7, plan §12).
//!
//! Runs the built `vibe` binary against THIS repo, strips the CLI envelope
//! keys, and asserts the resulting document validates against the shipped
//! `crates/vibe-cli/resources/package-tree.schema.v1.json` — plus a handful of
//! Phase-0-verified effective-load facts (redbook = static-transitive
//! declarer, addressable-specs = static-by-transitive, rust-ai-native umbrella
//! = none).

use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;

/// The vibevm workspace root — two parents up from this crate's manifest dir.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// The shipped JSON Schema, co-located with the producer.
fn schema() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/package-tree.schema.v1.json");
    let text = std::fs::read_to_string(&path).expect("schema file is present");
    serde_json::from_str(&text).expect("schema is valid JSON")
}

/// Run `vibe tree --json --path <root>` and return the parsed stdout.
fn run_tree_json(root: &std::path::Path) -> Value {
    let mut cmd = Command::cargo_bin("vibe").expect("vibe binary built");
    cmd.arg("tree").arg("--json").arg("--path").arg(root);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf-8 stdout");
    serde_json::from_str(&stdout).expect("stdout is JSON")
}

#[test]
fn tree_json_validates_against_schema_and_carries_known_facts() {
    let root = workspace_root();
    // The golden runs on the live repo state; a checkout without the lockfile
    // (e.g. a partial clone) has nothing to analyze — skip rather than fail.
    if !root.join("vibe.lock").exists() {
        eprintln!("skipping: no vibe.lock at {}", root.display());
        return;
    }

    let mut doc = run_tree_json(&root);

    // Envelope (PROP-036 §2.7 / plan §12 acceptance).
    assert_eq!(doc["ok"], Value::Bool(true), "envelope ok");
    assert_eq!(doc["command"], Value::String("tree".into()), "command");
    assert_eq!(
        doc["schema_version"],
        serde_json::json!(1),
        "schema_version"
    );

    // Strip the envelope-only keys → the pure schema document.
    if let Value::Object(map) = &mut doc {
        for key in ["ok", "command", "invoked_by", "unattended"] {
            map.remove(key);
        }
    }

    // Validate against the shipped schema (Draft 2020-12).
    let schema = schema();
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    let errors: Vec<String> = validator
        .iter_errors(&doc)
        .map(|e| format!("  {} @ {}", e, e.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "vibe tree --json does not validate against the v1 schema:\n{}",
        errors.join("\n")
    );

    let packages = doc["packages"].as_array().expect("packages array");
    let find = |id: &str| -> &Value {
        packages
            .iter()
            .find(|p| p["id"] == Value::String(id.into()))
            .unwrap_or_else(|| panic!("package `{id}` present in the tree"))
    };

    // redbook: the static-transitive DECLARER — effective static, its own
    // static-ness (T = false), physically in STATIC.md (PROP-036 §2.4).
    let redbook = find("org.vibevm.world/redbook");
    assert_eq!(redbook["load"]["type"], "static", "redbook load");
    assert_eq!(
        redbook["load"]["transitive"],
        Value::Bool(false),
        "redbook T"
    );
    assert_eq!(
        redbook["load"]["in_static_md"],
        Value::Bool(true),
        "redbook S"
    );
    assert_eq!(
        redbook["load"]["declared"], "static-transitive",
        "redbook declared link"
    );
    assert_eq!(redbook["load"]["origin"], "declared", "redbook origin");

    // addressable-specs: forced static by redbook's closure — T = true,
    // origin static-transitive (its own boot snippet has no `link`).
    let addr = find("org.vibevm.world/addressable-specs");
    assert_eq!(addr["load"]["type"], "static", "addressable-specs load");
    assert_eq!(addr["load"]["transitive"], Value::Bool(true), "addr T");
    assert_eq!(addr["load"]["origin"], "static-transitive", "addr origin");

    // rust-ai-native umbrella (PROP-028): ships no boot snippet → none.
    let umbrella = find("org.vibevm.ai-native/rust-ai-native");
    assert_eq!(umbrella["load"]["type"], "none", "umbrella load");
    assert_eq!(
        umbrella["load"]["in_index_md"],
        Value::Bool(false),
        "umbrella not in INDEX"
    );

    // The static lane size indicator is populated (PROP-036 §2.6).
    let static_md = &doc["boot"]["static_md"];
    assert!(static_md.is_object(), "static_md lane present");
    assert!(
        static_md["bytes"].as_u64().unwrap_or(0) > 0,
        "STATIC.md byte count is non-zero"
    );
}
