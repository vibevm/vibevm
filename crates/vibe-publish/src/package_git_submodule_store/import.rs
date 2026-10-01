//! Fixed-window selected commit/tree/blob import or cold inventory.
use std::collections::BTreeSet;
use std::io::Read;

use anyhow::{Context, Result, bail};

use crate::package_git_submodule::SubmoduleLimits;
use crate::package_git_submodule::tree::parse_entries;
use crate::package_git_tree::git::LocalGit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ImportSummary {
    pub objects: usize,
    pub total_bytes: u64,
}

struct Transfer<'a, 'b, 'h> {
    source: &'a LocalGit<'b>,
    destination: Option<&'a LocalGit<'b>>,
    limits: SubmoduleLimits,
    seen: BTreeSet<String>,
    total: u64,
    after_object: &'h mut dyn FnMut(usize),
}

impl Transfer<'_, '_, '_> {
    fn object_size(&self, oid: &str, cap: u64) -> Result<u64> {
        let row = self.source.output(&["cat-file", "-s", oid], &[], 32)?;
        let size = std::str::from_utf8(&row)?
            .strip_suffix('\n')
            .context("selected Git object size row malformed")?
            .parse::<u64>()?;
        if size > cap
            || self
                .total
                .checked_add(size)
                .is_none_or(|sum| sum > self.limits.max_total_raw_bytes)
        {
            bail!("selected Git object exceeds host-chosen per-object/aggregate cap before import");
        }
        Ok(size)
    }

    fn pump(&mut self, kind: &str, oid: &str, cap: u64) -> Result<()> {
        if !self.seen.insert(oid.into()) {
            return Ok(());
        }
        let max_count = self
            .limits
            .max_files
            .checked_add(self.limits.max_directories)
            .and_then(|n| n.checked_add(self.limits.max_children_per_tree))
            .and_then(|n| n.checked_add(2))
            .context("selected Git object-count cap overflow")?;
        if self.seen.len() > max_count {
            bail!("selected Git object count exceeds host-chosen cap");
        }
        let size = self.object_size(oid, cap)?;
        let (_, source) = self.source.with_raw_object(kind, oid, cap, |reader| {
            if let Some(dest) = self.destination {
                dest.write_raw_object_streamed(kind, oid, size, cap, reader)?;
            } else {
                std::io::copy(reader, &mut std::io::sink())?;
            }
            Ok(())
        })?;
        if source.size != size {
            bail!("selected Git object size changed during import");
        }
        if let Some(dest) = self.destination {
            let (_, copied) = dest.with_raw_object(kind, oid, cap, |reader| {
                std::io::copy(reader, &mut std::io::sink())?;
                Ok(())
            })?;
            if copied.size != source.size || copied.sha256 != source.sha256 {
                bail!("retained Git object bytes differ from held source object");
            }
        }
        self.total += size;
        (self.after_object)(self.seen.len());
        Ok(())
    }

    fn tree(&mut self, oid: &str, depth: usize) -> Result<()> {
        if depth > self.limits.max_depth {
            bail!("selected Git tree exceeds host-chosen depth cap");
        }
        if self.seen.contains(oid) {
            return Ok(());
        }
        let (entries, _) =
            self.source
                .with_raw_object("tree", oid, self.limits.max_tree_bytes, |reader| {
                    parse_entries(reader, self.source.oid_len, self.limits)
                })?;
        for entry in entries {
            match entry.mode.as_str() {
                "40000" | "040000" => self.tree(&entry.oid, depth + 1)?,
                "100644" | "100755" => self.pump("blob", &entry.oid, self.limits.max_blob_bytes)?,
                "120000" => bail!("committed symlink cannot be imported as flattened raw bytes"),
                "160000" => bail!("nested Gitlink needs a distinct recursive import"),
                _ => bail!("unsupported selected Git tree mode"),
            }
        }
        self.pump("tree", oid, self.limits.max_tree_bytes)
    }
}

pub(super) fn selected_commit(
    source: &LocalGit<'_>,
    destination: Option<&LocalGit<'_>>,
    commit_oid: &str,
    tree_oid: &str,
    limits: SubmoduleLimits,
) -> Result<ImportSummary> {
    selected_commit_with_hook(source, destination, commit_oid, tree_oid, limits, |_| {})
}

pub(super) fn selected_commit_with_hook(
    source: &LocalGit<'_>,
    destination: Option<&LocalGit<'_>>,
    commit_oid: &str,
    tree_oid: &str,
    limits: SubmoduleLimits,
    mut after_object: impl FnMut(usize),
) -> Result<ImportSummary> {
    if source.oid_len != commit_oid.len()
        || destination.is_some_and(|dest| dest.oid_len != source.oid_len)
    {
        bail!("selected Git object formats differ");
    }
    let mut transfer = Transfer {
        source,
        destination,
        limits,
        seen: BTreeSet::new(),
        total: 0,
        after_object: &mut after_object,
    };
    transfer.tree(tree_oid, 0)?;
    transfer.pump("commit", commit_oid, limits.max_commit_bytes)?;
    Ok(ImportSummary {
        objects: transfer.seen.len(),
        total_bytes: transfer.total,
    })
}
