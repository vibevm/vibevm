//! Stop-aware publication of a validated agent result.
//!
//! Each declared output is still published atomically, one file at a time,
//! through `vibe_safefs`'s owned publication — the same owner the install
//! metadata guard uses — so every write mints an [`OwnedFileReceipt`] from its
//! actual staging handle. A stop observed before the first write writes
//! nothing; a stop observed between writes (or after the last one, before the
//! execution returns) puts the already-published owned files back, newest
//! first, through the checked inverse: a name that no longer holds this
//! publication's object is foreign state and is left untouched and named.
//! Directories a publication created are removed only while empty.
//!
//! A step that COMPLETES hands its owned outputs to the operation's
//! pre-commit rollback ([`CancellationToken::adopt_completed_effect`]): a
//! later stop in the same operation puts them back through the same checked
//! inverse — a foreign replacement is preserved and named — and the
//! operation's commit releases them.
//!
//! Remote effects are not local effects: nothing here touches the token's
//! provider-side record, and a clean inverse leaves publication unmarked.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");

use std::path::Path;

use vibe_core::cancellation::CancellationToken;
use vibe_safefs::{OwnedFileReceipt, Pinned, Project, StableFileSnapshot};
use vibe_wire::generated::lifecycle::e1::reply::ReplyArtifact;

use super::{AgentError, ResultPlan, machine_path};

/// One output this execution published, with the evidence its inverse needs.
struct Owned {
    path: String,
    before: Option<StableFileSnapshot>,
    receipt: OwnedFileReceipt,
    size: usize,
    /// Directories this publication created, outermost first, as components.
    created: Vec<Vec<String>>,
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW"
)]
pub(super) fn apply(
    plan: &ResultPlan,
    project_root: &Path,
    token: &CancellationToken,
) -> Result<Vec<ReplyArtifact>, AgentError> {
    let unusable = |reason: String| AgentError::Output {
        path: machine_path(project_root),
        reason: format!("the selected project root is unusable: {reason}"),
        applied: Vec::new(),
        possibly_applied: Vec::new(),
        created_directories: Vec::new(),
    };
    let project = Project::open(project_root).map_err(|error| unusable(format!("{error:#}")))?;
    let directory = project
        .root_dir()
        .map_err(|error| unusable(format!("{error:#}")))?;
    let mut owned: Vec<Owned> = Vec::with_capacity(plan.rows.len());
    let mut created_display: Vec<String> = Vec::new();
    for row in &plan.rows {
        if token.checkpoint().is_err() {
            return Err(unwind(
                &project,
                &directory,
                owned,
                token,
                "between declared outputs",
            ));
        }
        let before = project
            .read_file_snapshot_bounded_in(&directory, &row.path, usize::MAX - 1)
            .map_err(|error| {
                kept(
                    &owned,
                    &created_display,
                    &row.path,
                    format!("its current state cannot be captured before publication: {error:#}"),
                    Vec::new(),
                    token,
                )
            })?;
        let published = match project.write_atomic_owned_in_with_mode(
            &directory,
            &row.path,
            row.content.as_bytes(),
            None,
        ) {
            Ok(published) => published,
            Err(error) => {
                created_display.extend(error.publication.created_display());
                let possibly = match error.publication.stage {
                    vibe_safefs::PublishStage::BeforePublication => Vec::new(),
                    vibe_safefs::PublishStage::PossiblyPublished => vec![row.path.clone()],
                };
                return Err(kept(
                    &owned,
                    &created_display,
                    &row.path,
                    format!("{:#}", error.publication),
                    possibly,
                    token,
                ));
            }
        };
        let created_count = published.publication.published.created_directories.len();
        created_display.extend(
            published
                .publication
                .published
                .created_directories
                .iter()
                .map(|path| machine_path(path)),
        );
        if project.probe_regular_nonempty(&row.path) != vibe_safefs::Presence::RegularNonEmpty {
            return Err(kept(
                &owned,
                &created_display,
                &row.path,
                "is not a regular, exclusively-owned, non-empty file after publication, so its \
                 declared `non-empty file` acceptance fails"
                    .into(),
                vec![row.path.clone()],
                token,
            ));
        }
        owned.push(Owned {
            path: row.path.clone(),
            before,
            receipt: published.receipt,
            size: row.content.len(),
            created: created_ancestors(&row.path, created_count),
        });
    }
    // The execution has not returned yet: a stop observed here still finds
    // every owned output reversible.
    if token.checkpoint().is_err() {
        return Err(unwind(
            &project,
            &directory,
            owned,
            token,
            "after the last declared output, before the execution completed",
        ));
    }
    if !owned.is_empty() {
        // The step completed, but the OPERATION has not: its outputs join the
        // operation's pre-commit rollback, so a later stop puts them back
        // through the same checked inverse instead of leaving them as
        // unowned local effects.
        token.adopt_completed_effect(Box::new(CompletedOutputs {
            project,
            directory,
            owned,
        }));
    }
    Ok(plan.planned.clone())
}

