//! Local byte-copy publication. A result.json commit record makes a completely
//! staged tree readable; no downstream activation or capture-proof promotion.
use super::{BoundDerivative, SourceInput, prepare_binding};
use crate::source_query::CapturedSourceFile;
use std::{collections::BTreeSet, path::PathBuf};
use vibe_core::manifest::{SourceSelector, derivation::RetainedSource};
use vibe_safefs::{ChildKind, Project};

#[derive(Debug, thiserror::Error)]
#[error(
    "local derivative: {reason}; residue={residue:?}; record_may_be_published={record_may_be_published}"
)]
pub struct LocalDerivativeError {
    pub reason: String,
    pub residue: Option<PathBuf>,
    pub record_may_be_published: bool,
}
fn error(reason: impl std::fmt::Display) -> LocalDerivativeError {
    LocalDerivativeError {
        reason: reason.to_string(),
        residue: None,
        record_may_be_published: false,
    }
}

/// Recapture the entire selected inventory with the incumbent bounded no-follow
/// reader. The retained observation is a comparison basis, never proof authority.
pub fn capture_retained(
    project: &Project,
    retained: &RetainedSource,
) -> Result<SourceInput, LocalDerivativeError> {
    check_root(project)?;
    retained.validate().map_err(error)?;
    vibe_safefs::judge_selection(retained.files.iter().map(|f| f.path.as_str())).map_err(error)?;
    check_membership(project, retained)?;
    let mut files = Vec::new();
    for file in &retained.files {
        let snapshot = project
            .read_file_snapshot_bounded(&file.path, file.bytes.len())
            .map_err(error)?
            .ok_or_else(|| error(format!("missing resource {}", file.path)))?;
        if snapshot.bytes != file.bytes {
            return Err(error(format!("changed bytes {}", file.path)));
        }
        files.push(CapturedSourceFile::new(file.path.clone(), snapshot).map_err(error)?);
    }
    check_membership(project, retained)?;
    check_root(project)?;
    SourceInput::new(retained.owner.clone(), retained.descriptor.clone(), files).map_err(error)
}

fn check_root(project: &Project) -> Result<(), LocalDerivativeError> {
    let current = Project::open(project.root_path()).map_err(error)?;
    if current.root_identity().map_err(error)? != project.root_identity().map_err(error)? {
        return Err(error(
            "source root name no longer identifies the held directory",
        ));
    }
    Ok(())
}

fn check_membership(
    project: &Project,
    retained: &RetainedSource,
) -> Result<(), LocalDerivativeError> {
    let SourceSelector::Directory(root) = retained.descriptor.selector() else {
        return Ok(());
    };
    let mut pending = vec![root.clone()];
    while let Some(path) = pending.pop() {
        let directory = project
            .dir(&path.split('/').collect::<Vec<_>>(), false)
            .map_err(error)?;
        let prefix = format!("{path}/");
        let expected: BTreeSet<_> = retained
            .files
            .iter()
            .filter_map(|f| f.path.strip_prefix(&prefix))
            .map(|p| p.split('/').next().unwrap().to_owned())
            .collect();
        let actual = project
            .child_names_bounded(&directory, expected.len())
            .map_err(error)?;
        vibe_safefs::judge_selection(actual.iter().map(String::as_str)).map_err(error)?;
        if actual.into_iter().collect::<BTreeSet<_>>() != expected {
            return Err(error(format!("changed membership {path}")));
        }
        for name in expected {
            let child = format!("{path}/{name}");
            let is_file = retained.files.iter().any(|f| f.path == child);
            match (
                is_file,
                project
                    .child_kind_nofollow(&directory, &name)
                    .map_err(error)?,
            ) {
                (true, Some(ChildKind::RegularFile)) => {}
                (false, Some(ChildKind::Directory)) => pending.push(child),
                _ => return Err(error(format!("changed resource kind {child}"))),
            }
        }
    }
    Ok(())
}

/// A checked, independently owned local result. Its retained sources remain
/// observed; installation/admission is a separate consumer operation.
#[derive(Debug)]
pub struct MaterializedDerivative {
    result: BoundDerivative,
    tree: Project,
}
impl MaterializedDerivative {
    pub fn result(&self) -> &BoundDerivative {
        &self.result
    }
    pub fn tree(&self) -> &Project {
        &self.tree
    }

