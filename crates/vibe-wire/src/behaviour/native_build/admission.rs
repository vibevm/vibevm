use std::collections::BTreeSet;

use vibe_core::manifest::{MechanismKey, MechanismRole, ProviderPin};

use super::*;
use crate::behaviour::native_mechanism::PROTOCOL_EPOCH;
use crate::behaviour::scalars::{
    is_canonical_decimal, is_lowercase_hex, is_portable_token, relative_path_defect,
};
use crate::generated::native::e1::{build_reply as reply, build_request as request};
use crate::generated::shared::{NativeDeployArtifactKind, NativeDeployArtifactShape};

#[allow(clippy::too_many_arguments)]
pub(super) fn base(
    envelope: u32,
    protocol: u32,
    identity_value: &request::InvocationIdentity,
    target_value: &request::BuildTarget,
    authority_value: &request::BuildAuthority,
    provider: &str,
    mechanism: &str,
    target_id: &str,
) -> Result<(), NativeBuildError> {
    epoch(envelope)?;
    epoch(protocol)?;
    identity(identity_value, provider, mechanism, target_id)?;
    target(target_value, target_id)?;
    authority(authority_value)
}

pub(super) fn require_chain(valid: bool) -> Result<(), NativeBuildError> {
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
) -> Result<(), NativeBuildError> {
    ProviderPin::parse(provider)
        .map_err(|_| fault("exact-identity", "expected provider is not an exact pin"))?;
    let key: MechanismKey = mechanism
        .parse()
        .map_err(|_| fault("exact-identity", "expected mechanism is invalid"))?;
    if key.role() != MechanismRole::Build
        || key.to_string() != mechanism
        || !is_portable_token(target)
    {
        return Err(fault(
            "exact-identity",
            "expected build identity is not canonical",
        ));
    }
    Ok(())
}

fn identity(
    value: &request::InvocationIdentity,
    provider: &str,
    mechanism: &str,
    target: &str,
) -> Result<(), NativeBuildError> {
    if value.provider != provider || value.mechanism != mechanism || value.target != target {
        return Err(fault(
            "exact-identity",
            "request identity differs from selected values",
        ));
    }
    Ok(())
}

fn target(value: &request::BuildTarget, expected: &str) -> Result<(), NativeBuildError> {
    if value.id != expected || !is_portable_token(&value.id) {
        return Err(fault(
            "exact-identity",
            "target id differs or is noncanonical",
        ));
    }
    if value.workdir != "." {
        relative(&value.workdir, "paths")?;
    }
    canonical_toml(value.config_toml.as_deref())?;
    bounded_collection(value.inputs.len(), "declared-sets")?;
    bounded_collection(value.outputs.len(), "declared-sets")?;
    if value.outputs.is_empty() {
        return Err(fault("declared-sets", "declared outputs are empty"));
    }
    let mut inputs = BTreeSet::new();
    for input in &value.inputs {
        let identity = match input {
            request::DeclaredInput::Path(v) => {
                relative(&v.path_relative, "paths")?;
                format!("path:{}", v.path_relative)
            }
            request::DeclaredInput::Artifact(v) => {
                token(&v.artifact, "declared-sets")?;
                format!("artifact:{}", v.artifact)
            }
        };
        if !inputs.insert(identity) {
            return Err(fault(
                "declared-sets",
                "declared inputs contain a duplicate",
            ));
        }
    }
    let mut outputs = BTreeSet::new();
    for output in &value.outputs {
        token(&output.id, "declared-sets")?;
        canonical_toml(output.select_toml.as_deref())?;
        if !outputs.insert(&output.id) {
            return Err(fault("declared-sets", "declared output ids are not unique"));
        }
    }
    Ok(())
}

fn authority(value: &request::BuildAuthority) -> Result<(), NativeBuildError> {
    absolute(&value.project_root_absolute)?;
    absolute(&value.build_root_absolute)?;
    relative(&value.build_root_relative, "paths")?;
    if join(&value.project_root_absolute, &value.build_root_relative) != value.build_root_absolute {
        return Err(fault(
            "paths",
            "build root does not belong to project authority",
        ));
    }
    Ok(())
}

