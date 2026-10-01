//! Strict raw Git tree walk and exact flattened-vault row comparison.
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use crate::package_git_payload::FileRow;
use crate::package_git_tree::git::LocalGit;

use super::{MetadataBlob, SubmoduleLimits};

const TREE_DOMAIN: &[u8] = b"vibevm/package-git-populated-submodule/e1\0";

pub(super) fn normalized_path(path: &str, max_path: usize) -> Result<Vec<&str>> {
    if path.is_empty() || path.len() > max_path {
        bail!("Gitlink path exceeds host-selected cap");
    }
    let components: Vec<&str> = path.split('/').collect();
    if components
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == ".." || part.contains('\\'))
    {
        bail!("Gitlink path is not a normalized source directory");
    }
    Ok(components)
}

#[derive(Debug)]
struct Entry {
    mode: String,
    name: String,
    oid: String,
}

fn raw_oid_hex(raw: &[u8]) -> String {
    let mut out = String::with_capacity(raw.len() * 2);
    for byte in raw {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn read_byte(reader: &mut dyn Read) -> Result<Option<u8>> {
    let mut one = [0_u8; 1];
    match reader.read(&mut one)? {
        0 => Ok(None),
        1 => Ok(Some(one[0])),
        _ => unreachable!(),
    }
}

fn parse_entries(
    reader: &mut dyn Read,
    oid_len: usize,
    limits: SubmoduleLimits,
) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    let raw_oid_len = oid_len / 2;
    loop {
        let Some(first) = read_byte(reader)? else {
            break;
        };
        let mut header = vec![first];
        loop {
            let next = read_byte(reader)?.context("Git tree entry ended before NUL")?;
            if next == 0 {
                break;
            }
            if header.len() >= limits.max_path_bytes.saturating_add(8) {
                bail!("Git tree entry header exceeds host-selected path cap");
            }
            header.push(next);
        }
        let space = header
            .iter()
            .position(|b| *b == b' ')
            .context("Git tree entry has no mode separator")?;
        let mode = std::str::from_utf8(&header[..space])?;
        let name = std::str::from_utf8(&header[space + 1..])?;
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.len() > limits.max_path_bytes
            || name.contains(['/', '\\', '\0'])
            || name.chars().any(char::is_control)
        {
            bail!("Git tree child name is not a portable selected path");
        }
        if !seen.insert(name.to_lowercase()) {
            bail!("Git tree has a case-fold alias or duplicate child");
        }
        if !matches!(
            mode,
            "100644" | "100755" | "40000" | "040000" | "120000" | "160000"
        ) {
            bail!("Git tree child has unsupported mode");
        }
        let mut oid_raw = vec![0_u8; raw_oid_len];
        reader.read_exact(&mut oid_raw)?;
        entries.push(Entry {
            mode: mode.into(),
            name: name.into(),
            oid: raw_oid_hex(&oid_raw),
        });
        if entries.len() > limits.max_children_per_tree {
            bail!("Git tree child count exceeds host-selected cap");
        }
    }
    Ok(entries)
}

fn frame(hash: &mut Sha256, tag: &[u8], bytes: &[u8]) {
    hash.update((tag.len() as u64).to_be_bytes());
    hash.update(tag);
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

pub(super) struct VaultRows {
    pub files: BTreeMap<String, FileRow>,
    pub directories: BTreeSet<String>,
}

pub(super) fn selected_vault_rows(
    files: &[FileRow],
    directories: &[String],
    prefix: &str,
) -> Result<VaultRows> {
    let selected = format!("{prefix}/");
    let mut selected_files = BTreeMap::new();
    for row in files {
        if let Some(relative) = row.path.strip_prefix(&selected) {
            if relative.is_empty()
                || selected_files
                    .insert(relative.into(), row.clone())
                    .is_some()
            {
                bail!("duplicate or empty flattened vault file path");
            }
        }
    }
    let mut selected_dirs = BTreeSet::new();
    for path in directories {
        if let Some(relative) = path.strip_prefix(&selected)
            && (!relative.is_empty() && !selected_dirs.insert(relative.into()))
        {
            bail!("duplicate flattened vault directory");
        }
    }
    if !directories.iter().any(|path| path == prefix) {
        bail!("vault lacks indexed submodule directory");
    }
    Ok(VaultRows {
        files: selected_files,
        directories: selected_dirs,
    })
}

pub(super) struct TreeComparison {
    pub files: usize,
    pub directories: usize,
    pub total_raw_bytes: u64,
    pub metadata: Vec<MetadataBlob>,
    pub digest: String,
}

struct Walk<'a> {
    git: &'a LocalGit<'a>,
    limits: SubmoduleLimits,
    expected: VaultRows,
    files: usize,
    directories: usize,
    total_raw_bytes: u64,
    metadata: Vec<MetadataBlob>,
    digest: Sha256,
}

impl Walk<'_> {
    fn add_bytes(&mut self, bytes: u64) -> Result<()> {
        self.total_raw_bytes = self
            .total_raw_bytes
            .checked_add(bytes)
            .context("Git submodule total byte overflow")?;
        if self.total_raw_bytes > self.limits.max_total_raw_bytes {
            bail!("Git submodule raw bytes exceed host-selected total");
        }
        Ok(())
    }

    fn tree(&mut self, oid: &str, parent: &str, depth: usize) -> Result<()> {
        if depth > self.limits.max_depth {
            bail!("Git submodule tree exceeds host-selected depth");
        }
        let (entries, tree_witness) =
            self.git
                .with_raw_object("tree", oid, self.limits.max_tree_bytes, |reader| {
                    parse_entries(reader, self.git.oid_len, self.limits)
                })?;
        frame(&mut self.digest, b"tree-oid", oid.as_bytes());
        frame(
            &mut self.digest,
            b"tree-sha",
            tree_witness.sha256.as_bytes(),
        );
        for entry in entries {
            let path = if parent.is_empty() {
                entry.name.clone()
            } else {
                format!("{parent}/{}", entry.name)
            };
            if path.len() > self.limits.max_path_bytes {
                bail!("Git submodule path exceeds host-selected cap");
            }
            match entry.mode.as_str() {
                "40000" | "040000" => {
                    self.directories += 1;
                    if self.directories > self.limits.max_directories {
                        bail!("Git submodule directory count exceeds host-selected cap");
                    }
                    if !self.expected.directories.remove(&path) {
                        bail!("Git submodule directory absent from exact vault");
                    }
                    frame(&mut self.digest, b"dir", path.as_bytes());
                    self.tree(&entry.oid, &path, depth + 1)?;
                }
                "120000" => bail!("Git submodule committed symlink is unsupported"),
                "160000" => bail!("nested Gitlink requires a separate recursive proof"),
                "100644" | "100755" => {
                    let (_, blob) = self.git.with_raw_object(
                        "blob",
                        &entry.oid,
                        self.limits.max_blob_bytes,
                        |reader| {
                            std::io::copy(reader, &mut std::io::sink())?;
                            Ok(())
                        },
                    )?;
                    self.add_bytes(blob.size)?;
                    if entry.name == ".gitmodules" {
                        self.metadata.push(MetadataBlob {
                            path: path.clone(),
                            git_oid: entry.oid.clone(),
                            sha256: blob.sha256.clone(),
                            size_bytes: blob.size,
                        });
                        frame(&mut self.digest, b"gitmodules", path.as_bytes());
                        frame(&mut self.digest, b"gitmodules-sha", blob.sha256.as_bytes());
                        continue;
                    }
                    if entry.name == ".git" {
                        bail!("committed .git entry cannot be flattened");
                    }
                    let row = self
                        .expected
                        .files
                        .remove(&path)
                        .context("committed Git file absent from flattened vault")?;
                    if row.size_bytes != blob.size || row.sha256 != blob.sha256 {
                        bail!("flattened vault bytes differ from raw committed Git blob");
                    }
                    match row.unix_mode {
                        Some(bits) if (bits & 0o111 != 0) == (entry.mode == "100755") => {}
                        None if entry.mode == "100644" => {}
                        None => bail!("Windows mode witness cannot prove executable Git blob yet"),
                        Some(_) => bail!("flattened vault mode differs from committed Git mode"),
                    }
                    self.files += 1;
                    if self.files > self.limits.max_files {
                        bail!("Git submodule file count exceeds host-selected cap");
                    }
                    frame(&mut self.digest, b"file", path.as_bytes());
                    frame(&mut self.digest, b"mode", entry.mode.as_bytes());
                    frame(&mut self.digest, b"blob-oid", entry.oid.as_bytes());
                    frame(&mut self.digest, b"blob-sha", blob.sha256.as_bytes());
                }
                _ => bail!("Git submodule tree mode unsupported"),
            }
        }
        Ok(())
    }
}

pub(super) fn compare(
    git: &LocalGit<'_>,
    tree_oid: &str,
    vault: VaultRows,
    limits: SubmoduleLimits,
) -> Result<TreeComparison> {
    let mut walk = Walk {
        git,
        limits,
        expected: vault,
        files: 0,
        directories: 0,
        total_raw_bytes: 0,
        metadata: Vec::new(),
        digest: Sha256::new(),
    };
    walk.digest.update(TREE_DOMAIN);
    walk.tree(tree_oid, "", 0)?;
    if !walk.expected.files.is_empty() {
        bail!("flattened vault has untracked/extra submodule files");
    }
    if !walk.expected.directories.is_empty() {
        bail!("flattened vault has extra or empty uncommitted directories");
    }
    Ok(TreeComparison {
        files: walk.files,
        directories: walk.directories,
        total_raw_bytes: walk.total_raw_bytes,
        metadata: walk.metadata,
        digest: format!("sha256:{:x}", walk.digest.finalize()),
    })
}
