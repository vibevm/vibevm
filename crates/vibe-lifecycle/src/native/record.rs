//! Native source cdylib records on the shared artifact-record plane.

use std::path::Path;

use vibe_core::manifest::{ArtifactKind, MechanismKey};
use vibe_wire::generated::artifact_record::{
    ArtifactKind as RecordKind, ArtifactShape, DigestAlgorithm, RelativeRoot,
};

use crate::mechanism::record::{
    ARTIFACT_RECORD_DIR, RecordFreshness, RecordInputs, build_record_with_root, read_record,
    sanitize, write_record,
};

use super::cargo::BuiltCdylib;
use super::path::{VerifiedFile, recorded_file};
use super::provider::{ProviderFacts, ProviderHome};
use super::{NativeArtifactError, NativePlatform};
use crate::mechanism::contain::forward_slashed;

pub(super) struct SourceRecordInputs<'a> {
    pub(super) selected_project_root: &'a Path,
    pub(super) provider: &'a ProviderFacts,
    pub(super) crate_dir: &'a str,
    pub(super) platform: NativePlatform,
    pub(super) record_id: &'a str,
    pub(super) build_provider: &'a str,
    pub(super) source_witness: &'a str,
    pub(super) config_witness: &'a str,
    pub(super) created_at: &'a str,
}

pub(super) fn write_source_record(
    inputs: &SourceRecordInputs<'_>,
    built: &BuiltCdylib,
) -> Result<String, NativeArtifactError> {
    let mechanism = cargo_key()?;
    let evidence = sanitize(&format!(
        "native cdylib provider={} version={} crate_dir={} cargo={} rustc={} cargo-fresh={} sha256 over {} byte(s) at {}",
        inputs.provider.identity,
        inputs.provider.version,
        inputs.crate_dir,
        built.toolchain.cargo,
        built.toolchain.rustc,
        built.fresh,
        built.file.bytes,
        built.file.relative,
    ));
    let record_inputs = RecordInputs {
        target: inputs.record_id,
        mechanism: &mechanism,
        provider_key: inputs.build_provider,
        provider_version: None,
        provider_hash: None,
        output_id: inputs.record_id,
        kind: ArtifactKind::File,
        shape: ArtifactShape::File,
        digest: &built.file.digest,
        path_absolute: &forward_slashed(&built.file.absolute),
        path_relative: &built.file.relative,
        freshness: RecordFreshness {
            inputs: Some(inputs.source_witness),
            config: Some(inputs.config_witness),
            toolchain: Some(&built.toolchain.digest),
        },
        platform: Some(inputs.platform.key()),
        media_type: None,
        created_at: inputs.created_at,
        evidence,
    };
    let root = match inputs.provider.home {
        ProviderHome::Dependency => RelativeRoot::Slot,
        ProviderHome::Host => RelativeRoot::Project,
    };
    let record = build_record_with_root(&record_inputs, root).map_err(|error| {
        NativeArtifactError::RecordWrite {
            record: record_path(inputs.record_id),
            reason: error.to_string(),
        }
    })?;
    write_record(inputs.selected_project_root, &record).map_err(|error| {
        NativeArtifactError::RecordWrite {
            record: record_path(inputs.record_id),
            reason: error.to_string(),
        }
    })
}

pub(super) struct SourceRecordExpectation<'a> {
    pub(super) selected_project_root: &'a Path,
    pub(super) provider: &'a ProviderFacts,
    pub(super) platform: NativePlatform,
    pub(super) record_id: &'a str,
    pub(super) build_provider: &'a str,
    pub(super) source_witness: &'a str,
    pub(super) config_witness: &'a str,
}

pub(super) fn revalidate_source_record(
    expected: &SourceRecordExpectation<'_>,
) -> Result<VerifiedFile, NativeArtifactError> {
    let path = record_path(expected.record_id);
    let record = read_record(expected.selected_project_root, expected.record_id)
        .map_err(|reason| unavailable(&path, reason))?
        .ok_or_else(|| unavailable(&path, "record is missing".to_owned()))?;
    let expected_root = match expected.provider.home {
        ProviderHome::Dependency => RelativeRoot::Slot,
        ProviderHome::Host => RelativeRoot::Project,
    };
    let mechanism = cargo_key()?.to_string();
    require(&path, record.id == expected.record_id, "record id changed")?;
    require(
        &path,
        record.kind == RecordKind::File && record.shape == ArtifactShape::File,
        "record is not a file artifact",
    )?;
    require(
        &path,
        record.path_relative.root == expected_root,
        "record relative root changed",
    )?;
    require(
        &path,
        record.producer.target == expected.record_id,
        "producer target changed",
    )?;
    require(
        &path,
        record.producer.mechanism == mechanism,
        "producer mechanism changed",
    )?;
    require(
        &path,
        record.producer.provider.key == expected.build_provider
            && record.producer.provider.version.is_none()
            && record.producer.provider.content_hash.is_none(),
        "selected build provider changed",
    )?;
    require(
        &path,
        record.freshness.inputs.as_deref() == Some(expected.source_witness),
        "source witness changed",
    )?;
    require(
        &path,
        record.freshness.config.as_deref() == Some(expected.config_witness),
        "handler/config witness changed",
    )?;
    require(
        &path,
        record
            .freshness
            .toolchain
            .as_deref()
            .is_some_and(valid_lower_hex_64),
        "toolchain witness is not exactly 64 lowercase hex characters",
    )?;
    require(
        &path,
        record.platform.as_deref() == Some(expected.platform.key()),
        "platform changed",
    )?;
    require(
        &path,
        record.digest.algorithm == DigestAlgorithm::Sha256,
        "digest algorithm changed",
    )?;
    let file = recorded_file(
        expected.provider,
        &record.path_relative.path,
        expected.platform,
    )
    .map_err(|reason| unavailable(&path, reason))?;
    require(
        &path,
        record.digest.value == file.digest,
        "artifact digest no longer matches current bytes",
    )?;
    Ok(file)
}

pub(super) fn record_path(id: &str) -> String {
    format!("{ARTIFACT_RECORD_DIR}/{id}.json")
}

fn require(record: &str, condition: bool, reason: &'static str) -> Result<(), NativeArtifactError> {
    if condition {
        Ok(())
    } else {
        Err(unavailable(record, reason.to_owned()))
    }
}

fn unavailable(record: &str, reason: String) -> NativeArtifactError {
    NativeArtifactError::SourceState {
        record: record.to_owned(),
        reason,
    }
}

fn valid_lower_hex_64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn cargo_key() -> Result<MechanismKey, NativeArtifactError> {
    "build:cargo"
        .parse::<MechanismKey>()
        .map_err(|error| NativeArtifactError::MechanismSelection {
            reason: format!("engine-owned logical key is invalid: {error}"),
        })
}