pub(super) fn staging(
    value: &request::StagingAuthority,
    authority: &request::BuildAuthority,
) -> Result<(), NativeBuildError> {
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
    plan: &request::BuildPlan,
    target: &request::BuildTarget,
) -> Result<(), NativeBuildError> {
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
        kind_shape(&planned.kind, &planned.shape)?;
        relative(&planned.path_relative, "paths")?;
        if !paths.insert(&planned.path_relative) {
            return Err(fault("accepted-chain", "plan paths are not unique"));
        }
    }
    Ok(())
}

pub(super) fn request_fingerprint(
    value: &request::BuildFingerprint,
) -> Result<(), NativeBuildError> {
    digest(&value.digest, "fingerprint")?;
    diagnostic(&value.summary, "fingerprint", true)
}

pub(super) fn staged(
    values: &[request::StagedOutput],
    plan: &request::BuildPlan,
) -> Result<(), NativeBuildError> {
    if values.len() != plan.outputs.len() {
        return Err(fault(
            "staged-authority",
            "staged output count differs from plan",
        ));
    }
    for (value, planned) in values.iter().zip(&plan.outputs) {
        if value.id != planned.id
            || value.kind != planned.kind
            || value.shape != planned.shape
            || value.path_relative != planned.path_relative
        {
            return Err(fault(
                "staged-authority",
                "staged output differs from accepted plan",
            ));
        }
    }
    Ok(())
}

pub(super) fn reply_plan(plan: &reply::BuildPlan) -> Result<(), NativeBuildError> {
    diagnostic(&plan.summary, "plan-outputs", true)?;
    bounded_nonempty(plan.outputs.len(), "plan-outputs")?;
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for output in &plan.outputs {
        token(&output.id, "plan-outputs")?;
        kind_shape(&reply_kind(&output.kind)?, &reply_shape(&output.shape)?)?;
        relative(&output.path_relative, "paths")?;
        if !ids.insert(&output.id) || !paths.insert(&output.path_relative) {
            return Err(fault(
                "plan-outputs",
                "planned output identities are not unique",
            ));
        }
    }
    Ok(())
}

pub(super) fn reply_fingerprint(value: &reply::BuildFingerprint) -> Result<(), NativeBuildError> {
    digest(&value.digest, "fingerprint")?;
    diagnostic(&value.summary, "fingerprint", true)
}

pub(super) fn reply_staged(values: &[reply::StagedOutput]) -> Result<(), NativeBuildError> {
    bounded_nonempty(values.len(), "staged-outputs")?;
    unique_rows(
        values
            .iter()
            .map(|v| (v.id.as_str(), v.path_relative.as_str())),
        "staged-outputs",
    )
}

pub(super) fn verified(values: &[reply::VerifiedOutput]) -> Result<(), NativeBuildError> {
    bounded_nonempty(values.len(), "verification-rows")?;
    unique_rows(
        values
            .iter()
            .map(|v| (v.id.as_str(), v.path_relative.as_str())),
        "verification-rows",
    )?;
    for value in values {
        digest(&value.digest, "verification-rows")?;
        if !is_canonical_decimal(&value.bytes) || value.bytes.parse::<u64>().is_err() {
            return Err(fault(
                "verification-rows",
                "verified byte count is not canonical u64",
            ));
        }
    }
    Ok(())
}

pub(super) fn exact_plan_outputs(
    declared: &[request::DeclaredOutput],
    planned: &[reply::PlannedOutput],
) -> Result<(), NativeBuildError> {
    if declared.len() != planned.len()
        || declared
            .iter()
            .zip(planned)
            .any(|(a, b)| a.id != b.id || reply_kind(&b.kind).as_ref().ok() != Some(&a.kind))
    {
        return Err(fault(
            "plan-outputs",
            "reply plan differs from declared output order",
        ));
    }
    Ok(())
}

fn reply_kind(value: &str) -> Result<NativeDeployArtifactKind, NativeBuildError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| fault("plan-outputs", "planned output kind is not closed"))
}

fn reply_shape(value: &str) -> Result<NativeDeployArtifactShape, NativeBuildError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| fault("plan-outputs", "planned output shape is not closed"))
}

