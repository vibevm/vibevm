//! Pure authored-manifest proposals over explicitly selected frozen grammars.
//! Successful plans are data, not filesystem grants or authenticated receipts.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN");
use super::{Manifest, SourceDescriptor};
use std::ops::Range;
use toml_edit::Document;
mod identity;
mod profiles;
mod replay;
mod source;
use identity::{diff_identity, identity};
use profiles::{check_shape as check_materialization_shape, materialization_edit};
use replay::replay_edits;

pub const LEGACY_ABSENT: &str = "materialization-legacy-absent-r1";
pub const CURRENT_ABSENT: &str = "materialization-current-absent-r1";
pub const CURRENT_EPOCH1: &str = "materialization-current-epoch1-r1";
pub const SNAPSHOT_TO_COPY: &str = "snapshot-to-copy-r1";
pub const IDENTITY: &str = "identity-r1";
pub const SOURCE_BOOT_LEGACY: &str = "source-boot-normal-absent-r1";
pub const SOURCE_BOOT_CURRENT: &str = "source-boot-normal-declared-absent-r1";
pub const BOOT_SOURCE_INSERT: &str = "boot-source-explicit-r1";
/// Authored profile epoch provenance, independent of product and wire versions.
/// ```
/// use vibe_core::manifest::ManifestMigrationEpoch;
/// assert_ne!(ManifestMigrationEpoch::Absent, ManifestMigrationEpoch::Epoch1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestMigrationEpoch {
    Absent,
    Epoch1,
}

/// ```
/// use vibe_core::manifest::{manifest_migration_profile_epoch, ManifestMigrationEpoch, CURRENT_ABSENT};
/// assert_eq!(manifest_migration_profile_epoch(CURRENT_ABSENT), Some(ManifestMigrationEpoch::Absent));
/// assert_eq!(manifest_migration_profile_epoch("unknown-r2"), None);
/// ```
/// Canonical metadata for the registered frozen profiles; unknown revisions have
/// no recognized epoch, and absence is never guessed to mean epoch one.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN",
    r = 1
)]
pub fn manifest_migration_profile_epoch(profile: &str) -> Option<ManifestMigrationEpoch> {
    match profile {
        LEGACY_ABSENT | CURRENT_ABSENT | SOURCE_BOOT_LEGACY | SOURCE_BOOT_CURRENT => {
            Some(ManifestMigrationEpoch::Absent)
        }
        CURRENT_EPOCH1 => Some(ManifestMigrationEpoch::Epoch1),
        _ => None,
    }
}

/// Caller choices; absence asks for input and never selects a default.
/// ```
/// use vibe_core::manifest::{MigrationSelection, CURRENT_ABSENT, IDENTITY};
/// let selected = MigrationSelection { source_profile: Some(CURRENT_ABSENT),
///     target_profile: Some(CURRENT_ABSENT), recipe: Some(IDENTITY), source_decisions: &[] };
/// assert!(selected.recipe.is_some());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MigrationSelection<'a> {
    pub source_profile: Option<&'a str>,
    pub target_profile: Option<&'a str>,
    pub recipe: Option<&'a str>,
    pub source_decisions: &'a [SourceDescriptor],
}

/// A missing caller selection with explicitly listed allowed alternatives.
/// ```
/// use vibe_core::manifest::*;
/// let ManifestMigrationOutcome::NeedsInput(questions) =
///     plan_manifest_migration("", MigrationSelection::default()) else { panic!("expected choices") };
/// let question: &MigrationInputRequirement = &questions[0];
/// assert_eq!(question.field, "source_profile");
/// assert!(question.choices.contains(&LEGACY_ABSENT.to_owned()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationInputRequirement {
    pub field: &'static str,
    pub choices: Vec<String>,
    pub resource_path: Option<String>,
}

/// Stable failure classification, distinct from missing caller input.
/// ```
/// use vibe_core::manifest::*;
/// let choice = MigrationSelection { source_profile: Some("unknown"), ..Default::default() };
/// let ManifestMigrationOutcome::Refused(error) = plan_manifest_migration("", choice) else { panic!("expected refusal") };
/// assert_eq!(error.kind, MigrationRefusalKind::UnknownSelection);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationRefusalKind {
    UnknownSelection,
    IncompatibleEdge,
    MalformedSyntax,
    UnrecognizedShape,
    InvalidCurrentManifest,
    UnprovableEdit,
}

/// Fixed reason; field paths may include authored keys, without parser diagnostics.
/// ```
/// use vibe_core::manifest::*;
/// let choice = MigrationSelection { recipe: Some("unknown"), ..Default::default() };
/// let ManifestMigrationOutcome::Refused(error) = plan_manifest_migration("", choice) else { panic!("expected refusal") };
/// let error: MigrationRefusal = error;
/// assert_eq!(error.field, "recipe");
/// assert_eq!(error.reason, "unknown profile or recipe revision");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRefusal {
    pub kind: MigrationRefusalKind,
    pub field: String,
    pub reason: &'static str,
}

