//! Opt-in retirement of one independently reproduced plain-copy slot.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PERSISTENCE");
use crate::{WorkspaceError, vibedeps};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use vibe_core::manifest::{LockedPackage, Manifest, Materialization, SpecFormat};
use vibe_safefs::{EntryState, FileIdentity, Pinned, Project, StableFileSnapshot};

type CheckResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
macro_rules! failure { ($($arg:tt)*) => { std::io::Error::other(format!($($arg)*)) }; }
macro_rules! require { ($condition:expr, $($arg:tt)*) => { if !$condition { return Err(failure!($($arg)*).into()); } }; }

/// Trusted Rust composition supplies a slot reproduced by the incumbent plain
/// copy materializer from an independently verified exact old source. Recheck
/// only that source identity; this port does not judge the live slot's custody.
///
/// ```
/// use std::path::{Path, PathBuf};
/// use vibe_core::manifest::LockedPackage;
/// use vibe_workspace::install::{CheckedRemovalPlan, PreparedRetirementSlot};
///
/// struct Reproduced {
///     row: LockedPackage,
///     slot: PathBuf,
///     source_unchanged: bool,
/// }
/// impl PreparedRetirementSlot for Reproduced {
///     fn package(&self) -> &LockedPackage { &self.row }
///     fn expected_slot(&self) -> &Path { &self.slot }
///     fn recheck_source(&self) -> Result<(), String> {
///         if self.source_unchanged { Ok(()) } else { Err("old source changed".into()) }
///     }
/// }
/// let row = |version: &str| -> Result<LockedPackage, toml::de::Error> {
///     toml::from_str(&format!(
///         "kind = 'tool'\nname = 'leaf'\ngroup = 'org.example'\nversion = '{version}'\n\
///          source_url = 'file:///fixture'\ncontent_hash = 'sha256:{}'\n",
///         "a".repeat(64)
///     ))
/// };
/// let temporary = tempfile::tempdir()?;
/// let expected = Reproduced {
///     row: row("1.0.0")?,
///     slot: temporary.path().join("reference"),
///     source_unchanged: true,
/// };
/// let slot: &dyn PreparedRetirementSlot = &expected;
/// assert!(slot.recheck_source().is_ok());
/// assert_eq!(slot.package().name.as_str(), "leaf");
/// // A retained row that is not exactly the reproduced one is refused before
/// // any slot is read.
/// let other = row("2.0.0")?;
/// assert!(CheckedRemovalPlan::capture(temporary.path(), &other, slot).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub trait PreparedRetirementSlot {
    fn package(&self) -> &LockedPackage;
    fn expected_slot(&self) -> &Path;
    fn recheck_source(&self) -> Result<(), String>;
}

