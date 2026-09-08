//! Prepared-carriage bridge for authored package targets.

use super::PreparedNativeMechanisms;

impl PreparedNativeMechanisms {
    /// Execute package targets with this exact prepared native carriage.
    ///
    /// ```
    /// use std::path::Path;
    /// use vibe_core::manifest::{ExtensionsControl, MechanismRoutes};
    /// use vibe_extension_registry::collect_mechanisms;
    /// use vibe_lifecycle::native::PreparedNativeMechanisms;
    /// use vibe_lifecycle::{
    ///     ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, PackageExecution,
    /// };
    ///
    /// let world = ExtensionWorld {
    ///     installed: Vec::new(),
    ///     host: HostExtensionSource {
    ///         provider: HostProvider {
    ///             identity: HostIdentity::ungrouped_project("demo"),
    ///             root: Path::new(".").to_path_buf(),
    ///             version: "0.1.0".into(),
    ///             kind: None,
    ///             content_hash: None,
    ///         },
    ///         declarations: Vec::new(),
    ///         controls: ExtensionsControl::default(),
    ///         mechanisms: Vec::new(),
    ///     },
    ///     effective_stack: None,
    /// };
    /// let registry = collect_mechanisms(&world).unwrap();
    /// let routes = MechanismRoutes::default();
    /// let execution = PackageExecution {
    ///     project_root: Path::new("."),
    ///     targets: &[],
    ///     registry: &registry,
    ///     routes: &routes,
    ///     package_root: PackageExecution::default_package_root(),
    ///     created_at: "2026-09-08T00:00:00Z",
    /// };
    /// assert!(
    ///     PreparedNativeMechanisms::default()
    ///         .execute_package_targets(&execution)
    ///         .unwrap()
    ///         .is_empty()
    /// );
    /// ```
    pub fn execute_package_targets(
        &self,
        execution: &crate::PackageExecution<'_>,
    ) -> Result<Vec<crate::PackageOutcome>, crate::PackageError> {
        crate::mechanism::package::execute_prepared_package_targets(execution, self)
    }
}
