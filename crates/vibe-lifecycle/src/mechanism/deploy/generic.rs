//! Closed file/config effects. Only the composed host can capture, admit and
//! approve; descriptor DTOs and serialized plans carry no authority.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use vibe_agent_projection::client_capabilities::ClientCapabilities;
use vibe_core::manifest::{ClientRequest, ClientScope};
use vibe_safefs::Project;

mod adoption;
mod apply;
mod grant;
mod inverse;
mod plan;
mod receipt;
pub use inverse::{ReceiptInspection, RetainedGrant, RetainedOperation, RetainedPlan};
#[cfg(test)]
mod m_08_c_3_cases;
#[cfg(test)]
mod tests;
pub use adoption::DeploymentDonor;
pub use apply::GenericOutcome;
pub use grant::ExactGrant;
pub use plan::{GenericPlan, GenericResource};

/// A scoped observation from the host's existing source owner, not a resolver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceIdentity {
    pub provider: String,
    pub provider_version: String,
    pub provider_digest: String,
    pub declaration: String,
    pub declaration_digest: String,
    pub descriptor: String,
    pub source: String,
    pub source_digest: String,
    pub descriptor_digest: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedObservation {
    pub identity: SourceIdentity,
    pub request: ClientRequest,
    pub selected: bool,
    pub enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControlObservation {
    pub principal: String,
    pub policy_generation: String,
}
/// Implemented by the trusted embedding host. Implementations must observe
/// only the requested captured selection, and return current policy/source
/// facts. Neither method may resolve/hydrate a source world or execute code.
pub trait GenericAuthority: Send + Sync {
    fn selected_current(&self, client: &str) -> Result<SelectedObservation, String>;
    fn current_control(&self) -> Result<ControlObservation, String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProtectedResource {
    pub scope: ClientScope,
    pub relative: String,
}
/// Host declaration of its COMPLETE control surface. Each category must be
/// explicitly populated; hosts with no such stores must supply the capability
/// boundary containing that category, not an empty denylist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProtectedControls {
    pub grants: Vec<ProtectedResource>,
    pub policy: Vec<ProtectedResource>,
    pub producer_trust: Vec<ProtectedResource>,
    pub publication_mode: Vec<ProtectedResource>,
}
impl ProtectedControls {
    fn rows(&self) -> impl Iterator<Item = &ProtectedResource> {
        self.grants
            .iter()
            .chain(&self.policy)
            .chain(&self.producer_trust)
            .chain(&self.publication_mode)
    }
}

/// Owned payload is supplied independently of source metadata. Every byte of
/// active launch configuration remains inspectable in the eventual plan.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum GenericPayload {
    File(Vec<u8>),
    Tree(BTreeMap<String, Vec<u8>>),
    JsonMember(serde_json::Value),
    TomlMember(toml::Value),
}
#[derive(Debug, Clone, PartialEq)]
pub struct CapturedClient {
    observation: SelectedObservation,
    host: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct AdmittedClient {
    capture: CapturedClient,
    payloads: Vec<GenericPayload>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericEffect {
    NoEffects,
    /// Lock/state/intent infrastructure may have been created.
    Infrastructure,
    /// A destination file/directory changed or its publication may have done so.
    Destinations,
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{reason} (effect boundary: {effect:?})")]
pub struct GenericError {
    pub reason: String,
    pub effect: GenericEffect,
}
fn refuse(reason: impl ToString) -> GenericError {
    GenericError {
        reason: reason.to_string(),
        effect: GenericEffect::NoEffects,
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn encoded(value: &impl Serialize) -> Result<Vec<u8>, GenericError> {
    serde_json::to_vec(value).map_err(refuse)
}

/// Roots and control authority are supplied by the embedding host, never by a
/// descriptor. Constructing/capturing/planning/approving creates no files.
pub struct GenericHost {
    project: Project,
    user: Project,
    settings: Project,
    authority: Arc<dyn GenericAuthority>,
    protected: ProtectedControls,
    roots: [String; 3],
    instance: u64,
    coordination: Option<Project>,
}
impl GenericHost {
    pub fn new(
        project: Project,
        user: Project,
        settings: Project,
        authority: Arc<dyn GenericAuthority>,
        protected: ProtectedControls,
    ) -> Result<Self, GenericError> {
        if [
            &protected.grants,
            &protected.policy,
            &protected.producer_trust,
            &protected.publication_mode,
        ]
        .iter()
        .any(|v| v.is_empty())
        {
            return Err(refuse("complete protected-control categories required"));
        }
        for row in protected.rows() {
            vibe_safefs::split_relative(&row.relative).map_err(refuse)?;
        }
        // Disjoint capabilities prevent alternate root spellings from hiding
        // aliases or routing descriptor writes into lifecycle state.
        for (a, b) in [(&project, &user), (&project, &settings), (&user, &settings)] {
            if vibe_safefs::paths_overlap(a.root_path(), b.root_path())
                || a.root_identity().map_err(refuse)? == b.root_identity().map_err(refuse)?
            {
                return Err(refuse(
                    "generic roots must be physically distinct and disjoint",
                ));
            }
        }
        let roots = [
            project.root_correlation_token().map_err(refuse)?,
            user.root_correlation_token().map_err(refuse)?,
            settings.root_correlation_token().map_err(refuse)?,
        ];
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Ok(Self {
            project,
            user,
            settings,
            authority,
            protected,
            roots,
            instance: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            coordination: None,
        })
    }
    /// Explicit shared lock-only capability for cross-family writers. The
    /// compatibility constructor uses its existing settings capability.
    pub fn with_coordination(mut self, arena: Project) -> Result<Self, GenericError> {
        arena.root_correlation_token().map_err(refuse)?;
        self.coordination = Some(arena);
        Ok(self)
    }
    fn arena(&self) -> &Project {
        self.coordination.as_ref().unwrap_or(&self.settings)
    }
    fn arena_identity(&self) -> Result<String, GenericError> {
        let retained = self.arena().root_correlation_token().map_err(refuse)?;
        let named = Project::open(self.arena().root_path()).map_err(refuse)?;
        if named.root_correlation_token().map_err(refuse)? != retained {
            return Err(refuse("coordination arena changed"));
        }
        Ok(retained)
    }
    fn root(&self, scope: ClientScope) -> &Project {
        match scope {
            ClientScope::Project => &self.project,
            ClientScope::User => &self.user,
        }
    }
    fn current(&self, capture: &CapturedClient) -> Result<(), GenericError> {
        if capture.host != self.instance {
            return Err(refuse("capture belongs to another host"));
        }
        let now = self
            .authority
            .selected_current(&capture.observation.request.client_id)
            .map_err(refuse)?;
        if !now.selected || !now.enabled || now != capture.observation {
            return Err(refuse(
                "selection, disabled state, provider, descriptor or source drift",
            ));
        }
        self.current_roots()
    }
    fn current_roots(&self) -> Result<(), GenericError> {
        self.arena_identity()?;
        for (root, token) in [&self.project, &self.user, &self.settings]
            .into_iter()
            .zip(&self.roots)
        {
            let named = Project::open(root.root_path()).map_err(refuse)?;
            if named.root_correlation_token().map_err(refuse)? != *token {
                return Err(refuse("root identity drift"));
            }
        }
        Ok(())
    }
    /// The canonical C1 map is checked as inert data against the CURRENT
    /// trusted selected provider. A caller-supplied row alone cannot capture.
    pub fn capture(
        &self,
        map: &ClientCapabilities,
        client: &str,
    ) -> Result<CapturedClient, GenericError> {
        let row = map
            .rows()
            .get(client)
            .ok_or_else(|| refuse("client absent from canonical map"))?;
        let observed = self.authority.selected_current(client).map_err(refuse)?;
        if !observed.selected
            || !observed.enabled
            || row.disabled
            || row.request != observed.request
            || row.provider != observed.identity.provider
            || row.descriptor_digest != observed.identity.descriptor_digest
        {
            return Err(refuse(
                "map does not match current selected enabled capture",
            ));
        }
        observed.request.validate().map_err(refuse)?;
        if [
            &observed.identity.provider,
            &observed.identity.provider_version,
            &observed.identity.declaration,
            &observed.identity.descriptor,
            &observed.identity.source,
        ]
        .iter()
        .any(|s| s.is_empty())
        {
            return Err(refuse("incomplete source identity"));
        }
        for hash in [
            &observed.identity.provider_digest,
            &observed.identity.declaration_digest,
            &observed.identity.source_digest,
            &observed.identity.descriptor_digest,
        ] {
            if hash.len() != 64
                || !hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(refuse("invalid captured identity digest"));
            }
        }
        let capture = CapturedClient {
            observation: observed,
            host: self.instance,
        };
        self.current(&capture)?;
        Ok(capture)
    }
    /// Explicit payload admission is separate from capture and approval.
    pub fn admit_payload(
        &self,
        capture: CapturedClient,
        payloads: Vec<GenericPayload>,
    ) -> Result<AdmittedClient, GenericError> {
        self.current(&capture)?;
        plan::validate_payloads(&capture.observation.request, &payloads)?;
        Ok(AdmittedClient { capture, payloads })
    }
}

#[cfg(test)]
mod m_10_d_2_cases;
