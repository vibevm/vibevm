//! Passive authored derivative data. Digests identify retained observations;
//! they are never package content hashes or production capture proofs.
use super::{
    SourceDescriptor, SourceInterpretation, SourceLoading, SourceRole, SourceSelector,
    declarant_path,
};
use crate::PackageRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

/// Full selected source, retained in the authored document, outside any cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedSource {
    pub owner: PackageRef,
    pub resource: String,
    pub descriptor: SourceDescriptor,
    pub files: Vec<RetainedFile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DerivationRecipe {
    #[serde(rename = "replace-selected-inventory/1")]
    ReplaceSelectedInventory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingWire {
    schema: u32,
    base_capture: String,
    base: RetainedSource,
    local: RetainedSource,
    recipe: DerivationRecipe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "BindingWire", into = "BindingWire")]
pub struct DerivationBinding(BindingWire);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid derivative binding: {0}")]
pub struct BindingError(pub &'static str);

impl RetainedSource {
    pub fn validate(&self) -> Result<(), BindingError> {
        if !self.owner.is_qualified()
            || !self.owner.version.is_exact_pin()
            || self.owner.kind.is_none()
        {
            return Err(BindingError("exact owner required"));
        }
        // Canonical, portable resource IDs cannot acquire case aliases.
        if self.resource.is_empty()
            || !self.resource.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'.')
            })
        {
            return Err(BindingError("canonical resource ID required"));
        }
        if self.files.is_empty() {
            return Err(BindingError(
                "retained source absent; cache references are not authored source",
            ));
        }
        let mut paths = BTreeSet::new();
        for file in &self.files {
            declarant_path(std::path::Path::new(&file.path))
                .map_err(|_| BindingError("source path"))?;
            let selected = match self.descriptor.selector() {
                SourceSelector::File(path) => file.path == *path,
                SourceSelector::Directory(path) => file.path.starts_with(&format!("{path}/")),
            };
            // Restrict this new grammar to ASCII portable paths, avoiding a
            // second Unicode alias normalizer beside the safefs owner.
            if !selected || !file.path.is_ascii() || !paths.insert(file.path.to_ascii_lowercase()) {
                return Err(BindingError("duplicate, alias or unselected source"));
            }
        }
        for path in &paths {
            for (offset, _) in path.match_indices('/') {
                if paths.contains(&path[..offset]) {
                    return Err(BindingError("file/directory collision"));
                }
            }
        }
        Ok(())
    }
    pub fn observation_id(&self) -> Result<String, BindingError> {
        self.validate()?;
        let mut hash = Framed::new("vibevm.derivative.observed-source/1");
        hash.field("owner", self.owner.to_string());
        hash.field("resource", &self.resource);
        hash.field(
            "selector-kind",
            match self.descriptor.selector() {
                SourceSelector::File(_) => "file",
                SourceSelector::Directory(_) => "directory",
            },
        );
        hash.field("selector-path", self.descriptor.selector().path());
        hash.field("media", self.descriptor.media());
        hash.field(
            "interpretation",
            match self.descriptor.interpretation() {
                SourceInterpretation::VibeSpec => "vibe-spec",
                SourceInterpretation::VibeDoc => "vibe-doc",
                SourceInterpretation::Commonmark => "commonmark",
                SourceInterpretation::XmlData => "xml-data",
                SourceInterpretation::Opaque => "opaque",
            },
        );
        hash.field(
            "role",
            match self.descriptor.role() {
                SourceRole::Spec => "spec",
                SourceRole::Documentation => "documentation",
                SourceRole::Resource => "resource",
            },
        );
        hash.field(
            "loading",
            match self.descriptor.loading() {
                SourceLoading::Scan => "scan",
                SourceLoading::Explicit => "explicit",
            },
        );
        hash.field("inventory", self.inventory_id()?);
        Ok(hash.finish())
    }
    pub fn inventory_id(&self) -> Result<String, BindingError> {
        self.validate()?;
        let mut hash = Framed::new("vibevm.derivative.inventory/1");
        let mut files: Vec<_> = self.files.iter().collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        hash.field("count", (files.len() as u64).to_be_bytes());
        for file in files {
            hash.field("path", &file.path);
            hash.field("bytes", &file.bytes);
        }
        Ok(hash.finish())
    }
}
impl TryFrom<BindingWire> for DerivationBinding {
    type Error = BindingError;
    fn try_from(wire: BindingWire) -> Result<Self, Self::Error> {
        if wire.schema != 1 {
            return Err(BindingError(
                "unsupported schema; explicit migration required",
            ));
        }
        wire.base.validate()?;
        wire.local.validate()?;
        if wire.base.owner.group == wire.local.owner.group
            && wire.base.owner.name == wire.local.owner.name
        {
            return Err(BindingError("upstream logical identity reused"));
        }
        if wire.base.descriptor != wire.local.descriptor {
            return Err(BindingError(
                "incompatible source interpretation or selection",
            ));
        }
        if wire.base_capture != wire.base.observation_id()? {
            return Err(BindingError("missing or stale exact base"));
        }
        Ok(Self(wire))
    }
}
impl From<DerivationBinding> for BindingWire {
    fn from(value: DerivationBinding) -> Self {
        value.0
    }
}
impl DerivationBinding {
    pub fn new(
        base: RetainedSource,
        local: RetainedSource,
        base_capture: String,
        recipe: DerivationRecipe,
    ) -> Result<Self, BindingError> {
        BindingWire {
            schema: 1,
            base_capture,
            base,
            local,
            recipe,
        }
        .try_into()
    }
    pub fn parse(text: &str) -> Result<Self, BindingError> {
        toml::from_str(text)
            .map_err(|_| BindingError("strict authored binding syntax or semantics"))
    }
    pub fn base(&self) -> &RetainedSource {
        &self.0.base
    }
    pub fn local(&self) -> &RetainedSource {
        &self.0.local
    }
    pub fn base_capture(&self) -> &str {
        &self.0.base_capture
    }
    pub fn provenance_id(&self) -> Result<String, BindingError> {
        let mut hash = Framed::new("vibevm.derivative.provenance/1");
        hash.field("base", &self.0.base_capture);
        hash.field("local", self.local().observation_id()?);
        hash.field("recipe", "replace-selected-inventory/1");
        Ok(hash.finish())
    }
    pub fn result_id(&self) -> Result<String, BindingError> {
        let mut hash = Framed::new("vibevm.derivative.result/1");
        hash.field("provenance", self.provenance_id()?);
        hash.field("inventory", self.local().inventory_id()?);
        Ok(hash.finish())
    }
    /// Validate a batch before any caller writes. There is no replacement order.
    pub fn validate_set(bindings: &[Self]) -> Result<(), BindingError> {
        let mut identities = BTreeSet::new();
        let mut footprints = BTreeMap::<_, BTreeSet<String>>::new();
        for binding in bindings {
            let local = binding.local();
            let key = (
                local.owner.group.clone(),
                local.owner.name.clone(),
                local.resource.clone(),
            );
            let paths = footprints
                .entry((local.owner.group.clone(), local.owner.name.clone()))
                .or_default();
            for file in &local.files {
                if !paths.insert(file.path.to_ascii_lowercase()) {
                    return Err(BindingError("local resource alias collision"));
                }
            }
            if !identities.insert(key) {
                return Err(BindingError("duplicate local identity"));
            }
            if bindings.iter().any(|other| {
                other.base().owner.group == local.owner.group
                    && other.base().owner.name == local.owner.name
            }) {
                return Err(BindingError("local identity aliases a selected upstream"));
            }
        }
        for paths in footprints.values() {
            for path in paths {
                for (offset, _) in path.match_indices('/') {
                    if paths.contains(&path[..offset]) {
                        return Err(BindingError("local resource file/directory collision"));
                    }
                }
            }
        }
        Ok(())
    }
}
struct Framed(Sha256);
impl Framed {
    fn new(domain: &str) -> Self {
        let mut this = Self(Sha256::new());
        this.field("domain", domain);
        this
    }
    fn field(&mut self, label: &str, bytes: impl AsRef<[u8]>) {
        let bytes = bytes.as_ref();
        self.0.update((label.len() as u64).to_be_bytes());
        self.0.update(label);
        self.0.update((bytes.len() as u64).to_be_bytes());
        self.0.update(bytes);
    }
    fn finish(self) -> String {
        format!("sha256:{:x}", self.0.finalize())
    }
}

/// This is a new authored format, with no implicit legacy upgrade. An explicit
/// identity migration reuses M13 replay and preserves every comment/token.
pub fn retain_binding_document(text: &str) -> Result<String, BindingError> {
    DerivationBinding::parse(text)?;
    super::replay_authored_tokens(text, &[]).map_err(|_| BindingError("authored token replay"))
}

#[cfg(test)]
mod m_10_b_1_cases;
