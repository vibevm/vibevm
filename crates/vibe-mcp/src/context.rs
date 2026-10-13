//! The per-request server context — the read-only project snapshot every
//! tool call sees, plus private lifecycle execution and optional finite
//! authored-file migration authority.
//!
//! The public half is unchanged from its birth in `lib.rs`: the project
//! root and the machine store root. The lifecycle policy carries exactly what
//! a hosted lifecycle run needs and NOTHING a surface could pay with: the
//! CLI-composed [`InstallPolicy`] (already-collapsed offline posture,
//! slot-integrity, spec-format default), the embedded-registry root of a
//! source install, and whether the default global registry may be seeded.
//! No user-configuration type, `[llm]` table, provider, model, endpoint,
//! token path or transport ever enters — the CLI stays the configuration
//! and environment composition root, and hands the decided answers down
//! through [`ServerContext::with_lifecycle_execution`]. Migration admission is
//! separately granted through [`ServerContext::with_manifest_migration_authority`];
//! tool arguments cannot alter its roots or finite file selection.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#lifecycle");

use std::path::PathBuf;

use specmark::spec;
use vibe_core::manifest::Lockfile;
use vibe_orchestrator::InstallPolicy;

use crate::SERVER_NAME;

mod document_edit;
mod migration;
#[cfg(test)]
#[path = "context/migration_tests.rs"]
mod migration_tests;

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
    document_edit_authority:
        Option<std::sync::Arc<vibe_orchestrator::document_analysis::edit::DocumentEditAuthority>>,
    document_analysis:
        Option<std::sync::Arc<vibe_orchestrator::document_analysis::DocumentAnalysisHost>>,
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
    /// One explicit finite migration grant, never supplied by tool arguments.
    manifest_migration_authority: Option<migration::ManifestMigrationAuthority>,
    fixed_package_bridge: Option<
        std::sync::Arc<vibe_orchestrator::machine_operation::fixed_package::FixedPackageRoute>,
    >,
    fixed_cargo_bridge:
        Option<std::sync::Arc<vibe_orchestrator::machine_operation::fixed_cargo::FixedCargoRoute>>,
    fixed_deploy_bridge: Option<
        std::sync::Arc<vibe_orchestrator::machine_operation::fixed_deploy::FixedDeployRoute>,
    >,
    fixed_lifecycle_bridge: Option<
        std::sync::Arc<vibe_orchestrator::machine_operation::fixed_lifecycle::FixedLifecycleRoute>,
    >,
    exact_local_install_bridge: Option<
        std::sync::Arc<
            vibe_orchestrator::machine_operation::exact_local_install::ExactLocalInstallRoute,
        >,
    >,
}

