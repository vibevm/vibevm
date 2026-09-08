//! Package record finalization for builtin and native providers.

use sha2::{Digest, Sha256};
use vibe_core::manifest::{ArtifactKind, ArtifactPackageTarget};
use vibe_wire::behaviour::artifact_record::validate;
use vibe_wire::generated::artifact_record::{ArtifactRecord, ArtifactShape};
use vibe_wire::generated::native::e1::package_request as request;
use vibe_wire::generated::shared::{
    NativeDeployArtifactKind as WireKind, NativeDeployArtifactShape as WireShape,
};

use crate::mechanism::record::{
    RecordError, RecordFreshness, RecordInputs, build_record, config_digest, sanitize, write_record,
};
use crate::native::{NativeMechanismBinding, PreparedNativeMechanism};

use super::protocol::{InputOrigin, PackagePlan, ResolvedInput, StagedArtifact};
use super::{
    Builtin, PackageError, PackageExecution, PackageProvider, PackageTargetRequest,
    PackagedArtifact,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn record_all_builtin(
    execution: &PackageExecution<'_>,
    request: &PackageTargetRequest<'_>,
    provider: &Builtin,
    plan: &PackagePlan,
    inputs_digest: &str,
    counted: usize,
    staged: &[StagedArtifact],
    pin: &str,
) -> Result<Vec<PackagedArtifact>, PackageError> {
    let descriptor = provider.descriptor();
    let config_fingerprint = config_digest(
        &request.target.mechanism,
        pin,
        std::slice::from_ref(&plan.summary),
        &plan.inputs,
    );
    let mut produced = Vec::with_capacity(staged.len());
    for artifact in staged {
        let verified = provider.verify(request, artifact)?;
        let evidence = sanitize(&format!(
            "{}; {}; engine-fresh over {counted} declared input(s) [{}]; {} file(s) covering {} \
             byte(s) at {}",
            descriptor.posture(),
            plan.summary,
            origins(request),
            verified.files,
            verified.bytes,
            verified.path_relative,
        ));
        let record = build_record(&RecordInputs {
            target: &request.target.id,
            mechanism: &request.target.mechanism,
            provider_key: pin,
            provider_version: None,
            provider_hash: None,
            output_id: &verified.output_id,
            kind: artifact.kind,
            shape: artifact.shape.clone(),
            digest: &verified.digest,
            path_absolute: &verified.path_absolute,
            path_relative: &verified.path_relative,
            freshness: RecordFreshness {
                inputs: Some(inputs_digest),
                config: Some(&config_fingerprint),
                toolchain: None,
            },
            platform: None,
            media_type: artifact.media_type.as_deref(),
            created_at: execution.created_at,
            evidence,
        })?;
        let path = write_record(execution.project_root, &record)?;
        produced.push(PackagedArtifact {
            id: verified.output_id,
            path_absolute: verified.path_absolute,
            path_relative: verified.path_relative,
            digest: verified.digest,
            bytes: verified.bytes,
            files: verified.files,
            record: path,
        });
    }
    Ok(produced)
}

pub(super) struct VerifiedNativeOutput {
    pub id: String,
    pub kind: ArtifactKind,
    pub shape: ArtifactShape,
    pub path_absolute: String,
    pub path_relative: String,
    pub digest: String,
    pub bytes: u64,
    pub files: usize,
    pub media_type: Option<String>,
}

pub(super) struct NativeRecordInputs<'a> {
    pub entry: &'a PreparedNativeMechanism,
    pub binding: &'a NativeMechanismBinding,
    pub input_digest: &'a str,
    pub config_digest: &'a str,
    pub provider_fingerprint: &'a str,
    pub evidence: &'a str,
}

pub(super) fn native_input_digest(inputs: &[ResolvedInput]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-native-package-inputs-v1\0");
    frame(&mut hash, "count", inputs.len().to_string().as_bytes());
    for input in inputs {
        frame(&mut hash, "name", input.name.as_bytes());
        frame(&mut hash, "reference", input.reference.as_bytes());
        frame(&mut hash, "path", input.relative.as_bytes());
        frame(&mut hash, "digest", input.digest.as_bytes());
        frame(&mut hash, "bytes", input.bytes.to_string().as_bytes());
        frame(&mut hash, "shape", shape_name(&input.shape).as_bytes());
        match input.origin {
            InputOrigin::ArtifactRecord { kind } => {
                frame(&mut hash, "origin", b"artifact-record");
                frame(&mut hash, "recorded-kind", kind.as_str().as_bytes());
            }
            InputOrigin::WorkspacePath => frame(&mut hash, "origin", b"workspace-path"),
        }
    }
    format!("{:x}", hash.finalize())
}

