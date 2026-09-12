//! The maintenance queue — `vibe doc todo` (PROP-057
//! `##OBS-MAINTENANCE-TOOLS`, `MAINTENANCE.md` §3 and §7).
//!
//! Everything here is a question about the tree AS IT STANDS. There is no
//! «since last time» anywhere in it, and that is the norm's decision
//! rather than an omission: a version is a behavioural contract, the
//! product inside one changes invisibly, and a queue built on comparison
//! would be measuring a history the project does not keep
//! (`##OBS-VERSION-CONTRACT`). The one exception is the version-change
//! section, which appears only when the package holds a surface snapshot
//! of some OTHER declared version — a comparison between two numbers the
//! owner chose, which is the only comparison there is.
//!
//! ## It measures; it does not gate
//!
//! The command prints its numbers and returns success whatever they say.
//! No technical lock binds a release of the product to its documentation:
//! ten releases a day and a hundred pull requests make drift between
//! reconciliations an accepted risk, and the answer to an accepted risk
//! is a measurement somebody reads, not a build somebody has to get past
//! (`##OBS-NO-RELEASE-LOCK`).
//!
//! ## Nothing here is zero because nobody looked
//!
//! Every section says whether it was measured, and every metric a run
//! could not take is `null` rather than `0`. The two states print the
//! same digit and mean opposite things: «no unresolvable citation» is a
//! clean manual, «no citation check ran» is a manual nobody asked. The
//! example runner is the loudest case — it builds a sandbox per fixture
//! and costs minutes, so it runs only when asked, and when it has not run
//! the queue says so instead of reporting no red examples.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-MAINTENANCE-TOOLS");

pub mod backlog;
pub mod history;
pub mod journal;
pub mod report;
pub mod reviews;
mod sections;

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use vibe_wire::generated::doc_todo::{DocTodo, SectionName, TodoMetrics, TodoSection};

use crate::citations::SpecSources;
use crate::coverage::Obligation;
use crate::error::Result;
use crate::examples::RunnerEnv;
use crate::surface::SurfaceEnv;

/// The schema version a queue written today carries.
pub const SCHEMA_VERSION: u32 = 1;

/// How long a page may go unread aloud before it enters the queue
/// (`MAINTENANCE.md` §3). Ninety days is the quarterly promise seen from
/// the page's side.
pub const PAGE_AGE_LIMIT_DAYS: i64 = 90;

/// The fifth small edit of a page since its last reading puts it in the
/// queue (`MAINTENANCE.md` §6, the rule of five edits). Accumulated
/// patches break the ladder of a page invisibly to each author of a
/// patch.
pub const EDITS_BEFORE_A_READING: usize = 5;

/// Everything the queue needs and refuses to discover for itself.
///
/// The clock, the corpus, the world a citation resolves against and the
/// binary an example runs — all named by the caller, because a queue
/// whose numbers depended on an unstated ambient value would be a queue
/// nobody could reproduce.
pub struct Inputs {
    /// The documenting package's own `<group>/<name>`.
    pub coordinate: String,
    /// Its version, for the report's own first line.
    pub package_version: String,
    /// The version number the PRODUCT declares. The version-change
    /// section compares recorded snapshots against this and nothing else.
    pub product_version: String,
    /// The world a `rule` citation resolves against.
    pub sources: SpecSources,
    /// The tree the obligations were observed in. Without it the coverage
    /// half is not measured rather than measured as zero.
    pub corpus_root: Option<PathBuf>,
    /// The obligations, from the grounding cell that owns «which files
    /// does this project observe».
    pub obligations: Vec<Obligation>,
    /// The bar the coverage and style reports state.
    pub min: u8,
    /// Today, for the age of a page. Called once, by the caller.
    pub today: NaiveDate,
    /// The host's debt file. Absent means «not measured».
    pub backlog: Option<PathBuf>,
    /// The journal. Absent means «not measured».
    pub journal: Option<PathBuf>,
    /// Present when the caller asked for the examples to actually run.
    pub examples: Option<RunnerEnv>,
    /// Present when the caller is willing to pay for a reading of the
    /// product's surface, which the version-change section needs.
    pub surface: Option<SurfaceEnv>,
}

/// Build the queue.
pub fn build(package_dir: &Path, inputs: &Inputs) -> Result<DocTodo> {
    let set = crate::pages::read_package(package_dir)?;
    let coverage = sections::coverage(package_dir, inputs)?;
    let examples = sections::examples(package_dir, inputs)?;
    let citations = sections::citations(package_dir, inputs)?;
    let adaptations = sections::adaptations(package_dir, inputs)?;
    let age = sections::page_age(package_dir, &set, inputs)?;
    let debt = sections::debt(inputs)?;
    let style = sections::style(package_dir, inputs);
    let version_change = sections::version_change(package_dir, inputs)?;

    let metrics = TodoMetrics {
        gaps: (coverage.section.items.len()
            + examples.section.items.len()
            + citations.section.items.len()) as u32,
        page_age_median_days: age.median_days,
        adaptation_divergences: adaptations.divergences,
        coverage_percent: coverage.percent,
        tics_per_k_words: style.tics_per_100k(&set),
        debt_p1: debt.p1,
        findings_without_decision: journal_metric(inputs)?,
        days_since_reconcile: age.days_since_reconcile,
    };

    Ok(DocTodo {
        schema_version: SCHEMA_VERSION,
        package: format!("{}@{}", inputs.coordinate, inputs.package_version),
        examples_measured: examples.section.measured,
        metrics,
        sections: vec![
            coverage.section,
            examples.section,
            citations.section,
            adaptations.section,
            age.section,
            debt.section,
            style.section,
            version_change,
        ],
    })
}

/// How many journal entries still owe a decision.
fn journal_metric(inputs: &Inputs) -> Result<Option<u32>> {
    let Some(path) = &inputs.journal else {
        return Ok(None);
    };
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(journal::undecided(path)? as u32))
}

/// An empty section that was not measured — the shape a skipped question
/// takes, so a reader can tell it from a question with no answers.
pub(crate) fn unmeasured(name: SectionName) -> TodoSection {
    TodoSection {
        name,
        measured: false,
        items: Vec::new(),
    }
}

/// The queue as a machine reads it.
pub fn to_json(queue: &DocTodo) -> String {
    // Generated from the schema, so it holds only JSON scalars and
    // sequences and cannot fail to serialise.
    let mut text = serde_json::to_string_pretty(queue).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests;