impl ServerContext {
    /// Explicit read-only document scope; no lifecycle/native authority.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-BUFFER-LAW")]
    pub fn with_document_analysis(
        mut self,
        host: std::sync::Arc<vibe_orchestrator::document_analysis::DocumentAnalysisHost>,
    ) -> Result<Self, vibe_orchestrator::document_analysis::DocumentAnalysisError> {
        if host.root() != self.project_root.as_path() {
            return Err(
                vibe_orchestrator::document_analysis::DocumentAnalysisError::InvalidDeclaration,
            );
        }
        self.document_analysis = Some(host);
        Ok(self)
    }
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-ANALYSIS-CAPTURE-LAW"
    )]
    pub fn document_client(
        &self,
    ) -> Result<
        vibe_orchestrator::document_analysis::DocumentClient,
        vibe_orchestrator::document_analysis::DocumentAnalysisError,
    > {
        let host = self
            .document_analysis
            .as_ref()
            .ok_or(vibe_orchestrator::document_analysis::DocumentAnalysisError::Unavailable)?;
        if host.root() != self.project_root.as_path() {
            return Err(
                vibe_orchestrator::document_analysis::DocumentAnalysisError::InvalidDeclaration,
            );
        }
        host.client()
    }
    /// Explicit trusted host grant. No namespace, policy or provider is initialized.
    pub fn with_fixed_lifecycle_host(
        self,
        host: std::sync::Arc<vibe_orchestrator::machine_submission::FixedLifecycleDecisionHost>,
    ) -> Result<Self, vibe_orchestrator::machine_submission::FixedDecisionError> {
        self.with_fixed_lifecycle_route(std::sync::Arc::new(
            vibe_orchestrator::machine_operation::fixed_lifecycle::FixedLifecycleRoute::new(host),
        ))
    }
    /// Retain an existing opaque route for a new connection, including its
    /// accepted references and live jobs. The physical root is checked again;
    /// no wire descriptor can supply this grant.
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use vibe_mcp::{ServerContext, machine_session::fixed::FixedMachineSessionRunner};
    /// use vibe_orchestrator::{machine_operation::fixed_lifecycle::FixedLifecycleRoute,
    ///     machine_submission::FixedLifecycleDecisionHost};
    /// fn reconnect(root: &std::path::Path, host: Arc<FixedLifecycleDecisionHost>) {
    ///     let route = Arc::new(FixedLifecycleRoute::new(host));
    ///     let context = ServerContext::with_store_root(root, root.join("unused"))
    ///         .with_fixed_lifecycle_route(route.clone()).unwrap();
    ///     let _connection = FixedMachineSessionRunner::new(context).unwrap();
    ///     // The embedding retains route for another explicitly scoped connection.
    /// }
    /// ```
    pub fn with_fixed_lifecycle_route(
        mut self,
        route: std::sync::Arc<
            vibe_orchestrator::machine_operation::fixed_lifecycle::FixedLifecycleRoute,
        >,
    ) -> Result<Self, vibe_orchestrator::machine_submission::FixedDecisionError> {
        route.compare_context_root(&self.project_root)?;
        self.fixed_lifecycle_bridge = Some(route);
        Ok(self)
    }
    pub(crate) fn fixed_lifecycle_bridge(
        &self,
    ) -> Option<
        &std::sync::Arc<vibe_orchestrator::machine_operation::fixed_lifecycle::FixedLifecycleRoute>,
    > {
        self.fixed_lifecycle_bridge.as_ref()
    }

    /// Retain one owner-granted exact-local Install route for this physical
    /// root. Tool and frame bytes cannot create the host or C1 namespace.
    pub fn with_exact_local_install_route(
        mut self,
        route: std::sync::Arc<
            vibe_orchestrator::machine_operation::exact_local_install::ExactLocalInstallRoute,
        >,
    ) -> Result<Self, vibe_orchestrator::machine_submission::ExactLocalInstallDecisionError> {
        route.compare_context_root(&self.project_root)?;
        self.exact_local_install_bridge = Some(route);
        Ok(self)
    }

    pub(crate) fn exact_local_install_bridge(
        &self,
    ) -> Option<
        &std::sync::Arc<
            vibe_orchestrator::machine_operation::exact_local_install::ExactLocalInstallRoute,
        >,
    > {
        self.exact_local_install_bridge.as_ref()
    }

    /// Retain the embedding's explicit fixed package route after checking its
    /// physical root. Neither a path nor a wire value can supply this grant.
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use vibe_mcp::ServerContext;
    /// use vibe_orchestrator::machine_operation::fixed_package::FixedPackageRoute;
    /// fn bind(root: &std::path::Path, route: Arc<FixedPackageRoute>) {
    ///     let _context = ServerContext::with_store_root(root, root.join("unused"))
    ///         .with_fixed_package_route(route).unwrap();
    /// }
    /// ```
    pub fn with_fixed_package_route(
        mut self,
        route: std::sync::Arc<
            vibe_orchestrator::machine_operation::fixed_package::FixedPackageRoute,
        >,
    ) -> Result<Self, vibe_orchestrator::machine_operation::fixed_package::FixedPackageHostError>
    {
        route.compare_context_root(&self.project_root)?;
        self.fixed_package_bridge = Some(route);
        Ok(self)
    }

    pub(crate) fn fixed_package_bridge(
        &self,
    ) -> Option<
        &std::sync::Arc<vibe_orchestrator::machine_operation::fixed_package::FixedPackageRoute>,
    > {
        self.fixed_package_bridge.as_ref()
    }

    /// Retain one trusted fixed Cargo route for this physical project root.
    pub fn with_fixed_cargo_route(
        mut self,
        route: std::sync::Arc<vibe_orchestrator::machine_operation::fixed_cargo::FixedCargoRoute>,
    ) -> Result<Self, vibe_orchestrator::machine_operation::fixed_cargo::FixedCargoHostError> {
        route.compare_context_root(&self.project_root)?;
        self.fixed_cargo_bridge = Some(route);
        Ok(self)
    }

    pub(crate) fn fixed_cargo_bridge(
        &self,
    ) -> Option<&std::sync::Arc<vibe_orchestrator::machine_operation::fixed_cargo::FixedCargoRoute>>
    {
        self.fixed_cargo_bridge.as_ref()
    }

    /// Retain a trusted fixed Deploy route with its three independently pinned
    /// roots. This context compares the project root; the owner rechecks all
    /// three roots before every inspect or submit. No route is constructed
    /// from request bytes or an ambient settings directory.
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use vibe_mcp::ServerContext;
    /// use vibe_orchestrator::machine_operation::fixed_deploy::FixedDeployRoute;
    /// fn bind(root: &std::path::Path, route: Arc<FixedDeployRoute>) {
    ///     let _ctx = ServerContext::with_store_root(root, root.join("unused"))
    ///         .with_fixed_deploy_route(route).unwrap();
    /// }
    /// ```
    pub fn with_fixed_deploy_route(
        mut self,
        route: std::sync::Arc<vibe_orchestrator::machine_operation::fixed_deploy::FixedDeployRoute>,
    ) -> Result<Self, vibe_orchestrator::machine_operation::fixed_deploy::FixedDeployHostError>
    {
        route.compare_context_root(&self.project_root)?;
        self.fixed_deploy_bridge = Some(route);
        Ok(self)
    }

    pub(crate) fn fixed_deploy_bridge(
        &self,
    ) -> Option<&std::sync::Arc<vibe_orchestrator::machine_operation::fixed_deploy::FixedDeployRoute>>
    {
        self.fixed_deploy_bridge.as_ref()
    }

    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        let store_root = vibe_core::settings::settings_dir()
            .map(|home| home.join("cache"))
            .unwrap_or_default();
        ServerContext {
            document_analysis: None,
            document_edit_authority: None,
            project_root: project_root.into(),
            store_root,
            lifecycle_policy: InstallPolicy::default(),
            embedded_registry_root: None,
            seed_default_registry: false,
            manifest_migration_authority: None,
            fixed_lifecycle_bridge: None,
            exact_local_install_bridge: None,
            fixed_package_bridge: None,
            fixed_cargo_bridge: None,
            fixed_deploy_bridge: None,
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
            document_analysis: None,
            document_edit_authority: None,
            project_root: project_root.into(),
            store_root: store_root.into(),
            lifecycle_policy: InstallPolicy::default(),
            embedded_registry_root: None,
            seed_default_registry: false,
            manifest_migration_authority: None,
            fixed_lifecycle_bridge: None,
            exact_local_install_bridge: None,
            fixed_package_bridge: None,
            fixed_cargo_bridge: None,
            fixed_deploy_bridge: None,
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
                document_analysis,
                document_edit_authority,
                project_root,
                store_root,
                lifecycle_policy,
                embedded_registry_root,
                seed_default_registry,
                manifest_migration_authority,
                fixed_lifecycle_bridge,
                exact_local_install_bridge,
                fixed_package_bridge,
                fixed_cargo_bridge,
                fixed_deploy_bridge,
            } = context;
            let _ = (
                document_analysis,
                document_edit_authority,
                project_root,
                store_root,
                lifecycle_policy,
                embedded_registry_root,
                seed_default_registry,
                manifest_migration_authority,
                fixed_lifecycle_bridge,
                exact_local_install_bridge,
                fixed_package_bridge,
                fixed_cargo_bridge,
                fixed_deploy_bridge,
            );
        }
        let _ = destructure;
    }
}

