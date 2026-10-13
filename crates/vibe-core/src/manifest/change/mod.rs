//! Durable authored change grammar; parsing/proposals never grant effects.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-AUTHORED-GRAMMAR");
use super::{Manifest, declarant_path, declarant_path_component};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
mod full_pass;
pub use full_pass::{
    ChangeOperationRole, PRODUCTION_REQUEST_RECIPE, ProductionBindings, ProductionObservation,
    ProductionRequestIdentity, SelectedFullPass,
};
mod identity;
mod migration;
mod rebase;
pub use rebase::{ChangeOriginRebasePlan, plan_origin_rebase};
mod validation;
mod work;
pub use identity::{
    BOUND_INTENT_RECIPE, INTENT_RECIPE, IntentIdentity, NormativeInput, WORK_CONTRIBUTION_RECIPE,
    WORK_RECIPE, WorkIdentity,
};
pub use migration::{
    CHANGE_IDENTITY, CHANGE_UPGRADE, ChangeMigrationPlan, ChangeMigrationSelection,
    plan_change_migration,
};
pub use work::{WorkGraph, WorkItem, WorkNeed, WorkNeedKind};
pub const CHANGE_SCHEMA: u32 = 1;
pub const CHANGES_DIR: &str = "changes";
pub const CHANGE_MANIFEST: &str = "change.toml";
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("change field {field}: {reason}")]
pub struct ChangeError {
    pub kind: ChangeErrorKind,
    pub field: String,
    pub reason: &'static str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeErrorKind {
    Syntax,
    UnknownEpoch,
    Identity,
    SelectedNode,
    Origin,
    Path,
    DuplicateId,
    DuplicatePath,
    Role,
    MissingRole,
    Migration,
    Work,
    Cycle,
    Input,
}
pub(crate) fn invalid(
    kind: ChangeErrorKind,
    field: impl Into<String>,
    reason: &'static str,
) -> ChangeError {
    ChangeError {
        kind,
        field: field.into(),
        reason,
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChangeId(String);
impl ChangeId {
    pub fn parse(text: &str) -> Result<Self, ChangeError> {
        let valid = text.strip_prefix("chg-").is_some_and(|s| {
            s.len() == 32
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        });
        if !valid {
            return Err(invalid(
                ChangeErrorKind::Identity,
                "change.id",
                "expected chg- and32lowercase hex digits",
            ));
        }
        declarant_path_component(text).map_err(|_| {
            invalid(
                ChangeErrorKind::Identity,
                "change.id",
                "not a portable component",
            )
        })?;
        Ok(Self(text.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Debug, Clone)]
pub struct ChangeContext {
    selected_node: String,
    coordinate: String,
    spec_root: PathBuf,
}
impl ChangeContext {
    pub fn for_node(
        manifest: &Manifest,
        selected_node: &str,
        configured_spec_root: Option<&Path>,
    ) -> Result<Self, ChangeError> {
        if selected_node != "." {
            declarant_path(Path::new(selected_node)).map_err(|_| {
                invalid(
                    ChangeErrorKind::SelectedNode,
                    "selected_node",
                    "not a portable relative node",
                )
            })?;
        }
        let node = manifest
            .consumer_node()
            .filter(|n| n.group.is_some())
            .ok_or_else(|| {
                invalid(
                    ChangeErrorKind::SelectedNode,
                    "consumer",
                    "qualified consumer identity required",
                )
            })?;
        crate::PackageName::parse(&node.name).map_err(|_| {
            invalid(
                ChangeErrorKind::SelectedNode,
                "consumer.name",
                "invalid qualified consumer name",
            )
        })?;
        let spec_root = configured_spec_root
            .map(Path::to_path_buf)
            .unwrap_or_else(crate::layout::current_specs_root);
        declarant_path(&spec_root).map_err(|_| {
            invalid(
                ChangeErrorKind::Path,
                "spec_root",
                "configured root must be portable/project-relative",
            )
        })?;
        Ok(Self {
            selected_node: selected_node.into(),
            coordinate: node.coordinate(),
            spec_root,
        })
    }
    pub fn spec_root(&self) -> &Path {
        &self.spec_root
    }
    pub fn directory(&self, id: &ChangeId) -> PathBuf {
        self.spec_root.join(CHANGES_DIR).join(id.as_str())
    }
    pub fn document_address(&self, id: &ChangeId, relative: &str) -> Result<String, ChangeError> {
        declarant_path(Path::new(relative)).map_err(|_| {
            invalid(
                ChangeErrorKind::Path,
                "document.path",
                "invalid relative document path",
            )
        })?;
        let stem = Path::new(relative).with_extension("");
        Ok(format!(
            "spec://{}/{}/{}/{}",
            self.coordinate,
            CHANGES_DIR,
            id.as_str(),
            crate::machine_json_path(&stem)
        ))
    }
    pub fn selected_node(&self) -> &str {
        &self.selected_node
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeDocumentRole {
    Intent,
    Refinement,
    Acceptance,
    Work,
    Closure,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeDocument {
    pub role: ChangeDocumentRole,
    pub path: String,
    pub normative: bool,
    pub input: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeOrigin {
    pub package: String,
    #[serde(default)]
    pub selected_content_hash: Option<String>,
    pub manifest_sha256: String,
    pub documents: Vec<ChangeOriginDocument>,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeOriginDocument {
    pub address: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Wire {
    pub schema: u32,
    pub change: Identity,
    pub origins: Vec<ChangeOrigin>,
    pub documents: Vec<ChangeDocument>,
    #[serde(default)]
    pub metadata: BTreeMap<String, toml::Value>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Identity {
    pub id: String,
    pub selected_node: String,
    pub slug: String,
    pub title: String,
}
#[derive(Debug, Clone)]
pub struct ChangeManifest {
    id: ChangeId,
    wire: Wire,
    directory: PathBuf,
    coordinate: String,
}
impl ChangeManifest {
    pub fn parse_str(
        text: &str,
        context: &ChangeContext,
        directory: &Path,
    ) -> Result<Self, ChangeError> {
        Self::parse_epoch(text, context, directory, CHANGE_SCHEMA)
    }
    pub(crate) fn parse_epoch(
        text: &str,
        context: &ChangeContext,
        directory: &Path,
        epoch: u32,
    ) -> Result<Self, ChangeError> {
        let wire: Wire = toml::from_str(text).map_err(|_| {
            invalid(
                ChangeErrorKind::Syntax,
                "manifest",
                "invalid change TOML shape",
            )
        })?;
        validation::validate(wire, context, directory, epoch)
    }
    pub fn id(&self) -> &ChangeId {
        &self.id
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn title(&self) -> &str {
        &self.wire.change.title
    }
    pub fn slug(&self) -> &str {
        &self.wire.change.slug
    }
    pub fn origins(&self) -> &[ChangeOrigin] {
        &self.wire.origins
    }
    pub fn documents(&self) -> &[ChangeDocument] {
        &self.wire.documents
    }
    pub fn metadata(&self) -> &BTreeMap<String, toml::Value> {
        &self.wire.metadata
    }
    pub fn selected_node(&self) -> &str {
        &self.wire.change.selected_node
    }
    pub fn coordinate(&self) -> &str {
        &self.coordinate
    }
}
pub fn validate_change_catalog(changes: &[ChangeManifest]) -> Result<(), ChangeError> {
    let mut ids = std::collections::BTreeSet::new();
    for c in changes {
        if !ids.insert(c.id.clone()) {
            return Err(invalid(
                ChangeErrorKind::DuplicateId,
                "change.id",
                "duplicate immutable change ID",
            ));
        }
    }
    Ok(())
}
#[cfg(test)]
mod m_02_b_1_cases;
#[cfg(test)]
mod m_02_b_2_cases;
#[cfg(test)]
mod m_02_c_1_cases;

#[cfg(test)]
mod m_03_a_2_cases;
