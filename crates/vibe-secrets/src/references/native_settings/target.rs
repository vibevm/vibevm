//! Fixed vendor-directory observation and confirmed private genesis.
use super::*;
use std::hash::{Hash, Hasher};
#[cfg(all(test, unix))]
mod tests;

pub(super) struct Target {
    pub project: Project,
    pub exists: bool,
    ancestor: Pinned,
    missing: Vec<&'static str>,
    pub witness: String,
}
fn components(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Pi => &[".pi", "agent"],
        Kind::Qwen => &[".qwen"],
        Kind::Codex => &[".codex"],
        Kind::OpenCode => &[".config", "opencode"],
        Kind::Hermes => &[".hermes"],
        Kind::Claude => &[".claude"],
    }
}
struct IdentityHash<'a>(&'a mut Sha256);
impl Hasher for IdentityHash<'_> {
    fn finish(&self) -> u64 {
        0
    }
    fn write(&mut self, bytes: &[u8]) {
        self.0.update((bytes.len() as u64).to_be_bytes());
        self.0.update(bytes);
    }
}
pub(super) fn observe(home: &Path, kind: Kind) -> Result<Target, Error> {
    if !home.is_absolute() || home.canonicalize().map_err(|_| Error::Unavailable)? != home {
        return Err(Error::Unavailable);
    }
    let project = Project::open(home).map_err(|_| Error::Unavailable)?;
    let mut ancestor = project.root_dir().map_err(|_| Error::Unavailable)?;
    ancestor
        .verify_user_owned_parent()
        .map_err(|_| Error::Unavailable)?;
    let mut hash = Sha256::new();
    hash.update(b"vibevm-native-directory-genesis/v1\0");
    ancestor
        .identity()
        .map_err(|_| Error::Unavailable)?
        .hash(&mut IdentityHash(&mut hash));
    let mut missing = Vec::new();
    for (index, component) in components(kind).iter().enumerate() {
        hash.update((component.len() as u64).to_be_bytes());
        hash.update(component.as_bytes());
        match ancestor
            .private_child(component, false)
            .map_err(|_| Error::Unavailable)?
        {
            Some(child) => {
                hash.update([1]);
                child
                    .identity()
                    .map_err(|_| Error::Unavailable)?
                    .hash(&mut IdentityHash(&mut hash));
                ancestor = child;
            }
            None => {
                // Genesis has stronger parent admission than existing-directory
                // inspection. No chmod of an existing home/ancestor is attempted.
                ancestor
                    .verify_user_private()
                    .map_err(|_| Error::Unavailable)?;
                hash.update([0]);
                missing.extend_from_slice(&components(kind)[index..]);
                for suffix in &missing {
                    hash.update((suffix.len() as u64).to_be_bytes());
                    hash.update(suffix.as_bytes());
                }
                break;
            }
        }
    }
    let witness = format!("sha256:{:x}", hash.finalize());
    let exists = missing.is_empty();
    Ok(Target {
        project,
        exists,
        ancestor,
        missing,
        witness,
    })
}
impl Target {
    pub fn directory(&self) -> Option<&Pinned> {
        self.exists.then_some(&self.ancestor)
    }
    pub fn create(self) -> Result<(Project, Pinned), Error> {
        if self.exists {
            return Ok((self.project, self.ancestor));
        }
        let mut parent = self.ancestor;
        for component in self.missing {
            // Exclusive admission refuses a race winner, never adopts it or
            // overwrites it. On failure own empty directories remain for review.
            parent = parent
                .create_private_child_exclusive(component)
                .map_err(|_| Error::Conflict)?;
        }
        Ok((self.project, parent))
    }
}