#[derive(Debug, PartialEq, Eq)]
struct Census {
    directories: BTreeMap<String, (FileIdentity, Option<u32>)>,
    files: BTreeMap<String, StableFileSnapshot>,
    bytes: usize,
}
impl Census {
    fn read(project: &Project, root: &Pinned) -> CheckResult<Self> {
        fn walk(
            project: &Project,
            directory: &Pinned,
            prefix: &str,
            depth: usize,
            out: &mut Census,
        ) -> CheckResult<()> {
            require!(
                depth <= 16 && out.directories.len() + out.files.len() < 1024,
                "retirement census limit"
            );
            out.directories.insert(
                prefix.into(),
                (directory.identity()?, directory.unix_mode()?),
            );
            let mut names = project.child_names_bounded(directory, 1024)?;
            names.sort();
            for name in names {
                require!(
                    out.directories.len() + out.files.len() < 1024,
                    "retirement census entry limit"
                );
                let path = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}/{name}")
                };
                match directory.open_child(&name) {
                    Ok(child) => walk(project, &child, &path, depth + 1, out)?,
                    Err(_) => {
                        let file = project
                            .read_file_snapshot_bounded_in(directory, &name, 1024 * 1024)?
                            .ok_or_else(|| failure!("retirement member disappeared"))?;
                        out.bytes = out
                            .bytes
                            .checked_add(file.bytes.len())
                            .ok_or_else(|| failure!("retirement bytes overflow"))?;
                        require!(out.bytes <= 8 * 1024 * 1024, "retirement census byte limit");
                        out.files.insert(path, file);
                    }
                }
            }
            Ok(())
        }
        let mut result = Self {
            directories: BTreeMap::new(),
            files: BTreeMap::new(),
            bytes: 0,
        };
        walk(project, root, "", 0, &mut result)?;
        Ok(result)
    }
    fn same_materialization(&self, other: &Self) -> bool {
        self.directories.len() == other.directories.len()
            && self.files.len() == other.files.len()
            && self.directories.iter().all(|(path, (_, mode))| {
                other
                    .directories
                    .get(path)
                    .is_some_and(|(_, other)| other == mode)
            })
            && self.files.iter().all(|(path, file)| {
                other.files.get(path).is_some_and(|other| {
                    file.bytes == other.bytes && file.unix_mode == other.unix_mode
                })
            })
    }
}
fn refused(path: &Path, reason: impl std::fmt::Display) -> WorkspaceError {
    WorkspaceError::SpecMaterialization {
        path: path.into(),
        reason: format!("checked retirement refused: {reason}"),
    }
}

