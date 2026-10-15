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
    files: BTreeMap<String, Option<vibe_safefs::StableFileSnapshot>>,
}
impl ClientStateWitness {
    pub fn observe_files(
        owner: &str,
        binding: &ClientStateBinding,
        root: &std::path::Path,
        files: &[&str],
    ) -> Result<Self, String> {
        if owner.is_empty() || files.is_empty() || files.len() > 64 || !root.is_absolute() {
            return Err("client-state owner requires a nonempty finite qualified closure".into());
        }
        let project = vibe_safefs::Project::open(root).map_err(|e| e.to_string())?;
        let mut observed = BTreeMap::new();
        for name in files {
            let snapshot = project
                .read_file_snapshot_bounded(name, 1024 * 1024)
                .map_err(|e| e.to_string())?;
            if observed.insert((*name).to_owned(), snapshot).is_some() {
                return Err("duplicate client-state member".into());
            }
        }
        Ok(Self {
            owner: owner.into(),
            binding: binding.clone(),
            root: std::fs::canonicalize(root).map_err(|e| e.to_string())?,
            files: observed,
        })
    }
    pub(super) fn matches_binding(&self, binding: &ClientStateBinding) -> bool {
        self.binding == *binding
    }
}
