//! Requirements between application packages (`PROP-065#app-dependencies`):
//! an installer option or one step names other application packages it
//! needs, in the `group/name@requirement` spelling of the manifest's
//! `[requires]` table, optionally with a scope.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies");

use serde::{Deserialize, Serialize};
use specmark::spec;

/// What a requirement holds after the installation that asked for it.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementScope {
    /// Needed only while the requiring steps run: installed hidden, never
    /// blocks its own removal. The default of a step-level requirement.
    Build,
    /// Needed by the installed application itself: installed exposed, and
    /// its removal is refused while a dependent exists. The default of an
    /// option-level requirement.
    Runtime,
}

impl RequirementScope {
    /// The stable spelling (`build`, `runtime`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Runtime => "runtime",
        }
    }
}

/// The table form `{ package = "group/name@req", scope = "runtime" }`.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementTable {
    pub package: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<RequirementScope>,
}

/// One entry of a `requires` list: a bare `"group/name@req"` string, or
/// the table form carrying an explicit scope.
///
/// ```
/// use vibe_core::manifest::{RequirementDecl, RequirementScope};
///
/// let bare: RequirementDecl = serde_json::from_str("\"org.example/gcc@1\"").unwrap();
/// assert_eq!(bare.package(), "org.example/gcc@1");
/// assert_eq!(bare.scope_or(RequirementScope::Build), RequirementScope::Build);
/// let table: RequirementDecl =
///     serde_json::from_str(r#"{"package":"org.example/py@3","scope":"runtime"}"#).unwrap();
/// assert_eq!(table.scope_or(RequirementScope::Build), RequirementScope::Runtime);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequirementDecl {
    Spelled(String),
    Table(RequirementTable),
}

impl RequirementDecl {
    /// The `group/name@requirement` spelling.
    pub fn package(&self) -> &str {
        match self {
            Self::Spelled(package) => package,
            Self::Table(table) => &table.package,
        }
    }

    /// The declared scope, or `default` (`build` for a step, `runtime` for
    /// an option) when none is declared.
    pub fn scope_or(&self, default: RequirementScope) -> RequirementScope {
        match self {
            Self::Spelled(_) => default,
            Self::Table(table) => table.scope.unwrap_or(default),
        }
    }

    /// The spelling is a fully qualified package reference.
    pub(super) fn validate(&self, table: &str) -> Result<(), String> {
        let spelling = self.package();
        let requested = crate::PackageRef::parse(spelling)
            .map_err(|error| format!("{table} requirement `{spelling}` is invalid: {error}"))?;
        if requested.group.is_none() {
            return Err(format!(
                "{table} requirement `{spelling}` must be fully qualified as <group>/<name>[@<requirement>]"
            ));
        }
        Ok(())
    }
}
