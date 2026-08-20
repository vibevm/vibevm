//! Parity sentinel for every hand-maintained copy of the package-kind
//! vocabulary. `formats/vocabularies.json` is the declared domain; one test
//! aggregates every drift so adding a kind reports all lagging copies at once.

use std::collections::BTreeSet;
use std::path::PathBuf;

use vibe_core::PackageKind as CorePackageKind;
use vibe_wire::generated::shared::PackageKind as WirePackageKind;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

fn read_json(relative: &str) -> serde_json::Value {
    let text = read(relative);
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{relative} parses: {error}"))
}

fn domain() -> Vec<String> {
    read_json("formats/vocabularies.json")["package_kind"]["enum"]
        .as_array()
        .expect("formats/vocabularies.json package_kind.enum is an array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("every package_kind.enum member is a string")
                .to_owned()
        })
        .collect()
}

fn ordered(
    drift: &mut Vec<String>,
    copy: &str,
    path: &str,
    expected: &[String],
    actual: &[String],
) {
    if actual != expected {
        drift.push(format!(
            "{copy} [{path}]\n    expected (domain order): {expected:?}\n    actual:                  {actual:?}"
        ));
    }
}

fn unordered(
    drift: &mut Vec<String>,
    copy: &str,
    path: &str,
    expected: &[String],
    actual: &[String],
) {
    let expected_set: BTreeSet<&str> = expected.iter().map(String::as_str).collect();
    let actual_set: BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    if actual_set != expected_set || actual.len() != expected.len() {
        drift.push(format!(
            "{copy} [{path}]\n    expected (set): {expected_set:?}\n    actual:         {actual:?}"
        ));
    }
}

