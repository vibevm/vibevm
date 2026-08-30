//! The engine-owned record laws.
//!
//! The load-bearing one is [`a_record_that_does_not_validate_is_never_written`]:
//! the A2 behaviour cell runs BEFORE any byte reaches the filesystem, so a
//! producer that would emit an invalid record fails at its own boundary
//! instead of leaving a bad file for a later reader to find. Removing that
//! gate — or recording a digest of something other than the produced bytes
//! — is a packet mutation, and this cell is where it goes red.
//!
//! Since R8-PACKAGE the cell serves BOTH producing phases, so the shape
//! law has its own pin: a `file` record carries `sha256`, a `directory`
//! record carries `sha256-tree/1`, and neither can claim the other's
//! algorithm because the algorithm is derived from the shape here.

use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::ArtifactKind;
use vibe_wire::generated::artifact_record::{ArtifactShape, DigestAlgorithm};

use super::super::cargo::plan_tests::key;
use super::*;

/// An absolute root in the platform's own spelling — the A2 absolute-path
/// law reads `X:/…` on Windows and `/…` elsewhere.
fn root() -> &'static str {
    if cfg!(windows) {
        "C:/w/demo"
    } else {
        "/w/demo"
    }
}

fn digest_of(byte: u8) -> String {
    std::iter::repeat_n(format!("{byte:02x}"), 32).collect()
}

/// One canonical executable record's inputs, in the record's own
/// vocabulary — exactly what the build executor hands this cell.
fn inputs<'a>(
    mechanism: &'a MechanismKey,
    absolute: &'a str,
    digest: &'a str,
    config: &'a str,
    toolchain: &'a str,
    created_at: &'a str,
) -> RecordInputs<'a> {
    RecordInputs {
        target: "vibe-helper",
        mechanism,
        provider_key: "org.vibevm/vibe#cargo",
        provider_version: None,
        provider_hash: None,
        output_id: "vibe-helper.exe",
        kind: ArtifactKind::Executable,
        shape: ArtifactShape::File,
        digest,
        path_absolute: absolute,
        path_relative: "target/debug/vibe-helper.exe",
        freshness: RecordFreshness {
            inputs: None,
            config: Some(config),
            toolchain: Some(toolchain),
        },
        platform: Some("x86_64-pc-windows-msvc"),
        media_type: None,
        created_at,
        evidence: "sha256 verified over 4096 byte(s)".to_owned(),
    }
}

fn absolute() -> String {
    format!("{}/target/debug/vibe-helper.exe", root())
}

