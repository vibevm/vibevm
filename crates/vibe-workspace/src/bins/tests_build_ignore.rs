//! Build preflight tests that must prove failure before any Cargo spawn.

use std::path::PathBuf;

use super::{BinsError, DeclaredBinary, build_binary, prepare_build_output_ignores};
use crate::vibedeps;

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build")]
fn build_preflight_uses_the_authoritative_dependency_root_without_spawning_cargo() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("nested").join(vibedeps::VIBEDEPS_DIR);
    let slot = root.join("org.demo.tool").join("1.0.0");
    std::fs::create_dir_all(&slot).unwrap();
    let bin = declared_at(root.clone(), slot);

    prepare_build_output_ignores(&bin).unwrap();
    let ignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(
        vibedeps::BUILD_OUTPUT_IGNORES
            .iter()
            .all(|rule| ignore.lines().any(|line| line == *rule)),
        "{ignore}"
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build")]
fn malformed_binary_slot_fails_before_cargo_with_the_typed_layout_error() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir
        .path()
        .join("authoritative")
        .join(vibedeps::VIBEDEPS_DIR);
    let bin = declared_at(
        root,
        dir.path().join("not-the-dependency-root").join("slot"),
    );

    let error = build_binary(&bin, false).unwrap_err();
    assert!(matches!(error, BinsError::MalformedSlot { .. }));
    let message = error.to_string();
    assert!(message.contains(vibedeps::VIBEDEPS_DIR), "{message}");
    assert!(message.contains("vibe install"), "{message}");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build")]
fn unrelated_ancestor_named_vibedeps_cannot_override_the_authoritative_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("project").join(vibedeps::VIBEDEPS_DIR);
    let unrelated = dir
        .path()
        .join(vibedeps::VIBEDEPS_DIR)
        .join("org.demo.tool")
        .join("1.0.0");
    let error = build_binary(&declared_at(root, unrelated), false).unwrap_err();
    assert!(matches!(error, BinsError::MalformedSlot { .. }));
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build")]
fn slot_depth_must_be_exactly_coordinate_then_version() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(vibedeps::VIBEDEPS_DIR);
    for slot in [
        root.join("org.demo.tool"),
        root.join("org.demo.tool").join("1.0.0").join("extra"),
    ] {
        let error = build_binary(&declared_at(root.clone(), slot), false).unwrap_err();
        assert!(matches!(error, BinsError::MalformedSlot { .. }));
    }

    let wrong_root = dir.path().join("dependencies");
    let slot = wrong_root.join("org.demo.tool").join("1.0.0");
    let error = build_binary(&declared_at(wrong_root, slot), false).unwrap_err();
    assert!(matches!(error, BinsError::MalformedSlot { .. }));
}

fn declared_at(vibedeps_root: PathBuf, slot: PathBuf) -> DeclaredBinary {
    DeclaredBinary {
        decl: vibe_core::manifest::BinaryDecl {
            name: "fixture-tool".into(),
            crate_dir: "crates/fixture-tool".into(),
            description: None,
        },
        package: "org.vibevm/fixture".into(),
        group: "org.vibevm".into(),
        vibedeps_root,
        slot,
    }
}
