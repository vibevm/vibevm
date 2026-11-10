//! What became of the project plane's weak dependencies — one row per
//! `[recommends]` reference the solve reached, and the `[suggests]` hints it
//! read. Pure over the finished closure; it resolves nothing of its own.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-077#PROJECT-RECOMMENDS");

use std::collections::BTreeMap;

use vibe_core::Group;
use vibe_core::visibility::Analysis;
use vibe_resolver::{CandidateMetadata, ResolvedGraph};

/// What became of one `[recommends]` reference of the solved graph.
///
/// ```
/// use vibe_install::RecommendationOutcome;
///
/// assert_eq!(RecommendationOutcome::Installed.as_str(), "installed");
/// assert_eq!(RecommendationOutcome::Skipped.as_str(), "skipped");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendationOutcome {
    /// In the resolution because it was recommended — the weak edge is what
    /// admits it (PROP-050 `#WEAK-EDGES-ARE-ORDINARY-EDGES`).
    Installed,
    /// In the resolution anyway: a declared edge or a root already admits it,
    /// and the strong edge is what the lock records.
    Satisfied,
    /// Not in the resolution, with the reason — never a solve failure
    /// (PROP-003 `#CAP-WEAK-DEPS-WARNING`).
    Skipped,
}

impl RecommendationOutcome {
    /// The wire word for this outcome.
    ///
    /// ```
    /// use vibe_install::RecommendationOutcome;
    ///
    /// assert_eq!(RecommendationOutcome::Satisfied.as_str(), "satisfied");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            RecommendationOutcome::Installed => "installed",
            RecommendationOutcome::Satisfied => "satisfied",
            RecommendationOutcome::Skipped => "skipped",
        }
    }
}

/// One `[recommends]` reference, its recommender, and what became of it.
///
/// ```
/// use vibe_install::{RecommendationOutcome, RecommendationRow};
///
/// let row = RecommendationRow {
///     package: "org.x/companion".into(),
///     by: "org.x/recommender".into(),
///     outcome: RecommendationOutcome::Installed,
///     reason: None,
/// };
/// assert!(row.reason.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecommendationRow {
    /// The recommended coordinate, as the recommender spells it.
    pub package: String,
    /// The recommender's coordinate — the owner of the weak edge.
    pub by: String,
    pub outcome: RecommendationOutcome,
    /// Present exactly for a skip: the one line that says why.
    pub reason: Option<String>,
}

/// One `[suggests]` reference: read, reported, installed nowhere.
///
/// ```
/// use vibe_install::SuggestionRow;
///
/// let row = SuggestionRow { package: "org.x/hint".into(), by: "org.x/suggester".into() };
/// assert_eq!(row.package, "org.x/hint");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestionRow {
    pub package: String,
    /// The package whose `[suggests]` table named it.
    pub by: String,
}

/// The project plane's whole weak-dependency account for one solve.
///
/// ```
/// use vibe_install::WeakDependencyReport;
///
/// let empty = WeakDependencyReport::default();
/// assert!(empty.recommended.is_empty() && empty.suggested.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WeakDependencyReport {
    /// One row per `[recommends]` reference of every selected package, in
    /// package order and then declaration order.
    pub recommended: Vec<RecommendationRow>,
    /// Every `[suggests]` reference of every selected package, same order.
    pub suggested: Vec<SuggestionRow>,
}

impl WeakDependencyReport {
    /// `true` when there is nothing to say — no weak table in the closure.
    ///
    /// ```
    /// use vibe_install::WeakDependencyReport;
    ///
    /// assert!(WeakDependencyReport::default().is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.recommended.is_empty() && self.suggested.is_empty()
    }
}

/// The reason a recommendation the greedy expansion tried is not in the graph.
/// The expansion drops a failed trial without keeping its proof (PROP-003
/// §2.3.3), so this is the honest disjunction rather than an invented cause.
const UNSATISFIED: &str = "no version satisfies it, or it conflicts with the resolved graph";

/// The reason every recommendation is skipped when the user declined them.
const DECLINED: &str = "declined by --no-recommends";

/// Read the finished closure's weak tables into report rows.
///
/// `recommends` is the invocation posture: with the greedy expansion not run
/// at all, every reference is a skip naming the flag, which is what makes the
/// decline visible instead of silent (PROP-077 `#NO-RECOMMENDS-FLAG`).
pub(super) fn weak_report(
    selected: &BTreeMap<(Group, String), CandidateMetadata>,
    graph: &ResolvedGraph,
    analysis: &Analysis,
    recommends: bool,
) -> WeakDependencyReport {
    let mut report = WeakDependencyReport::default();
    for ((group, name), metadata) in selected {
        let by = format!("{group}/{name}");
        for reference in &metadata.view.recommends.packages {
            let Some(target_group) = reference.group.as_ref() else {
                continue;
            };
            let coordinate = format!("{target_group}/{}", reference.name);
            let selected_target = graph.find(target_group, reference.name.as_str()).is_some();
            let weak = analysis
                .effective
                .get(&coordinate)
                .is_some_and(|provenance| provenance.weak);
            let (outcome, reason) = match (selected_target, weak, recommends) {
                (true, true, _) => (RecommendationOutcome::Installed, None),
                (true, false, _) => (RecommendationOutcome::Satisfied, None),
                (false, _, true) => (
                    RecommendationOutcome::Skipped,
                    Some(UNSATISFIED.to_string()),
                ),
                (false, _, false) => (RecommendationOutcome::Skipped, Some(DECLINED.to_string())),
            };
            report.recommended.push(RecommendationRow {
                package: coordinate,
                by: by.clone(),
                outcome,
                reason,
            });
        }
        for reference in &metadata.view.suggests.packages {
            let Some(target_group) = reference.group.as_ref() else {
                continue;
            };
            report.suggested.push(SuggestionRow {
                package: format!("{target_group}/{}", reference.name),
                by: by.clone(),
            });
        }
    }
    report
}
