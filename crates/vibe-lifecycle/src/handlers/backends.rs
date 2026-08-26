//! The injected non-process handler backends.
//!
//! Split out of `handlers/mod.rs` when the agent handler arrived: the parent
//! owns the process wire, while every seam the lifecycle only *calls* — a
//! provider-scoped binary builder, an algorithmic package binding, and now a
//! transport-neutral agent service — is declared here. Each one keeps its
//! refusing default beside it, so a runtime that does not configure the seam
//! fails with remediation instead of silently skipping the contribution.

use std::path::PathBuf;

use specmark::spec;
use vibe_wire::generated::lifecycle_state::StateArtifact;

use crate::ExtensionRegistryRow;

/// Injectable provider-scoped binary resolution/build seam.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_lifecycle::ExtensionRegistryRow;
/// use vibe_lifecycle::handlers::BinaryBackend;
/// struct Missing;
/// impl BinaryBackend for Missing {
///     fn resolve_or_build(&self, _: &ExtensionRegistryRow, name: &str)
///         -> Result<PathBuf, String> { Err(format!("missing {name}")) }
/// }
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-BINARY")]
pub trait BinaryBackend: Send + Sync {
    fn resolve_or_build(&self, row: &ExtensionRegistryRow, name: &str) -> Result<PathBuf, String>;
}

#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-BINARY")]
pub struct NoBinaryBackend;
impl BinaryBackend for NoBinaryBackend {
    fn resolve_or_build(&self, _row: &ExtensionRegistryRow, name: &str) -> Result<PathBuf, String> {
        Err(format!("no binary backend configured for `{name}`"))
    }
}

/// Canonical artifact emitted by an injected algorithmic package binding.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-PACKAGE")]
pub struct PackageBindingArtifact {
    pub id: String,
    pub kind: String,
    pub path: String,
}

/// Result of one package binding before it is lowered to the lifecycle reply.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-PACKAGE")]
pub struct PackageBindingOutcome {
    pub artifacts: Vec<PackageBindingArtifact>,
    pub message: Option<String>,
}

/// Transport-neutral injected owner for algorithmic package bindings. The
/// lifecycle crate knows the reserved execution identity but not the concrete
/// skill writer that serves it.
///
/// ```
/// use vibe_lifecycle::{PackageBindingBackend, PackageBindingOutcome};
/// use vibe_wire::generated::lifecycle_state::StateArtifact;
///
/// /// A minimal algorithmic backend: owns nothing, echoes one message.
/// struct Echo;
///
/// impl PackageBindingBackend for Echo {
///     fn probe(&self, _key: &str, _artifacts: &[StateArtifact]) -> Result<bool, String> {
///         Ok(false)
///     }
///
///     fn execute(&self, key: &str) -> Result<PackageBindingOutcome, String> {
///         Ok(PackageBindingOutcome {
///             artifacts: Vec::new(),
///             message: Some(format!("echo `{key}`")),
///         })
///     }
/// }
///
/// let backend: &dyn PackageBindingBackend = &Echo;
/// assert!(!backend.probe("@vibe/package/skill/demo", &[]).unwrap());
/// let outcome = backend.execute("@vibe/package/skill/demo").unwrap();
/// assert_eq!(
///     outcome.message.as_deref(),
///     Some("echo `@vibe/package/skill/demo`")
/// );
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PRESET-LAW")]
pub trait PackageBindingBackend: Send + Sync {
    /// Verify the strict owner receipt and every recorded owned output before
    /// lifecycle state may hydrate this internal execution as `fresh`.
    fn probe(&self, key: &str, artifacts: &[StateArtifact]) -> Result<bool, String>;

    fn execute(&self, key: &str) -> Result<PackageBindingOutcome, String>;
}

#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PRESET-LAW")]
pub struct NoPackageBindingBackend;
impl PackageBindingBackend for NoPackageBindingBackend {
    fn probe(&self, _key: &str, _artifacts: &[StateArtifact]) -> Result<bool, String> {
        Ok(false)
    }

    fn execute(&self, key: &str) -> Result<PackageBindingOutcome, String> {
        Err(format!("no package binding backend configured for `{key}`"))
    }
}