/// One original UTF-8 byte range and its proposed replacement token.
/// ```
/// use vibe_core::manifest::MigrationTokenEdit;
/// let edit = MigrationTokenEdit { field: "package.materialization", before_span: 0..10,
///     before_token: "'snapshot'".into(), after_token: "'copy'".into() };
/// assert_eq!(edit.before_span.len(), edit.before_token.len());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationTokenEdit {
    pub field: &'static str,
    pub before_span: Range<usize>,
    pub before_token: String,
    pub after_token: String,
}

/// Owned in-memory proposal, not an application or filesystem grant.
/// ```
/// use vibe_core::manifest::*;
/// let text = "[package]\nname='demo'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n";
/// let choice = MigrationSelection { source_profile: Some(CURRENT_ABSENT),
///     target_profile: Some(CURRENT_ABSENT), recipe: Some(IDENTITY), source_decisions: &[] };
/// let ManifestMigrationOutcome::NoOp(plan) = plan_manifest_migration(text, choice) else { panic!("expected no-op") };
/// let plan: ManifestMigrationPlan = plan;
/// assert_eq!(plan.before, plan.after);
/// assert!(plan.edits.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestMigrationPlan {
    pub source_profile: &'static str,
    pub target_profile: &'static str,
    pub recipe: &'static str,
    pub before: String,
    pub after: String,
    pub edits: Vec<MigrationTokenEdit>,
    pub before_identity: String,
    pub after_identity: String,
    pub diff_identity: String,
    pub checked_manifest: Manifest,
}

/// Complete proposals, validated identity, missing selections and refusal stay distinct.
/// ```
/// use vibe_core::manifest::*;
/// assert!(matches!(plan_manifest_migration("", MigrationSelection::default()),
///     ManifestMigrationOutcome::NeedsInput(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestMigrationOutcome {
    Planned(ManifestMigrationPlan),
    NoOp(ManifestMigrationPlan),
    NeedsInput(Vec<MigrationInputRequirement>),
    Refused(MigrationRefusal),
}

type Checked<T> = Result<T, MigrationRefusal>;
const EDGES: [(&str, &str, &str); 3] = [
    (LEGACY_ABSENT, CURRENT_ABSENT, SNAPSHOT_TO_COPY),
    (CURRENT_ABSENT, CURRENT_ABSENT, IDENTITY),
    (CURRENT_EPOCH1, CURRENT_EPOCH1, IDENTITY),
];

