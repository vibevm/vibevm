//! The ONE chained end-to-end run: a real Cargo build, then a package
//! target that consumes the BUILD RECORD's output.
//!
//! It reuses R8-CARGO's real-build fixture pattern — a dependency-free
//! crate, offline, its own workspace root, one executable — and keeps the
//! real-build cost to exactly this test, as §5.0.7 accepted for its own
//! suite.
//!
//! What only this test can prove: the two executors really compose
//! through the ENGINE'S OWN STATE. The package target names
//! `fixture-tool.bin`, an id that appears nowhere on disk; the executable
//! Cargo produced is called something else entirely, in a directory the
//! package target never mentions; and the placed file is compared byte for
//! byte against the built one. A package executor that guessed a path
//! could not have found it, and one that trusted the record without
//! re-proving it would not notice the tamper the last assertion makes.

use specmark::verifies;
use vibe_core::manifest::{
    ArtifactBuildTarget, ArtifactInput, ArtifactKind, ArtifactOutput, MechanismRoutes,
};

use super::support::*;
use super::*;
use crate::mechanism::build::{BuildExecution, execute_build_targets};

/// The artifact id — deliberately unlike both other names.
const OUTPUT_ID: &str = "fixture-tool.bin";
/// The Cargo package name.
const PACKAGE: &str = "vibe-r8p-fixture";
/// The `[[bin]]` target name, and therefore the executable's stem.
const BIN: &str = "r8pack";

const MANIFEST: &str = concat!(
    "[package]\nname = \"vibe-r8p-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n",
    "[[bin]]\nname = \"r8pack\"\npath = \"src/main.rs\"\n\n",
    "[workspace]\n",
);

const MAIN: &str = "fn main() {\n    println!(\"r8-package\");\n}\n";

fn fixture_build_target() -> ArtifactBuildTarget {
    ArtifactBuildTarget {
        id: "fixture-tool".to_owned(),
        mechanism: key("build:cargo"),
        provider: None,
        workdir: ".".to_owned(),
        inputs: None,
        outputs: vec![ArtifactOutput {
            id: OUTPUT_ID.to_owned(),
            kind: ArtifactKind::Executable,
            select: Some(config(&format!("package = \"{PACKAGE}\"\nbin = \"{BIN}\""))),
        }],
        config: Some(config("profile = \"release\"\noffline = true\n")),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_package_target_consumes_a_real_build_output_through_its_record() {
    let project = temp();
    let root = project.path();
    write(root, "Cargo.toml", MANIFEST);
    write(root, "src/main.rs", MAIN);
    write_demo_plugin(root);

    let world = empty_world();
    let plane = registry(&world);
    let routes = MechanismRoutes::default();

    // ---- the build half: a real compile, recorded by the engine -------
    let build_targets = vec![fixture_build_target()];
    let built = match execute_build_targets(&BuildExecution {
        project_root: root,
        targets: &build_targets,
        registry: &plane,
        routes: &routes,
        build_root: BuildExecution::default_build_root(),
        offline: true,
        created_at: "2026-08-30T12:00:00Z",
    }) {
        Ok(outcomes) => outcomes,
        Err(error) => panic!("the fixture crate builds: {error}"),
    };
    assert_eq!(built.len(), 1);
    let executable = &built[0].produced[0];
    assert_eq!(executable.id, OUTPUT_ID);
    assert_eq!(
        executable.record,
        format!("{}/{OUTPUT_ID}.json", crate::ARTIFACT_RECORD_DIR),
    );

    // ---- the package half: it names the ID, never the path -----------
    let package_targets = vec![plugin_target(
        "toolkit",
        "plugin",
        vec![ArtifactInput::Artifact {
            artifact: OUTPUT_ID.to_owned(),
        }],
        &[(OUTPUT_ID, "com.example.tools/r8pack.bin")],
    )];
    let packaged =
        match execute_package_targets(&execution(root, &package_targets, &plane, &routes)) {
            Ok(outcomes) => outcomes,
            Err(error) => panic!("the plugin packages the built tool: {error}"),
        };

    assert_eq!(packaged.len(), 1);
    let produced = &packaged[0].produced[0];
    assert_eq!(produced.files, 3, "plugin.json, the SKILL.md and the tool");
    let staged = root.join("target/vibe-package/toolkit/com.example.tools/r8pack.bin");
    let (staged_bytes, built_bytes) = match (
        std::fs::read(&staged),
        std::fs::read(root.join(&executable.path_relative)),
    ) {
        (Ok(staged), Ok(built)) => (staged, built),
        (staged, built) => panic!("both artifacts read: {staged:?} / {built:?}"),
    };
    assert_eq!(
        staged_bytes, built_bytes,
        "the placed file is the executable the build record named",
    );

    // ---- and the record is not merely believed ------------------------
    //
    // Tamper with the built artifact behind its own record and package
    // again: the digest is recomputed from the bytes that are there NOW,
    // so the stale input refuses instead of being packaged.
    write(root, &executable.path_relative, "tampered");
    let refusal = execute_package_targets(&execution(root, &package_targets, &plane, &routes))
        .expect_err("an artifact that changed behind its record is never packaged");
    match &refusal {
        PackageError::InputStale {
            input, recorded, ..
        } => {
            assert_eq!(input, OUTPUT_ID);
            assert_eq!(recorded, &executable.digest);
        }
        other => panic!("expected a stale-input refusal, got {other}"),
    }
}
