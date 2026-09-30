//! Explicit captured-inventory authored-fact walk.
//!
//! The host supplies already read bytes. This entry neither walks a suffix
//! tree nor opens files; every semantic read goes through the selected source
//! descriptor before `vibe-specdoc` sees those bytes.

use std::collections::{BTreeMap, BTreeSet};

use vibe_core::manifest::{PackageMeta, SourceSelector};

use super::{DeclaredDocumentObservation, observe_declared_document};
use crate::AuthoredFact;

/// One host-captured package-relative path. `None` means unavailable bytes,
/// not an empty file. The host retains origin/containment authority.
#[derive(Debug, Clone, Copy)]
pub struct CapturedDeclaredDocument<'a> {
    pub path: &'a str,
    pub bytes: Option<&'a [u8]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredWalkError {
    InvalidCoordinate(String),
    InvalidPath(String),
    DuplicatePath(String),
    DuplicateFact {
        address: String,
        first: String,
        second: String,
    },
}

/// Per-file observations retain raw/unavailable/ambiguous states; `facts`
/// includes only selected native readers and is complete only for the
/// supplied inventory plus exact-file declarations synthesized as missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredTreeObservation<'a> {
    pub documents: Vec<(String, DeclaredDocumentObservation<'a>)>,
    pub facts: Vec<AuthoredFact>,
    /// False when an unclassified, ambiguous, unavailable, unsupported or
    /// invalid file could still change the authored fact set, or a directory
    /// default lacks a host-proved complete descendant census.
    pub complete: bool,
}

/// Join a bounded host inventory to package declarations before parsing.
/// Exact-file rows missing from the inventory become `Unavailable`; directory
/// defaults classify supplied descendants but this function cannot invent a
/// directory census the host did not provide.
pub fn observe_declared_source<'a>(
    package: &PackageMeta,
    captured: &[CapturedDeclaredDocument<'a>],
) -> Result<DeclaredTreeObservation<'a>, DeclaredWalkError> {
    let coordinate = format!("{}/{}", package.group, package.name);
    super::super::split_coordinate(&coordinate)
        .map_err(|error| DeclaredWalkError::InvalidCoordinate(error.to_string()))?;
    let mut paths = BTreeMap::<String, Option<&'a [u8]>>::new();
    let mut folded = BTreeSet::new();
    for row in captured {
        package
            .select_source(row.path)
            .map_err(|error| DeclaredWalkError::InvalidPath(format!("{error:?}")))?;
        if !folded.insert(row.path.to_lowercase()) {
            return Err(DeclaredWalkError::DuplicatePath(row.path.to_owned()));
        }
        paths.insert(row.path.to_owned(), row.bytes);
    }
    let mut declared_spellings = BTreeMap::<String, String>::new();
    for descriptor in &package.sources {
        if let SourceSelector::File(path) = descriptor.selector() {
            if let Some(first) = declared_spellings.insert(path.to_lowercase(), path.clone())
                && first != *path
            {
                return Err(DeclaredWalkError::DuplicatePath(path.clone()));
            }
            paths.entry(path.clone()).or_insert(None);
        }
    }
    let mut documents = Vec::with_capacity(paths.len());
    let mut facts = Vec::new();
    // A supplied inventory does not prove that every descendant of a declared
    // directory was supplied. Until the host carries a separate census proof,
    // even successfully parsed descendants are only a partial observation.
    let mut complete = !package
        .sources
        .iter()
        .any(|descriptor| matches!(descriptor.selector(), SourceSelector::Directory(_)));
    let mut minter = BTreeMap::<String, String>::new();
    for (path, bytes) in paths {
        let observed = observe_declared_document(package, &path, bytes)
            .map_err(|error| DeclaredWalkError::InvalidPath(format!("{error:?}")))?;
        if let DeclaredDocumentObservation::Facts { facts: rows, .. } = &observed {
            for fact in rows {
                if let Some(first) = minter.insert(fact.address.clone(), path.clone()) {
                    return Err(DeclaredWalkError::DuplicateFact {
                        address: fact.address.clone(),
                        first,
                        second: path,
                    });
                }
                facts.push(fact.clone());
            }
        } else if !matches!(&observed, DeclaredDocumentObservation::Raw { .. }) {
            complete = false;
        }
        documents.push((path, observed));
    }
    facts.sort_by(|left, right| left.address.cmp(&right.address));
    Ok(DeclaredTreeObservation {
        documents,
        facts,
        complete,
    })
}

#[cfg(test)]
mod tests;
