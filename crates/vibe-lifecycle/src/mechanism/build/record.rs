//! Neutral native-build fingerprint and record finalization.

use sha2::{Digest, Sha256};
use vibe_core::manifest::{ArtifactBuildTarget, ArtifactInput, ArtifactKind};
use vibe_wire::behaviour::artifact_record::validate;
use vibe_wire::generated::artifact_record::ArtifactShape;

use crate::native::{NativeMechanismBinding, PreparedNativeMechanism};

use super::{BuildError, BuildExecution, ProducedArtifact};
use crate::mechanism::record::{
    RecordError, RecordFreshness, RecordInputs, build_record, sanitize, write_record,
};

pub(super) struct VerifiedNativeOutput {
    pub id: String,
    pub kind: ArtifactKind,
    pub shape: ArtifactShape,
    pub path_absolute: String,
    pub path_relative: String,
    pub digest: String,
    pub bytes: u64,
    pub files: usize,
    pub fresh: bool,
}

pub(super) struct NativeRecordInputs<'a> {
    pub entry: &'a PreparedNativeMechanism,
    pub binding: &'a NativeMechanismBinding,
    pub config: &'a str,
    pub fingerprint: &'a str,
    pub evidence: &'a str,
}

pub(super) fn config_fingerprint(
    target: &ArtifactBuildTarget,
    pin: &str,
) -> Result<String, String> {
    let mut hash = Sha256::new();
    hash.update(b"vibe-native-build-config-v1\0");
    frame(
        &mut hash,
        "mechanism",
        target.mechanism.to_string().as_bytes(),
    );
    frame(&mut hash, "provider", pin.as_bytes());
    frame(&mut hash, "target", target.id.as_bytes());
    frame(&mut hash, "workdir", target.workdir.as_bytes());
    optional_table(&mut hash, "config", target.config.as_ref())?;
    hash.update(if target.inputs.is_some() {
        b"inputs=1\0"
    } else {
        b"inputs=0\0"
    });
    let inputs = target.inputs.as_deref().unwrap_or_default();
    frame(
        &mut hash,
        "input-count",
        inputs.len().to_string().as_bytes(),
    );
    for input in inputs {
        match input {
            ArtifactInput::Path { path } => {
                frame(&mut hash, "input-kind", b"path");
                frame(
                    &mut hash,
                    "input-value",
                    path.display().to_string().replace('\\', "/").as_bytes(),
                );
            }
            ArtifactInput::Artifact { artifact } => {
                frame(&mut hash, "input-kind", b"artifact");
                frame(&mut hash, "input-value", artifact.as_bytes());
            }
        }
    }
    frame(
        &mut hash,
        "output-count",
        target.outputs.len().to_string().as_bytes(),
    );
    for output in &target.outputs {
        frame(&mut hash, "output-id", output.id.as_bytes());
        frame(&mut hash, "output-kind", output.kind.as_str().as_bytes());
        optional_table(&mut hash, "output-select", output.select.as_ref())?;
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn optional_table(
    hash: &mut Sha256,
    label: &str,
    value: Option<&vibe_core::manifest::ExtensionConfig>,
) -> Result<(), String> {
    let Some(value) = value else {
        frame(hash, label, b"absent");
        return Ok(());
    };
    let encoded = toml::to_string(value.as_table()).map_err(|error| error.to_string())?;
    frame(hash, label, encoded.as_bytes());
    Ok(())
}

fn frame(hash: &mut Sha256, label: &str, value: &[u8]) {
    hash.update(label.as_bytes());
    hash.update(b"\0");
    hash.update((value.len() as u64).to_le_bytes());
    hash.update(value);
}

pub(super) fn record_native_outputs(
    execution: &BuildExecution<'_>,
    target: &ArtifactBuildTarget,
    inputs: &NativeRecordInputs<'_>,
    outputs: Vec<VerifiedNativeOutput>,
) -> Result<Vec<ProducedArtifact>, BuildError> {
    let mut records = Vec::with_capacity(outputs.len());
    for output in &outputs {
        let record = build_record(&RecordInputs {
            target: &target.id,
            mechanism: &target.mechanism,
            provider_key: &inputs.binding.pin,
            provider_version: Some(&inputs.entry.provider_version),
            provider_hash: inputs.entry.provider_hash.as_deref(),
            output_id: &output.id,
            kind: output.kind,
            shape: output.shape.clone(),
            digest: &output.digest,
            path_absolute: &output.path_absolute,
            path_relative: &output.path_relative,
            freshness: RecordFreshness {
                inputs: None,
                config: Some(inputs.config),
                toolchain: Some(inputs.fingerprint),
            },
            platform: Some(inputs.entry.platform.key()),
            media_type: None,
            created_at: execution.created_at,
            evidence: sanitize(&format!(
                "{}; independent engine proof {} at {}",
                inputs.evidence,
                match output.shape {
                    ArtifactShape::File => {
                        format!("sha256 over {} byte(s)", output.bytes)
                    }
                    ArtifactShape::Directory => format!(
                        "sha256-tree/1 over {} file(s) and {} byte(s)",
                        output.files, output.bytes
                    ),
                },
                output.path_relative
            )),
        })?;
        validate(&record).map_err(|error| {
            BuildError::Record(RecordError::Invalid {
                output: output.id.clone(),
                reason: error.to_string(),
            })
        })?;
        records.push(record);
    }
    let mut produced = Vec::with_capacity(outputs.len());
    for (output, record) in outputs.into_iter().zip(records) {
        let path = write_record(execution.project_root, &record)?;
        produced.push(ProducedArtifact {
            id: output.id,
            path_absolute: output.path_absolute,
            path_relative: output.path_relative,
            digest: output.digest,
            bytes: output.bytes,
            fresh: output.fresh,
            record: path,
        });
    }
    Ok(produced)
}
