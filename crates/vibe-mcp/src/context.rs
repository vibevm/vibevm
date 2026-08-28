//! The per-request server context — the read-only project snapshot every
//! tool call sees, plus this server's PRIVATE lifecycle execution
//! authority (R7.4 A15c3).
//!
//! The public half is unchanged from its birth in `lib.rs`: the project
//! root and the machine store root. The private half carries exactly what
//! a hosted lifecycle run needs and NOTHING a surface could pay with: the
//! CLI-composed [`InstallPolicy`] (already-collapsed offline posture,
//! slot-integrity, spec-format default), the embedded-registry root of a
//! source install, and whether the default global registry may be seeded.
//! No user-configuration type, `[llm]` table, provider, model, endpoint,
//! token path or transport ever enters — the CLI stays the configuration
//! and environment composition root, and hands the decided answers down
//! through [`ServerContext::with_lifecycle_execution`].

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#lifecycle");

use std::path::PathBuf;

use specmark::spec;
use vibe_core::manifest::Lockfile;
use vibe_orchestrator::InstallPolicy;

use crate::SERVER_NAME;

/// Authority for the project the server exposes: stable project/package-store
/// roots plus the CLI-decided lifecycle policy. Read-only tools reload their
/// files on every call; `lifecycle_run` is the one mutating tool and executes
/// under the shared workspace lease.
///
/// ```
/// use vibe_mcp::ServerContext;
/// let ctx = ServerContext::new("/some/project");
/// assert!(ctx.project_root.ends_with("project"));
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#server")]
pub struct ServerContext {
    /// Project root — the directory containing `vibe.toml` and
    /// `vibe.lock`.
    pub project_root: PathBuf,
    /// Root of the machine-global package store (`~/.vibe/cache/`,
    /// PROP-010 §2.7) the lazy-pull subskill readers read payload
    /// from. Carried on the context rather than resolved per call so
    /// tests inject a temp root and never touch the real `~/.vibe`;
    /// production gets it from the one settings chokepoint. The path is
    /// joined from the documented layout rather than obtained through the
    /// registry resolver because this field serves the lazy-pull readers;
    /// lifecycle installation resolves the same machine store through its
    /// lower package-source/install stack.
    pub store_root: PathBuf,
    /// The CLI-composed prerequisite-install policy for hosted lifecycle
    /// execution. Crate-private: no tool argument, ambient environment
    /// read or configuration type can alter it — only the surface that
    /// built this context decided it.
    pub(crate) lifecycle_policy: InstallPolicy,
    /// The embedded-registry root of a source install, when the surface
    /// carries one. Threaded into the package-source environment exactly
    /// as the CLI's own closure threads it; never located here.
    pub(crate) embedded_registry_root: Option<PathBuf>,
    /// Whether the hosted registry environment may seed the default
    /// global registry before loading it (suppressed when the CLI composition
    /// root observes `VIBE_NO_DEFAULT_REGISTRY`; embedded-root discovery has
    /// its separate CI rule).
    pub(crate) seed_default_registry: bool,
}

impl ServerContext {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        let store_root = vibe_core::settings::settings_dir()
            .map(|home| home.join("cache"))
            .unwrap_or_default();
        ServerContext {
            project_root: project_root.into(),
            store_root,
            lifecycle_policy: InstallPolicy::default(),
            embedded_registry_root: None,
            seed_default_registry: false,
        }
    }

    /// A context whose store root is the caller's — the test seam for
    /// the lazy-pull readers: a temp root keeps a test off the
    /// operator's real `~/.vibe/cache/`.
    pub fn with_store_root(
        project_root: impl Into<PathBuf>,
        store_root: impl Into<PathBuf>,
    ) -> Self {
        ServerContext {
            project_root: project_root.into(),
            store_root: store_root.into(),
            lifecycle_policy: InstallPolicy::default(),
            embedded_registry_root: None,
            seed_default_registry: false,
        }
    }

    /// Grant this server lifecycle execution authority — the one builder
    /// that can, and only what a hosted run needs: the already-decided
    /// install `policy`, the surface's `embedded_registry_root` (if it
    /// carries one), and whether default-registry `seed`ing is permitted.
    ///
    /// Compatibility defaults: a context built without this builder keeps
    /// [`InstallPolicy::default()`] (online, default slot integrity, no
    /// spec-format override), no embedded root and no seeding. This preserves
    /// pre-A15c3 embedders; it is deliberately not an offline-by-default
    /// security posture.
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use vibe_mcp::ServerContext;
    ///
    /// let ctx = ServerContext::new("/some/project").with_lifecycle_execution(
    ///     vibe_orchestrator::InstallPolicy::default(),
    ///     Some(PathBuf::from("/install/packages")),
    ///     true,
    /// );
    /// // The authority is private to the crate: the value exposes no
    /// // configuration surface a tool argument could reach.
    /// assert!(ctx.project_root.ends_with("project"));
    /// ```
    #[must_use]
    pub fn with_lifecycle_execution(
        self,
        policy: InstallPolicy,
        embedded_registry_root: Option<PathBuf>,
        seed_default_registry: bool,
    ) -> Self {
        ServerContext {
            lifecycle_policy: policy,
            embedded_registry_root,
            seed_default_registry,
            ..self
        }
    }

    /// Load the project's lockfile fresh on every call. Returns an
    /// empty lockfile if `vibe.lock` does not exist yet — callers
    /// surface the empty-state through their normal output rather
    /// than aborting with `Lockfile not found`.
    pub fn load_lockfile(&self) -> Result<Lockfile, vibe_core::Error> {
        let path = self.project_root.join(Lockfile::FILENAME);
        if !path.exists() {
            return Ok(Lockfile::empty(SERVER_NAME, "0"));
        }
        Lockfile::read(&path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_constructors_keep_the_online_neutral_lifecycle_defaults() {
        for context in [
            ServerContext::new("project"),
            ServerContext::with_store_root("project", "store"),
        ] {
            assert!(!context.lifecycle_policy.offline);
            assert!(context.lifecycle_policy.spec_format_default.is_none());
            assert!(context.embedded_registry_root.is_none());
            assert!(!context.seed_default_registry);
        }
    }

    #[test]
    fn lifecycle_authority_is_one_explicit_builder_and_preserves_other_roots() {
        let policy = InstallPolicy {
            offline: true,
            ..InstallPolicy::default()
        };
        let context = ServerContext::with_store_root("project", "store").with_lifecycle_execution(
            policy,
            Some(PathBuf::from("embedded")),
            true,
        );
        assert_eq!(context.project_root, PathBuf::from("project"));
        assert_eq!(context.store_root, PathBuf::from("store"));
        assert!(context.lifecycle_policy.offline);
        assert_eq!(
            context.embedded_registry_root,
            Some(PathBuf::from("embedded"))
        );
        assert!(context.seed_default_registry);
    }

    #[test]
    fn context_carries_exactly_public_roots_and_private_lifecycle_authority() {
        fn destructure(context: ServerContext) {
            let ServerContext {
                project_root,
                store_root,
                lifecycle_policy,
                embedded_registry_root,
                seed_default_registry,
            } = context;
            let _ = (
                project_root,
                store_root,
                lifecycle_policy,
                embedded_registry_root,
                seed_default_registry,
            );
        }
        let _ = destructure;
    }
}
