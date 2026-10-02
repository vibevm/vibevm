//! Fixed package owner entry points. Caller-held lease and host-issued scope
//! are the only authority inputs; this module constructs the one target itself.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-PACKAGE");

use std::path::PathBuf;

use vibe_core::manifest::{
    ArtifactInput, ArtifactKind, ArtifactOutput, ArtifactPackageTarget, ExtensionsControl,
    MechanismRoutes,
};
use vibe_extension_registry::collect_mechanisms;

use super::{
    Result, StaticFileJournalInspection, execute_fixed_static_file, inspect_fixed_static_file_cold,
    refused,
};
use crate::mechanism::package::PackageExecution;
use crate::{
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, LifecycleLease,
    SubmissionNamespace, SubmissionScope,
};

/// Submit the one builtin `source.bin` to `launcher.bin` package operation.
/// The package owner itself reserves C1 and retains the result. `created_at`
/// is the first owner's trusted clock value included in its canonical plan;
/// same-key replay uses that retained plan, not the retrying host's clock.
///
/// ```no_run
/// use vibe_lifecycle::{LifecycleLease, SubmissionReservations, SubmissionScope,
///     submit_fixed_static_file_package};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let lease = LifecycleLease::acquire(std::path::Path::new("/trusted/project"))?;
/// let scope = SubmissionScope {
///     context: "host-context".into(), target: "host-target".into(),
///     principal: "host-principal".into(),
/// };
/// let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000)?;
/// let result = submit_fixed_static_file_package(
///     &lease, &namespace, &scope, "copy-key", "2026-10-03T00:00:00Z",
/// )?;
/// # let _ = result;
/// # Ok(())
/// # }
/// ```
pub fn submit_fixed_static_file_package(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    created_at: &str,
) -> Result<StaticFileJournalInspection> {
    let mechanism = "package:static-file".parse().map_err(refused)?;
    let target = ArtifactPackageTarget {
        id: "launcher".into(),
        mechanism,
        provider: None,
        when: None,
        inputs: Some(vec![ArtifactInput::Path {
            path: PathBuf::from("source.bin"),
        }]),
        outputs: vec![ArtifactOutput {
            id: "launcher.bin".into(),
            kind: ArtifactKind::File,
            select: None,
        }],
        config: None,
    };
    let world = ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("fixed-static-file"),
                root: PathBuf::from("."),
                version: env!("CARGO_PKG_VERSION").into(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    };
    let registry = collect_mechanisms(&world).map_err(refused)?;
    let routes = MechanismRoutes::default();
    let targets = [target];
    let execution = PackageExecution {
        project_root: lease.root(),
        targets: &targets,
        registry: &registry,
        routes: &routes,
        package_root: PackageExecution::default_package_root(),
        created_at,
    };
    execute_fixed_static_file(lease, namespace, scope, key, &execution)
}

/// Inspect an existing key through the package owner's retained capsule and
/// result. Inspection has no package execution or reservation path.
///
/// ```no_run
/// use vibe_lifecycle::{LifecycleLease, SubmissionScope, inspect_fixed_static_file_package};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let lease = LifecycleLease::acquire(std::path::Path::new("/trusted/project"))?;
/// let scope = SubmissionScope {
///     context: "host-context".into(), target: "host-target".into(),
///     principal: "host-principal".into(),
/// };
/// let retained = inspect_fixed_static_file_package(
///     &lease, "host-issued-namespace", &scope, "copy-key",
/// )?;
/// # let _ = retained;
/// # Ok(())
/// # }
/// ```
pub fn inspect_fixed_static_file_package(
    lease: &LifecycleLease,
    namespace_id: &str,
    scope: &SubmissionScope,
    key: &str,
) -> Result<Option<StaticFileJournalInspection>> {
    inspect_fixed_static_file_cold(lease, namespace_id, scope, key)
}

#[cfg(test)]
#[path = "owner_api/tests.rs"]
mod tests;
