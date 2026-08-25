use std::fs;

use tempfile::tempdir;

use crate::manifest::{ExtensionKey, Manifest};
use crate::{Error, Group, PackageName};

const PROJECT: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";

fn parse(body: &str) -> Manifest {
    Manifest::parse_str(&format!("{PROJECT}\n{body}")).unwrap()
}

fn parse_toml_error(body: &str) -> String {
    match Manifest::parse_str(&format!("{PROJECT}\n{body}")) {
        Err(Error::ParseToml { detail, .. }) => detail,
        other => panic!("expected strict TOML shape failure, got {other:?}"),
    }
}

#[test]
fn plural_controls_parse_in_authored_order_beside_unchanged_declarations() {
    let manifest = parse(
        r#"
[[extension]]
id = "direct"
point = "phase:verify"
handler = { kind = "builtin", name = "log" }

[extensions]
disable = ["org.demo/tools#old", "__host__/demo#direct"]

[[extensions.use]]
ref = "org.demo/tools#first"
config = { mode = "compact" }

[[extensions.use]]
ref = "org.demo/tools#second"
"#,
    );

    assert_eq!(manifest.extensions[0].id, "direct");
    assert_eq!(
        manifest
            .extension_controls
            .uses
            .iter()
            .map(|entry| entry.reference.as_str())
            .collect::<Vec<_>>(),
        ["org.demo/tools#first", "org.demo/tools#second"]
    );
    assert_eq!(
        manifest
            .extension_controls
            .disable
            .iter()
            .map(ExtensionKey::as_str)
            .collect::<Vec<_>>(),
        ["org.demo/tools#old", "__host__/demo#direct"]
    );
    assert_eq!(
        manifest.extension_controls.uses[0]
            .config
            .as_ref()
            .unwrap()
            .as_table()["mode"]
            .as_str(),
        Some("compact")
    );
    assert!(manifest.extension_controls.uses[1].config.is_none());
}

#[test]
fn plural_controls_roundtrip_preserves_none_some_empty_and_opaque_spelling() {
    let manifest = parse(
        r#"
[extensions]
disable = ["", "  opaque/# key  "]

[[extensions.use]]
ref = "org.demo/tools#inherit"

[[extensions.use]]
ref = "org.demo/tools#clear"
config = {}

[[extensions.use]]
ref = "not/a/package#still-opaque"
config = { nan = nan, zero = -0.0, at = 1979-05-27T07:32:00Z, nested = { values = [1, 1.0, true] } }
"#,
    );

    assert!(manifest.extension_controls.uses[0].config.is_none());
    assert!(
        manifest.extension_controls.uses[1]
            .config
            .as_ref()
            .is_some_and(|config| config.is_empty())
    );
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    let disable = rendered.find("disable =").unwrap();
    let first_use = rendered.find("[[extensions.use]]").unwrap();
    assert!(
        disable < first_use,
        "scalar controls precede use rows:\n{rendered}"
    );
    assert!(!rendered.contains("[[extension.use]]"), "{rendered}");

    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, reparsed);
    assert!(reparsed.extension_controls.uses[0].config.is_none());
    assert!(
        reparsed.extension_controls.uses[1]
            .config
            .as_ref()
            .is_some_and(|config| config.is_empty())
    );
    assert_eq!(
        reparsed.extension_controls.disable[1].as_str(),
        "  opaque/# key  "
    );
}

#[test]
fn plural_control_wire_rejects_unknowns_and_wrong_shapes() {
    for (body, expected) in [
        ("[extensions]\nmystery = true\n", "mystery"),
        (
            "[[extensions.use]]\nref = \"org.demo/pkg#x\"\nmystery = true\n",
            "mystery",
        ),
        ("[[extensions.use]]\nconfig = {}\n", "ref"),
        (
            "[[extensions.use]]\nref = \"org.demo/pkg#x\"\nconfig = \"no\"\n",
            "map",
        ),
        ("[extensions]\ndisable = [\"ok\", 1]\n", "string"),
    ] {
        let detail = parse_toml_error(body);
        assert!(
            detail.contains(expected),
            "expected `{expected}` in strict-shape error:\n{detail}"
        );
    }
}

