//! Consumer-side controls for selecting declared extensions.
//!
//! Keys are deliberately opaque. The lifecycle collector compares their exact
//! spelling and carries provider metadata separately; this module never
//! reverse-engineers a package coordinate from a key.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION");

use std::fmt;

use crate::{Group, PackageName};

use super::ExtensionConfig;

/// Stable printable identity of one extension declaration.
///
/// Authored values retain every character exactly. Constructed package and
/// host identities use the two closed spellings accepted by the lifecycle
/// contract, but the resulting value still has no component-extraction API.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtensionKey(String);

impl ExtensionKey {
    /// Retain an authored `ref` or `disable` value without parsing or
    /// normalization.
    pub fn authored(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    /// Construct the key for a declaration supplied by a package.
    pub fn for_package(group: &Group, name: &PackageName, id: &str) -> Self {
        Self(format!("{group}/{name}#{id}"))
    }

    /// Construct the reserved key for a declaration supplied by a project.
    pub fn for_host(project_name: &str, id: &str) -> Self {
        Self(format!("__host__/{project_name}#{id}"))
    }

    /// The exact stored spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ExtensionKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// One ordered `[[extensions.use]]` activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionUse {
    pub reference: ExtensionKey,
    /// Retains the authored distinction between no override and an explicitly
    /// empty override. The collector decides the latter's effective meaning.
    pub config: Option<ExtensionConfig>,
}

/// Consumer-side extension activation and disable controls.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtensionsControl {
    pub uses: Vec<ExtensionUse>,
    pub disable: Vec<ExtensionKey>,
}

impl ExtensionsControl {
    /// Whether the plural `[extensions]` namespace can be omitted entirely.
    pub fn is_empty(&self) -> bool {
        self.uses.is_empty() && self.disable.is_empty()
    }
}
