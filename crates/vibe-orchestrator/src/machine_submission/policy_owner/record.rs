use super::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use vibe_lifecycle::process::{SelectedFileWitness, is_valid_run_id, propose_run_id};

pub(super) const CAP: usize = 16 * 1024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Scope {
    context: String,
    target: String,
    principal: String,
}
impl Scope {
    fn from_host(scope: &SubmissionScope) -> Self {
        Self {
            context: scope.context.clone(),
            target: scope.target.clone(),
            principal: scope.principal.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Facts {
    approve_captured_script: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    approve_exact_local_install: bool,
    allow_detach: bool,
    allow_cancel_pre_dispatch: bool,
    allow_cancel_deferred_boundary: bool,
}
impl Facts {
    pub(super) fn from_host(f: HostPolicyFacts) -> Self {
        Self {
            approve_captured_script: f.approve_captured_script,
            approve_exact_local_install: f.approve_exact_local_install,
            allow_detach: f.allow_detach,
            allow_cancel_pre_dispatch: f.allow_cancel_pre_dispatch,
            allow_cancel_deferred_boundary: f.allow_cancel_deferred_boundary,
        }
    }
    pub(super) fn host(&self) -> HostPolicyFacts {
        HostPolicyFacts {
            approve_captured_script: self.approve_captured_script,
            approve_exact_local_install: self.approve_exact_local_install,
            allow_detach: self.allow_detach,
            allow_cancel_pre_dispatch: self.allow_cancel_pre_dispatch,
            allow_cancel_deferred_boundary: self.allow_cancel_deferred_boundary,
        }
    }
}
fn is_false(value: &bool) -> bool {
    !*value
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    schema: u32,
    root_token: String,
    scope: Scope,
    owner_id: String,
    pub(super) revision: u64,
    pub(super) source: String,
    pub(super) facts: Facts,
}
pub(super) fn validate_text(text: &str) -> Result<(), PolicyError> {
    if text.is_empty()
        || text.len() > 256
        || text.trim() != text
        || text.chars().any(char::is_control)
    {
        return Err(PolicyError::InvalidRecord(
            "identity must be nonempty trimmed UTF-8, at most 256 bytes, without controls".into(),
        ));
    }
    Ok(())
}
pub(super) fn validate_scope(scope: &SubmissionScope) -> Result<(), PolicyError> {
    for text in [&scope.context, &scope.target, &scope.principal] {
        validate_text(text)?;
    }
    Ok(())
}
fn field(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    out.extend_from_slice(bytes);
}
fn scope_bytes(out: &mut Vec<u8>, scope: &Scope) {
    for text in [&scope.context, &scope.target, &scope.principal] {
        field(out, text.as_bytes());
    }
}
fn hash(bytes: Vec<u8>) -> String {
    SelectedFileWitness::from_bytes(bytes).sha256
}
pub(super) fn path(scope: &SubmissionScope) -> String {
    let mut bytes = b"vibevm/machine-policy/scope/e1\0".to_vec();
    scope_bytes(&mut bytes, &Scope::from_host(scope));
    let digest = hash(bytes);
    let hex = digest
        .strip_prefix("sha256:")
        .expect("SHA-256 witness prefix");
    assert!(
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
    );
    format!(".vibe/machine-policy/{hex}.json")
}
impl Record {
    pub(super) fn new(
        scope: &SubmissionScope,
        root_token: &str,
        source: TrustedPolicySource,
        facts: HostPolicyFacts,
        diagnostic_root: &Path,
    ) -> Result<Self, PolicyError> {
        Ok(Self {
            schema: 1,
            root_token: root_token.into(),
            scope: Scope::from_host(scope),
            owner_id: propose_run_id(diagnostic_root).map_err(infrastructure)?,
            revision: 1,
            source: source.0,
            facts: Facts::from_host(facts),
        })
    }
    pub(super) fn validate(
        &self,
        scope: &SubmissionScope,
        root_token: &str,
    ) -> Result<(), PolicyError> {
        validate_text(&self.source)?;
        for text in [
            &self.scope.context,
            &self.scope.target,
            &self.scope.principal,
        ] {
            validate_text(text)?;
        }
        let valid_token = self.root_token.strip_prefix("sha256:").is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        });
        if self.schema != 1
            || self.revision == 0
            || !is_valid_run_id(&self.owner_id)
            || !valid_token
        {
            return Err(PolicyError::InvalidRecord(
                "invalid schema, revision, owner identity or root token".into(),
            ));
        }
        if self.scope != Scope::from_host(scope) || self.root_token != root_token {
            return Err(PolicyError::ForeignScope);
        }
        Ok(())
    }
    pub(super) fn snapshot(&self) -> PolicySnapshot {
        let mut bytes = b"vibevm/machine-policy/generation/e1\0".to_vec();
        field(&mut bytes, self.owner_id.as_bytes());
        field(&mut bytes, &self.revision.to_be_bytes());
        field(&mut bytes, self.source.as_bytes());
        field(
            &mut bytes,
            &[
                self.facts.approve_captured_script as u8,
                self.facts.allow_detach as u8,
                self.facts.allow_cancel_pre_dispatch as u8,
                self.facts.allow_cancel_deferred_boundary as u8,
            ],
        );
        // Preserve old generations when the new permission remains denied.
        // An enabled Install grant must have a distinct generation.
        if self.facts.approve_exact_local_install {
            field(&mut bytes, b"exact-local-install:enabled");
        }
        field(&mut bytes, self.root_token.as_bytes());
        scope_bytes(&mut bytes, &self.scope);
        PolicySnapshot {
            generation: hash(bytes),
            facts: self.facts.host(),
            source: TrustedPolicySource(self.source.clone()),
        }
    }
}
