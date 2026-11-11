//! Demand construction of the existing ShellGit backend, never a second resolver.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::*;
use std::sync::{Arc, Mutex, OnceLock};
use vibe_core::manifest::AuthKind;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportRequirement {
    Incumbent,
    Controlled,
}
/// Operation-owned constructor of the one `ShellGit` backend, consulted only
/// when a git-driving call actually needs a transport.
///
/// ```
/// use std::sync::Arc;
/// use vibe_registry::{GitError, ShellGit};
/// use vibe_registry::git_backend::deferred::{
///     DeferredShellGit, ShellGitFactory, TransportRequirement,
/// };
///
/// struct Incumbent;
/// impl ShellGitFactory for Incumbent {
///     fn requirement(&self) -> Result<TransportRequirement, GitError> {
///         Ok(TransportRequirement::Incumbent)
///     }
///     fn preflight(&self, _address: &str) -> Result<(), GitError> {
///         Ok(())
///     }
///     fn open(&self, _requirement: TransportRequirement) -> Result<ShellGit, GitError> {
///         Ok(ShellGit::new())
///     }
/// }
///
/// // Construction opens nothing; the factory runs on the first git demand.
/// let backend = DeferredShellGit::new(Arc::new(Incumbent));
/// assert!(format!("{backend:?}").starts_with("DeferredShellGit"));
/// ```
pub trait ShellGitFactory: Send + Sync {
    fn requirement(&self) -> Result<TransportRequirement, GitError>;
    fn preflight(&self, address: &str) -> Result<(), GitError>;
    fn open(&self, requirement: TransportRequirement) -> Result<ShellGit, GitError>;
}
struct State {
    factory: Arc<dyn ShellGitFactory>,
    opened: OnceLock<Arc<ShellGit>>,
    gate: Mutex<()>,
    transfers: OnceLock<vibe_core::progress::transfer::Transfers>,
}
#[derive(Clone)]
pub struct DeferredShellGit {
    state: Arc<State>,
    requirement: Option<TransportRequirement>,
    auth: Option<AuthKind>,
    anonymous: bool,
}
impl std::fmt::Debug for DeferredShellGit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeferredShellGit")
            .field("requirement", &self.requirement)
            .field("auth", &self.auth)
            .finish_non_exhaustive()
    }
}
impl DeferredShellGit {
    pub fn new(factory: Arc<dyn ShellGitFactory>) -> Self {
        Self {
            state: Arc::new(State {
                factory,
                opened: OnceLock::new(),
                gate: Mutex::new(()),
                transfers: OnceLock::new(),
            }),
            requirement: None,
            auth: None,
            anonymous: false,
        }
    }
    fn mode(&self) -> Result<TransportRequirement, GitError> {
        self.requirement
            .map_or_else(|| self.state.factory.requirement(), Ok)
    }
    fn backend(&self) -> Result<Arc<dyn GitBackend>, GitError> {
        if self.state.opened.get().is_none() {
            let _guard = self
                .state
                .gate
                .lock()
                .map_err(|_| GitError::NativeContext {
                    reason: "transport initialization lock poisoned",
                })?;
            if self.state.opened.get().is_none() {
                let value = Arc::new(self.state.factory.open(self.mode()?)?);
                if let Some(transfers) = self.state.transfers.get() {
                    value.attach_transfers(transfers);
                }
                let _ = self.state.opened.set(value);
            }
        }
        let value = self
            .state
            .opened
            .get()
            .ok_or(GitError::NativeContext {
                reason: "transport initialization produced no backend",
            })?
            .clone();
        let mut value: Arc<dyn GitBackend> = value;
        if self.anonymous {
            value = value
                .anonymized_for_public()
                .ok_or(GitError::NativeContext {
                    reason: "anonymous transport clone unavailable",
                })?;
        }
        if let Some(auth) = self.auth
            && let Some(bound) = value.bound_for_registry_auth(auth)?
        {
            value = bound;
        }
        Ok(value)
    }
    fn contact(&self, address: &str) -> Result<Arc<dyn GitBackend>, GitError> {
        self.state.factory.preflight(address)?;
        self.backend()
    }
}
impl GitBackend for DeferredShellGit {
    fn uses_native_materialisation(&self) -> bool {
        self.requirement == Some(TransportRequirement::Controlled)
    }
    fn native_materialisation_for(&self, _address: &str) -> Result<bool, GitError> {
        Ok(self.mode()? == TransportRequirement::Controlled)
    }
    fn preflight_selected_address(&self, address: &str) -> Result<(), GitError> {
        self.state.factory.preflight(address)
    }
    fn preflight_registry_auth(&self, auth: AuthKind) -> Result<(), GitError> {
        if self.auth.is_some_and(|bound| bound != auth) {
            return Err(GitError::NativeContext {
                reason: "registry auth differs from deferred binding",
            });
        }
        if self.mode()? == TransportRequirement::Controlled && auth == AuthKind::CredentialHelper {
            return Err(GitError::NativeContext {
                reason: "credential-helper route cannot satisfy controlled transport",
            });
        }
        Ok(())
    }
    fn bound_for_registry_auth(
        &self,
        auth: AuthKind,
    ) -> Result<Option<Arc<dyn GitBackend>>, GitError> {
        if (self.anonymous || self.auth.is_some_and(|bound| bound != auth))
            && auth != AuthKind::None
        {
            return Err(GitError::NativeContext {
                reason: "deferred credential binding cannot widen",
            });
        }
        let mut value = self.clone();
        value.requirement = Some(self.mode()?);
        value.auth = Some(auth);
        value.anonymous |= auth == AuthKind::None;
        Ok(Some(Arc::new(value)))
    }
    fn anonymized_for_public(&self) -> Option<Arc<dyn GitBackend>> {
        let mut value = self.clone();
        value.anonymous = true;
        value.auth = Some(AuthKind::None);
        Some(Arc::new(value))
    }
    fn attach_transfers(&self, t: &vibe_core::progress::transfer::Transfers) {
        let _ = self.state.transfers.set(t.clone());
        if let Some(value) = self.state.opened.get() {
            value.attach_transfers(t);
        }
    }
    fn preflight_materialisation(&self) -> Result<(), GitError> {
        if self.mode()? == TransportRequirement::Controlled {
            return Err(GitError::NativeUnavailable {
                operation: "legacy publication under controlled transport",
            });
        }
        Ok(())
    }
    fn prepared_serving_endpoint(&self, p: &Path) -> Result<Option<String>, GitError> {
        self.backend()?.prepared_serving_endpoint(p)
    }
    fn origin_url(&self, p: &Path) -> Result<String, GitError> {
        self.backend()?.origin_url(p)
    }
    fn recorded_origin_url(&self, p: &Path) -> Result<String, GitError> {
        self.backend()?.recorded_origin_url(p)
    }
    fn head_commit(&self, p: &Path) -> Result<Option<String>, GitError> {
        self.backend()?.head_commit(p)
    }
    fn head_tree(&self, p: &Path) -> Result<Option<String>, GitError> {
        self.backend()?.head_tree(p)
    }
    fn working_tree_dirty(&self, p: &Path) -> Result<bool, GitError> {
        self.backend()?.working_tree_dirty(p)
    }
    fn clean_worktree(&self, p: &Path) -> Result<(), GitError> {
        self.preflight_materialisation()?;
        self.backend()?.clean_worktree(p)
    }
    fn set_remote_url(&self, p: &Path, remote: &str, url: &str) -> Result<(), GitError> {
        self.backend()?.set_remote_url(p, remote, url)
    }
    fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
        self.contact(url)?.list_tags(url)
    }
    fn fetch_file_at_ref(
        &self,
        url: &str,
        reference: &str,
        path: &str,
    ) -> Result<Vec<u8>, GitError> {
        self.contact(url)?.fetch_file_at_ref(url, reference, path)
    }
    fn bootstrap(&self, url: &str, reference: &str, dest: &Path) -> Result<(), GitError> {
        self.state.factory.preflight(url)?;
        self.preflight_materialisation()?;
        self.backend()?.bootstrap(url, reference, dest)
    }
    fn bootstrap_data(&self, url: &str, reference: &str, dest: &Path) -> Result<(), GitError> {
        self.state.factory.preflight(url)?;
        self.preflight_materialisation()?;
        self.backend()?.bootstrap_data(url, reference, dest)
    }
    fn bootstrap_embedded(&self, url: &str, reference: &str, dest: &Path) -> Result<(), GitError> {
        self.state.factory.preflight(url)?;
        self.preflight_materialisation()?;
        self.backend()?.bootstrap_embedded(url, reference, dest)
    }
    fn update(&self, dest: &Path, reference: &str) -> Result<(), GitError> {
        self.preflight_materialisation()?;
        self.backend()?.update(dest, reference)
    }
    fn update_data(&self, dest: &Path, reference: &str) -> Result<(), GitError> {
        self.preflight_materialisation()?;
        self.backend()?.update_data(dest, reference)
    }
    fn prepare_ref(
        &self,
        url: &str,
        reference: &str,
        reuse: Option<&Path>,
        objects: &Path,
        destination: &Path,
    ) -> Result<GitPreparedRevision, GitError> {
        self.contact(url)?
            .prepare_ref(url, reference, reuse, objects, destination)
    }
    fn prepare_native_source(
        &self,
        url: &str,
        reference: &str,
        reuse: Option<&Path>,
        objects: &Path,
    ) -> Result<Option<shell::native_admission::NativeSource>, GitError> {
        self.contact(url)?
            .prepare_native_source(url, reference, reuse, objects)
    }
    fn prepare_pinned_native_source(
        &self,
        url: &str,
        reference: &str,
        reuse: Option<&Path>,
        objects: &Path,
    ) -> Result<Option<shell::native_admission::NativeSource>, GitError> {
        self.contact(url)?
            .prepare_pinned_native_source(url, reference, reuse, objects)
    }
    fn stage_native_materialisation(
        &self,
        objects: &Path,
        revision: &GitPreparedRevision,
        bytes: super::PayloadBytes,
    ) -> Result<Option<shell::native_admission::NativeMaterialisation>, GitError> {
        self.backend()?
            .stage_native_materialisation(objects, revision, bytes)
    }
    fn normalise_owned_worktree(&self, dest: &Path) -> Result<(), GitError> {
        self.backend()?.normalise_owned_worktree(dest)
    }
    fn file_at_commit(
        &self,
        objects: &Path,
        commit: &str,
        path: &str,
    ) -> Result<Vec<u8>, GitError> {
        self.backend()?.file_at_commit(objects, commit, path)
    }
    fn apply_prepared_ref(
        &self,
        objects: &Path,
        revision: &GitPreparedRevision,
        origin: &str,
        authenticated: Option<&str>,
        dest: &Path,
        refresh: bool,
    ) -> Result<(), GitError> {
        self.preflight_materialisation()?;
        self.backend()?
            .apply_prepared_ref(objects, revision, origin, authenticated, dest, refresh)
    }
}