pub(super) fn native_config_digest(
    target: &ArtifactPackageTarget,
    pin: &str,
) -> Result<String, String> {
    let mut hash = Sha256::new();
    hash.update(b"vibe-native-package-config-v1\0");
    frame(
        &mut hash,
        "mechanism",
        target.mechanism.to_string().as_bytes(),
    );
    frame(&mut hash, "provider", pin.as_bytes());
    frame(&mut hash, "target", target.id.as_bytes());
    optional_table(&mut hash, "config", target.config.as_ref())?;
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

pub(super) fn record_native_outputs(
    execution: &PackageExecution<'_>,
    request: &PackageTargetRequest<'_>,
    inputs: &NativeRecordInputs<'_>,
    outputs: Vec<VerifiedNativeOutput>,
) -> Result<Vec<PackagedArtifact>, PackageError> {
    let mut records: Vec<ArtifactRecord> = Vec::with_capacity(outputs.len());
    for output in &outputs {
        let independent = match output.shape {
            ArtifactShape::File => format!("sha256 over {} byte(s)", output.bytes),
            ArtifactShape::Directory => format!(
                "sha256-tree/1 over {} file(s) and {} byte(s)",
                output.files, output.bytes
            ),
        };
        let record = build_record(&RecordInputs {
            target: &request.target.id,
            mechanism: &request.target.mechanism,
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
                inputs: Some(inputs.input_digest),
                config: Some(inputs.config_digest),
                toolchain: Some(inputs.provider_fingerprint),
            },
            platform: None,
            media_type: output.media_type.as_deref(),
            created_at: execution.created_at,
            evidence: sanitize(&format!(
                "{}; provider image sha256 {} for {}; independent engine proof {independent} at {}",
                inputs.evidence,
                inputs.entry.digest,
                inputs.entry.platform.key(),
                output.path_relative,
            )),
        })?;
        validate(&record).map_err(|error| {
            PackageError::Record(RecordError::Invalid {
                output: output.id.clone(),
                reason: error.to_string(),
            })
        })?;
        records.push(record);
    }
    let mut produced = Vec::with_capacity(outputs.len());
    for (output, record) in outputs.into_iter().zip(records) {
        let path = write_record(execution.project_root, &record)?;
        produced.push(PackagedArtifact {
            id: output.id,
            path_absolute: output.path_absolute,
            path_relative: output.path_relative,
            digest: output.digest,
            bytes: output.bytes,
            files: output.files,
            record: path,
        });
    }
    Ok(produced)
}

fn origins(request: &PackageTargetRequest<'_>) -> String {
    let recorded = request
        .inputs
        .iter()
        .filter(|input| input.origin.recorded_kind().is_some())
        .count();
    let workspace = request.inputs.len() - recorded;
    format!(
        "{}={recorded} {}={workspace}",
        InputOrigin::RECORD_SPELLING,
        InputOrigin::WORKSPACE_SPELLING,
    )
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

fn shape_name(shape: &ArtifactShape) -> &'static str {
    match shape {
        ArtifactShape::File => "file",
        ArtifactShape::Directory => "directory",
    }
}

pub(super) fn wire_kind(kind: ArtifactKind) -> WireKind {
    match kind {
        ArtifactKind::Executable => WireKind::Executable,
        ArtifactKind::Archive => WireKind::Archive,
        ArtifactKind::File => WireKind::File,
        ArtifactKind::Directory => WireKind::Directory,
        ArtifactKind::Skill => WireKind::Skill,
        ArtifactKind::AgentPlugin => WireKind::AgentPlugin,
    }
}

pub(super) fn wire_shape(shape: &ArtifactShape) -> WireShape {
    match shape {
        ArtifactShape::File => WireShape::File,
        ArtifactShape::Directory => WireShape::Directory,
    }
}

pub(super) fn artifact_shape(shape: &WireShape) -> ArtifactShape {
    match shape {
        WireShape::File => ArtifactShape::File,
        WireShape::Directory => ArtifactShape::Directory,
    }
}