fn kind_shape(
    kind: &NativeDeployArtifactKind,
    shape: &NativeDeployArtifactShape,
) -> Result<(), NativeBuildError> {
    let directory = matches!(
        kind,
        NativeDeployArtifactKind::Directory | NativeDeployArtifactKind::AgentPlugin
    );
    if directory == matches!(shape, NativeDeployArtifactShape::Directory) {
        Ok(())
    } else {
        Err(fault(
            "plan-outputs",
            "artifact kind and physical shape disagree",
        ))
    }
}

pub(super) fn exact_staged_outputs(
    plan: &request::BuildPlan,
    staged: &[reply::StagedOutput],
) -> Result<(), NativeBuildError> {
    if plan.outputs.len() != staged.len()
        || plan
            .outputs
            .iter()
            .zip(staged)
            .any(|(a, b)| a.id != b.id || a.path_relative != b.path_relative)
    {
        return Err(fault(
            "staged-outputs",
            "apply reply differs from accepted plan order",
        ));
    }
    Ok(())
}

pub(super) fn exact_verified_outputs(
    staged: &[request::StagedOutput],
    verified: &[reply::VerifiedOutput],
) -> Result<(), NativeBuildError> {
    if staged.len() != verified.len()
        || staged
            .iter()
            .zip(verified)
            .any(|(a, b)| a.id != b.id || a.path_relative != b.path_relative)
    {
        return Err(fault(
            "verification-rows",
            "verify reply differs from staged output order",
        ));
    }
    Ok(())
}

pub(super) fn reply_head(value: &reply::BuildReply) -> (BuildOperation, u32, u32) {
    match value {
        reply::BuildReply::Plan(v) => (BuildOperation::Plan, v.envelope, v.protocol),
        reply::BuildReply::Fingerprint(v) => (BuildOperation::Fingerprint, v.envelope, v.protocol),
        reply::BuildReply::Apply(v) => (BuildOperation::Apply, v.envelope, v.protocol),
        reply::BuildReply::Verify(v) => (BuildOperation::Verify, v.envelope, v.protocol),
    }
}

fn canonical_toml(value: Option<&str>) -> Result<(), NativeBuildError> {
    let Some(value) = value else {
        return Ok(());
    };
    let table: toml::Table = toml::from_str(value)
        .map_err(|_| fault("canonical-config", "config is not a TOML table"))?;
    if toml::to_string(&table).ok().as_deref() != Some(value) {
        return Err(fault("canonical-config", "config is not canonical TOML"));
    }
    Ok(())
}

fn unique_rows<'a>(
    values: impl Iterator<Item = (&'a str, &'a str)>,
    law: &'static str,
) -> Result<(), NativeBuildError> {
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for (id, path) in values {
        token(id, law)?;
        relative(path, "paths")?;
        if !ids.insert(id) || !paths.insert(path) {
            return Err(fault(law, "output identities are not unique"));
        }
    }
    Ok(())
}

fn absolute(value: &str) -> Result<(), NativeBuildError> {
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
        return Err(fault("paths", "absolute path has a noncanonical segment"));
    }
    Ok(())
}

fn relative(value: &str, law: &'static str) -> Result<(), NativeBuildError> {
    diagnostic(value, law, true)?;
    if relative_path_defect(value).is_some() {
        Err(fault(law, "path is not canonical relative"))
    } else {
        Ok(())
    }
}

