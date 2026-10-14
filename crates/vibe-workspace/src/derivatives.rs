//! Passive local replacement lifecycle; durable authoring and activation stay
//! with their existing owners. Captures are observed bytes, not capture proofs.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#COMMON-SERVICE");

use crate::source_query::CapturedSourceFile;
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::PackageRef;
use vibe_core::manifest::{SourceDescriptor, SourceSelector};

/// Immutable selected source data, reused for both single files and trees.
/// The preparation owner must retain the original authored files durably.
/// Neither this catalogue nor any fixture digest authenticates a package.
#[derive(Debug, Clone)]
pub struct SourceInput {
    coordinate: PackageRef,
    descriptor: SourceDescriptor,
    files: BTreeMap<String, CapturedSourceFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DerivativeError {
    #[error("an exact qualified package coordinate is required")]
    Coordinate,
    #[error("invalid or colliding selected inventory")]
    Inventory,
    #[error("local replacement must have a distinct package identity")]
    UpstreamIdentity,
    #[error("replacement is bound to a different base or descriptor")]
    Incompatible,
}

impl SourceInput {
    pub fn new(
        coordinate: PackageRef,
        descriptor: SourceDescriptor,
        files: Vec<CapturedSourceFile>,
    ) -> Result<Self, DerivativeError> {
        if !coordinate.is_qualified()
            || !coordinate.version.is_exact_pin()
            || coordinate.kind.is_none()
        {
            return Err(DerivativeError::Coordinate);
        }
        let mut inventory = BTreeMap::new();
        let mut identities = BTreeSet::new();
        for file in files {
            let path = file.relative_path();
            let selected = match descriptor.selector() {
                SourceSelector::File(target) => path == target,
                SourceSelector::Directory(root) => path.starts_with(&format!("{root}/")),
            };
            if !selected
                || !identities.insert(vibe_safefs::identity_key(path))
                || inventory.insert(path.to_owned(), file).is_some()
            {
                return Err(DerivativeError::Inventory);
            }
        }
        if inventory.is_empty() {
            return Err(DerivativeError::Inventory);
        }
        // A file cannot also be a parent directory in the selected output.
        for path in inventory.keys() {
            for (offset, _) in path.match_indices('/') {
                if identities.contains(&vibe_safefs::identity_key(&path[..offset])) {
                    return Err(DerivativeError::Inventory);
                }
            }
        }
        Ok(Self {
            coordinate,
            descriptor,
            files: inventory,
        })
    }
    pub fn coordinate(&self) -> &PackageRef {
        &self.coordinate
    }
    pub fn descriptor(&self) -> &SourceDescriptor {
        &self.descriptor
    }
    pub fn files(&self) -> &BTreeMap<String, CapturedSourceFile> {
        &self.files
    }
    fn same_basis(&self, other: &Self) -> bool {
        self.coordinate == other.coordinate
            && self.descriptor == other.descriptor
            && self.files.len() == other.files.len()
            && self.files.iter().all(|(path, file)| {
                other
                    .files
                    .get(path)
                    .is_some_and(|other| file.snapshot().bytes == other.snapshot().bytes)
            })
    }
}

/// Borrowed durable authored source plus the exact original basis it targets.
/// No constructor takes DerivedOutput: a cache cannot supply authored ownership.
#[derive(Debug)]
pub struct AuthoredReplacement<'a> {
    base: &'a SourceInput,
    local: &'a SourceInput,
}
impl<'a> AuthoredReplacement<'a> {
    pub fn new(base: &'a SourceInput, local: &'a SourceInput) -> Result<Self, DerivativeError> {
        if base.coordinate.group == local.coordinate.group
            && base.coordinate.name == local.coordinate.name
        {
            return Err(DerivativeError::UpstreamIdentity);
        }
        if base.descriptor != local.descriptor {
            return Err(DerivativeError::Incompatible);
        }
        Ok(Self { base, local })
    }
}

/// Disposable output cannot outlive its independently retained authored inputs.
#[derive(Debug)]
pub struct DerivedOutput<'a> {
    source: &'a SourceInput,
    base: &'a SourceInput,
}
impl DerivedOutput<'_> {
    pub fn files(&self) -> &BTreeMap<String, CapturedSourceFile> {
        self.source.files()
    }
    pub fn authored_source(&self) -> &SourceInput {
        self.source
    }
    pub fn base(&self) -> &SourceInput {
        self.base
    }
}

/// Explicit closed effect set for all five operations. No activation authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effects {
    NoExternalEffects,
}
#[derive(Debug)]
pub struct Outcome<T> {
    pub value: T,
    pub effects: Effects,
}
fn passive<T>(value: T) -> Outcome<T> {
    Outcome {
        value,
        effects: Effects::NoExternalEffects,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Change {
    Added(String),
    Removed(String),
    Replaced(String),
}

/// Inert handoff; consumers still require their own source proof, artifact
/// validation, destination ownership and activation contract. It grants nothing.
#[derive(Debug)]
pub struct ApplyHandoff<'a> {
    output: DerivedOutput<'a>,
}
impl ApplyHandoff<'_> {
    pub fn output(&self) -> &DerivedOutput<'_> {
        &self.output
    }
}

pub struct DerivativeService;
impl DerivativeService {
    pub fn derive<'a>(
        base: &'a SourceInput,
        authored: &AuthoredReplacement<'a>,
    ) -> Result<Outcome<DerivedOutput<'a>>, DerivativeError> {
        if !base.same_basis(authored.base) {
            return Err(DerivativeError::Incompatible);
        }
        Ok(passive(DerivedOutput {
            source: authored.local,
            base,
        }))
    }
    pub fn show<'a>(output: &'a DerivedOutput<'a>) -> Outcome<&'a SourceInput> {
        passive(output.source)
    }
    pub fn diff(output: &DerivedOutput<'_>) -> Outcome<Vec<Change>> {
        let mut changes = Vec::new();
        for (path, original) in output.base.files() {
            match output.files().get(path) {
                None => changes.push(Change::Removed(path.clone())),
                Some(local) if original.snapshot().bytes != local.snapshot().bytes => {
                    changes.push(Change::Replaced(path.clone()))
                }
                _ => {}
            }
        }
        for path in output
            .files()
            .keys()
            .filter(|path| !output.base.files().contains_key(*path))
        {
            changes.push(Change::Added(path.clone()));
        }
        passive(changes)
    }
    /// Explicit re-authoring against a new basis; never silently moves edits.
    pub fn rebase<'a>(
        previous: &DerivedOutput<'_>,
        expected_old: &SourceInput,
        new_base: &'a SourceInput,
        revised: &AuthoredReplacement<'a>,
    ) -> Result<Outcome<DerivedOutput<'a>>, DerivativeError> {
        if !previous.base.same_basis(expected_old) {
            return Err(DerivativeError::Incompatible);
        }
        Self::derive(new_base, revised)
    }
    pub fn apply<'a>(
        output: DerivedOutput<'a>,
        current_base: &SourceInput,
    ) -> Result<Outcome<ApplyHandoff<'a>>, DerivativeError> {
        if !output.base.same_basis(current_base) {
            return Err(DerivativeError::Incompatible);
        }
        Ok(passive(ApplyHandoff { output }))
    }
}

#[cfg(test)]
mod m_10_a_2_cases;

mod binding;
pub use binding::{BoundDerivative, prepare_binding};

pub mod compatibility;

mod materialize;
pub use materialize::{LocalDerivativeError, MaterializedDerivative, capture_retained};
mod inspection;
pub use inspection::{DerivativeInspection, FileInspection, InventoryDifference};

#[cfg(test)]
mod m_10_c_1_cases;