#[cfg(test)]
mod m_16_d_1_cases {
    use super::*;
    use std::sync::Arc;
    use vibe_orchestrator::document_analysis::{
        AnalysisLimits, BufferIntent, ByteEncoding, DocumentAnalysisHost, ResourceDeclaration,
        ResourceRole,
    };
    #[test]
    fn trusted_context_keeps_two_document_clients_isolated() {
        let root = tempfile::tempdir().unwrap();
        let text = "[project]\ngroup='org.overlay'\nname='saved'\nversion='1.0.0'\n";
        std::fs::write(root.path().join("vibe.toml"), text).unwrap();
        let host = DocumentAnalysisHost::new(
            root.path(),
            "vibe.toml",
            vec![ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            }],
            AnalysisLimits {
                resource_bytes: 1024 * 1024,
                capture_bytes: 8 * 1024 * 1024,
                open_buffers: 4,
            },
        )
        .unwrap();
        let id = host.resource("vibe.toml").unwrap();
        let context = ServerContext::with_store_root(root.path(), root.path().join("unused-store"))
            .with_document_analysis(host.clone())
            .unwrap();
        let a = context.document_client().unwrap();
        let b = context.document_client().unwrap();
        let dirty = text.replace("name='saved'", "name='dirty'");
        a.open(
            1,
            id,
            1,
            ByteEncoding::Utf8,
            Arc::from(dirty.as_bytes()),
            BufferIntent::Replace,
        )
        .unwrap();
        assert_eq!(
            a.freeze()
                .unwrap()
                .manifest(id)
                .unwrap()
                .consumer_node()
                .unwrap()
                .name,
            "dirty"
        );
        assert_eq!(
            b.freeze()
                .unwrap()
                .manifest(id)
                .unwrap()
                .consumer_node()
                .unwrap()
                .name,
            "saved"
        );
        assert_eq!(
            std::fs::read(root.path().join("vibe.toml")).unwrap(),
            text.as_bytes()
        );
        let other = tempfile::tempdir().unwrap();
        assert!(matches!(
            ServerContext::with_store_root(other.path(), other.path().join("unused"))
                .with_document_analysis(host),
            Err(vibe_orchestrator::document_analysis::DocumentAnalysisError::InvalidDeclaration)
        ));
        assert!(matches!(
            ServerContext::with_store_root(root.path(), root.path().join("unused"))
                .document_client(),
            Err(vibe_orchestrator::document_analysis::DocumentAnalysisError::Unavailable)
        ));
    }
}

#[cfg(test)]
#[path = "context/m_16_d_2_cases.rs"]
mod m_16_d_2_cases;
