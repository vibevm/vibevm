//! Exact logical tree seal and before/after equality judgment.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::model::{EntryKind, Inventory};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreeSeal {
    pub tree_digest: String,
    pub entries: Vec<TreeSealEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreeSealEntry {
    pub path: String,
    pub kind: TreeEntryKind,
    pub sha256: Option<String>,
    pub bytes: Option<u64>,
    pub mode: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TreeEntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeDifference {
    DuplicateExpected(String),
    DuplicateObserved(String),
    Missing(String),
    Extra(String),
    Changed(String),
    DigestChanged { expected: String, observed: String },
}

impl TreeSeal {
    #[must_use]
    pub fn from_inventory(inventory: &Inventory) -> Self {
        let mut entries = inventory
            .entries
            .iter()
            .map(|entry| TreeSealEntry {
                path: entry.path.clone(),
                kind: match entry.kind {
                    EntryKind::File => TreeEntryKind::File,
                    EntryKind::Directory => TreeEntryKind::Directory,
                },
                sha256: entry.sha256.clone(),
                bytes: entry.bytes,
                mode: entry.unix_mode,
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
        Self {
            tree_digest: inventory.tree_digest.clone(),
            entries,
        }
    }

    #[must_use]
    pub fn compare(&self, observed: &Self) -> Vec<TreeDifference> {
        let mut differences = Vec::new();
        if self.tree_digest != observed.tree_digest {
            differences.push(TreeDifference::DigestChanged {
                expected: self.tree_digest.clone(),
                observed: observed.tree_digest.clone(),
            });
        }
        let expected = index(&self.entries, true, &mut differences);
        let actual = index(&observed.entries, false, &mut differences);
        for (path, expected) in &expected {
            match actual.get(path) {
                None => differences.push(TreeDifference::Missing(path.clone())),
                Some(actual) if *actual != *expected => {
                    differences.push(TreeDifference::Changed(path.clone()))
                }
                Some(_) => {}
            }
        }
        for path in actual.keys() {
            if !expected.contains_key(path) {
                differences.push(TreeDifference::Extra(path.clone()));
            }
        }
        differences
    }
}

pub fn observe(root: &Path) -> Result<TreeSeal, super::HealthError> {
    let project = vibe_safefs::Project::open(root).map_err(|error| {
        super::HealthError::Tree(format!("opening tree `{}`: {error:#}", root.display()))
    })?;
    let inventory = crate::inventory::collect(&project)
        .map_err(|error| super::HealthError::Tree(error.to_string()))?;
    Ok(TreeSeal::from_inventory(&inventory))
}

fn index<'a>(
    entries: &'a [TreeSealEntry],
    expected: bool,
    differences: &mut Vec<TreeDifference>,
) -> BTreeMap<String, &'a TreeSealEntry> {
    let mut seen = BTreeSet::new();
    let mut answer = BTreeMap::new();
    for entry in entries {
        if !seen.insert(entry.path.clone()) {
            differences.push(if expected {
                TreeDifference::DuplicateExpected(entry.path.clone())
            } else {
                TreeDifference::DuplicateObserved(entry.path.clone())
            });
        }
        answer.insert(entry.path.clone(), entry);
    }
    answer
}
