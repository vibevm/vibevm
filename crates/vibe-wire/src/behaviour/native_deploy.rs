//! Relational admission for package-supplied native deploy wires.
//!
//! Generated types own structural decoding and reader strictness. This module
//! owns the epoch, exact-pin, canonical-set, path/config, exchange, and bounded
//! diagnostic laws JTD cannot express. It performs no loading or dispatch.

use std::collections::BTreeSet;
use std::fmt;

use super::native_mechanism::{NativeMechanismError, duplicate, epoch, scalar};
use crate::generated::native::e1::{deploy_reply, deploy_request, mechanism_manifest};

pub use super::native_mechanism::{DIAGNOSTIC_CAP_BYTES, PROTOCOL_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeployError {
    inner: NativeMechanismError,
}

impl NativeDeployError {
    pub const fn law(&self) -> &'static str {
        self.inner.law()
    }
}

impl From<NativeMechanismError> for NativeDeployError {
    fn from(inner: NativeMechanismError) -> Self {
        Self { inner }
    }
}

impl fmt::Display for NativeDeployError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "native deploy {}: {}",
            self.inner.law, self.inner.message
        )
    }
}

impl std::error::Error for NativeDeployError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}

const fn deploy_error(law: &'static str, message: &'static str) -> NativeDeployError {
    NativeDeployError {
        inner: NativeMechanismError { law, message },
    }
}

pub const ENVELOPE_EPOCH: u32 = 1;

pub const RELATIONAL_LAWS: &[&str] = &[
    "artifact-kinds",
    "canonical-config",
    "descriptor-name",
    "descriptor-id",
    "envelope-protocol",
    "exact-pin",
    "fail-message",
    "paths",
    "protocol",
    "reply-operation",
    "resource-identity",
    "role-operation-set",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployOperation {
    Plan,
    Fingerprint,
    Apply,
    Verify,
    Remove,
    Recover,
}

impl DeployOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Fingerprint => "fingerprint",
            Self::Apply => "apply",
            Self::Verify => "verify",
            Self::Remove => "remove",
            Self::Recover => "recover",
        }
    }
}

impl fmt::Display for DeployOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

pub fn validate_manifest(
    manifest: &mechanism_manifest::MechanismManifest,
) -> Result<(), NativeDeployError> {
    super::native_mechanism::validate_manifest(manifest).map_err(Into::into)
}

pub fn validate_request(
    request: &deploy_request::DeployRequest,
    expected_provider: &str,
    expected_mechanism: &str,
) -> Result<DeployOperation, NativeDeployError> {
    scalar(expected_provider, "exact-pin", true)?;
    scalar(expected_mechanism, "exact-pin", true)?;
    let (operation, envelope_value, protocol_value, identity) = match request {
        deploy_request::DeployRequest::Plan(value) => (
            DeployOperation::Plan,
            value.envelope,
            value.protocol,
            &value.identity,
        ),
        deploy_request::DeployRequest::Fingerprint(value) => (
            DeployOperation::Fingerprint,
            value.envelope,
            value.protocol,
            &value.identity,
        ),
        deploy_request::DeployRequest::Apply(value) => (
            DeployOperation::Apply,
            value.envelope,
            value.protocol,
            &value.identity,
        ),
        deploy_request::DeployRequest::Verify(value) => (
            DeployOperation::Verify,
            value.envelope,
            value.protocol,
            &value.identity,
        ),
        deploy_request::DeployRequest::Remove(value) => (
            DeployOperation::Remove,
            value.envelope,
            value.protocol,
            &value.identity,
        ),
        deploy_request::DeployRequest::Recover(value) => (
            DeployOperation::Recover,
            value.envelope,
            value.protocol,
            &value.identity,
        ),
    };
    validate_request_head(
        envelope_value,
        protocol_value,
        identity,
        expected_provider,
        expected_mechanism,
    )?;
    match request {
        deploy_request::DeployRequest::Plan(value) => {
            validate_artifact(&value.artifact)?;
            validate_authority(&value.authority)?;
            validate_config(value.config_toml.as_deref())?;
        }
        deploy_request::DeployRequest::Fingerprint(value) => {
            validate_artifact(&value.artifact)?;
            validate_config(value.config_toml.as_deref())?;
        }
        deploy_request::DeployRequest::Apply(value) => {
            validate_artifact(&value.artifact)?;
            validate_optional_absolute(value.staging.as_deref(), "staging")?;
        }
        deploy_request::DeployRequest::Recover(value) => {
            validate_artifact(&value.artifact)?;
            validate_optional_absolute(value.staging.as_deref(), "staging")?;
        }
        deploy_request::DeployRequest::Verify(_) | deploy_request::DeployRequest::Remove(_) => {}
    }
    Ok(operation)
}

