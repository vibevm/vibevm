//! `[[application.migration]]` — the entries that carry an application's
//! user state between two installed versions (PROP-075
//! `#MIGRATION-DECLARATION`).
//!
//! A row names a stable `id`, a human `name`, an optional `description`,
//! whether an attended offer preselects it (`default`), and the two step
//! declarations `export` and `import`. Each of those is **exactly one step
//! declaration of the existing `StepDecl` grammar** (PROP-065 `#steps`),
//! restricted by validation to the kinds `recipe` and `shell`: a migration
//! moves user state and must not install, download or delegate. No new
//! `StepKind` variant exists.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION");

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::StepDecl;

/// One `[[application.migration]]` row.
///
/// ```
/// use vibe_core::manifest::MigrationDecl;
///
/// let row: MigrationDecl = toml::from_str(
///     "id = \"global-packages\"\nname = \"Global packages\"\ndefault = true\n\
///      [export]\nid = \"export\"\ntitle = \"List them\"\nkind = \"recipe\"\nscript = \"m/out.sh\"\n\
///      [import]\nid = \"import\"\ntitle = \"Install them\"\nkind = \"recipe\"\nscript = \"m/in.sh\"\n",
/// )
/// .unwrap();
/// assert_eq!(row.id, "global-packages");
/// assert!(row.default);
/// assert_eq!(row.export.kind.name(), "recipe");
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationDecl {
    /// A stable token naming the entry; `--only ID` selects it.
    pub id: String,
    /// What the entry carries, in one human phrase.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether an attended offer preselects this entry.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub default: bool,
    /// The step the **old** version runs, with the old installation's
    /// payload and proxy directories first on PATH.
    pub export: StepDecl,
    /// The step the **new** version runs, with the new installation's
    /// payload and proxy directories first on PATH.
    pub import: StepDecl,
}

/// The two sides of one migration entry, named as the record and the
/// refusals name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION")]
pub enum MigrationSide {
    Export,
    Import,
}

impl MigrationSide {
    /// The stable spelling (`export`, `import`), which is also the TOML key.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Import => "import",
        }
    }
}

impl MigrationDecl {
    /// The declaration of one side.
    pub fn step(&self, side: MigrationSide) -> &StepDecl {
        match side {
            MigrationSide::Export => &self.export,
            MigrationSide::Import => &self.import,
        }
    }

    /// Both sides in the order they run.
    pub fn sides(&self) -> [(MigrationSide, &StepDecl); 2] {
        [
            (MigrationSide::Export, &self.export),
            (MigrationSide::Import, &self.import),
        ]
    }
}
