use super::*;
use crate::behaviour::native_mechanism::PROTOCOL_EPOCH;
use crate::behaviour::scalars::{
    is_canonical_decimal, is_lowercase_hex, is_portable_token, relative_path_defect,
};
use crate::generated::native::e1::{package_reply as reply, package_request as request};
use crate::generated::shared::{NativeDeployArtifactKind, NativeDeployArtifactShape};
use std::collections::BTreeSet;
use vibe_core::manifest::{MechanismKey, MechanismRole, ProviderPin};
#[allow(clippy::too_many_arguments)]
pub(super) fn base(
    envelope: u32,
    protocol: u32,
    identity_value: &request::InvocationIdentity,
    target_value: &request::PackageTarget,
    inputs_value: &[request::ResolvedInput],
    authority_value: &request::PackageAuthority,
    provider: &str,
    mechanism: &str,
    target_id: &str,
) -> Result<(), NativePackageError> {
    epoch(envelope)?;
    epoch(protocol)?;
    identity(identity_value, provider, mechanism, target_id)?;
    target(target_value, target_id)?;
    authority(authority_value, target_id)?;
    inputs(inputs_value, authority_value)
}
pub(super) fn require_chain(valid: bool) -> Result<(), NativePackageError> {
    if valid {
        Ok(())
    } else {
        Err(fault(
            "accepted-chain",
            "request differs from retained accepted state",
        ))
    }
}
pub(super) fn validate_expected(
    provider: &str,
    mechanism: &str,
    target: &str,
) -> Result<(), NativePackageError> {
    ProviderPin::parse(provider)
        .map_err(|_| fault("exact-identity", "expected provider is not an exact pin"))?;
    let key: MechanismKey = mechanism
        .parse()
        .map_err(|_| fault("exact-identity", "expected mechanism is invalid"))?;
    if key.role() != MechanismRole::Package
        || key.to_string() != mechanism
        || !is_portable_token(target)
    {
        return Err(fault(
            "exact-identity",
            "expected package identity is not canonical",
        ));
    }
    Ok(())
}

fn identity(
    value: &request::InvocationIdentity,
    provider: &str,
    mechanism: &str,
    target: &str,
) -> Result<(), NativePackageError> {
    if value.provider == provider && value.mechanism == mechanism && value.target == target {
        Ok(())
    } else {
        Err(fault(
            "exact-identity",
            "request identity differs from selected values",
        ))
    }
}

fn target(value: &request::PackageTarget, expected: &str) -> Result<(), NativePackageError> {
    if value.id != expected || !is_portable_token(&value.id) {
        return Err(fault(
            "exact-identity",
            "target id differs or is noncanonical",
        ));
    }
    canonical_toml(value.config_toml.as_deref())?;
    bounded_nonempty(value.outputs.len(), "declared-outputs")?;
    let mut ids = BTreeSet::new();
    for output in &value.outputs {
        token(&output.id, "declared-outputs")?;
        if !ids.insert(&output.id) {
            return Err(fault(
                "declared-outputs",
                "declared output ids are not unique",
            ));
        }
    }
    Ok(())
}

fn authority(value: &request::PackageAuthority, target: &str) -> Result<(), NativePackageError> {
    for absolute_path in [
        &value.project_root_absolute,
        &value.package_root_absolute,
        &value.output_root_absolute,
    ] {
        absolute(absolute_path)?;
    }
    relative(&value.package_root_relative, "paths")?;
    relative(&value.output_root_relative, "paths")?;
    if join(&value.project_root_absolute, &value.package_root_relative)
        != value.package_root_absolute
        || join(&value.project_root_absolute, &value.output_root_relative)
            != value.output_root_absolute
        || join(&value.package_root_absolute, target) != value.output_root_absolute
    {
        return Err(fault(
            "paths",
            "package authority roots do not form one exact tree",
        ));
    }
    Ok(())
}

