//! `plan`'s two laws: it reports the argv it WOULD run, and it does
//! nothing else.
//!
//! The purity test is a real filesystem observation rather than a promise
//! in a comment: it walks the project tree before and after, and any
//! created path — a `target/` directory, a `Cargo.lock`, a state file —
//! fails it. That is the shape mutation 2 of the packet turns red by
//! moving the build into `plan`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::{
    ArtifactBuildTarget, ArtifactInput, ArtifactKind, ArtifactOutput, ExtensionConfig,
};

use super::*;

pub(crate) fn key(spelling: &str) -> vibe_core::manifest::MechanismKey {
    match spelling.parse() {
        Ok(parsed) => parsed,
        Err(error) => panic!("`{spelling}` is a mechanism key: {error}"),
    }
}

pub(crate) fn config(toml_text: &str) -> ExtensionConfig {
    match toml_text.parse::<toml::Table>() {
        Ok(parsed) => ExtensionConfig::from_table(parsed),
        Err(error) => panic!("the fixture table parses: {error}"),
    }
}

/// One executable-producing build target in the canonical shape.
pub(crate) fn target(id: &str) -> ArtifactBuildTarget {
    ArtifactBuildTarget {
        id: id.to_owned(),
        mechanism: key("build:cargo"),
        provider: None,
        workdir: ".".to_owned(),
        inputs: Some(vec![
            ArtifactInput::Path {
                path: PathBuf::from("Cargo.toml"),
            },
            ArtifactInput::Path {
                path: PathBuf::from("src/**"),
            },
        ]),
        outputs: vec![ArtifactOutput {
            id: format!("{id}.exe"),
            kind: ArtifactKind::Executable,
            select: Some(config("bin = \"vibe-r8-fixture\"")),
        }],
        config: Some(config("profile = \"release\"\noffline = true")),
    }
}

fn request<'a>(target: &'a ArtifactBuildTarget, root: &'a Path) -> BuildTargetRequest<'a> {
    BuildTargetRequest {
        target,
        project_root: root,
        build_root: crate::mechanism::DEFAULT_BUILD_ROOT,
        offline: true,
    }
}

/// Every path below a root, relative and sorted — the observation the
/// purity law is judged by.
fn tree(root: &Path) -> BTreeSet<String> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .path()
                .strip_prefix(root)
                .ok()
                .map(|path| path.display().to_string().replace('\\', "/"))
        })
        .filter(|path| !path.is_empty())
        .collect()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn plan_reports_the_argv_it_would_run() {
    let root = match TempDir::new() {
        Ok(root) => root,
        Err(error) => panic!("a temp project opens: {error}"),
    };
    let declared = target("vibe-helper");
    let plan = match CargoProvider.plan(&request(&declared, root.path())) {
        Ok(plan) => plan,
        Err(error) => panic!("the target plans: {error}"),
    };

    assert_eq!(plan.build_argv[0], "build");
    assert_eq!(
        plan.build_argv[1],
        "--message-format=json-render-diagnostics"
    );
    assert_eq!(plan.build_argv[2], "--target-dir");
    assert_eq!(
        PathBuf::from(&plan.build_argv[3]),
        root.path().join("target")
    );
    assert!(plan.build_argv.contains(&"--profile".to_owned()));
    assert!(plan.build_argv.contains(&"release".to_owned()));
    assert!(plan.build_argv.contains(&"--offline".to_owned()));
    assert_eq!(
        plan.metadata_argv,
        vec![
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline"
        ]
    );
    assert_eq!(plan.workdir, root.path());
    assert_eq!(plan.target_dir, root.path().join("target"));
    assert_eq!(plan.inputs, vec!["path:Cargo.toml", "path:src/**"]);
    assert_eq!(plan.outputs.len(), 1);
    assert_eq!(plan.outputs[0].id, "vibe-helper.exe");
    assert_eq!(
        plan.outputs[0].select.bin.as_deref(),
        Some("vibe-r8-fixture")
    );
    assert!(!plan.network, "an offline target reaches nothing");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn plan_has_no_side_effect() {
    let root = match TempDir::new() {
        Ok(root) => root,
        Err(error) => panic!("a temp project opens: {error}"),
    };
    if let Err(error) = std::fs::write(root.path().join("Cargo.toml"), "[package]\n") {
        panic!("the fixture file writes: {error}");
    }
    let before = tree(root.path());

    let declared = target("vibe-helper");
    let plan = CargoProvider.plan(&request(&declared, root.path()));
    assert!(plan.is_ok(), "the target plans");

    assert_eq!(
        tree(root.path()),
        before,
        "`plan` validates, resolves and reports — it builds nothing",
    );
    assert!(!root.path().join("target").exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_workdir_joins_the_declarant_relative_segments() {
    let root = match TempDir::new() {
        Ok(root) => root,
        Err(error) => panic!("a temp project opens: {error}"),
    };
    let mut declared = target("vibe-helper");
    declared.workdir = "crates/helper".to_owned();

    let plan = match CargoProvider.plan(&request(&declared, root.path())) {
        Ok(plan) => plan,
        Err(error) => panic!("the target plans: {error}"),
    };

    assert_eq!(plan.workdir, root.path().join("crates").join("helper"));
    // The output root stays the ENGINE's, not the workdir's.
    assert_eq!(plan.target_dir, root.path().join("target"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_output_kind_this_provider_cannot_produce_refuses_at_plan() {
    let root = match TempDir::new() {
        Ok(root) => root,
        Err(error) => panic!("a temp project opens: {error}"),
    };
    let mut declared = target("vibe-helper");
    declared.outputs[0].kind = ArtifactKind::Archive;

    let refusal = CargoProvider
        .plan(&request(&declared, root.path()))
        .expect_err("a Cargo build target produces executables");

    match &refusal {
        MechanismError::UnsupportedKind {
            kind, supported, ..
        } => {
            assert_eq!(kind, "archive");
            assert_eq!(supported, "executable");
        }
        other => panic!("expected an unsupported-kind refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_descriptor_declares_the_section_three_two_posture() {
    let descriptor = CargoProvider.descriptor();

    assert_eq!(descriptor.key, "org.vibevm/vibe#cargo");
    assert!(descriptor.supports(ArtifactKind::Executable));
    assert!(!descriptor.supports(ArtifactKind::Directory));
    assert_eq!(
        descriptor.posture(),
        "provider org.vibevm/vibe#cargo effect=workspace network=when-online privilege=none \
         reversibility=n/a ops=plan+fingerprint+apply+verify",
    );
}