/// A private-fields plan ties the expected materialization to the observed live
/// physical slot and parent. It cannot be constructed by a caller's bool.
#[derive(Debug)]
pub struct CheckedRemovalPlan {
    project: Project,
    parent: Pinned,
    parent_path: PathBuf,
    parent_identity: FileIdentity,
    name: String,
    state: EntryState,
    census: Census,
}
impl CheckedRemovalPlan {
    pub fn slot_relative(&self) -> Result<String, WorkspaceError> {
        let path = self.parent.join(&self.name);
        path.strip_prefix(self.project.root_path())
            .ok()
            .and_then(|path| path.to_str())
            .map(|path| path.replace('\\', "/"))
            .ok_or_else(|| refused(&path, "slot path cannot be represented losslessly"))
    }
    pub fn capture(
        workspace_root: &Path,
        old: &LockedPackage,
        expected: &dyn PreparedRetirementSlot,
    ) -> Result<Self, WorkspaceError> {
        let path =
            vibedeps::slot_abs_path(workspace_root, &old.group, old.name.as_str(), &old.version);
        if expected.package() != old || old.materialization != Materialization::Copy {
            return Err(refused(
                &path,
                "requires exact retained plain-copy identity",
            ));
        }
        expected
            .recheck_source()
            .map_err(|error| refused(&path, error))?;
        let expected_project =
            Project::open(expected.expected_slot()).map_err(|error| refused(&path, error))?;
        let expected_root = expected_project
            .root_dir()
            .map_err(|error| refused(&path, error))?;
        let reference = Census::read(&expected_project, &expected_root)
            .map_err(|error| refused(&path, error))?;
        let metadata = reference
            .files
            .get(vibe_core::layout::SLOT_RECORD_FILENAME)
            .ok_or_else(|| refused(&path, "expected canonical record missing"))?;
        let record = vibedeps::read_slot_record_bytes(
            &expected
                .expected_slot()
                .join(vibe_core::layout::SLOT_RECORD_FILENAME),
            &metadata.bytes,
        )
        .map_err(|error| refused(&path, error))?;
        if record.source_hash != old.content_hash
            || record.spec_format != SpecFormat::Mixed
            || record.converter_recipe.is_some()
            || record.overlay_hash.is_some()
            || record.derived_hash.is_some()
        {
            return Err(refused(
                &path,
                "expected record is not the exact plain-copy old source",
            ));
        }
        let manifest = reference
            .files
            .get(Manifest::FILENAME)
            .ok_or_else(|| refused(&path, "expected manifest missing"))?;
        let manifest = Manifest::parse_str(
            std::str::from_utf8(&manifest.bytes).map_err(|error| refused(&path, error))?,
        )
        .map_err(|error| refused(&path, error))?;
        let package = manifest
            .require_package()
            .map_err(|error| refused(&path, error))?;
        if package.group != old.group
            || package.name != old.name
            || package.version != old.version
            || package.materialization != Materialization::Copy
        {
            return Err(refused(
                &path,
                "expected manifest differs from locked identity",
            ));
        }
        let mut expected_files = record
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect::<std::collections::BTreeSet<_>>();
        expected_files.insert(vibe_core::layout::SLOT_RECORD_FILENAME.into());
        if expected_files != reference.files.keys().cloned().collect() {
            return Err(refused(&path, "expected tree includes unrecorded files"));
        }
        let mut expected_dirs = std::collections::BTreeSet::from([String::new()]);
        for file in &expected_files {
            for parent in Path::new(file).ancestors().skip(1) {
                expected_dirs.insert(
                    parent
                        .to_str()
                        .ok_or_else(|| refused(&path, "non-UTF8 expected directory"))?
                        .replace('\\', "/"),
                );
            }
        }
        if expected_dirs != reference.directories.keys().cloned().collect() {
            return Err(refused(
                &path,
                "expected tree includes unproved empty directories",
            ));
        }
        for file in &record.files {
            if reference
                .files
                .get(&file.path)
                .is_none_or(|snapshot| snapshot.sha256 != file.sha256)
            {
                return Err(refused(&path, "expected record bytes disagree"));
            }
        }
        let project = Project::open(workspace_root).map_err(|error| refused(&path, error))?;
        let parent_path = path
            .parent()
            .ok_or_else(|| refused(&path, "slot parent missing"))?
            .to_path_buf();
        let relative = parent_path
            .strip_prefix(workspace_root)
            .map_err(|error| refused(&path, error))?;
        let components = relative
            .iter()
            .map(|part| {
                part.to_str()
                    .ok_or_else(|| refused(&path, "non-UTF8 slot parent"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let parent = project
            .dir_if_present(&components)
            .map_err(|error| refused(&path, error))?
            .ok_or_else(|| refused(&path, "slot parent absent"))?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| refused(&path, "invalid slot name"))?
            .to_owned();
        let root = parent
            .open_child(&name)
            .map_err(|error| refused(&path, error))?;
        let state = parent
            .inspect_child_state(&name)
            .map_err(|error| refused(&path, error))?
            .ok_or_else(|| refused(&path, "slot absent"))?;
        let census = Census::read(&project, &root).map_err(|error| refused(&path, error))?;
        if !census.same_materialization(&reference) {
            return Err(refused(
                &path,
                "complete live slot differs from reproduced materialization",
            ));
        }
        let again = Census::read(
            &project,
            &parent
                .open_child(&name)
                .map_err(|error| refused(&path, error))?,
        )
        .map_err(|error| refused(&path, error))?;
        if census != again {
            return Err(refused(&path, "live slot changed during census"));
        }
        expected
            .recheck_source()
            .map_err(|error| refused(&path, error))?;
        let parent_identity = parent.identity().map_err(|error| refused(&path, error))?;
        Ok(Self {
            project,
            parent,
            parent_path,
            parent_identity,
            name,
            state,
            census,
        })
    }
    fn recheck(&self) -> CheckResult<()> {
        require!(
            Project::open(&self.parent_path)?.root_identity()? == self.parent_identity,
            "original parent replaced"
        );
        require!(
            self.parent.inspect_child_state(&self.name)?.as_ref() == Some(&self.state),
            "slot identity changed"
        );
        require!(
            Census::read(&self.project, &self.parent.open_child(&self.name)?)? == self.census,
            "slot tree changed"
        );
        Ok(())
    }
}

/// Successful custody outcome; retained backup is not an operation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedRetirementBackup {
    pub path: PathBuf,
    pub state: RetirementBackupState,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetirementBackupState {
    PublishedBackupRetained,
    RestoredContainerRetained,
}

/// This step preserves its backup on every unverified exit, including errors
/// before it is registered in InstallRollback and ordinary Drop on success.
#[derive(Debug)]
pub(super) struct CheckedRemoval {
    plan: CheckedRemovalPlan,
    stash: PathBuf,
    stash_parent: Pinned,
}
impl CheckedRemoval {
    pub(super) fn execute(
        plan: CheckedRemovalPlan,
        stash_root: &Path,
    ) -> Result<Self, WorkspaceError> {
        let path = plan.parent.join(&plan.name);
        plan.recheck().map_err(|error| refused(&path, error))?;
        let stash = tempfile::Builder::new()
            .prefix(".vibe-retirement-")
            .tempdir_in(stash_root)
            .map_err(|error| refused(&path, error))?
            .keep();
        if stash.to_str().is_none() {
            return Err(refused(
                &path,
                "retirement container path cannot be represented losslessly; slot not moved",
            ));
        }
        let stash_parent = Project::open(&stash)
            .and_then(|project| project.root_dir())
            .map_err(|error| refused(&path, format!("container created, slot not moved: {error}; recovery container retained at {}", stash.display())))?;
        let step = Self {
            plan,
            stash,
            stash_parent,
        };
        match step.plan.parent.retire_directory_noreplace_to(
            &step.stash_parent,
            &step.plan.name,
            "slot",
            &step.plan.state,
        ) {
            Ok(()) => {}
            Err(error) => {
                let recovery = step.recovery_path();
                return Err(refused(
                    &path,
                    format!(
                        "{error}; recovery backup retained at {}",
                        recovery.display()
                    ),
                ));
            }
        }
        step.recheck_stash().map_err(|error| {
            refused(
                &path,
                format!(
                    "{error}; recovery backup retained at {}",
                    step.recovery_path().display()
                ),
            )
        })?;
        Ok(step)
    }
    fn recovery_path(&self) -> PathBuf {
        self.stash.clone()
    }
    fn recheck_stash(&self) -> CheckResult<()> {
        require!(
            Project::open(self.stash_parent.path())?.root_identity()?
                == self.stash_parent.identity()?,
            "retirement stash root replaced"
        );
        require!(
            self.stash_parent.inspect_child_state("slot")?.as_ref() == Some(&self.plan.state),
            "retirement stash identity changed"
        );
        require!(
            Census::read(&self.plan.project, &self.stash_parent.open_child("slot")?)?
                == self.plan.census,
            "retirement stash content changed"
        );
        Ok(())
    }
    pub(super) fn restore(self) -> Result<RetainedRetirementBackup, String> {
        let result = (|| -> CheckResult<()> {
            self.recheck_stash()?;
            require!(
                Project::open(&self.plan.parent_path)?.root_identity()?
                    == self.plan.parent_identity,
                "restore parent changed"
            );
            self.stash_parent.retire_directory_noreplace_to(
                &self.plan.parent,
                "slot",
                &self.plan.name,
                &self.plan.state,
            )?;
            self.plan.recheck()?;
            Ok(())
        })();
        if let Err(error) = result {
            let recovery = self.recovery_path();
            return Err(format!(
                "{error:#}; recovery backup retained at {}",
                recovery.display()
            ));
        }
        let recovery = self.recovery_path();
        Ok(RetainedRetirementBackup {
            path: recovery,
            state: RetirementBackupState::RestoredContainerRetained,
        })
    }
    pub(super) fn discard(self) -> Result<RetainedRetirementBackup, String> {
        if let Err(error) = self.recheck_stash() {
            let recovery = self.recovery_path();
            return Err(format!(
                "{error:#}; recovery backup retained at {}",
                recovery.display()
            ));
        }
        let recovery = self.recovery_path();
        Ok(RetainedRetirementBackup {
            path: recovery,
            state: RetirementBackupState::PublishedBackupRetained,
        })
    }
}

#[cfg(all(test, target_os = "linux"))]
#[path = "checked_removal_tests.rs"]
mod tests;
