//! The run's memory of the remote registries that answered «not here».
//!
//! One install asks the walk about the same package several times — the
//! version listing, the selection, the metadata read (exact, then latest),
//! the download-source lookup — and each walk used to put the same question
//! to every remote registry listed before the one that holds the package
//! (B-199: nine half-second calls to re-resolve an in-tree package). Within
//! one resolver, which is one invocation, a remote's «not here» does not
//! change, so it is asked once and its answer is replayed.
//!
//! Only the answers on which every walk moves on are remembered: a missing
//! package, and a 401 / 403 from a public registry outside strict-auth
//! (PROP-002 §2.3.1). Replaying them keeps the declared order intact — the
//! remote still comes first in the walk, it just does not cost a second
//! round trip to say the same thing. A hit, a missing version, a network or
//! auth failure is never remembered, so nothing that halts or selects is
//! served from memory.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use std::sync::Mutex;

use super::*;

/// The answer a remote gave that the walk walked past.
#[derive(Clone)]
enum Miss {
    /// The registry has no repository for the package.
    Unknown,
    /// A public registry refused anonymously; the URL it refused on (and
    /// the HTTP status, when seen) is kept so the replayed error reads the
    /// same as the first.
    PublicRefusal(String, Option<u16>),
}

/// Per-resolver memory of remote misses, keyed by registry name and
/// `<group>/<name>`. A `Mutex` because a resolver is shared by reference
/// across the resolver's callers; contention is one map lookup per walk step.
#[derive(Default)]
pub(crate) struct RemoteMisses(Mutex<HashMap<(String, String), Miss>>);

impl RemoteMisses {
    fn get(&self, key: &(String, String)) -> Option<Miss> {
        // A poisoned lock only means a panic elsewhere mid-insert; the map
        // still holds complete entries, so it stays usable.
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
    }

    fn insert(&self, key: (String, String), miss: Miss) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, miss);
    }
}

impl MultiRegistryResolver {
    /// Put one walk step's question to a remote registry, unless it already
    /// answered «not here» for this package in this run.
    ///
    /// The progress detail is the observable count of round trips:
    /// `checking remote registry <name>` is emitted only when the remote is
    /// actually asked, `remote registry <name>: not here (answered earlier in
    /// this run)` when its earlier answer is replayed.
    pub(super) fn ask_remote<T>(
        &self,
        reg: &GitPerPackageRegistry,
        group: &Group,
        name: &str,
        task: &vibe_core::progress::ProgressTask,
        lookup: impl FnOnce() -> Result<T, RegistryError>,
    ) -> Result<T, RegistryError> {
        let key = (reg.name().to_string(), format!("{group}/{name}"));
        if let Some(miss) = self.misses.get(&key) {
            task.detail(format!(
                "remote registry {}: not here (answered earlier in this run)",
                reg.name()
            ));
            return Err(match miss {
                Miss::Unknown => RegistryError::UnknownPackage {
                    group: group.clone(),
                    name: name.to_string(),
                },
                Miss::PublicRefusal(url, status) => {
                    RegistryError::Git(GitError::AuthFailed { url, status })
                }
            });
        }
        // The URL belongs on this line as much as the name: `-v` has to be
        // enough to tell which endpoint a walk step actually reached, without
        // the reader cross-referencing `vibe registry list`.
        task.detail(format!(
            "checking remote registry {} ({}) for {group}/{name}",
            reg.name(),
            reg.org_url()
        ));
        let result = lookup();
        if let Err(err) = &result
            && let Some(miss) = self.walked_past(reg, err)
        {
            self.misses.insert(key, miss);
        }
        result
    }

    /// The miss to remember for an error, when every walk moves past it.
    fn walked_past(&self, reg: &GitPerPackageRegistry, err: &RegistryError) -> Option<Miss> {
        match err {
            RegistryError::UnknownPackage { .. } => Some(Miss::Unknown),
            RegistryError::Git(GitError::AuthFailed { url, status })
                if matches!(reg.auth_kind(), vibe_core::manifest::AuthKind::None)
                    && !self.strict_auth =>
            {
                Some(Miss::PublicRefusal(url.clone(), *status))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "negative_tests.rs"]
mod tests;
