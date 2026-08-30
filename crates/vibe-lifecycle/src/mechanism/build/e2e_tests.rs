//! The ONE real end-to-end build (§5.0.7).
//!
//! "the acceptance's 'Rust fixture built' is ONE real end-to-end test
//! compiling a dependency-free fixture crate (offline-safe, temp target
//! dir) and asserting the executable is taken only from the
//! compiler-artifact message — the real-build cost is accepted for
//! exactly one test per suite."
//!
//! Everything the test asserts is arranged so that a guessed path could
//! not have produced it. Three names are deliberately all different — the
//! artifact output id (`fixture-tool.bin`), the Cargo package name
//! (`vibe-r8-fixture`) and the `[[bin]]` target name (`r8probe`) — and the
//! profile is `release`, so `target/<profile>/<name>` has four ways to be
//! wrong and exactly one right answer, which only the message stream
//! carries. The digest is then recomputed here from the file's own bytes,
//! so a record restating a plan value would fail too.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::{
    ArtifactBuildTarget, ArtifactKind, ArtifactOutput, ExtensionsControl, MechanismRoutes,
};
use vibe_extension_registry::collect_mechanisms;
use vibe_wire::behaviour::artifact_record::validate;
use vibe_wire::generated::artifact_record::{
    ArtifactRecord, DigestAlgorithm, RelativeRoot, VerificationStatus,
};

use super::super::cargo::plan_tests::{config, key};
use super::*;
use crate::{ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider};

/// The artifact id — deliberately unlike both other names.
const OUTPUT_ID: &str = "fixture-tool.bin";
/// The Cargo package name.
const PACKAGE: &str = "vibe-r8-fixture";
/// The `[[bin]]` target name, and therefore the executable's stem.
const BIN: &str = "r8probe";

const MANIFEST: &str = concat!(
    "[package]\nname = \"vibe-r8-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n",
    "[[bin]]\nname = \"r8probe\"\npath = \"src/main.rs\"\n\n",
    // Its own workspace root: the fixture must never be absorbed by a
    // workspace that happens to sit above the temp directory.
    "[workspace]\n",
);

const MAIN: &str = "fn main() {\n    println!(\"r8\");\n}\n";

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        panic!("the fixture directory creates: {error}");
    }
    if let Err(error) = std::fs::write(&path, contents) {
        panic!("the fixture file writes: {error}");
    }
}

/// The build target that names the fixture crate's one executable.
fn fixture_target() -> ArtifactBuildTarget {
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

fn empty_world() -> ExtensionWorld {
    ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: PathBuf::from("."),
                version: "0.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_dependency_free_fixture_crate_builds_and_records_its_executable() {
    let project = match TempDir::new() {
        Ok(project) => project,
        Err(error) => panic!("a temp project opens: {error}"),
    };
    let root = project.path();
    write(root, "Cargo.toml", MANIFEST);
    write(root, "src/main.rs", MAIN);

    let world = empty_world();
    let registry = match collect_mechanisms(&world) {
        Ok(registry) => registry,
        Err(error) => panic!("the empty world collects: {error}"),
    };
    let routes = MechanismRoutes::default();
    let targets = vec![fixture_target()];
    let outcomes = match execute_build_targets(&BuildExecution {
        project_root: root,
        targets: &targets,
        registry: &registry,
        routes: &routes,
        build_root: BuildExecution::default_build_root(),
        offline: true,
        created_at: "2026-08-30T12:00:00Z",
    }) {
        Ok(outcomes) => outcomes,
        Err(error) => panic!("the fixture crate builds: {error}"),
    };

    // The routing half: nothing pinned, nothing routed, the shipped
    // default answered, and it displaced nothing.
    assert_eq!(outcomes.len(), 1);
    let outcome = &outcomes[0];
    assert_eq!(outcome.target, "fixture-tool");
    assert_eq!(outcome.mechanism, "build:cargo");
    assert_eq!(outcome.provider, "org.vibevm/vibe#cargo");
    assert_eq!(outcome.via, "the shipped builtin default");
    assert_eq!(outcome.displaced_default, None);

    // The artifact half: exactly one output, at the path the message
    // stream named — the release profile and the `[[bin]]` name, neither
    // of which is the artifact id or the package name.
    assert_eq!(outcome.produced.len(), 1);
    let produced = &outcome.produced[0];
    assert_eq!(produced.id, OUTPUT_ID);
    let expected_stem = if cfg!(windows) {
        format!("{BIN}.exe")
    } else {
        BIN.to_owned()
    };
    assert_eq!(
        produced.path_relative,
        format!("target/release/{expected_stem}"),
        "the executable is the one Cargo reported, not a guess",
    );
    assert!(
        produced.path_absolute.ends_with(&produced.path_relative),
        "{} does not end with {}",
        produced.path_absolute,
        produced.path_relative,
    );

    // The digest is of the produced bytes, recomputed here independently.
    let bytes = match std::fs::read(root.join(&produced.path_relative)) {
        Ok(bytes) => bytes,
        Err(error) => panic!("the produced executable reads: {error}"),
    };
    assert_eq!(produced.bytes, bytes.len() as u64);
    assert_eq!(produced.digest, format!("{:x}", Sha256::digest(&bytes)));

    // The record half: written by the engine, at the engine's own path,
    // and valid under the A2 reader that will consume it later.
    assert_eq!(
        produced.record,
        format!("{}/{OUTPUT_ID}.json", crate::ARTIFACT_RECORD_DIR)
    );
    let record_bytes = match std::fs::read(root.join(&produced.record)) {
        Ok(record_bytes) => record_bytes,
        Err(error) => panic!("the artifact record reads: {error}"),
    };
    let record: ArtifactRecord = match serde_json::from_slice(&record_bytes) {
        Ok(record) => record,
        Err(error) => panic!("the artifact record parses: {error}"),
    };
    if let Err(error) = validate(&record) {
        panic!("the written record satisfies the A2 laws: {error}");
    }
    assert_eq!(record.id, OUTPUT_ID);
    assert_eq!(record.digest.algorithm, DigestAlgorithm::Sha256);
    assert_eq!(record.digest.value, produced.digest);
    assert_eq!(record.path_relative.root, RelativeRoot::Project);
    assert_eq!(record.path_relative.path, produced.path_relative);
    assert_eq!(record.producer.target, "fixture-tool");
    assert_eq!(record.producer.mechanism, "build:cargo");
    assert_eq!(record.producer.provider.key, "org.vibevm/vibe#cargo");
    assert_eq!(record.verification.status, VerificationStatus::Verified);

    // Provider-fresh: no engine-side input census, a real toolchain
    // digest, and the evidence names the toolchain that produced it.
    assert_eq!(record.freshness.inputs, None);
    assert_eq!(
        record.freshness.toolchain.as_ref().map(String::len),
        Some(64)
    );
    let evidence = record
        .verification
        .evidence
        .as_deref()
        .unwrap_or("<no evidence>");
    assert!(evidence.contains("org.vibevm/vibe#cargo"), "{evidence}");
    assert!(evidence.contains("cargo "), "{evidence}");
    assert!(evidence.contains("rustc "), "{evidence}");
    assert!(evidence.contains("cargo-fresh="), "{evidence}");
}