fn compact(text: &str) -> String {
    text.replace("///", "")
        .replace("//!", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn prose(drift: &mut Vec<String>, copy: &str, path: &str, expected_fragment: &str) {
    let body = compact(&read(path));
    let expected_fragment = compact(expected_fragment);
    if !body.contains(&expected_fragment) {
        drift.push(format!(
            "{copy} [{path}]\n    missing exact enumeration: {expected_fragment}"
        ));
    }
}

fn prose_count(
    drift: &mut Vec<String>,
    copy: &str,
    path: &str,
    expected_fragment: &str,
    expected_count: usize,
) {
    let body = compact(&read(path));
    let expected_fragment = compact(expected_fragment);
    let actual_count = body.matches(&expected_fragment).count();
    if actual_count != expected_count {
        drift.push(format!(
            "{copy} [{path}]\n    expected {expected_count} exact enumeration(s): {expected_fragment}\n    actual count: {actual_count}"
        ));
    }
}

fn string_array_after(path: &str, marker: &str) -> Vec<String> {
    let body = read(path);
    let tail = body
        .split_once(marker)
        .unwrap_or_else(|| panic!("{path} contains marker {marker:?}"))
        .1;
    let start = tail
        .find('[')
        .unwrap_or_else(|| panic!("{path} has an array after {marker:?}"));
    let end = tail[start..]
        .find(']')
        .map(|offset| start + offset + 1)
        .unwrap_or_else(|| panic!("{path} closes the array after {marker:?}"));
    serde_json::from_str(&tail[start..end])
        .unwrap_or_else(|error| panic!("{path} array after {marker:?} parses: {error}"))
}

fn linked_schema(drift: &mut Vec<String>, path: &str) {
    let document = read_json(path);
    let names = document["metadata"]["x-vocabularies"].as_array();
    let declares = names.is_some_and(|values| {
        values
            .iter()
            .any(|value| value.as_str() == Some("package_kind"))
    });
    let references = serde_json::to_string(&document)
        .expect("a parsed schema serialises")
        .contains(r#""ref":"package_kind""#);
    if !declares || !references {
        drift.push(format!(
            "linked schema [{path}]\n    expected metadata.x-vocabularies and a ref to package_kind; declares={declares}, references={references}"
        ));
    }
}

fn variant_name(wire: &str) -> String {
    let mut chars = wire.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn spec_kind_headings() -> Vec<String> {
    let body = read("VIBEVM-SPEC.md");
    let section = body
        .split_once("### 4.1 The installable kinds")
        .expect("VIBEVM-SPEC.md carries §4.1")
        .1
        .split_once("### 4.2 The directory layout")
        .expect("VIBEVM-SPEC.md carries §4.2")
        .0;
    section
        .lines()
        .filter_map(|line| {
            line.strip_prefix("**`")
                .and_then(|tail| tail.split_once("`** —"))
                .map(|(kind, _)| kind.to_owned())
        })
        .collect()
}

#[test]
fn package_kind_copies_match_the_declared_domain() {
    let expected = domain();
    let mut drift = Vec::new();

    let core_all: Vec<String> = CorePackageKind::ALL
        .iter()
        .map(|kind| kind.as_str().to_owned())
        .collect();
    ordered(
        &mut drift,
        "enum source: vibe_core::PackageKind::ALL",
        "crates/vibe-core/src/package_ref/kind.rs",
        &expected,
        &core_all,
    );

    let core_parser: Vec<String> = expected
        .iter()
        .filter_map(|wire| wire.parse::<CorePackageKind>().ok())
        .map(|kind| kind.as_str().to_owned())
        .collect();
    ordered(
        &mut drift,
        "enum behaviour: vibe_core::PackageKind FromStr/as_str",
        "crates/vibe-core/src/package_ref/kind.rs",
        &expected,
        &core_parser,
    );

    let generated_wire: Vec<String> = expected
        .iter()
        .filter_map(|wire| {
            let encoded = serde_json::to_string(wire).expect("a string serialises");
            let value: WirePackageKind =
                serde_json::from_str(&encoded).expect("the open wire vocabulary parses");
            (!matches!(value, WirePackageKind::Unknown(_))).then(|| value.as_str().to_owned())
        })
        .collect();
    ordered(
        &mut drift,
        "generated wire enum/serde named variants",
        "crates/vibe-wire/src/generated/shared.rs",
        &expected,
        &generated_wire,
    );

    let known: Vec<String> = WirePackageKind::known()
        .iter()
        .map(|kind| kind.as_str().to_owned())
        .collect();
    ordered(
        &mut drift,
        "behaviour: PackageKind::known()/as_str",
        "crates/vibe-wire/src/behaviour/vocabularies.rs",
        &expected,
        &known,
    );

    let scanner = read("crates/vibe-index/src/scanner/manifest.rs");
    let missing_scanner_arms: Vec<String> = expected
        .iter()
        .filter_map(|wire| {
            let variant = variant_name(wire);
            let arm = format!("CorePackageKind::{variant} => PackageKind::{variant}");
            (!scanner.contains(&arm)).then_some(wire.clone())
        })
        .collect();
    if !missing_scanner_arms.is_empty() {
        drift.push(format!(
            "behaviour: core -> index package_kind mapper [crates/vibe-index/src/scanner/manifest.rs]\n    missing domain values: {missing_scanner_arms:?}"
        ));
    }

    prose(
        &mut drift,
        "index type mirror is a re-export, not another enum copy",
        "crates/vibe-index/src/types/kinds.rs",
        "pub use vibe_wire::generated::shared::{NamingConvention, PackageKind};",
    );

    linked_schema(&mut drift, "schemas/list_report.jtd.json");
    linked_schema(&mut drift, "schemas/registry_sync_report.jtd.json");
    for path in [
        "schemas/index_cli/e1/capabilities_report.jtd.json",
        "schemas/index_cli/e1/outdated_report.jtd.json",
        "schemas/index_cli/e1/purls_report.jtd.json",
        "schemas/index_cli/e1/search_report.jtd.json",
    ] {
        linked_schema(&mut drift, path);
    }

    let tree_schema = read_json("crates/vibe-cli/resources/package-tree.schema.v1.json");
    let tree_kinds: Vec<String> = tree_schema["$defs"]["package"]["properties"]["kind"]["enum"]
        .as_array()
        .expect("package-tree kind enum is an array")
        .iter()
        .map(|value| value.as_str().expect("kind is a string").to_owned())
        .collect();
    ordered(
        &mut drift,
        "JSON Schema enum: package-tree package.kind",
        "crates/vibe-cli/resources/package-tree.schema.v1.json",
        &expected,
        &tree_kinds,
    );

    let prompt_kinds = string_array_after(
        "crates/vibe-cli/src/commands/init/prompts.rs",
        "let kind_items = vec!",
    );
    unordered(
        &mut drift,
        "interactive init package-kind choices",
        "crates/vibe-cli/src/commands/init/prompts.rs",
        &expected,
        &prompt_kinds,
    );

    let reference_kinds =
        string_array_after("xtask/src/batch_review/refs.rs", "const KINDS: &[&str] = &");
    ordered(
        &mut drift,
        "batch-review package-reference kinds",
        "xtask/src/batch_review/refs.rs",
        &expected,
        &reference_kinds,
    );

    let codegen_fixture =
        string_array_after("xtask/src/codegen/vocabulary/tests.rs", "json!({\"enum\": ");
    ordered(
        &mut drift,
        "codegen inline-package-kind witness",
        "xtask/src/codegen/vocabulary/tests.rs",
        &expected,
        &codegen_fixture,
    );

    let open_vocabulary_fixture =
        string_array_after("crates/vibe-wire/tests/open_vocabulary.rs", "for wire in ");
    unordered(
        &mut drift,
        "open-vocabulary named-value test fixture",
        "crates/vibe-wire/tests/open_vocabulary.rs",
        &expected,
        &open_vocabulary_fixture,
    );

    let plain = expected.join(", ");
    let ticked = expected
        .iter()
        .map(|kind| format!("`{kind}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let slashed = expected.join("` / `");

    prose_count(
        &mut drift,
        "core BadPackageKind message + its doctest",
        "crates/vibe-core/src/error.rs",
        &format!("must be one of: {plain}"),
        2,
    );
    prose(
        &mut drift,
        "vibe init --kind help",
        "crates/vibe-cli/src/cli/pkg.rs",
        &format!("Package kind: {plain}. Default: tool."),
    );
    prose(
        &mut drift,
        "vibe list --kind help",
        "crates/vibe-cli/src/cli/pkg.rs",
        &format!("Filter by package kind ({plain})."),
    );
    prose(
        &mut drift,
        "vibe search --kind help",
        "crates/vibe-cli/src/cli/pkg.rs",
        &format!("Restrict results to a single package kind ({ticked})."),
    );
    prose(
        &mut drift,
        "vibe top-level long help",
        "crates/vibe-cli/src/cli.rs",
        &format!("Manages installable building blocks — {plain} — and assembles"),
    );
    prose(
        &mut drift,
        "vibe-index list --kind help",
        "crates/vibe-index/src/cli/list.rs",
        &format!("Keep only packages of this kind: {plain}. The wire vocabulary"),
    );
    prose(
        &mut drift,
        "vibe-index search --kind help",
        "crates/vibe-index/src/cli/search.rs",
        &format!("Keep only hits of this kind: {plain}. The wire vocabulary"),
    );
    prose(
        &mut drift,
        "MCP skill-template package-kind list",
        "crates/vibe-mcp/src/skill_template.md",
        &format!("a package of any kind ({ticked}) can carry skills"),
    );
    prose(
        &mut drift,
        "nullable index-list schema prose",
        "schemas/index_cli/e1/list_report.jtd.json",
        &format!("the open vocabulary ({plain}, or any future kind"),
    );

    prose(
        &mut drift,
        "PROP-000 package identity list",
        "spec/common/PROP-000.md",
        &format!("kind ∈ {{{plain}}}"),
    );
    prose(
        &mut drift,
        "PROP-000 vocabulary invariant list",
        "spec/common/PROP-000.md",
        &format!("The installable kinds are {ticked}"),
    );
    prose(
        &mut drift,
        "boot core terminology list",
        "spec/boot/00-core.md",
        &format!("only six installable kinds — {ticked}"),
    );

    unordered(
        &mut drift,
        "VIBEVM-SPEC §4.1 installable-kind headings",
        "VIBEVM-SPEC.md",
        &expected,
        &spec_kind_headings(),
    );
    prose(
        &mut drift,
        "VIBEVM-SPEC package-identity inline list",
        "VIBEVM-SPEC.md",
        &format!("`kind` (`{slashed}`) stays"),
    );
    prose(
        &mut drift,
        "VIBEVM-SPEC manifest-example inline list",
        "VIBEVM-SPEC.md",
        &format!("one of: {plain} — metadata, not identity"),
    );
    prose(
        &mut drift,
        "VIBEVM-SPEC glossary Kind list",
        "VIBEVM-SPEC.md",
        &format!("**Kind.** One of {ticked}. The category of a package."),
    );

    assert!(
        drift.is_empty(),
        "package_kind vocabulary drift from formats/vocabularies.json; every lagging copy is named:\n\n{}",
        drift.join("\n\n")
    );
}