/// The outputs of an agent step that completed, held by the operation until
/// it commits (released) or a later stop rolls it back (undone).
struct CompletedOutputs {
    project: Project,
    directory: Pinned,
    owned: Vec<Owned>,
}

impl vibe_core::cancellation::CompletedEffect for CompletedOutputs {
    fn undo(self: Box<Self>) -> Result<(), String> {
        let (_, retained) = unwind_rows(&self.project, &self.directory, self.owned);
        if retained.is_empty() {
            Ok(())
        } else {
            Err(CompletedOutputsRetained { retained }.to_string())
        }
    }
}

/// A later stop rolled the operation back, but a completed agent step's
/// output no longer holds what the step wrote.
#[derive(Debug, thiserror::Error)]
#[error(
    "a completed agent step's output(s) {} no longer hold what the step published; the \
     changed replacement was preserved, not rolled back (violates \
     spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW; fix: inspect \
     each named path, keep or remove it by hand, then run the operation again)",
    retained.join(", ")
)]
struct CompletedOutputsRetained {
    retained: Vec<String>,
}

/// A non-stop failure keeps the existing partial-state report; whatever stays
/// published is an effect the enclosing rollback does not own.
fn kept(
    owned: &[Owned],
    created: &[String],
    failed: &str,
    reason: String,
    possibly_applied: Vec<String>,
    token: &CancellationToken,
) -> AgentError {
    if !owned.is_empty() || !possibly_applied.is_empty() || !created.is_empty() {
        token.mark_unowned_effects();
    }
    AgentError::Output {
        path: failed.to_string(),
        reason,
        applied: owned.iter().map(|row| row.path.clone()).collect(),
        possibly_applied,
        created_directories: created.to_vec(),
    }
}

/// Put every owned output back, newest first. Recovery is not itself
/// cancelled by the stop that requested it.
fn unwind(
    project: &Project,
    directory: &Pinned,
    owned: Vec<Owned>,
    token: &CancellationToken,
    stage: &str,
) -> AgentError {
    let (restored, retained) = unwind_rows(project, directory, owned);
    if !retained.is_empty() {
        token.mark_unowned_effects();
    }
    AgentError::Cancelled {
        stage: stage.to_string(),
        remote: token.llm_effect().to_string(),
        restored,
        retained,
    }
}

/// Put `owned` back newest first; returns the names restored and the names
/// (or created directories) left in place because they hold foreign state.
fn unwind_rows(
    project: &Project,
    directory: &Pinned,
    mut owned: Vec<Owned>,
) -> (Vec<String>, Vec<String>) {
    let mut restored = Vec::new();
    let mut retained = Vec::new();
    while let Some(row) = owned.pop() {
        if !restore(project, directory, &row) {
            retained.push(row.path.clone());
            continue;
        }
        restored.push(row.path.clone());
        for components in row.created.iter().rev() {
            if !remove_empty_directory(project, components) {
                retained.push(format!("{}/", components.join("/")));
            }
        }
    }
    (restored, retained)
}

/// The checked inverse: only this publication's own object is replaced by
/// the captured before-image (or removed, when the name was absent). `false`
/// means the name holds foreign state, which stays untouched.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO"
)]
fn restore(project: &Project, directory: &Pinned, row: &Owned) -> bool {
    let cap = row
        .size
        .max(row.before.as_ref().map_or(0, |before| before.bytes.len()));
    let Ok(current) = project.read_file_snapshot_bounded_in(directory, &row.path, cap) else {
        return false;
    };
    if current == row.before {
        return true;
    }
    if !current
        .as_ref()
        .is_some_and(|current| row.receipt.matches_snapshot(current))
    {
        return false;
    }
    match &row.before {
        Some(before) => project
            .write_atomic_in_with_mode(directory, &row.path, &before.bytes, before.unix_mode)
            .is_ok(),
        None => project.remove_file_in(directory, &row.path).is_ok(),
    }
}

/// Whether the created directory is gone (removed now, or already absent).
fn remove_empty_directory(project: &Project, components: &[String]) -> bool {
    let Some((name, parent)) = components.split_last() else {
        return true;
    };
    let parent: Vec<&str> = parent.iter().map(String::as_str).collect();
    let Ok(Some(holder)) = project.dir_if_present(&parent) else {
        return false;
    };
    match project.remove_dir_if_empty(&holder, name) {
        Ok(true) => true,
        Ok(false) => project
            .dir_if_present(&[parent.as_slice(), &[name.as_str()]].concat())
            .is_ok_and(|present| present.is_none()),
        Err(_) => false,
    }
}

/// A publication creates the missing tail of the output's ancestor chain, so
/// `count` created directories are its deepest `count` ancestors.
fn created_ancestors(path: &str, count: usize) -> Vec<Vec<String>> {
    let components: Vec<String> = path.split('/').map(str::to_string).collect();
    let ancestors = components.len().saturating_sub(1);
    (ancestors.saturating_sub(count)..ancestors)
        .map(|depth| components[..=depth].to_vec())
        .collect()
}