    /// Publish only into an absent direct child of a disjoint output parent.
    /// The authored TOML is supplied from independent retained storage. Both
    /// current source roots are checked before staging and before the commit
    /// record. Interrupted stages are reported, never adopted or cleaned here.
    pub fn derive(
        text: &str,
        base: &Project,
        local: &Project,
        parent: &Project,
        name: &str,
    ) -> Result<Self, LocalDerivativeError> {
        vibe_safefs::ensure_safe_component(name).map_err(error)?;
        let binding =
            vibe_core::manifest::derivation::DerivationBinding::parse(text).map_err(error)?;
        let destination = parent.root_path().join(name);
        let absent = Project::pin_absent_path(&destination).map_err(error)?;
        for source in [base, local] {
            if vibe_safefs::paths_overlap(source.root_path(), &destination)
                || absent.descends_from(source).map_err(error)?
            {
                return Err(error("output overlaps retained source"));
            }
        }
        let observed = capture_retained(base, binding.base())?;
        capture_retained(local, binding.local())?;
        let result = prepare_binding(text, Some(&observed)).map_err(error)?.value;
        let encoded = serde_json::to_vec(result.record()).map_err(error)?;
        check_root(parent)?;
        let holder = parent.root_dir().map_err(error)?;
        if parent
            .child_names(&holder)
            .map_err(error)?
            .iter()
            .any(|existing| vibe_safefs::identity_key(existing) == vibe_safefs::identity_key(name))
        {
            return Err(error(
                "result destination already exists or has a case alias",
            ));
        }
        let stage = holder.create_child_exclusive(name).map_err(|e| {
            let mut failure = error(&e);
            if e.created() {
                failure.residue = Some(destination.clone());
            }
            failure
        })?;
        let staged = (|| {
            let entry = parent.subproject(&[name]).map_err(error)?;
            if entry.root_identity().map_err(error)? != stage.identity().map_err(error)? {
                return Err(error("staging root changed"));
            }
            let tree = entry.ensure_subproject(&["tree"]).map_err(error)?;
            for file in &result.binding().local().files {
                let (parents, name) = vibe_safefs::split_relative(&file.path).map_err(error)?;
                let directory = tree
                    .dir(
                        &parents.iter().map(String::as_str).collect::<Vec<_>>(),
                        true,
                    )
                    .map_err(error)?;
                tree.publish_new_in(&directory, &name, &file.bytes)
                    .map_err(error)?;
            }
            validate_tree(&tree, result.binding().local())?;
            capture_retained(base, result.binding().base())?;
            capture_retained(local, result.binding().local())?;
            parent
                .subproject(&[name])
                .and_then(|p| p.root_identity())
                .map_err(error)?
                .eq(&stage.identity().map_err(error)?)
                .then_some(())
                .ok_or_else(|| error("staging root replaced"))?;
            entry
                .publish_new_in(&entry.root_dir().map_err(error)?, "result.json", &encoded)
                .map_err(|e| {
                    let mut failure = error(&e);
                    failure.record_may_be_published =
                        e.stage == vibe_safefs::PublishStage::PossiblyPublished;
                    failure
                })?;
            Self::read(&entry, base, local, encoded.len()).map_err(|mut e| {
                e.record_may_be_published = true;
                e
            })
        })();
        staged.map_err(|mut e| {
            e.residue = Some(destination);
            e
        })
    }

    /// Require the existing strict record and every actual output member.
    /// The bound authored source and current base are checked independently.
    pub fn read(
        entry: &Project,
        base: &Project,
        local: &Project,
        record_cap: usize,
    ) -> Result<Self, LocalDerivativeError> {
        check_root(entry)?;
        let bytes = entry
            .read_file_snapshot_bounded("result.json", record_cap)
            .map_err(error)?
            .ok_or_else(|| error("unpublished stage: result record absent"))?;
        let record: vibe_wire::generated::derivative_result::DerivativeResult =
            serde_json::from_slice(&bytes.bytes).map_err(error)?;
        let binding = record.checked_binding().map_err(error)?;
        let current = capture_retained(base, binding.base())?;
        capture_retained(local, binding.local())?;
        let result = BoundDerivative::read(record, Some(&current)).map_err(error)?;
        let tree = entry.subproject(&["tree"]).map_err(error)?;
        validate_tree(&tree, result.binding().local())?;
        Ok(Self { result, tree })
    }

    pub fn verify_tree(&self) -> Result<(), LocalDerivativeError> {
        validate_tree(&self.tree, self.result.binding().local())
    }
}

fn validate_tree(tree: &Project, retained: &RetainedSource) -> Result<(), LocalDerivativeError> {
    // Validate all output ancestors too: no extra file outside the selector can
    // hide in a published tree. Empty unexpected directories also refuse.
    let mut pending = vec![String::new()];
    while let Some(path) = pending.pop() {
        let directory = if path.is_empty() {
            tree.root_dir()
        } else {
            tree.dir(&path.split('/').collect::<Vec<_>>(), false)
        }
        .map_err(error)?;
        let prefix = if path.is_empty() {
            String::new()
        } else {
            format!("{path}/")
        };
        let expected: BTreeSet<_> = retained
            .files
            .iter()
            .filter_map(|f| f.path.strip_prefix(&prefix))
            .map(|p| p.split('/').next().unwrap().to_owned())
            .collect();
        let actual = tree
            .child_names_bounded(&directory, expected.len())
            .map_err(error)?;
        if actual.into_iter().collect::<BTreeSet<_>>() != expected {
            return Err(error("output membership changed"));
        }
        for name in expected {
            let child = format!("{prefix}{name}");
            if !retained.files.iter().any(|f| f.path == child) {
                pending.push(child);
            }
        }
    }
    capture_retained(tree, retained).map(|_| ())
}