pub(super) fn parse_kind(value: &str) -> Result<WireKind, String> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| "provider plan returned an unknown artifact kind".to_owned())
}

pub(super) fn parse_shape(value: &str) -> Result<WireShape, String> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| "provider plan returned an unknown artifact shape".to_owned())
}

pub(super) fn plan_path_fault(plan: &request::PackagePlan) -> Option<(&str, &str)> {
    for (index, output) in plan.outputs.iter().enumerate() {
        if output.path_relative == "." && !matches!(output.shape, WireShape::Directory) {
            return Some((&output.id, "`.` is legal only for a directory output root"));
        }
        for other in &plan.outputs[index + 1..] {
            if overlaps(&output.path_relative, &other.path_relative) {
                return Some((&output.id, "planned output roots overlap by path segment"));
            }
        }
    }
    None
}

pub(super) fn planned_owner_count(plan: &request::PackagePlan, candidate: &str) -> usize {
    plan.outputs
        .iter()
        .filter(|output| match output.shape {
            WireShape::File => output.path_relative == candidate,
            WireShape::Directory if output.path_relative == "." => true,
            WireShape::Directory => descendant(&output.path_relative, candidate),
        })
        .count()
}

fn overlaps(a: &str, b: &str) -> bool {
    a == "." || b == "." || a == b || descendant(a, b) || descendant(b, a)
}

fn descendant(root: &str, candidate: &str) -> bool {
    candidate
        .strip_prefix(root)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vibe_core::manifest::{ArtifactOutput, ExtensionConfig, MechanismKey};

    use super::*;

    const PIN: &str = "org.example/packagers#skill-v2";

    fn must<T, E: std::fmt::Debug>(value: Result<T, E>, context: &str) -> T {
        match value {
            Ok(value) => value,
            Err(error) => panic!("{context}: {error:?}"),
        }
    }

    fn config(value: &str) -> ExtensionConfig {
        must(
            value
                .parse::<toml::Table>()
                .map(ExtensionConfig::from_table),
            "fixture config",
        )
    }

    fn target() -> ArtifactPackageTarget {
        ArtifactPackageTarget {
            id: "native-package".to_owned(),
            mechanism: must::<MechanismKey, _>("package:static-skill".parse(), "package key"),
            provider: None,
            when: None,
            inputs: None,
            outputs: vec![ArtifactOutput {
                id: "file-out".to_owned(),
                kind: ArtifactKind::File,
                select: Some(config("name='file'")),
            }],
            config: Some(config("mode='native'")),
        }
    }

    fn input() -> ResolvedInput {
        ResolvedInput {
            name: "built-input".to_owned(),
            reference: "artifact:built-input".to_owned(),
            absolute: PathBuf::from("C:/work/target/built-input.bin"),
            relative: "target/built-input.bin".to_owned(),
            digest: "a".repeat(64),
            bytes: 11,
            shape: ArtifactShape::File,
            origin: InputOrigin::ArtifactRecord {
                kind: ArtifactKind::Executable,
            },
        }
    }

    #[test]
    fn native_fingerprints_are_independent_and_cover_every_owned_axis() {
        let original = target();
        let inputs = vec![input()];
        let input_digest = native_input_digest(&inputs);
        let config_digest = must(native_config_digest(&original, PIN), "config digest");
        let input_mutations: [fn(&mut ResolvedInput); 7] = [
            |value| value.name.push_str("-other"),
            |value| value.reference.push_str("-other"),
            |value| value.relative.push_str(".other"),
            |value| value.digest = "b".repeat(64),
            |value| value.bytes += 1,
            |value| value.shape = ArtifactShape::Directory,
            |value| value.origin = InputOrigin::WorkspacePath,
        ];
        for mutate in input_mutations {
            let mut changed = inputs.clone();
            mutate(&mut changed[0]);
            assert_ne!(native_input_digest(&changed), input_digest);
        }
        let mut changed_config = original.clone();
        changed_config.config = Some(config("mode='other'"));
        let mut changed_output = original.clone();
        changed_output.outputs[0].select = Some(config("name='other'"));
        for changed in [&changed_config, &changed_output] {
            assert_ne!(
                must(native_config_digest(changed, PIN), "changed config"),
                config_digest
            );
        }
        assert_ne!(
            must(
                native_config_digest(&original, "org.example/packagers#other"),
                "changed pin"
            ),
            config_digest
        );
        assert_ne!(input_digest, config_digest);
    }
}
