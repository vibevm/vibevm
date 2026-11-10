//! What the inventory retains about rendering and migration, the two facts
//! the `vibe-user-applications/7` generation exists for: the template a
//! rendered manifest was produced from (PROP-075 `#TEMPLATE-LOCK-RECORD`),
//! the `[[application.migration]]` entries an installation's package
//! declared (`#MIGRATION-DECLARATION`), and the migrations already carried
//! into it (`#MIGRATION-FAILURE`).
//!
//! All three are retained by the **new** installation, and all three are
//! optional: a home with none of them keeps writing
//! `vibe-user-applications/6`, which is what leaves every existing PATH
//! proxy untouched (PROP-059 `#proxy-index-upgrade`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE");

use specmark::spec;

/// The template an installed version's manifest was rendered from.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD")]
pub struct InstalledTemplate {
    /// The template document's own revision.
    pub revision: String,
    /// The version the render produced.
    pub produced: String,
    pub rendered_sha256: String,
    pub digest_source_sha256: Option<String>,
    pub source_url: Option<String>,
    pub digest_url: Option<String>,
}

/// One `[[application.migration]]` entry as the installation retains it, so
/// an offer and `--only ID` need no package tree.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION")]
pub struct MigrationEntryRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// An attended offer preselects this entry.
    pub default: bool,
}

/// How one migration entry ended (PROP-075 `#MIGRATION-FAILURE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
pub enum MigrationResult {
    /// Both sides ran.
    Done,
    /// The export stopped the entry before its import and changed nothing.
    ExportFailed,
    /// The import failed; the new version stays installed and the slot's
    /// default stays exactly where the user put it.
    ImportFailed,
    /// The export succeeded and the import has not run yet.
    Pending,
}

impl MigrationResult {
    /// The stable spelling of this result.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::ExportFailed => "export_failed",
            Self::ImportFailed => "import_failed",
            Self::Pending => "pending",
        }
    }

    /// Whether the entry carried everything it declared.
    pub fn carried(self) -> bool {
        matches!(self, Self::Done)
    }
}

/// One migration already carried into this installation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
pub struct MigrationOutcomeRecord {
    pub id: String,
    /// The installation key the state was carried from.
    pub from: String,
    /// The installation key the state was carried to.
    pub to: String,
    pub result: MigrationResult,
    /// The step declaration's own id, for a failure.
    pub step: Option<String>,
    /// The kept work directory.
    pub work: Option<String>,
    /// The failing side's output log inside that directory.
    pub log: Option<String>,
    /// RFC 3339 time the entry ended.
    pub at: Option<String>,
}
