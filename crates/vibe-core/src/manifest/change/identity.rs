//! Versioned meaningful-input identities, not source authentication.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-INDEPENDENT-IDENTITIES");
use super::{ChangeError, ChangeErrorKind, ChangeManifest, WorkGraph, WorkItem, invalid};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
pub const INTENT_RECIPE: &str = "sha256:vibe-change-intent-v1";
pub const WORK_RECIPE: &str = "sha256:vibe-change-work-v1";
pub const WORK_CONTRIBUTION_RECIPE: &str = "sha256:vibe-change-work-contribution-v1";
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentIdentity {
    digest: String,
}
impl IntentIdentity {
    pub fn recipe(&self) -> &'static str {
        INTENT_RECIPE
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkIdentity {
    recipe: &'static str,
    digest: String,
}
impl WorkIdentity {
    pub fn recipe(&self) -> &'static str {
        self.recipe
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
}
#[derive(Debug, Clone, Copy)]
pub struct NormativeInput<'a> {
    pub path: &'a str,
    pub bytes: &'a [u8],
}
struct Framed(Sha256);
impl Framed {
    fn new(recipe: &str) -> Self {
        let mut hash = Sha256::new();
        hash.update(recipe.as_bytes());
        hash.update(b"\0epoch=1\0");
        Self(hash)
    }
    fn field(&mut self, label: &str, value: impl AsRef<[u8]>) {
        let value = value.as_ref();
        self.0.update((label.len() as u64).to_be_bytes());
        self.0.update(label);
        self.0.update((value.len() as u64).to_be_bytes());
        self.0.update(value);
    }
    fn count(&mut self, label: &str, n: usize) {
        self.field(label, (n as u64).to_be_bytes());
    }
    fn scope(&mut self, change: &ChangeManifest) {
        self.field("schema", change.wire.schema.to_be_bytes());
        self.field("consumer", change.coordinate());
        self.field("selected-node", change.selected_node());
        self.field("change-id", change.id().as_str());
    }
    fn finish(self) -> String {
        format!("sha256:{:x}", self.0.finalize())
    }
}
impl ChangeManifest {
    /// Exact normative observations only. Callers provide captured bytes;
    /// this function neither loads source nor promotes their trust strength.
    pub fn intent_identity(
        &self,
        inputs: &[NormativeInput<'_>],
    ) -> Result<IntentIdentity, ChangeError> {
        let mut observed = BTreeMap::new();
        for input in inputs {
            if observed.insert(input.path, input.bytes).is_some() {
                return Err(invalid(
                    ChangeErrorKind::Input,
                    "intent.inputs",
                    "duplicate normative observation",
                ));
            }
        }
        let mut documents: Vec<_> = self
            .documents()
            .iter()
            .filter(|d| d.normative && d.input)
            .collect();
        documents.sort_by(|a, b| a.path.cmp(&b.path));
        if observed.len() != documents.len()
            || documents
                .iter()
                .any(|d| !observed.contains_key(d.path.as_str()))
        {
            return Err(invalid(
                ChangeErrorKind::Input,
                "intent.inputs",
                "exact normative observation set required",
            ));
        }
        let mut hash = Framed::new(INTENT_RECIPE);
        hash.scope(self);
        let mut origins: Vec<_> = self.origins().iter().collect();
        origins.sort_by(|a, b| a.package.cmp(&b.package));
        hash.count("origins", origins.len());
        for origin in origins {
            hash.field("origin-package", &origin.package);
            hash.field("origin-manifest", &origin.manifest_sha256);
            let mut docs: Vec<_> = origin.documents.iter().collect();
            docs.sort_by(|a, b| a.address.cmp(&b.address));
            hash.count("origin-documents", docs.len());
            for doc in docs {
                hash.field("origin-address", &doc.address);
                hash.field("origin-bytes", &doc.sha256);
            }
        }
        hash.count("normative-documents", documents.len());
        for doc in documents {
            let role = match doc.role {
                super::ChangeDocumentRole::Intent => "intent",
                super::ChangeDocumentRole::Refinement => "refinement",
                super::ChangeDocumentRole::Acceptance => "acceptance",
                _ => unreachable!("shared validator refuses normative work/closure"),
            };
            hash.field("normative-role", role);
            hash.field("normative-path", &doc.path);
            hash.field("normative-bytes", observed[doc.path.as_str()]);
        }
        Ok(IntentIdentity {
            digest: hash.finish(),
        })
    }
}
fn items(hash: &mut Framed, items: &[&WorkItem]) {
    hash.count("items", items.len());
    for item in items {
        hash.field("item-id", &item.id);
        hash.count("predecessors", item.predecessors.len());
        for id in &item.predecessors {
            hash.field("predecessor", id);
        }
        hash.count("needs", item.needs.len());
        for need in &item.needs {
            hash.field("need-kind", need.kind.as_str());
            hash.field("need-target", &need.target);
        }
        hash.count("touches", item.touches.len());
        for path in &item.touches {
            hash.field("touch", path);
        }
    }
}
impl WorkGraph {
    pub fn identity(&self, change: &ChangeManifest) -> WorkIdentity {
        let mut hash = Framed::new(WORK_RECIPE);
        hash.scope(change);
        items(&mut hash, &self.items().collect::<Vec<_>>());
        WorkIdentity {
            recipe: WORK_RECIPE,
            digest: hash.finish(),
        }
    }
    pub fn contribution_identity(
        &self,
        change: &ChangeManifest,
        selected: &[&str],
    ) -> Result<WorkIdentity, ChangeError> {
        let contribution = self.contribution(selected)?;
        let mut roots = selected.to_vec();
        roots.sort();
        let mut hash = Framed::new(WORK_CONTRIBUTION_RECIPE);
        hash.scope(change);
        hash.count("selected", roots.len());
        for root in roots {
            hash.field("selected-id", root);
        }
        items(&mut hash, &contribution);
        Ok(WorkIdentity {
            recipe: WORK_CONTRIBUTION_RECIPE,
            digest: hash.finish(),
        })
    }
}
