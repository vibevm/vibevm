//! Complete bounded capability-relative census of the selected source tree.
use std::collections::BTreeMap;

use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use vibe_safefs::{ChildKind, Project};

use super::{Census, FileRow, PayloadLimits, TREE_DOMAIN};

pub(super) fn scan(project: &Project, limits: &PayloadLimits, skip_git: bool) -> Result<Census> {
    let mut scan = Scan {
        project,
        limits,
        skip_git,
        directories: Vec::new(),
        files: Vec::new(),
        aliases: BTreeMap::new(),
        total_bytes: 0,
        skipped_gitmodules: false,
    };
    scan.directory("")?;
    scan.directories.sort();
    scan.files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut hash = Sha256::new();
    hash.update(TREE_DOMAIN);
    for directory in &scan.directories {
        frame(&mut hash, b"directory", directory.as_bytes());
    }
    for file in &scan.files {
        frame(&mut hash, b"file", file.path.as_bytes());
        frame(&mut hash, b"size", &file.size_bytes.to_be_bytes());
        frame(&mut hash, b"sha256", file.sha256.as_bytes());
        frame(&mut hash, b"mode", &serde_json::to_vec(&file.unix_mode)?);
    }
    Ok(Census {
        directories: scan.directories,
        files: scan.files,
        skipped_gitmodules: scan.skipped_gitmodules,
        tree_sha256: format!("sha256:{:x}", hash.finalize()),
    })
}

fn frame(hash: &mut Sha256, tag: &[u8], value: &[u8]) {
    hash.update((tag.len() as u64).to_be_bytes());
    hash.update(tag);
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value);
}

struct Scan<'a> {
    project: &'a Project,
    limits: &'a PayloadLimits,
    skip_git: bool,
    directories: Vec<String>,
    files: Vec<FileRow>,
    aliases: BTreeMap<String, String>,
    total_bytes: u64,
    skipped_gitmodules: bool,
}
impl Scan<'_> {
    fn directory(&mut self, parent: &str) -> Result<()> {
        let components: Vec<&str> = if parent.is_empty() {
            Vec::new()
        } else {
            parent.split('/').collect()
        };
        let held = self.project.dir(&components, false)?;
        let mut names = self
            .project
            .child_names_bounded(&held, self.limits.max_children_per_directory)?;
        names.sort();
        for name in names {
            let reserved_fold = name.to_lowercase();
            if matches!(reserved_fold.as_str(), ".git" | ".gitmodules")
                && !matches!(name.as_str(), ".git" | ".gitmodules")
            {
                bail!("portable Git metadata alias is not publishable: `{name}`");
            }
            if matches!(name.as_str(), ".git" | ".gitmodules") {
                if self.skip_git {
                    self.skipped_gitmodules |= name == ".gitmodules";
                    continue;
                }
                bail!("staged payload contains forbidden Git metadata entry `{name}`");
            }
            let relative = if parent.is_empty() {
                name.clone()
            } else {
                format!("{parent}/{name}")
            };
            let depth = relative.split('/').count();
            if depth > self.limits.max_depth || relative.len() > self.limits.max_path_bytes {
                bail!(
                    "package Git payload path exceeds caller-selected depth/byte limit: `{relative}`"
                );
            }
            let key = relative.to_lowercase();
            if let Some(existing) = self.aliases.insert(key, relative.clone())
                && existing != relative
            {
                bail!("portable case-fold collision: `{existing}` and `{relative}`");
            }
            match self.project.child_kind_nofollow(&held, &name)? {
                Some(ChildKind::Directory) => {
                    if self.directories.len() >= self.limits.max_directories {
                        bail!("package Git payload exceeds caller-selected directory count");
                    }
                    self.directories.push(relative.clone());
                    self.directory(&relative)?;
                }
                Some(ChildKind::RegularFile) => {
                    if self.files.len() >= self.limits.max_files {
                        bail!("package Git payload exceeds caller-selected file count");
                    }
                    let state = self
                        .project
                        .stable_file_state_bounded(&relative, self.limits.max_file_bytes)?
                        .ok_or_else(|| {
                            anyhow::anyhow!("file disappeared during census: `{relative}`")
                        })?;
                    self.total_bytes =
                        self.total_bytes.checked_add(state.bytes).ok_or_else(|| {
                            anyhow::anyhow!("package Git payload byte total overflow")
                        })?;
                    if self.total_bytes > self.limits.max_total_bytes {
                        bail!("package Git payload exceeds caller-selected aggregate byte limit");
                    }
                    self.files.push(FileRow {
                        path: relative,
                        size_bytes: state.bytes,
                        sha256: format!("sha256:{}", state.sha256),
                        unix_mode: state.unix_mode,
                    });
                }
                None => bail!("entry disappeared during census: `{relative}`"),
            }
        }
        Ok(())
    }
}
