//! The field table a schema reference produces.

use std::fs;
use std::path::Path;

use super::generate;

fn tree(root: &Path) {
    fs::create_dir_all(root.join("schemas")).expect("mkdir");
    fs::create_dir_all(root.join("formats")).expect("mkdir");
    fs::write(
        root.join("schemas/demo_report.jtd.json"),
        r#"{
          "properties": {
            "ok": { "type": "boolean", "metadata": { "description": "Always true." } },
            "packages": { "elements": { "ref": "entry" } }
          },
          "optionalProperties": {
            "registry": { "type": "string", "nullable": true }
          },
          "definitions": {
            "entry": {
              "properties": {
                "kind": { "enum": ["flow", "doc"] },
                "files": { "elements": { "type": "string" } }
              }
            }
          }
        }"#,
    )
    .expect("write schema");
    fs::write(
        root.join("formats/REGISTRY.toml"),
        "[format.demo-report]\nepoch = 1\nschema = \"schemas/demo_report.jtd.json\"\n\
         [format.unschemed]\nepoch = 1\nschema = \"none\"\n",
    )
    .expect("write registry");
}

#[test]
fn a_path_and_its_format_id_produce_the_same_table() {
    let tmp = tempfile::tempdir().expect("tempdir");
    tree(tmp.path());
    let by_path = generate("schemas/demo_report.jtd.json", tmp.path()).expect("by path");
    let by_id = generate("demo-report", tmp.path()).expect("by id");
    assert_eq!(by_path, by_id);
}

#[test]
fn the_table_names_every_member_with_its_obligation_and_its_form() {
    let tmp = tempfile::tempdir().expect("tempdir");
    tree(tmp.path());
    let table = generate("demo-report", tmp.path()).expect("renders");
    assert!(
        table.contains("| `ok` | yes | boolean | Always true. |"),
        "{table}"
    );
    assert!(
        table.contains("| `packages` | yes | list of `entry` |"),
        "{table}"
    );
    assert!(
        table.contains("| `registry` | no | string or null |"),
        "an optional nullable member says both: {table}"
    );
}

#[test]
fn a_definition_gets_a_table_of_its_own() {
    let tmp = tempfile::tempdir().expect("tempdir");
    tree(tmp.path());
    let table = generate("demo-report", tmp.path()).expect("renders");
    assert!(table.contains("`entry`\n"), "{table}");
    assert!(
        table.contains("| `kind` | yes | one of `flow`, `doc` |"),
        "{table}"
    );
}

#[test]
fn the_same_tree_renders_the_same_bytes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    tree(tmp.path());
    assert_eq!(
        generate("demo-report", tmp.path()).expect("first"),
        generate("demo-report", tmp.path()).expect("second")
    );
}

#[test]
fn a_format_nobody_inventoried_has_no_address() {
    let tmp = tempfile::tempdir().expect("tempdir");
    tree(tmp.path());
    let e = generate("ghost-report", tmp.path()).expect_err("refused");
    assert!(e.to_string().contains("not inventoried"), "{e}");
}

#[test]
fn an_inventoried_format_without_a_schema_says_so_rather_than_rendering_nothing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    tree(tmp.path());
    let e = generate("unschemed", tmp.path()).expect_err("refused");
    assert!(e.to_string().contains("no schema"), "{e}");
}
