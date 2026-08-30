//! `[[binary]]` → build-target projection laws (§5.0.6).
//!
//! The first test is the field-for-field equivalence pin: it names every
//! member of the produced [`ArtifactBuildTarget`] and the exact value the
//! lowering owes it, so a silent change to any one of them is a red line
//! rather than a diff nobody reads.

use specmark::verifies;

use super::*;

/// Parse one `[[binary]]` row exactly as a manifest carries it.
fn binary(toml_row: &str) -> BinaryDecl {
    match toml::from_str(toml_row) {
        Ok(declaration) => declaration,
        Err(error) => panic!("the fixture row parses: {error}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn projection_is_field_for_field_equivalent() {
    let declaration = binary(concat!(
        "name = \"vibe-helper\"\n",
        "crate = \"crates/vibe-helper\"\n",
        "description = \"the helper tool\"\n",
    ));

    let target = build_target_for_binary(&declaration);

    assert_eq!(target.id, "vibe-helper");
    assert_eq!(target.mechanism.to_string(), "build:cargo");
    assert_eq!(target.provider, None);
    assert_eq!(target.workdir, "crates/vibe-helper");
    assert_eq!(target.inputs, None);
    assert_eq!(target.config, None);
    assert_eq!(target.outputs.len(), 1);

    let output = &target.outputs[0];
    assert_eq!(output.id, "vibe-helper");
    assert_eq!(output.kind, ArtifactKind::Executable);
    let select = match output.select.as_ref() {
        Some(select) => select.as_table(),
        None => panic!("the executable output selects a `[[bin]]` target"),
    };
    assert_eq!(select.len(), 1);
    assert_eq!(select["bin"].as_str(), Some("vibe-helper"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn the_projected_target_answers_the_incumbent_row_law() {
    let target = build_target_for_binary(&binary(
        "name = \"rust-ai-native-conform\"\ncrate = \"crates/conform-cli\"",
    ));

    assert!(target.validate().is_ok());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn description_is_dropped_because_the_target_grammar_has_no_member_for_it() {
    let with_prose = build_target_for_binary(&binary(
        "name = \"x\"\ncrate = \"crates/x\"\ndescription = \"prose\"",
    ));
    let without_prose = build_target_for_binary(&binary("name = \"x\"\ncrate = \"crates/x\""));

    assert_eq!(with_prose, without_prose);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_backslashed_crate_dir_lowers_to_the_one_forward_slashed_spelling() {
    let declaration = BinaryDecl {
        name: "x".to_owned(),
        crate_dir: std::path::PathBuf::from(r"crates\nested\x"),
        description: None,
    };

    let target = build_target_for_binary(&declaration);

    assert_eq!(target.workdir, "crates/nested/x");
    assert!(target.validate().is_ok());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn an_empty_crate_dir_lowers_to_the_declaring_root_itself() {
    let declaration = BinaryDecl {
        name: "x".to_owned(),
        crate_dir: std::path::PathBuf::new(),
        description: None,
    };

    assert_eq!(build_target_for_binary(&declaration).workdir, ".");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn role_and_name_are_the_literal() {
    // The pin behind `cargo_key`'s recorded deviation: the module's own
    // literal really is a role and a portable token, so the parse the
    // projection performs cannot fail.
    let key = match BUILD_CARGO.parse::<MechanismKey>() {
        Ok(key) => key,
        Err(error) => panic!("`build:cargo` parses: {error}"),
    };
    assert_eq!(key.role(), crate::manifest::MechanismRole::Build);
    assert_eq!(key.name(), "cargo");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_non_portable_binary_name_refuses_through_the_incumbent_law() {
    // The projection is deliberately unvalidating: an illegal `[[binary]]`
    // name produces a target the ONE row law refuses, rather than a second
    // grammar check here that could drift from it.
    let declaration = BinaryDecl {
        name: "Vibe Helper".to_owned(),
        crate_dir: std::path::PathBuf::from("crates/x"),
        description: None,
    };

    let target = build_target_for_binary(&declaration);

    assert!(target.validate().is_err());
}
