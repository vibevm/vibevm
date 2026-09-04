//! No-follow pinning for explicit absolute file names and absent destinations.

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use super::{Pinned, Project};
use crate::FileIdentity;

/// An absolute file selection whose complete parent chain is pinned no-follow.
#[derive(Debug)]
pub struct PinnedAbsoluteFile {
    parent: Pinned,
    name: String,
    parent_components: Vec<String>,
    ancestor_identities: Vec<FileIdentity>,
}

/// An absent absolute destination described by one held existing ancestor and
/// the safe components that do not exist below it.
#[derive(Debug)]
pub struct PinnedAbsentPath {
    ancestor: Pinned,
    ancestor_identity: FileIdentity,
    suffix: Vec<String>,
    ancestor_identities: Vec<FileIdentity>,
}

impl Project {
    /// Pin every parent of an absolute file without following a link or
    /// reparse point. The final name is retained for a later bounded read.
    pub fn pin_absolute_file(path: &Path) -> Result<PinnedAbsoluteFile> {
        let (anchor, mut components) = absolute_parts(path)?;
        let name = components
            .pop()
            .ok_or_else(|| anyhow::anyhow!("absolute file path has no filename"))?;
        let mut parent = open_anchor(&anchor)?;
        let mut ancestor_identities = vec![parent.identity()?];
        for component in &components {
            parent = parent.open_child(component)?;
            ancestor_identities.push(parent.identity()?);
        }
        Ok(PinnedAbsoluteFile {
            parent,
            name,
            parent_components: components,
            ancestor_identities,
        })
    }

    /// Pin the nearest existing ancestor of an absolute path and prove the
    /// requested destination itself is absent. Existing links, files and
    /// reparse points anywhere in the walk refuse.
    pub fn pin_absent_path(path: &Path) -> Result<PinnedAbsentPath> {
        let (anchor, components) = absolute_parts(path)?;
        if components.is_empty() {
            bail!("absent destination cannot be a filesystem anchor");
        }
        let mut ancestor = open_anchor(&anchor)?;
        let mut ancestor_identities = vec![ancestor.identity()?];
        for (index, component) in components.iter().enumerate() {
            match ancestor.open_child_checked(component) {
                Ok(Some(child)) => {
                    ancestor = child;
                    ancestor_identities.push(ancestor.identity()?);
                }
                Ok(None) => {
                    let ancestor_identity = *ancestor_identities
                        .last()
                        .expect("filesystem anchor identity was recorded");
                    return Ok(PinnedAbsentPath {
                        ancestor,
                        ancestor_identity,
                        suffix: components[index..].to_vec(),
                        ancestor_identities,
                    });
                }
                Err(error) => {
                    return Err(error.context(format!(
                        "pinning absent destination `{}` at component `{component}`",
                        path.display()
                    )));
                }
            }
        }
        bail!("destination `{}` already exists", path.display())
    }
}

impl PinnedAbsoluteFile {
    /// Stable bounded snapshot through the already-pinned absolute parent.
    pub fn read_snapshot_bounded(
        &self,
        project: &Project,
        cap: usize,
    ) -> Result<crate::StableFileSnapshot> {
        project
            .read_file_snapshot_bounded_in(&self.parent, &self.name, cap)?
            .ok_or_else(|| anyhow::anyhow!("`{}` is absent", self.display_path().display()))
    }

    /// Portable path below `project` when one pinned ancestor is exactly the
    /// project's held root identity. Aliased root spellings therefore classify
    /// by capability ancestry rather than lexical prefix.
    pub fn relative_to(&self, project: &Project) -> Result<Option<String>> {
        let project_identity = project.root_identity()?;
        let Some(index) = self
            .ancestor_identities
            .iter()
            .rposition(|identity| *identity == project_identity)
        else {
            return Ok(None);
        };
        let mut relative = self.parent_components[index..].to_vec();
        relative.push(self.name.clone());
        Ok(Some(relative.join("/")))
    }

    /// Absolute display spelling only; reads continue through the capability.
    pub fn display_path(&self) -> PathBuf {
        self.parent.join(&self.name)
    }
}

impl PinnedAbsentPath {
    /// Nearest existing parent held for the lifetime of this proof.
    pub fn existing_parent(&self) -> &Pinned {
        &self.ancestor
    }

    /// Whether the absent destination descends from the pinned project root,
    /// including when the absolute spelling reaches it through an alias.
    pub fn descends_from(&self, project: &Project) -> Result<bool> {
        let project_identity = project.root_identity()?;
        Ok(self.ancestor_identities.contains(&project_identity))
    }

    /// Canonical identity of the absent slot: held existing-parent identity
    /// plus the exact unresolved suffix. The parent identity stays opaque;
    /// callers receive only a domain-separated digest suitable for plan IDs.
    pub fn identity_token(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(b"vibe-safefs-absent-path-e1\0");
        hash.update(self.ancestor_identity_bytes());
        for component in &self.suffix {
            hash.update(b"\0");
            hash.update(output_component_identity(component).as_bytes());
        }
        format!("sha256:{:x}", hash.finalize())
    }

    fn ancestor_identity_bytes(&self) -> [u8; 16] {
        self.ancestor_identity.identity_bytes()
    }
}

fn absolute_parts(path: &Path) -> Result<(PathBuf, Vec<String>)> {
    if !path.is_absolute() {
        bail!("path must be absolute: `{}`", path.display());
    }
    let mut anchor = PathBuf::new();
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => anchor.push(prefix.as_os_str()),
            Component::RootDir => anchor.push(std::path::MAIN_SEPARATOR_STR),
            Component::Normal(value) => {
                let component = value
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("path is not UTF-8"))?
                    .to_owned();
                crate::ensure_safe_component(&component)?;
                components.push(component);
            }
            Component::CurDir | Component::ParentDir => {
                bail!(
                    "absolute path contains a dot component: `{}`",
                    path.display()
                );
            }
        }
    }
    Ok((anchor, components))
}

#[cfg(windows)]
fn output_component_identity(component: &str) -> String {
    component.to_lowercase()
}

#[cfg(not(windows))]
fn output_component_identity(component: &str) -> String {
    component.to_owned()
}

fn open_anchor(path: &Path) -> Result<Pinned> {
    let dir = cap_std::fs::Dir::open_ambient_dir(path, cap_std::ambient_authority())
        .with_context(|| format!("opening filesystem anchor `{}`", path.display()))?;
    Ok(Pinned {
        dir,
        path: path.to_path_buf(),
    })
}
