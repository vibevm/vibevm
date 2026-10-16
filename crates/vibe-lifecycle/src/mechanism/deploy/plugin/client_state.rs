//! Trusted embedding contract for finite client-list currentness. No vendor
//! storage layout is inferred here; an unqualified client remains unavailable.
use std::{collections::BTreeMap, path::PathBuf};

/// Exact list semantics selected by the lifecycle owner (currently user scope).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientStateBinding {
    pub client: String,
    pub coordinate: String,
    pub scope: String,
    pub user_home: PathBuf,
}

/// An owner must account for ALL state affecting the selected list/activation
/// semantics, including absence and membership. Implementations may only read;
/// no process, credential/history scan, hydration or publication is permitted.
/// This trusted Rust injection is not a wire/path grant or a boolean assertion
/// that an arbitrary vendor layout is complete. Owners document their format.
pub trait ClientStateOwner: Send + Sync {
    fn observe(&self, binding: &ClientStateBinding) -> Result<ClientStateWitness, String>;
}

/// Finite exact file membership and bytes, including selected absent files.
/// Only the qualified owner can choose this complete closure; SafeFS proves
/// observations, not the owner's semantic completeness claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientStateWitness {
    owner: String,
    binding: ClientStateBinding,
    root: PathBuf,
    root_identity: String,
    files: BTreeMap<String, Option<vibe_safefs::StableFileSnapshot>>,
    directories: BTreeMap<String, Option<DirectoryMembership>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectoryMembership {
    identity: String,
    children: BTreeMap<String, vibe_safefs::ChildKind>,
}
impl ClientStateWitness {
    pub fn observe_files(
        owner: &str,
        binding: &ClientStateBinding,
        root: &std::path::Path,
        files: &[&str],
    ) -> Result<Self, String> {
        Self::observe_closure(owner, binding, root, files, &[])
    }

    /// Observe an explicitly qualified finite closure without discovering a
    /// vendor layout. `directories` names each selected directory ("." is the
    /// admitted root); absence is retained. Every direct child name and type is
    /// observed through SafeFS, including otherwise unknown cache versions.
    ///
    /// At most 64 files (1 MiB each), 64 directories, 64 children per directory
    /// and 256 child entries total are admitted. Limits refuse, never truncate.
    /// Directory observations bracket file reads and the named root is reopened
    /// before returning. This is finite currentness evidence, not hostile ABA
    /// atomicity or an immutable namespace guarantee. The trusted owner still
    /// establishes semantic completeness and refuses unqualified frontiers.
    pub fn observe_closure(
        owner: &str,
        binding: &ClientStateBinding,
        root: &std::path::Path,
        files: &[&str],
        directories: &[&str],
    ) -> Result<Self, String> {
        if owner.is_empty()
            || files.is_empty()
            || files.len() > 64
            || directories.len() > 64
            || !root.is_absolute()
        {
            return Err("client-state owner requires a nonempty finite qualified closure".into());
        }
        let project = vibe_safefs::Project::open(root).map_err(|e| e.to_string())?;
        let root_identity = project
            .root_correlation_token()
            .map_err(|e| e.to_string())?;
        let before = directory_membership(&project, directories)?;
        let mut observed = BTreeMap::new();
        for name in files {
            let snapshot = project
                .read_file_snapshot_bounded(name, 1024 * 1024)
                .map_err(|e| e.to_string())?;
            if observed.insert((*name).to_owned(), snapshot).is_some() {
                return Err("duplicate client-state member".into());
            }
        }
        if directory_membership(&project, directories)? != before
            || vibe_safefs::Project::open(root)
                .and_then(|p| p.root_correlation_token())
                .map_err(|e| e.to_string())?
                != root_identity
        {
            return Err("client-state closure changed during observation".into());
        }
        Ok(Self {
            owner: owner.into(),
            binding: binding.clone(),
            root: std::fs::canonicalize(root).map_err(|e| e.to_string())?,
            root_identity,
            files: observed,
            directories: before,
        })
    }
    pub(super) fn matches_binding(&self, binding: &ClientStateBinding) -> bool {
        self.binding == *binding
    }
}

fn directory_membership(
    project: &vibe_safefs::Project,
    directories: &[&str],
) -> Result<BTreeMap<String, Option<DirectoryMembership>>, String> {
    let mut result = BTreeMap::new();
    let mut total = 0usize;
    for name in directories {
        let components = if *name == "." {
            Vec::new()
        } else {
            let parts: Vec<_> = name.split('/').collect();
            if parts.iter().any(|p| matches!(*p, "" | "." | "..")) {
                return Err("client-state directory must be normalized relative path".into());
            }
            parts
        };
        let value = match project
            .dir_if_present(&components)
            .map_err(|e| e.to_string())?
        {
            None => None,
            Some(directory) => {
                let identity = directory
                    .identity()
                    .map_err(|e| e.to_string())?
                    .observation_correlation_token();
                let names = project
                    .child_names_bounded(&directory, 64)
                    .map_err(|e| e.to_string())?;
                total += names.len();
                if total > 256 {
                    return Err("client-state directory closure exceeds bound".into());
                }
                let mut children = BTreeMap::new();
                for child in names {
                    let kind = project
                        .child_kind_nofollow(&directory, &child)
                        .map_err(|e| e.to_string())?
                        .ok_or("client-state child disappeared")?;
                    children.insert(child, kind);
                }
                Some(DirectoryMembership { identity, children })
            }
        };
        if result.insert((*name).to_owned(), value).is_some() {
            return Err("duplicate client-state directory".into());
        }
    }
    Ok(result)
}