pub fn validate_reply(
    expected: DeployOperation,
    reply: &deploy_reply::DeployReply,
) -> Result<(), NativeDeployError> {
    let (found, envelope_value, protocol_value) = reply_head(reply);
    epoch(envelope_value, ENVELOPE_EPOCH, "envelope-protocol")?;
    epoch(protocol_value, PROTOCOL_EPOCH, "envelope-protocol")?;
    if found != expected {
        return Err(deploy_error(
            "reply-operation",
            "reply operation differs from the retained request operation",
        ));
    }
    validate_reply_result(reply)
}

pub fn validate_exchange(
    request: &deploy_request::DeployRequest,
    expected_provider: &str,
    expected_mechanism: &str,
    reply: &deploy_reply::DeployReply,
) -> Result<(), NativeDeployError> {
    let operation = validate_request(request, expected_provider, expected_mechanism)?;
    validate_reply(operation, reply)
}

fn validate_request_head(
    envelope_value: u32,
    protocol_value: u32,
    identity: &deploy_request::InvocationIdentity,
    expected_provider: &str,
    expected_mechanism: &str,
) -> Result<(), NativeDeployError> {
    epoch(envelope_value, ENVELOPE_EPOCH, "envelope-protocol")?;
    epoch(protocol_value, PROTOCOL_EPOCH, "envelope-protocol")?;
    for (field, value) in [
        ("identity.provider", identity.provider.as_str()),
        ("identity.mechanism", identity.mechanism.as_str()),
        ("identity.target", identity.target.as_str()),
        ("identity.profile", identity.profile.as_str()),
    ] {
        let _ = field;
        scalar(value, "exact-pin", true)?;
    }
    exact_pin("identity.provider", expected_provider, &identity.provider)?;
    exact_pin(
        "identity.mechanism",
        expected_mechanism,
        &identity.mechanism,
    )
}

fn validate_artifact(artifact: &deploy_request::DeployArtifact) -> Result<(), NativeDeployError> {
    absolute_path(&artifact.path_absolute, "artifact.path_absolute")?;
    relative_path(&artifact.path_relative, "artifact.path_relative")
}

fn validate_authority(
    authority: &deploy_request::DeployAuthority,
) -> Result<(), NativeDeployError> {
    absolute_path(&authority.project_root, "authority.project_root")?;
    absolute_path(&authority.settings_root, "authority.settings_root")?;
    absolute_path(&authority.user_home, "authority.user_home")?;
    for (field, client) in [
        ("authority.clients.claude.path", &authority.clients.claude),
        ("authority.clients.codex.path", &authority.clients.codex),
        (
            "authority.clients.opencode.path",
            &authority.clients.opencode,
        ),
    ] {
        if let deploy_request::DeployClient::Resolved(value) = client {
            absolute_path(&value.path, field)?;
        }
    }
    Ok(())
}

fn validate_optional_absolute(
    path: Option<&str>,
    field: &'static str,
) -> Result<(), NativeDeployError> {
    path.map_or(Ok(()), |path| absolute_path(path, field))
}

fn validate_config(config: Option<&str>) -> Result<(), NativeDeployError> {
    let Some(config) = config else {
        return Ok(());
    };
    let table: toml::Table = toml::from_str(config)
        .map_err(|_| deploy_error("canonical-config", "config_toml is not a TOML table"))?;
    let canonical = toml::to_string(&table).map_err(|_| {
        deploy_error(
            "canonical-config",
            "config_toml cannot be canonically encoded",
        )
    })?;
    if canonical != config {
        return Err(deploy_error(
            "canonical-config",
            "config_toml is not in canonical TOML spelling",
        ));
    }
    Ok(())
}

fn reply_head(reply: &deploy_reply::DeployReply) -> (DeployOperation, u32, u32) {
    match reply {
        deploy_reply::DeployReply::Plan(value) => {
            (DeployOperation::Plan, value.envelope, value.protocol)
        }
        deploy_reply::DeployReply::Fingerprint(value) => {
            (DeployOperation::Fingerprint, value.envelope, value.protocol)
        }
        deploy_reply::DeployReply::Apply(value) => {
            (DeployOperation::Apply, value.envelope, value.protocol)
        }
        deploy_reply::DeployReply::Verify(value) => {
            (DeployOperation::Verify, value.envelope, value.protocol)
        }
        deploy_reply::DeployReply::Remove(value) => {
            (DeployOperation::Remove, value.envelope, value.protocol)
        }
        deploy_reply::DeployReply::Recover(value) => {
            (DeployOperation::Recover, value.envelope, value.protocol)
        }
    }
}