fn inputs(
    values: &[request::ResolvedInput],
    authority: &request::PackageAuthority,
) -> Result<(), NativePackageError> {
    bounded_collection(values.len(), "resolved-inputs")?;
    let mut names = BTreeSet::new();
    let mut references = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for value in values {
        diagnostic(&value.reference, "resolved-inputs", true)?;
        relative(&value.path_relative, "paths")?;
        absolute(&value.path_absolute)?;
        if join(&authority.project_root_absolute, &value.path_relative) != value.path_absolute {
            return Err(fault(
                "paths",
                "resolved input is outside project authority",
            ));
        }
        digest(&value.digest, "resolved-inputs")?;
        decimal(
            &value.bytes,
            "resolved-inputs",
            "input byte count is not canonical u64",
        )?;
        let expected_reference = match &value.origin {
            request::InputOrigin::ArtifactRecord(origin) => {
                token(&value.name, "resolved-inputs")?;
                kind_shape(&origin.recorded_kind, &value.shape, "resolved-inputs")?;
                format!("artifact:{}", value.name)
            }
            request::InputOrigin::WorkspacePath(_) => {
                if value.name != value.path_relative {
                    return Err(fault(
                        "resolved-inputs",
                        "workspace input name differs from its path",
                    ));
                }
                format!("path:{}", value.path_relative)
            }
        };
        if value.reference != expected_reference {
            return Err(fault(
                "resolved-inputs",
                "input reference disagrees with its origin",
            ));
        }
        if !names.insert(&value.name)
            || !references.insert(&value.reference)
            || !paths.insert(&value.path_relative)
        {
            return Err(fault("resolved-inputs", "resolved inputs are not unique"));
        }
    }
    Ok(())
}

pub(super) fn staging(
    value: &request::StagingAuthority,
    authority: &request::PackageAuthority,
) -> Result<(), NativePackageError> {
    absolute(&value.root_absolute)?;
    relative(&value.root_relative, "paths")?;
    if join(&authority.project_root_absolute, &value.root_relative) != value.root_absolute {
        return Err(fault(
            "staged-authority",
            "staging root does not belong to project authority",
        ));
    }
    Ok(())
}

pub(super) fn request_plan(
    plan: &request::PackagePlan,
    target: &request::PackageTarget,
) -> Result<(), NativePackageError> {
    diagnostic(&plan.summary, "accepted-chain", true)?;
    if plan.outputs.len() != target.outputs.len() {
        return Err(fault(
            "accepted-chain",
            "plan output count differs from declaration",
        ));
    }
    let mut paths = BTreeSet::new();
    for (planned, declared) in plan.outputs.iter().zip(&target.outputs) {
        if planned.id != declared.id || planned.kind != declared.kind {
            return Err(fault(
                "accepted-chain",
                "plan output identity differs from declaration",
            ));
        }
        kind_shape(&planned.kind, &planned.shape, "accepted-chain")?;
        output_relative(&planned.path_relative, "paths")?;
        planned_media(planned.media_type.as_deref(), &planned.shape)?;
        if !paths.insert(&planned.path_relative) {
            return Err(fault("accepted-chain", "plan paths are not unique"));
        }
    }
    Ok(())
}

pub(super) fn request_fingerprint(
    value: &request::PackageFingerprint,
    input_count: usize,
) -> Result<(), NativePackageError> {
    digest(&value.digest, "fingerprint")?;
    decimal(
        &value.counted_inputs,
        "fingerprint",
        "input count is not canonical u64",
    )?;
    if value.counted_inputs.parse::<u64>().ok() != u64::try_from(input_count).ok() {
        return Err(fault(
            "fingerprint",
            "input census differs from resolved inputs",
        ));
    }
    Ok(())
}

pub(super) fn staged(
    values: &[request::StagedOutput],
    plan: &request::PackagePlan,
) -> Result<(), NativePackageError> {
    if values.len() != plan.outputs.len()
        || values.iter().zip(&plan.outputs).any(|(value, planned)| {
            value.id != planned.id
                || value.kind != planned.kind
                || value.shape != planned.shape
                || value.path_relative != planned.path_relative
                || value.media_type != planned.media_type
        })
    {
        return Err(fault(
            "staged-authority",
            "staged output differs from accepted plan",
        ));
    }
    Ok(())
}

pub(super) fn reply_plan(plan: &reply::PackagePlan) -> Result<(), NativePackageError> {
    diagnostic(&plan.summary, "plan-outputs", true)?;
    bounded_nonempty(plan.outputs.len(), "plan-outputs")?;
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for output in &plan.outputs {
        token(&output.id, "plan-outputs")?;
        let shape = reply_shape(&output.shape)?;
        kind_shape(&reply_kind(&output.kind)?, &shape, "plan-outputs")?;
        output_relative(&output.path_relative, "paths")?;
        planned_media(output.media_type.as_deref(), &shape)?;
        if !ids.insert(&output.id) || !paths.insert(&output.path_relative) {
            return Err(fault(
                "plan-outputs",
                "planned output identities are not unique",
            ));
        }
    }
    Ok(())
}