fn join(root: &str, relative: &str) -> String {
    format!("{}/{}", root.trim_end_matches('/'), relative)
}
pub(super) fn epoch(value: u32) -> Result<(), NativeBuildError> {
    if value == ENVELOPE_EPOCH && value == PROTOCOL_EPOCH {
        Ok(())
    } else {
        Err(fault(
            "envelope-protocol",
            "wire epoch differs from admitted epoch",
        ))
    }
}
fn digest(value: &str, law: &'static str) -> Result<(), NativeBuildError> {
    if is_lowercase_hex(value, 64) {
        Ok(())
    } else {
        Err(fault(law, "digest is not 64 lowercase hex"))
    }
}
fn token(value: &str, law: &'static str) -> Result<(), NativeBuildError> {
    if is_portable_token(value) {
        Ok(())
    } else {
        Err(fault(law, "identifier is not a portable token"))
    }
}
fn bounded_collection(len: usize, law: &'static str) -> Result<(), NativeBuildError> {
    if len <= COLLECTION_CAP {
        Ok(())
    } else {
        Err(fault(law, "collection exceeds its fixed cap"))
    }
}
fn bounded_nonempty(len: usize, law: &'static str) -> Result<(), NativeBuildError> {
    if len == 0 {
        Err(fault(law, "collection must not be empty"))
    } else {
        bounded_collection(len, law)
    }
}
pub(super) fn diagnostic(
    value: &str,
    law: &'static str,
    nonblank: bool,
) -> Result<(), NativeBuildError> {
    if value.len() > crate::behaviour::native_mechanism::DIAGNOSTIC_CAP_BYTES
        || value.chars().any(char::is_control)
        || (nonblank && value.trim().is_empty())
    {
        Err(fault(law, "text violates its bounded control-free law"))
    } else {
        Ok(())
    }
}
pub(super) const fn fault(law: &'static str, message: &'static str) -> NativeBuildError {
    NativeBuildError { law, message }
}

#[cfg(test)]
mod p1_tests {
    use super::*;
    use crate::behaviour::native_build::tests::{reply_value, request_value};

    fn planned(kind: &str, shape: &str) -> reply::PlannedOutput {
        reply::PlannedOutput {
            id: "tool".to_owned(),
            kind: kind.to_owned(),
            shape: shape.to_owned(),
            path_relative: "tool".to_owned(),
        }
    }

    #[test]
    fn reply_kind_shape_and_declared_kind_are_one_exact_descriptor() {
        for (kind, shape) in [
            ("executable", "directory"),
            ("file", "directory"),
            ("directory", "file"),
            ("agent-plugin", "file"),
        ] {
            let error = reply_plan(&reply::BuildPlan {
                summary: "one output".to_owned(),
                outputs: vec![planned(kind, shape)],
            })
            .unwrap_err();
            assert_eq!(error.law(), "plan-outputs");
        }
        for (kind, shape) in [
            ("executable", "file"),
            ("skill", "file"),
            ("directory", "directory"),
            ("agent-plugin", "directory"),
        ] {
            reply_plan(&reply::BuildPlan {
                summary: "one output".to_owned(),
                outputs: vec![planned(kind, shape)],
            })
            .unwrap();
        }
        let declared = request::DeclaredOutput {
            id: "tool".to_owned(),
            kind: NativeDeployArtifactKind::Executable,
            select_toml: None,
        };
        let wrong_kind = planned("file", "file");
        assert_eq!(
            exact_plan_outputs(&[declared], &[wrong_kind])
                .unwrap_err()
                .law(),
            "plan-outputs"
        );
    }

    #[test]
    fn schema_registration_and_recursive_reader_asymmetry_remain_exact() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let registry = std::fs::read_to_string(root.join("formats/REGISTRY.toml")).unwrap();
        assert!(registry.contains("[format.native-build-request]"));
        assert!(registry.contains("[format.native-build-reply]"));
        let mut request_value = request_value("plan");
        request_value["future"] = true.into();
        request_value["target"]["future"] = true.into();
        request_value["target"]["outputs"][0]["future"] = true.into();
        assert!(serde_json::from_value::<request::BuildRequest>(request_value).is_ok());
        for path in [
            &["future"][..],
            &["result", "future"],
            &["result", "plan", "future"],
        ] {
            let mut value = reply_value("plan");
            let mut cursor = &mut value;
            for component in &path[..path.len() - 1] {
                cursor = &mut cursor[*component];
            }
            cursor[path[path.len() - 1]] = true.into();
            assert!(decode_reply(&serde_json::to_vec(&value).unwrap()).is_err());
        }
        let reply = reply_value("apply").to_string();
        for forbidden in ["provider", "record", "root_absolute", "authority"] {
            assert!(!reply.contains(forbidden));
        }
    }
}
