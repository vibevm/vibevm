//! Shared relational admission for package-authored native mechanism manifests.

use std::collections::BTreeSet;
use std::fmt;

use crate::generated::native::e1::mechanism_manifest;
use crate::generated::shared::{
    NativeDeployArtifactKind, NativeDeployOperation, NativeMechanismRole,
};

pub const PROTOCOL_EPOCH: u32 = 1;
pub const DIAGNOSTIC_CAP_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMechanismError {
    pub(crate) law: &'static str,
    pub(crate) message: &'static str,
}

impl NativeMechanismError {
    pub const fn law(&self) -> &'static str {
        self.law
    }
}

impl fmt::Display for NativeMechanismError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "native mechanism {}: {}", self.law, self.message)
    }
}

impl std::error::Error for NativeMechanismError {}

pub fn validate_manifest(
    manifest: &mechanism_manifest::MechanismManifest,
) -> Result<(), NativeMechanismError> {
    let mut ids = BTreeSet::new();
    for descriptor in &manifest.mechanisms {
        scalar(&descriptor.id, "descriptor-id", true)?;
        if !ids.insert(descriptor.id.as_str()) {
            return Err(duplicate("descriptor-id"));
        }
        scalar(&descriptor.name, "descriptor-name", true)?;
        epoch(descriptor.protocol, PROTOCOL_EPOCH, "protocol")?;
        validate_operations(&descriptor.role, &descriptor.operations)?;
        validate_artifact_kinds(&descriptor.artifact_kinds)?;
    }
    Ok(())
}

fn validate_operations(
    role: &NativeMechanismRole,
    operations: &[NativeDeployOperation],
) -> Result<(), NativeMechanismError> {
    const PRODUCE: [NativeDeployOperation; 4] = [
        NativeDeployOperation::Plan,
        NativeDeployOperation::Fingerprint,
        NativeDeployOperation::Apply,
        NativeDeployOperation::Verify,
    ];
    const DEPLOY: [NativeDeployOperation; 6] = [
        NativeDeployOperation::Plan,
        NativeDeployOperation::Fingerprint,
        NativeDeployOperation::Apply,
        NativeDeployOperation::Verify,
        NativeDeployOperation::Remove,
        NativeDeployOperation::Recover,
    ];
    let valid = match role {
        NativeMechanismRole::Build | NativeMechanismRole::Package => operations == PRODUCE,
        NativeMechanismRole::Deploy => operations == DEPLOY,
        NativeMechanismRole::Acquire => false,
    };
    if valid {
        Ok(())
    } else {
        Err(NativeMechanismError {
            law: "role-operation-set",
            message: "operations differ from the exact canonical set for the descriptor role",
        })
    }
}

fn validate_artifact_kinds(kinds: &[NativeDeployArtifactKind]) -> Result<(), NativeMechanismError> {
    if kinds.is_empty() {
        return artifact_order();
    }
    let mut previous = None;
    let mut seen = [false; 6];
    for kind in kinds {
        let rank = match kind {
            NativeDeployArtifactKind::Executable => 0,
            NativeDeployArtifactKind::Archive => 1,
            NativeDeployArtifactKind::File => 2,
            NativeDeployArtifactKind::Directory => 3,
            NativeDeployArtifactKind::Skill => 4,
            NativeDeployArtifactKind::AgentPlugin => 5,
        };
        if seen[rank] || previous.is_some_and(|prior| prior >= rank) {
            return artifact_order();
        }
        seen[rank] = true;
        previous = Some(rank);
    }
    Ok(())
}

fn artifact_order() -> Result<(), NativeMechanismError> {
    Err(NativeMechanismError {
        law: "artifact-kinds",
        message: "artifact kinds must be nonempty, unique, and canonical",
    })
}

pub(crate) fn epoch(
    found: u32,
    expected: u32,
    law: &'static str,
) -> Result<(), NativeMechanismError> {
    if found == expected {
        Ok(())
    } else {
        Err(NativeMechanismError {
            law,
            message: "wire epoch differs from the admitted epoch",
        })
    }
}

pub(crate) fn scalar(
    value: &str,
    law: &'static str,
    nonblank: bool,
) -> Result<(), NativeMechanismError> {
    let message = if nonblank && value.trim().is_empty() {
        Some("scalar is blank")
    } else if value.len() > DIAGNOSTIC_CAP_BYTES {
        Some("scalar exceeds the 8192-byte cap")
    } else if value.chars().any(char::is_control) {
        Some("scalar contains a control character")
    } else {
        None
    };
    message.map_or(Ok(()), |message| Err(NativeMechanismError { law, message }))
}

pub(crate) const fn duplicate(law: &'static str) -> NativeMechanismError {
    NativeMechanismError {
        law,
        message: "collection contains a duplicate identity",
    }
}
