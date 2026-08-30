//! The `verify` half's refusal laws, pinned directly — the e2e proves the
//! happy path, and nothing else exercised these arms: reviewer mutation
//! showed the whole containment check deletable with every suite green.
//!
//! `verify` receives exactly what the message stream carried, and that
//! stream is another process's output: the three refusals below are what
//! stand between "cargo said so" and "the engine minted an identity for
//! it". Containment first (an artifact outside the engine-owned build root
//! has no project-relative identity to mint), then the link law (the final
//! component must be the produced file, not a name for something else),
//! then plain absence.

use std::path::PathBuf;

use super::super::BuildTargetRequest;
use super::super::error::MechanismError;
use super::{BuildProvider, CargoProvider, SelectedArtifact};

use super::plan_tests::target;

fn selected(executable: PathBuf) -> SelectedArtifact {
    SelectedArtifact {
        output_id: "app.exe".to_owned(),
        kind: vibe_core::manifest::ArtifactKind::Executable,
        executable,
        fresh: false,
        package_id: "app 0.1.0".to_owned(),
        bin: "app".to_owned(),
    }
}

/// An executable the message stream placed OUTSIDE the engine-owned build
/// root refuses with the containment arm — never digested, never recorded.
///
/// The fixture path sits INSIDE the project but outside `target/`,
/// deliberately: a fully foreign path also fails the later
/// project-relative step, so only this shape isolates the build-root law
/// itself — the reviewer mutation that deleted the check stayed green
/// until the fixture took this form.
#[test]
fn an_artifact_outside_the_engine_owned_build_root_refuses() {
    let project = tempfile::TempDir::new().expect("a temp project");
    let src = project.path().join("src");
    std::fs::create_dir_all(&src).expect("the source dir");
    let foreign_file = src.join("app.exe");
    std::fs::write(&foreign_file, b"bytes").expect("the in-project non-artifact writes");

    let fixture = target("app");
    let request = BuildTargetRequest {
        target: &fixture,
        project_root: project.path(),
        build_root: "target",
        offline: true,
    };
    let error = CargoProvider
        .verify(&request, &selected(foreign_file))
        .expect_err("an artifact outside the build root has no identity to mint");
    assert!(
        matches!(error, MechanismError::OutputOutsideBuildRoot { .. }),
        "the containment arm names the refusal: {error}"
    );
}

/// A reported path whose final component is a symlink refuses through the
/// link law — the produced file, not a name for something else.
#[cfg(unix)]
#[test]
fn a_symlinked_final_component_refuses() {
    let project = tempfile::TempDir::new().expect("a temp project");
    let dir = project.path().join("target").join("release");
    std::fs::create_dir_all(&dir).expect("the build root");
    let real = dir.join("real.exe");
    std::fs::write(&real, b"bytes").expect("writes");
    let link = dir.join("app.exe");
    std::os::unix::fs::symlink(&real, &link).expect("links");

    let fixture = target("app");
    let request = BuildTargetRequest {
        target: &fixture,
        project_root: project.path(),
        build_root: "target",
        offline: true,
    };
    let error = CargoProvider
        .verify(&request, &selected(link))
        .expect_err("a link is not the produced file");
    assert!(matches!(error, MechanismError::OutputMissing { .. }));
}

/// A reported path that does not exist refuses plainly — the message said
/// so, the filesystem disagrees, and the engine believes the filesystem.
#[test]
fn a_missing_reported_artifact_refuses() {
    let project = tempfile::TempDir::new().expect("a temp project");
    let ghost = project
        .path()
        .join("target")
        .join("release")
        .join("app.exe");

    let fixture = target("app");
    let request = BuildTargetRequest {
        target: &fixture,
        project_root: project.path(),
        build_root: "target",
        offline: true,
    };
    let error = CargoProvider
        .verify(&request, &selected(ghost))
        .expect_err("a reported artifact must exist");
    assert!(matches!(error, MechanismError::OutputMissing { .. }));
}