/// Plan exact text without filesystem reads, writes, providers or implicit choices.
/// This finite grammar is not the entire historical/current manifest language.
/// ```
/// use vibe_core::manifest::*;
/// let text = "[package]\nname='demo'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\nmaterialization='snapshot'\n";
/// let selection = MigrationSelection { source_profile: Some(LEGACY_ABSENT),
///     target_profile: Some(CURRENT_ABSENT), recipe: Some(SNAPSHOT_TO_COPY), source_decisions: &[] };
/// let ManifestMigrationOutcome::Planned(plan) = plan_manifest_migration(text, selection) else { panic!("expected plan") };
/// assert_eq!(plan.after, text.replace("'snapshot'", "'copy'"));
/// assert_eq!(plan.edits.len(), 1);
/// assert!(matches!(plan_manifest_migration(text, MigrationSelection::default()), ManifestMigrationOutcome::NeedsInput(_)));
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN",
    r = 1
)]
pub fn plan_manifest_migration(
    text: &str,
    selection: MigrationSelection<'_>,
) -> ManifestMigrationOutcome {
    match plan(text, selection) {
        Ok(outcome) => outcome,
        Err(refusal) => ManifestMigrationOutcome::Refused(refusal),
    }
}
fn refusal(kind: MigrationRefusalKind, field: &str, reason: &'static str) -> MigrationRefusal {
    MigrationRefusal {
        kind,
        field: field.into(),
        reason,
    }
}
fn unprovable(field: &str) -> MigrationRefusal {
    refusal(
        MigrationRefusalKind::UnprovableEdit,
        field,
        "selected token span or replacement cannot be proved",
    )
}
fn check_shape(document: &Document<&str>, profile: &str) -> Checked<()> {
    if matches!(profile, SOURCE_BOOT_LEGACY | SOURCE_BOOT_CURRENT) {
        source::check_source_shape(document, profile)
    } else {
        check_materialization_shape(document, profile)
    }
}
fn plan(text: &str, selection: MigrationSelection<'_>) -> Checked<ManifestMigrationOutcome> {
    let selected = [
        selection.source_profile,
        selection.target_profile,
        selection.recipe,
    ];
    let source_lane = !selection.source_decisions.is_empty()
        || selected.iter().flatten().any(|value| {
            matches!(
                *value,
                SOURCE_BOOT_LEGACY | SOURCE_BOOT_CURRENT | BOOT_SOURCE_INSERT
            )
        });
    let fields = ["source_profile", "target_profile", "recipe"];
    for (index, value) in selected.iter().enumerate() {
        if let Some(value) = value {
            let known = if source_lane {
                if index == 2 {
                    matches!(*value, BOOT_SOURCE_INSERT | IDENTITY)
                } else {
                    matches!(*value, SOURCE_BOOT_LEGACY | SOURCE_BOOT_CURRENT)
                }
            } else if index == 2 {
                matches!(*value, SNAPSHOT_TO_COPY | IDENTITY)
            } else {
                matches!(*value, LEGACY_ABSENT | CURRENT_ABSENT | CURRENT_EPOCH1)
            };
            if !known {
                return Err(refusal(
                    MigrationRefusalKind::UnknownSelection,
                    fields[index],
                    "unknown profile or recipe revision",
                ));
            }
        }
    }
    let source_edges = [
        (SOURCE_BOOT_LEGACY, SOURCE_BOOT_CURRENT, BOOT_SOURCE_INSERT),
        (SOURCE_BOOT_CURRENT, SOURCE_BOOT_CURRENT, IDENTITY),
    ];
    let edges: Vec<_> = (if source_lane {
        &source_edges[..]
    } else {
        &EDGES[..]
    })
    .iter()
    .copied()
    .filter(|(source, target, recipe)| {
        selected
            .iter()
            .zip([source, target, recipe])
            .all(|(choice, value)| choice.is_none_or(|choice| choice == *value))
    })
    .collect();
    if edges.is_empty() {
        return Err(refusal(
            MigrationRefusalKind::IncompatibleEdge,
            "selection",
            "no registered edge matches the explicit choices",
        ));
    }
    if selected.iter().any(Option::is_none) {
        let mut requirements = Vec::new();
        for index in 0..3 {
            if selected[index].is_none() {
                let mut choices = Vec::new();
                for edge in &edges {
                    let value = [edge.0, edge.1, edge.2][index];
                    if !choices.contains(&value) {
                        choices.push(value);
                    }
                }
                requirements.push(MigrationInputRequirement {
                    field: fields[index],
                    choices: choices.into_iter().map(str::to_owned).collect(),
                    resource_path: None,
                });
            }
        }
        return Ok(ManifestMigrationOutcome::NeedsInput(requirements));
    }
    let (source, target, recipe) = edges[0];
    let input = Document::parse(text).map_err(|_| {
        refusal(
            MigrationRefusalKind::MalformedSyntax,
            "document",
            "malformed TOML",
        )
    })?;
    check_shape(&input, source)?;
    if recipe != BOOT_SOURCE_INSERT && !selection.source_decisions.is_empty() {
        return Err(profiles::shape("source_decisions"));
    }
    let edits = if recipe == BOOT_SOURCE_INSERT {
        match source::source_insert_edit(text, &input, selection.source_decisions)? {
            Ok(edit) => vec![edit],
            Err(question) => return Ok(ManifestMigrationOutcome::NeedsInput(vec![question])),
        }
    } else if recipe == SNAPSHOT_TO_COPY {
        vec![materialization_edit(text, &input)?]
    } else {
        Vec::new()
    };
    let after = replay_edits(text, &edits)?;
    let output = Document::parse(after.as_str()).map_err(|_| unprovable("document"))?;
    check_shape(&output, target)?;
    let checked_manifest = Manifest::parse_str(&after).map_err(|_| {
        refusal(
            MigrationRefusalKind::InvalidCurrentManifest,
            "document",
            "recognized output refused by current Manifest owner",
        )
    })?;
    let result = ManifestMigrationPlan {
        source_profile: source,
        target_profile: target,
        recipe,
        before: text.to_owned(),
        before_identity: identity(b"vibe-manifest-content-r1", &[text.as_bytes()]),
        after_identity: identity(b"vibe-manifest-content-r1", &[after.as_bytes()]),
        diff_identity: diff_identity(source, target, recipe, text, &after, &edits),
        after,
        edits,
        checked_manifest,
    };
    Ok(if result.edits.is_empty() {
        ManifestMigrationOutcome::NoOp(result)
    } else {
        ManifestMigrationOutcome::Planned(result)
    })
}
/// Shared exact-token replay for validated authored-format proposals.
pub(crate) fn replay_authored_tokens(
    text: &str,
    edits: &[MigrationTokenEdit],
) -> Result<String, MigrationRefusal> {
    replay_edits(text, edits)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_replay;

#[cfg(test)]
mod tests_source;