fn temp() -> TempDir {
    match TempDir::new() {
        Ok(root) => root,
        Err(error) => panic!("a temp project opens: {error}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_valid_record_publishes_and_reads_back_through_the_a2_cell() {
    let (absolute, produced, config, toolchain) = (
        absolute(),
        digest_of(0xab),
        digest_of(0x01),
        digest_of(0xcd),
    );
    let mechanism = key("build:cargo");
    let record = match build_record(&inputs(
        &mechanism,
        &absolute,
        &produced,
        &config,
        &toolchain,
        "2026-08-30T12:34:56Z",
    )) {
        Ok(record) => record,
        Err(error) => panic!("the record builds: {error}"),
    };
    let project = temp();

    let path = match write_record(project.path(), &record) {
        Ok(path) => path,
        Err(error) => panic!("the record publishes: {error}"),
    };

    assert_eq!(path, ".vibe/state/artifacts/vibe-helper.exe.json");
    let bytes = match std::fs::read(
        project
            .path()
            .join(".vibe/state/artifacts/vibe-helper.exe.json"),
    ) {
        Ok(bytes) => bytes,
        Err(error) => panic!("the published record reads back: {error}"),
    };
    let reread: vibe_wire::generated::artifact_record::ArtifactRecord =
        match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(error) => panic!("the published record parses: {error}"),
        };
    assert_eq!(reread, record);
    assert!(vibe_wire::behaviour::artifact_record::validate(&reread).is_ok());
    assert_eq!(reread.digest.value, produced);
    assert_eq!(reread.digest.algorithm, DigestAlgorithm::Sha256);
    assert_eq!(reread.producer.mechanism, "build:cargo");
    assert_eq!(reread.producer.provider.key, "org.vibevm/vibe#cargo");
    assert_eq!(reread.path_relative.path, "target/debug/vibe-helper.exe");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn the_reader_finds_a_written_record_and_reports_absence_as_absence() {
    let (absolute, produced, config, toolchain) = (
        absolute(),
        digest_of(0xab),
        digest_of(0x01),
        digest_of(0xcd),
    );
    let mechanism = key("build:cargo");
    let record = match build_record(&inputs(
        &mechanism,
        &absolute,
        &produced,
        &config,
        &toolchain,
        "2026-08-30T12:34:56Z",
    )) {
        Ok(record) => record,
        Err(error) => panic!("the record builds: {error}"),
    };
    let project = temp();
    if let Err(error) = write_record(project.path(), &record) {
        panic!("the record publishes: {error}");
    }

    match read_record(project.path(), "vibe-helper.exe") {
        Ok(Some(found)) => assert_eq!(found, record),
        Ok(None) => panic!("the record the engine just wrote is readable"),
        Err(error) => panic!("the record reads: {error}"),
    }
    match read_record(project.path(), "never-produced.exe") {
        Ok(None) => {}
        Ok(Some(_)) => panic!("nothing produced `never-produced.exe`"),
        Err(error) => panic!("an absent record is absence, not a failure: {error}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_record_that_does_not_validate_is_never_written() {
    let (absolute, produced, config, toolchain) = (
        absolute(),
        digest_of(0xab),
        digest_of(0x01),
        digest_of(0xcd),
    );
    let mechanism = key("build:cargo");
    let mut record = match build_record(&inputs(
        &mechanism,
        &absolute,
        &produced,
        &config,
        &toolchain,
        "2026-08-30T12:34:56Z",
    )) {
        Ok(record) => record,
        Err(error) => panic!("the record builds: {error}"),
    };
    // A digest that is not 64 lowercase hex: exactly the shape a producer
    // that restated a plan's value instead of digesting the produced
    // bytes would emit.
    record.digest.value = "NOT-A-DIGEST".to_owned();
    let project = temp();

    let refusal =
        write_record(project.path(), &record).expect_err("an invalid record is never published");

    match &refusal {
        RecordError::Invalid { output, .. } => assert_eq!(output, "vibe-helper.exe"),
        other => panic!("expected a record-validation refusal, got {other}"),
    }
    assert!(
        !project.path().join(".vibe").exists(),
        "the refusal happens before any byte reaches the filesystem",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn an_unspellable_artifact_id_refuses_before_it_becomes_a_path() {
    let (absolute, produced, config, toolchain) = (
        absolute(),
        digest_of(0xab),
        digest_of(0x01),
        digest_of(0xcd),
    );
    let mechanism = key("build:cargo");
    let mut declared = inputs(
        &mechanism,
        &absolute,
        &produced,
        &config,
        &toolchain,
        "2026-08-30T12:34:56Z",
    );
    declared.output_id = "../escape";
    let record = match build_record(&declared) {
        Ok(record) => record,
        Err(error) => panic!("the record builds: {error}"),
    };
    let project = temp();

    let refusal = write_record(project.path(), &record)
        .expect_err("the id grammar is what keeps the id a single path component");

    assert!(matches!(refusal, RecordError::Invalid { .. }));
    assert!(!project.path().join(".vibe").exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn the_freshness_triple_is_the_honest_provider_fresh_one() {
    let (absolute, produced, config, toolchain) = (
        absolute(),
        digest_of(0xab),
        digest_of(0x01),
        digest_of(0xcd),
    );
    let mechanism = key("build:cargo");

    let record = match build_record(&inputs(
        &mechanism,
        &absolute,
        &produced,
        &config,
        &toolchain,
        "2026-08-30T12:34:56Z",
    )) {
        Ok(record) => record,
        Err(error) => panic!("the record builds: {error}"),
    };

    assert_eq!(
        record.freshness.inputs, None,
        "Cargo owns inputs the engine does not model, and absence says so",
    );
    assert_eq!(record.freshness.config.as_deref(), Some(config.as_str()));
    assert_eq!(
        record.freshness.toolchain.as_deref(),
        Some(toolchain.as_str())
    );
    assert_eq!(record.platform.as_deref(), Some("x86_64-pc-windows-msvc"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_directory_record_carries_the_tree_algorithm_and_a_file_record_does_not() {
    let (absolute, produced, config, toolchain) = (
        absolute(),
        digest_of(0xab),
        digest_of(0x01),
        digest_of(0xcd),
    );
    let mechanism = key("package:agent-plugin");
    let mut declared = inputs(
        &mechanism,
        &absolute,
        &produced,
        &config,
        &toolchain,
        "2026-08-30T12:34:56Z",
    );
    declared.shape = ArtifactShape::Directory;
    declared.kind = ArtifactKind::AgentPlugin;

    let directory = match build_record(&declared) {
        Ok(record) => record,
        Err(error) => panic!("the record builds: {error}"),
    };
    declared.shape = ArtifactShape::File;
    declared.kind = ArtifactKind::File;
    let file = match build_record(&declared) {
        Ok(record) => record,
        Err(error) => panic!("the record builds: {error}"),
    };

    assert_eq!(directory.digest.algorithm, DigestAlgorithm::Sha256Tree);
    assert_eq!(file.digest.algorithm, DigestAlgorithm::Sha256);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_clock_that_is_not_rfc3339_refuses() {
    let (absolute, produced, config, toolchain) = (
        absolute(),
        digest_of(0xab),
        digest_of(0x01),
        digest_of(0xcd),
    );
    let mechanism = key("build:cargo");

    let refusal = build_record(&inputs(
        &mechanism,
        &absolute,
        &produced,
        &config,
        &toolchain,
        "yesterday",
    ))
    .expect_err("a record's timestamp is RFC 3339");

    match &refusal {
        RecordError::Clock { value, .. } => assert_eq!(value, "yesterday"),
        other => panic!("expected a clock refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn the_config_fingerprint_is_stable_and_provider_sensitive() {
    let mechanism = key("build:cargo");
    let argv = vec!["build".to_owned(), "--offline".to_owned()];
    let declared = vec!["path:Cargo.toml".to_owned()];

    let first = config_digest(&mechanism, "org.vibevm/vibe#cargo", &argv, &declared);
    let again = config_digest(&mechanism, "org.vibevm/vibe#cargo", &argv, &declared);
    assert_eq!(first, again, "one config hashes to one value");
    assert_eq!(first.len(), 64);

    // §4.1: "Provider changes invalidate the target even when its logical
    // mechanism name did not change."
    let other_provider = config_digest(&mechanism, "org.example/tools#cargo-v2", &argv, &declared);
    assert_ne!(first, other_provider);

    let other_argv = config_digest(
        &mechanism,
        "org.vibevm/vibe#cargo",
        &["build".to_owned()],
        &declared,
    );
    assert_ne!(first, other_argv);

    let other_inputs = config_digest(&mechanism, "org.vibevm/vibe#cargo", &argv, &[]);
    assert_ne!(first, other_inputs);
}