#[test]
fn pure_virtual_workspace_can_control_extensions_but_cannot_declare_one() {
    let controls = r#"
[extensions]
disable = ["org.demo/tools#old"]

[[extensions.use]]
ref = "org.demo/tools#announce"
"#;
    for role in [
        PROJECT,
        "[package]\ngroup = \"org.demo\"\nname = \"pkg\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        "[workspace]\nmembers = []\n",
    ] {
        let controlled = Manifest::parse_str(&format!("{role}\n{controls}")).unwrap();
        assert_eq!(controlled.extension_controls.uses.len(), 1);
        assert_eq!(controlled.extension_controls.disable.len(), 1);
    }

    let error = Manifest::parse_str(
        r#"[workspace]
members = []

[[extension]]
id = "announce"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("pure virtual `[workspace]`"), "{error}");
}

#[test]
fn duplicate_use_rows_remain_ordered_manifest_data_for_the_collector() {
    let manifest = parse(
        r#"
[[extensions.use]]
ref = "org.demo/tools#same"
config = { value = 1 }

[[extensions.use]]
ref = "org.demo/tools#same"
config = { value = 2 }
"#,
    );
    assert_eq!(manifest.extension_controls.uses.len(), 2);
    assert_eq!(
        manifest.extension_controls.uses[0]
            .config
            .as_ref()
            .unwrap()
            .as_table()["value"]
            .as_integer(),
        Some(1)
    );
    assert_eq!(
        manifest.extension_controls.uses[1]
            .config
            .as_ref()
            .unwrap()
            .as_table()["value"]
            .as_integer(),
        Some(2)
    );
}

#[test]
fn extension_keys_are_exact_opaque_values_with_closed_constructors() {
    let group = Group::parse("org.demo").unwrap();
    let name = PackageName::parse("tools").unwrap();
    let package = ExtensionKey::for_package(&group, &name, "announce#tail");
    assert_eq!(package.as_str(), "org.demo/tools#announce#tail");
    assert_eq!(package.to_string(), package.as_str());

    let host = ExtensionKey::for_host("odd/# project", "id#tail");
    assert_eq!(host.as_str(), "__host__/odd/# project#id#tail");
    assert_eq!(ExtensionKey::for_host("", "").as_str(), "__host__/#");

    let authored = ExtensionKey::authored("  not/a#package  ");
    assert_eq!(authored.as_str(), "  not/a#package  ");
}

#[test]
fn manifest_write_preserves_commented_plural_control_structure() {
    let project = tempdir().unwrap();
    let path = project.path().join(Manifest::FILENAME);
    fs::write(
        &path,
        r#"[project]
name = "demo"
version = "0.1.0"

# KEEP-CONTROLS-PREFIX
[extensions] # KEEP-CONTROLS-HEADER
# KEEP-DISABLE
disable = ["org.demo/tools#old"]

# KEEP-USE-PREFIX
[[extensions.use]] # KEEP-USE-HEADER
ref = "org.demo/tools#announce"

# KEEP-CONFIG-PREFIX
[extensions.use.config] # KEEP-CONFIG-HEADER
# KEEP-CONFIG-MESSAGE
message = "hello"
"#,
    )
    .unwrap();

    let mut manifest = Manifest::read(&path).unwrap();
    manifest.project.as_mut().unwrap().version = "0.2.0".into();
    manifest.write(&path).unwrap();

    let written = fs::read_to_string(&path).unwrap();
    for marker in [
        "KEEP-CONTROLS-PREFIX",
        "KEEP-CONTROLS-HEADER",
        "KEEP-DISABLE",
        "KEEP-USE-PREFIX",
        "KEEP-USE-HEADER",
        "KEEP-CONFIG-PREFIX",
        "KEEP-CONFIG-HEADER",
        "KEEP-CONFIG-MESSAGE",
    ] {
        assert!(written.contains(marker), "lost {marker}:\n{written}");
    }
    assert!(written.contains("version = \"0.2.0\""), "{written}");
    assert!(!written.contains("version = \"0.1.0\""), "{written}");
    assert_eq!(Manifest::read(&path).unwrap(), manifest);
}

#[test]
fn absent_plural_controls_leave_legacy_serialized_bytes_unchanged() {
    let manifest = Manifest::parse_str(PROJECT).unwrap();
    assert!(manifest.extension_controls.is_empty());
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    assert_eq!(
        rendered,
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\nauthors = []\n"
    );
    assert!(!rendered.contains("extensions"));
}
