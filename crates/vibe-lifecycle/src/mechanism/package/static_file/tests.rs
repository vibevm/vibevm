use std::path::PathBuf;

use sha2::{Digest, Sha256};
use specmark::verifies;
use vibe_core::manifest::{ArtifactInput, ArtifactKind, ArtifactOutput, ArtifactPackageTarget};

use super::*;
use crate::mechanism::package::protocol::{InputOrigin, ResolvedInput};
use crate::mechanism::package::support::{config, key, run_default, temp};
use crate::{PackageError, PackageExecution};

fn target(input: &str, output: &str) -> ArtifactPackageTarget {
    ArtifactPackageTarget {
        id: "launcher".to_owned(),
        mechanism: key("package:static-file"),
        provider: None,
        inputs: Some(vec![ArtifactInput::Path {
            path: PathBuf::from(input),
        }]),
        outputs: vec![ArtifactOutput {
            id: output.to_owned(),
            kind: ArtifactKind::File,
            select: None,
        }],
        config: None,
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn an_opaque_workspace_file_is_copied_exactly_and_recorded() {
    let root = temp();
    let source = root.path().join("launchers/claudez.ps1");
    std::fs::create_dir_all(source.parent().expect("a parent")).expect("the source root creates");
    let bytes = b"#!/opaque\r\n\0\xff";
    std::fs::write(&source, bytes).expect("the opaque source writes");

    let mut declared = target("launchers/claudez.ps1", "claudez.ps1");
    declared.config = Some(config(""));
    let outcomes = run_default(root.path(), &[declared]).expect("the static file packages");

    assert_eq!(outcomes[0].provider, BUILTIN_STATIC_FILE_PIN);
    assert_eq!(outcomes[0].produced.len(), 1);
    let produced = &outcomes[0].produced[0];
    assert_eq!(produced.id, "claudez.ps1");
    assert_eq!(produced.files, 1);
    assert_eq!(produced.bytes, bytes.len() as u64);
    assert_eq!(
        std::fs::read(root.path().join(&produced.path_relative)).expect("the output reads"),
        bytes,
    );
    assert!(root.path().join(&produced.record).is_file());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn unsafe_filename_and_nonempty_config_refuse_before_the_output_root_exists() {
    let root = temp();
    std::fs::write(root.path().join("source.ps1"), b"bytes").expect("the source writes");
    let unsafe_target = target("source.ps1", "../escape.ps1");
    let error = run_default(root.path(), &[unsafe_target]).expect_err("traversal refuses");
    assert!(matches!(
        error,
        PackageError::Provider(MechanismError::Config { .. })
    ));
    assert!(!root.path().join("target").exists());

    let mut configured = target("source.ps1", "safe.ps1");
    configured.config = Some(config("mode = \"copy\""));
    let error = run_default(root.path(), &[configured]).expect_err("nonempty config refuses");
    assert!(matches!(
        error,
        PackageError::Provider(MechanismError::Config { .. })
    ));
    assert!(!root.path().join("target").exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn invalid_input_and_output_shapes_refuse_before_a_write() {
    let root = temp();
    std::fs::create_dir_all(root.path().join("directory")).expect("the directory creates");
    let error = run_default(root.path(), &[target("directory", "copy.bin")])
        .expect_err("a directory is not a file input");
    assert!(matches!(error, PackageError::InputSourceMissing { .. }));
    assert!(!root.path().join("target").exists());

    std::fs::write(root.path().join("source.bin"), b"bytes").expect("the source writes");
    let mut wrong_kind = target("source.bin", "copy.bin");
    wrong_kind.outputs[0].kind = ArtifactKind::Archive;
    let error = run_default(root.path(), &[wrong_kind]).expect_err("a non-file kind refuses");
    assert!(matches!(
        error,
        PackageError::Provider(MechanismError::UnsupportedKind { .. })
    ));
    assert!(!root.path().join("target").exists());

    let path = root.path().join("recorded.bin");
    std::fs::write(&path, b"bytes").expect("the recorded-shaped input writes");
    let recorded = [ResolvedInput {
        name: "recorded.bin".to_owned(),
        reference: "artifact:recorded.bin".to_owned(),
        absolute: path,
        relative: "recorded.bin".to_owned(),
        digest: format!("{:x}", Sha256::digest(b"bytes")),
        bytes: 5,
        shape: vibe_wire::generated::artifact_record::ArtifactShape::File,
        origin: InputOrigin::ArtifactRecord {
            kind: ArtifactKind::File,
        },
    }];
    let row = target("source.bin", "copy.bin");
    let request = PackageTargetRequest {
        target: &row,
        project_root: root.path(),
        package_root: PackageExecution::default_package_root(),
        inputs: &recorded,
    };
    let error = StaticFileProvider
        .plan(&request)
        .expect_err("a recorded artifact is not a workspace path");
    assert!(matches!(
        error,
        MechanismError::ArtifactInputRejected { .. }
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn a_hard_link_source_refuses_before_the_existing_output_is_reset() {
    let root = temp();
    std::fs::write(root.path().join("source.bin"), b"opaque").expect("the source writes");
    std::fs::hard_link(
        root.path().join("source.bin"),
        root.path().join("second-name.bin"),
    )
    .expect("the second hard-link name creates");
    let output = root.path().join("target/vibe-package/launcher");
    std::fs::create_dir_all(&output).expect("the prior output directory creates");
    std::fs::write(output.join("sentinel"), b"keep").expect("the sentinel writes");

    let error = run_default(root.path(), &[target("second-name.bin", "launcher.bin")])
        .expect_err("a multiply-linked source refuses");

    assert!(matches!(
        error,
        PackageError::Provider(MechanismError::PackageWrite { .. })
    ));
    assert_eq!(std::fs::read(output.join("sentinel")).unwrap(), b"keep");
    assert!(!output.join("launcher.bin").exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY")]
fn a_source_changed_after_resolution_refuses_without_resetting_prior_output() {
    let root = temp();
    let source = root.path().join("source.bin");
    std::fs::write(&source, b"resolved").expect("the resolved source writes");
    let resolved = [ResolvedInput {
        name: "source.bin".to_owned(),
        reference: "path:source.bin".to_owned(),
        absolute: source.clone(),
        relative: "source.bin".to_owned(),
        digest: format!("{:x}", Sha256::digest(b"resolved")),
        bytes: 8,
        shape: vibe_wire::generated::artifact_record::ArtifactShape::File,
        origin: InputOrigin::WorkspacePath,
    }];
    let row = target("source.bin", "launcher.bin");
    let request = PackageTargetRequest {
        target: &row,
        project_root: root.path(),
        package_root: PackageExecution::default_package_root(),
        inputs: &resolved,
    };
    let plan = StaticFileProvider
        .plan(&request)
        .expect("the resolved target plans");
    let output = root.path().join("target/vibe-package/launcher");
    std::fs::create_dir_all(&output).expect("the prior output creates");
    std::fs::write(output.join("sentinel"), b"keep").expect("the sentinel writes");
    std::fs::write(&source, b"changed!").expect("the source changes after resolution");

    let error = StaticFileProvider
        .apply(&request, &plan)
        .expect_err("the expected-state gate refuses before reset");

    assert!(matches!(error, MechanismError::PackageWrite { .. }));
    assert_eq!(std::fs::read(output.join("sentinel")).unwrap(), b"keep");
    assert!(!output.join("launcher.bin").exists());
}