fn validate_reply_result(reply: &deploy_reply::DeployReply) -> Result<(), NativeDeployError> {
    match reply {
        deploy_reply::DeployReply::Plan(value) => match &value.result {
            deploy_reply::PlanResult::Ok(result) => {
                unique_resources(
                    result.resources.iter().map(|row| row.resource.as_str()),
                    "plan.resources",
                )?;
                unique_resources(
                    result.lock_resources.iter().map(String::as_str),
                    "plan.lock_resources",
                )
            }
            deploy_reply::PlanResult::Fail(result) => fail_message(&result.message),
        },
        deploy_reply::DeployReply::Fingerprint(value) => match &value.result {
            deploy_reply::FingerprintResult::Ok(_) => Ok(()),
            deploy_reply::FingerprintResult::Fail(result) => fail_message(&result.message),
        },
        deploy_reply::DeployReply::Apply(value) => match &value.result {
            deploy_reply::ApplyResult::Ok(result) => unique_resources(
                result.completed.iter().map(String::as_str),
                "apply.completed",
            ),
            deploy_reply::ApplyResult::Fail(result) => fail_message(&result.message),
        },
        deploy_reply::DeployReply::Verify(value) => match &value.result {
            deploy_reply::VerifyResult::Ok(result) => unique_resources(
                result.observed.iter().map(|row| row.resource.as_str()),
                "verify.observed",
            ),
            deploy_reply::VerifyResult::Fail(result) => fail_message(&result.message),
        },
        deploy_reply::DeployReply::Remove(value) => match &value.result {
            deploy_reply::RemoveResult::Ok(result) => {
                unique_resources(result.removed.iter().map(String::as_str), "remove.removed")?;
                unique_resources(
                    result.expected_remaining.iter().map(String::as_str),
                    "remove.expected_remaining",
                )
            }
            deploy_reply::RemoveResult::Fail(result) => fail_message(&result.message),
        },
        deploy_reply::DeployReply::Recover(value) => match &value.result {
            deploy_reply::RecoverResult::Ok(result) => unique_resources(
                result.completed.iter().map(String::as_str),
                "recover.completed",
            ),
            deploy_reply::RecoverResult::Fail(result) => fail_message(&result.message),
        },
    }
}

fn fail_message(message: &str) -> Result<(), NativeDeployError> {
    Ok(scalar(message, "fail-message", true)?)
}

fn unique_resources<'a>(
    resources: impl IntoIterator<Item = &'a str>,
    field: &'static str,
) -> Result<(), NativeDeployError> {
    let mut seen = BTreeSet::new();
    for resource in resources {
        let _ = field;
        scalar(resource, "resource-identity", true)?;
        if !seen.insert(resource) {
            return Err(duplicate("resource-identity").into());
        }
    }
    Ok(())
}

fn exact_pin(_field: &'static str, expected: &str, found: &str) -> Result<(), NativeDeployError> {
    if found != expected {
        return Err(deploy_error(
            "exact-pin",
            "request identity differs from the selected exact pin",
        ));
    }
    Ok(())
}

fn absolute_path(value: &str, field: &'static str) -> Result<(), NativeDeployError> {
    path_scalar(value, field)?;
    let tail = if let Some(tail) = value.strip_prefix('/') {
        tail
    } else if value.len() >= 3
        && value.as_bytes()[0].is_ascii_alphabetic()
        && value.as_bytes()[1] == b':'
        && value.as_bytes()[2] == b'/'
    {
        &value[3..]
    } else {
        return path_error(field, value, "is not an absolute forward-slashed path");
    };
    safe_segments(tail, field, value)
}

fn relative_path(value: &str, field: &'static str) -> Result<(), NativeDeployError> {
    path_scalar(value, field)?;
    if value.starts_with('/')
        || (value.len() >= 2
            && value.as_bytes()[0].is_ascii_alphabetic()
            && value.as_bytes()[1] == b':')
    {
        return path_error(field, value, "is not a relative path");
    }
    safe_segments(value, field, value)
}

fn path_scalar(value: &str, field: &'static str) -> Result<(), NativeDeployError> {
    let _ = field;
    scalar(value, "paths", true)?;
    if value.contains('\\') {
        return path_error(field, value, "contains a backslash");
    }
    Ok(())
}

fn safe_segments(tail: &str, field: &'static str, original: &str) -> Result<(), NativeDeployError> {
    if tail.is_empty() {
        return Ok(());
    }
    if tail
        .split('/')
        .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return path_error(field, original, "contains an empty, `.` or `..` segment");
    }
    Ok(())
}

fn path_error<T>(
    _field: &'static str,
    _value: &str,
    _reason: &'static str,
) -> Result<T, NativeDeployError> {
    Err(deploy_error(
        "paths",
        "path does not satisfy its absolute/relative safe spelling",
    ))
}
