//! §7.0.7's `[[binary]]` lowering, at the assembly that arms the fences.
//!
//! > "The same assembly that arms the fences lowers legacy `[[binary]]`
//! > rows through the R8-CARGO projection into the build target set; an id
//! > collision between a lowered row and an authored `[[artifacts.build]]`
//! > row is a typed refusal (two claimants for one identity), never a
//! > silent merge."

use specmark::verifies;
use vibe_core::manifest::{
    ArtifactBuildTarget, ArtifactKind, ArtifactOutput, ArtifactsSection, BinaryDecl, MechanismKey,
};

use super::mechanism::lower_binaries;

/// One legacy `[[binary]]` row.
fn binary(name: &str) -> BinaryDecl {
    BinaryDecl {
        name: name.to_owned(),
        crate_dir: std::path::PathBuf::from("crates/tool"),
        description: None,
    }
}

/// One authored `[[artifacts.build]]` row with the given ids.
fn authored(id: &str, output: &str) -> ArtifactsSection {
    let mechanism: MechanismKey = match "build:cargo".parse() {
        Ok(key) => key,
        Err(error) => panic!("the fixture key parses: {error}"),
    };
    ArtifactsSection {
        build: vec![ArtifactBuildTarget {
            id: id.to_owned(),
            mechanism,
            provider: None,
            workdir: ".".to_owned(),
            inputs: None,
            outputs: vec![ArtifactOutput {
                id: output.to_owned(),
                kind: ArtifactKind::Executable,
                select: None,
            }],
            config: None,
        }],
        package: Vec::new(),
    }
}

/// A project with no `[[binary]]` gets exactly its authored rows back —
/// the historical set, byte for byte.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_project_with_no_binary_row_lowers_nothing() {
    let artifacts = authored("helper", "helper.exe");

    let lowered = lower_binaries(Some(&artifacts), &[]).expect("nothing to lower");

    assert_eq!(lowered, artifacts.build);
}

/// A `[[binary]]` becomes an ordinary build target, so the executor has
/// no legacy case at all.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_binary_row_lowers_into_an_ordinary_build_target() {
    let lowered = lower_binaries(None, &[binary("tool")]).expect("the row lowers");

    assert_eq!(lowered.len(), 1);
    assert_eq!(lowered[0].id, "tool");
    assert_eq!(lowered[0].mechanism.to_string(), "build:cargo");
    assert_eq!(lowered[0].workdir, "crates/tool");
    assert_eq!(lowered[0].outputs.len(), 1);
    assert_eq!(lowered[0].outputs[0].id, "tool");
    assert_eq!(lowered[0].outputs[0].kind, ArtifactKind::Executable);
    assert!(lowered[0].validate().is_ok(), "and it is a legal target");
}

/// Authored rows come FIRST and the lowered ones after, so a project that
/// declares both keeps its authored order.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn authored_rows_keep_their_order_and_lowered_rows_follow() {
    let artifacts = authored("helper", "helper.exe");

    let lowered = lower_binaries(Some(&artifacts), &[binary("tool")]).expect("both survive");

    assert_eq!(
        lowered
            .iter()
            .map(|target| target.id.as_str())
            .collect::<Vec<_>>(),
        ["helper", "tool"],
    );
}

/// The law: two claimants for one identity refuse, and the refusal names
/// the row it refused.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_lowered_row_colliding_with_an_authored_target_id_refuses() {
    let artifacts = authored("tool", "tool.exe");

    let error = lower_binaries(Some(&artifacts), &[binary("tool")])
        .expect_err("two claimants for `tool` are never merged");

    let rendered = format!("{error:#}");
    assert!(rendered.contains("`tool`"), "{rendered}");
    assert!(
        rendered.contains("two claimants for one identity are never merged"),
        "{rendered}",
    );
    assert!(
        rendered.contains("PROP-054#ARTIFACT-REGISTRY"),
        "{rendered}"
    );
}

/// The same law over the OUTPUT identity: the projection mints an output
/// id from the binary's name too, so an authored output claiming that
/// name is the same collision.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_lowered_row_colliding_with_an_authored_output_id_refuses() {
    let artifacts = authored("helper", "tool");

    let error = lower_binaries(Some(&artifacts), &[binary("tool")])
        .expect_err("an authored OUTPUT already claims `tool`");

    assert!(format!("{error:#}").contains("`tool`"));
}

/// Two `[[binary]]` rows with one name are the same collision — a
/// manifest cannot reach it (names are unique within a package), and a
/// programmatically built set can.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn two_binary_rows_with_one_name_refuse() {
    let error = lower_binaries(None, &[binary("tool"), binary("tool")])
        .expect_err("one identity, one producer");

    assert!(format!("{error:#}").contains("`tool`"));
}
