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

/// How one ecosystem adapter verb ended (PROP-075 `#ECOSYSTEM-ADAPTER`).
/// `Skipped` is an adapter that declares no such verb, or one whose item
/// list was empty, and is never an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
pub enum ExtrasResult {
    Done,
    Failed,
    Skipped,
}

impl ExtrasResult {
    /// The stable spelling of this outcome.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

/// What one declared ecosystem adapter last did in this installation
/// (PROP-075 `#SLOT-EXTRAS`): the final list its `install` verb was handed
/// and what its `list` verb observed, each with its time and outcome.
///
/// It rides the `vibe-user-applications/8` generation P7d already paid for
/// — as **optional** members, so a home that never ran an adapter verb
/// keeps writing the lowest protocol it can and every existing PATH proxy
/// goes on reading it (PROP-059 `#proxy-index-upgrade`).
///
/// A failing apply never rolls the runtime back, so this record is the one
/// place a later `vibe app describe` can still say what was **not**
/// carried. Every item is opaque: held and echoed, never parsed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
pub struct ExtrasRecord {
    /// The adapter's opaque id, as the package wrote it.
    pub id: String,
    /// The final list the `install` verb was last handed, in order.
    pub applied: Vec<String>,
    /// RFC 3339 time of that apply.
    pub applied_at: Option<String>,
    pub applied_outcome: Option<ExtrasResult>,
    /// What the `list` verb last observed, in the order it reported.
    pub observed: Vec<String>,
    /// RFC 3339 time of that observation.
    pub observed_at: Option<String>,
    pub observed_outcome: Option<ExtrasResult>,
}

impl ExtrasRecord {
    /// A record of `id` alone, which is what an installation carries
    /// before any verb has run in it.
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            ..Self::default()
        }
    }

    /// Whether this record retains anything at all. One that does not is
    /// never written, so an adapter that only ever skipped does not force
    /// a home onto the `/8` reader.
    pub fn is_empty(&self) -> bool {
        self.applied.is_empty()
            && self.applied_at.is_none()
            && self.applied_outcome.is_none()
            && self.observed.is_empty()
            && self.observed_at.is_none()
            && self.observed_outcome.is_none()
    }
}