pub(super) fn reply_fingerprint(
    value: &reply::PackageFingerprint,
) -> Result<(), NativePackageError> {
    digest(&value.digest, "fingerprint")?;
    decimal(
        &value.counted_inputs,
        "fingerprint",
        "input count is not canonical u64",
    )
}

pub(super) fn reply_staged(values: &[reply::StagedOutput]) -> Result<(), NativePackageError> {
    bounded_nonempty(values.len(), "staged-outputs")?;
    unique_rows(
        values
            .iter()
            .map(|v| (v.id.as_str(), v.path_relative.as_str())),
        "staged-outputs",
    )
}

pub(super) fn verified(values: &[reply::VerifiedOutput]) -> Result<(), NativePackageError> {
    bounded_nonempty(values.len(), "verification-rows")?;
    unique_rows(
        values
            .iter()
            .map(|v| (v.id.as_str(), v.path_relative.as_str())),
        "verification-rows",
    )?;
    for value in values {
        digest(&value.digest, "verification-rows")?;
        decimal(
            &value.bytes,
            "verification-rows",
            "verified byte count is not canonical u64",
        )?;
        decimal(
            &value.files,
            "verification-rows",
            "verified file count is not canonical u64",
        )?;
    }
    Ok(())
}

pub(super) fn exact_plan_outputs(
    declared: &[request::DeclaredOutput],
    planned: &[reply::PlannedOutput],
) -> Result<(), NativePackageError> {
    if declared.len() != planned.len() {
        return Err(fault(
            "plan-outputs",
            "reply plan differs from declared output order",
        ));
    }
    for (declared, planned) in declared.iter().zip(planned) {
        let kind = reply_kind(&planned.kind)?;
        let shape = reply_shape(&planned.shape)?;
        if declared.id != planned.id || declared.kind != kind {
            return Err(fault(
                "plan-outputs",
                "reply plan differs from declared output order",
            ));
        }
        kind_shape(&kind, &shape, "plan-outputs")?;
    }
    Ok(())
}

pub(super) fn exact_fingerprint(
    value: &reply::PackageFingerprint,
    inputs: usize,
) -> Result<(), NativePackageError> {
    if value.counted_inputs.parse::<u64>().ok() == u64::try_from(inputs).ok() {
        Ok(())
    } else {
        Err(fault(
            "fingerprint",
            "reply input census differs from resolved inputs",
        ))
    }
}

pub(super) fn exact_staged_outputs(
    plan: &request::PackagePlan,
    staged: &[reply::StagedOutput],
) -> Result<(), NativePackageError> {
    if plan.outputs.len() == staged.len()
        && plan
            .outputs
            .iter()
            .zip(staged)
            .all(|(a, b)| a.id == b.id && a.path_relative == b.path_relative)
    {
        Ok(())
    } else {
        Err(fault(
            "staged-outputs",
            "apply reply differs from accepted plan order",
        ))
    }
}

pub(super) fn exact_verified_outputs(
    staged: &[request::StagedOutput],
    verified: &[reply::VerifiedOutput],
) -> Result<(), NativePackageError> {
    if staged.len() == verified.len()
        && staged.iter().zip(verified).all(|(a, b)| {
            a.id == b.id
                && a.path_relative == b.path_relative
                && (!matches!(a.shape, NativeDeployArtifactShape::File) || b.files == "1")
        })
    {
        Ok(())
    } else {
        Err(fault(
            "verification-rows",
            "verify reply differs from staged output order",
        ))
    }
}

pub(super) fn reply_head(value: &reply::PackageReply) -> (PackageOperation, u32, u32) {
    match value {
        reply::PackageReply::Plan(v) => (PackageOperation::Plan, v.envelope, v.protocol),
        reply::PackageReply::Fingerprint(v) => {
            (PackageOperation::Fingerprint, v.envelope, v.protocol)
        }
        reply::PackageReply::Apply(v) => (PackageOperation::Apply, v.envelope, v.protocol),
        reply::PackageReply::Verify(v) => (PackageOperation::Verify, v.envelope, v.protocol),
    }
}

fn reply_kind(value: &str) -> Result<NativeDeployArtifactKind, NativePackageError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| fault("plan-outputs", "planned output kind is not closed"))
}

fn reply_shape(value: &str) -> Result<NativeDeployArtifactShape, NativePackageError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| fault("plan-outputs", "planned output shape is not closed"))
}

fn kind_shape(
    kind: &NativeDeployArtifactKind,
    shape: &NativeDeployArtifactShape,
    law: &'static str,
) -> Result<(), NativePackageError> {
    let directory = matches!(
        kind,
        NativeDeployArtifactKind::Directory | NativeDeployArtifactKind::AgentPlugin
    );
    if directory == matches!(shape, NativeDeployArtifactShape::Directory) {
        Ok(())
    } else {
        Err(fault(law, "artifact kind and physical shape disagree"))
    }
}

fn canonical_toml(value: Option<&str>) -> Result<(), NativePackageError> {
    let Some(value) = value else {
        return Ok(());
    };
    let table: toml::Table = toml::from_str(value)
        .map_err(|_| fault("canonical-config", "config is not a TOML table"))?;
    if toml::to_string(&table).ok().as_deref() == Some(value) {
        Ok(())
    } else {
        Err(fault("canonical-config", "config is not canonical TOML"))
    }
}

fn unique_rows<'a>(
    values: impl Iterator<Item = (&'a str, &'a str)>,
    law: &'static str,
) -> Result<(), NativePackageError> {
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for (id, path) in values {
        token(id, law)?;
        output_relative(path, "paths")?;
        if !ids.insert(id) || !paths.insert(path) {
            return Err(fault(law, "output identities are not unique"));
        }
    }
    Ok(())
}

fn absolute(value: &str) -> Result<(), NativePackageError> {
    diagnostic(value, "paths", true)?;
    if value.contains('\\') {
        return Err(fault("paths", "absolute path contains a backslash"));
    }
    let tail = if let Some(tail) = value.strip_prefix('/') {
        tail
    } else if value.len() >= 3
        && value.as_bytes()[0].is_ascii_uppercase()
        && &value.as_bytes()[1..3] == b":/"
    {
        &value[3..]
    } else {
        return Err(fault("paths", "path is not canonical absolute"));
    };
    if tail
        .split('/')
        .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        Err(fault("paths", "absolute path has a noncanonical segment"))
    } else {
        Ok(())
    }
}

fn relative(value: &str, law: &'static str) -> Result<(), NativePackageError> {
    diagnostic(value, law, true)?;
    if relative_path_defect(value).is_some() {
        Err(fault(law, "path is not canonical relative"))
    } else {
        Ok(())
    }
}

fn output_relative(value: &str, law: &'static str) -> Result<(), NativePackageError> {
    if value == "." {
        Ok(())
    } else {
        relative(value, law)
    }
}

fn join(root: &str, relative: &str) -> String {
    format!("{}/{}", root.trim_end_matches('/'), relative)
}
pub(super) fn epoch(value: u32) -> Result<(), NativePackageError> {
    if value == ENVELOPE_EPOCH && value == PROTOCOL_EPOCH {
        Ok(())
    } else {
        Err(fault(
            "envelope-protocol",
            "wire epoch differs from admitted epoch",
        ))
    }
}
fn digest(value: &str, law: &'static str) -> Result<(), NativePackageError> {
    if is_lowercase_hex(value, 64) {
        Ok(())
    } else {
        Err(fault(law, "digest is not 64 lowercase hex"))
    }
}
fn decimal(
    value: &str,
    law: &'static str,
    message: &'static str,
) -> Result<(), NativePackageError> {
    if is_canonical_decimal(value) && value.parse::<u64>().is_ok() {
        Ok(())
    } else {
        Err(fault(law, message))
    }
}
fn token(value: &str, law: &'static str) -> Result<(), NativePackageError> {
    if is_portable_token(value) {
        Ok(())
    } else {
        Err(fault(law, "identifier is not a portable token"))
    }
}
fn bounded_collection(len: usize, law: &'static str) -> Result<(), NativePackageError> {
    if len <= COLLECTION_CAP {
        Ok(())
    } else {
        Err(fault(law, "collection exceeds its fixed cap"))
    }
}
fn bounded_nonempty(len: usize, law: &'static str) -> Result<(), NativePackageError> {
    if len == 0 {
        Err(fault(law, "collection must not be empty"))
    } else {
        bounded_collection(len, law)
    }
}
fn planned_media(
    value: Option<&str>,
    shape: &NativeDeployArtifactShape,
) -> Result<(), NativePackageError> {
    if matches!(shape, NativeDeployArtifactShape::Directory) && value.is_some() {
        return Err(fault(
            "media-type",
            "directory output cannot carry a media type",
        ));
    }
    match value {
        Some(value) => diagnostic(value, "media-type", true),
        None => Ok(()),
    }
}
