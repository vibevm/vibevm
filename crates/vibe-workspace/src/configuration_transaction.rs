//! Retained whole-operation recovery evidence around the existing boot journal.
//! This coordinator never expands the child journal's INDEX/STATIC role algebra.
//! Failed and interrupted phases keep exact before-images and explicit uncertainty.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Workspace, WorkspaceError};

pub const RECOVERY_DIRECTORY: &str = ".vibe-configuration-recovery";

#[derive(Clone)]
struct BoundaryFault {
    root: PathBuf,
    phase: String,
}
thread_local! {
    static BOUNDARY_FAULT: std::cell::RefCell<Option<BoundaryFault>> = const { std::cell::RefCell::new(None) };
}

/// Run a synchronous fault-injection exercise against actual configuration
/// writers. The one-shot fault is bound to this thread, workspace and completed
/// phase; it never changes filesystem identity or observes another operation.
/// Unwinding restores the previous injection scope.
#[doc(hidden)]
pub fn with_boundary_fault<T>(root: &Path, phase: &str, action: impl FnOnce() -> T) -> T {
    struct Reset(Option<BoundaryFault>);
    impl Drop for Reset {
        fn drop(&mut self) {
            BOUNDARY_FAULT.with(|fault| *fault.borrow_mut() = self.0.take());
        }
    }
    let previous = BOUNDARY_FAULT.with(|fault| {
        fault.replace(Some(BoundaryFault {
            root: root.to_path_buf(),
            phase: phase.into(),
        }))
    });
    let _reset = Reset(previous);
    action()
}

fn inject_boundary_fault(root: &Path, phase: &str) -> Result<(), WorkspaceError> {
    let fire = BOUNDARY_FAULT.with(|fault| {
        let mut fault = fault.borrow_mut();
        if fault
            .as_ref()
            .is_some_and(|fault| fault.root == root && fault.phase == phase)
        {
            fault.take();
            true
        } else {
            false
        }
    });
    if fire {
        Err(error(
            root,
            format!("injected interruption after {phase} writer checkpoint"),
        ))
    } else {
        Ok(())
    }
}

fn error(path: &Path, reason: impl ToString) -> WorkspaceError {
    WorkspaceError::Io {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileImage {
    bytes: Vec<u8>,
    readonly: bool,
    #[cfg(unix)]
    mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Image {
    // None is absent; Some includes every relative directory and file. Keeping
    // empty directories distinguishes an absent slot from an empty existing slot.
    directories: Option<Vec<String>>,
    files: BTreeMap<String, FileImage>,
}

/// The wire path grammar cannot encode native backslashes or non-UTF8 names.
/// Reject them before writes rather than changing their filesystem identity.
pub(crate) fn portable_relative_path(path: &Path) -> Result<String, WorkspaceError> {
    let mut parts = Vec::new();
    for part in path.components() {
        let Component::Normal(name) = part else {
            return Err(error(path, "unsupported relative filename"));
        };
        let name = name
            .to_str()
            .ok_or_else(|| error(path, "non-UTF8 filename is unsupported"))?;
        if name.contains('\\') || name.contains(':') {
            return Err(error(
                path,
                "unsupported native filename; refusing lossy normalization",
            ));
        }
        parts.push(name);
    }
    Ok(parts.join("/"))
}

fn safe_path(root: &Path, relative: &Path) -> Result<PathBuf, WorkspaceError> {
    portable_relative_path(relative)?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(error(
            relative,
            "recovery targets must be nonempty workspace-relative paths",
        ));
    }
    let mut path = root.to_path_buf();
    for part in relative.components() {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(error(&path, "recovery refuses symbolic links"));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(error(&path, e)),
        }
    }
    Ok(path)
}

fn capture(path: &Path) -> Result<Image, WorkspaceError> {
    fn visit(root: &Path, path: &Path, image: &mut Image) -> Result<(), WorkspaceError> {
        let meta = fs::symlink_metadata(path).map_err(|e| error(path, e))?;
        let relative =
            portable_relative_path(path.strip_prefix(root).map_err(|e| error(path, e))?)?;
        if meta.is_dir() {
            image
                .directories
                .as_mut()
                .expect("present image")
                .push(relative);
            let mut entries = fs::read_dir(path)
                .map_err(|e| error(path, e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| error(path, e))?;
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                visit(root, &entry.path(), image)?;
            }
        } else if meta.file_type().is_file() {
            #[cfg(unix)]
            use std::os::unix::fs::PermissionsExt;
            image.files.insert(
                relative,
                FileImage {
                    bytes: fs::read(path).map_err(|e| error(path, e))?,
                    readonly: meta.permissions().readonly(),
                    #[cfg(unix)]
                    mode: meta.permissions().mode(),
                },
            );
        } else {
            return Err(error(path, "recovery refuses non-regular state"));
        }
        Ok(())
    }
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Image {
            directories: None,
            files: BTreeMap::new(),
        }),
        Err(e) => Err(error(path, e)),
        Ok(_) => {
            let mut image = Image {
                directories: Some(vec![]),
                files: BTreeMap::new(),
            };
            visit(path, path, &mut image)?;
            Ok(image)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema: u32,
    operation: String,
    phase: String,
    in_flight: bool,
    external_effects_possible: bool,
    before: BTreeMap<PathBuf, Image>,
    checkpoint: BTreeMap<PathBuf, Image>,
}

/// The same object surrounds actual install/removal writes. It deliberately
/// has no rollback-on-Drop: an error or panic cannot prove external compensation.
pub struct ConfigurationTransaction {
    root: PathBuf,
    directory: PathBuf,
    journal: Journal,
}

fn sync_directory(path: &Path) -> Result<(), WorkspaceError> {
    #[cfg(unix)]
    fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| error(path, e))?;
    Ok(())
}

fn targets(workspace: &Workspace, slots: &[PathBuf]) -> Result<Vec<PathBuf>, WorkspaceError> {
    let mut targets = vec![PathBuf::from("vibe.lock")];
    for (rel, _) in workspace.iter_nodes() {
        let prefix = if rel == "." {
            PathBuf::new()
        } else {
            PathBuf::from(rel)
        };
        targets.push(prefix.join("vibe.toml"));
        targets.push(prefix.join(vibe_core::layout::current_boot_dir()));
        for name in crate::boot_artifacts::REDIRECT_FILES {
            targets.push(prefix.join(name));
        }
    }
    for slot in slots {
        safe_path(&workspace.root, slot)?;
        if !slot.starts_with(vibe_core::layout::current_vibedeps_root()) {
            return Err(error(
                slot,
                "slot recovery requires a dependency-slot target",
            ));
        }
        targets.push(slot.clone());
    }
    targets.sort();
    targets.dedup();
    let mut result: Vec<PathBuf> = Vec::new();
    for target in targets {
        safe_path(&workspace.root, &target)?;
        if !result.iter().any(|parent| target.starts_with(parent)) {
            result.push(target);
        }
    }
    Ok(result)
}

fn validate_image(image: &Image, path: &Path) -> Result<(), WorkspaceError> {
    if image.directories.is_none() && !image.files.is_empty() {
        return Err(error(path, "absent recovery image contains files"));
    }
    for key in image.files.keys().chain(image.directories.iter().flatten()) {
        if !key.is_empty() && portable_relative_path(Path::new(key))? != *key {
            return Err(error(path, "noncanonical recovery image path"));
        }
    }
    if let Some(dirs) = &image.directories {
        let unique = dirs.iter().collect::<std::collections::BTreeSet<_>>();
        if unique.len() != dirs.len() || dirs.iter().any(|dir| image.files.contains_key(dir)) {
            return Err(error(path, "duplicate or conflicting image entries"));
        }
        if !dirs.contains(&String::new()) && !image.files.contains_key("") {
            return Err(error(path, "present image lacks root"));
        }
        for key in image.files.keys().chain(dirs.iter()) {
            if !key.is_empty() {
                let parent = Path::new(key).parent().unwrap();
                let parent = portable_relative_path(parent)?;
                if !dirs.contains(&parent) {
                    return Err(error(path, "image entry has no parent directory"));
                }
            }
        }
    }
    Ok(())
}

// A resumed restore may contain a mixture of the two proved images, but
// never an extra file, changed bytes, symlink or unproved permission state.
fn validate_restore_state(
    current: &Image,
    before: &Image,
    after: &Image,
    path: &Path,
) -> Result<(), WorkspaceError> {
    for key in current
        .files
        .keys()
        .chain(before.files.keys())
        .chain(after.files.keys())
    {
        let value = current.files.get(key);
        if value != before.files.get(key) && value != after.files.get(key) {
            return Err(error(
                path,
                format!("third state at `{key}`; refusing recovery"),
            ));
        }
    }
    for dir in before.directories.iter().flatten() {
        if after
            .directories
            .as_ref()
            .is_some_and(|dirs| dirs.contains(dir))
            && !current
                .directories
                .as_ref()
                .is_some_and(|dirs| dirs.contains(dir))
        {
            return Err(error(path, "missing proved directory; refusing recovery"));
        }
    }
    for dir in current.directories.iter().flatten() {
        if !before
            .directories
            .as_ref()
            .is_some_and(|dirs| dirs.contains(dir))
            && !after
                .directories
                .as_ref()
                .is_some_and(|dirs| dirs.contains(dir))
        {
            return Err(error(path, "third directory state; refusing recovery"));
        }
    }
    Ok(())
}

fn restore_image(path: &Path, before: &Image, after: &Image) -> Result<(), WorkspaceError> {
    let current = capture(path)?;
    validate_restore_state(&current, before, after, path)?;
    if &current == before {
        return Ok(());
    }
    // Remove only files proven by one of the two retained images.
    for key in current.files.keys() {
        if !before.files.contains_key(key) {
            let target = if key.is_empty() {
                path.to_path_buf()
            } else {
                safe_path(path, Path::new(key))?
            };
            fs::remove_file(&target).map_err(|e| error(&target, e))?;
        }
    }
    let mut dirs = current.directories.clone().unwrap_or_default();
    dirs.sort_by_key(|dir| std::cmp::Reverse(dir.len()));
    for dir in dirs {
        if !before
            .directories
            .as_ref()
            .is_some_and(|dirs| dirs.contains(&dir))
        {
            let target = if dir.is_empty() {
                path.to_path_buf()
            } else {
                safe_path(path, Path::new(&dir))?
            };
            fs::remove_dir(&target).map_err(|e| error(&target, e))?;
        }
    }
    if let Some(dirs) = &before.directories {
        let mut dirs = dirs.clone();
        dirs.sort_by_key(String::len);
        for dir in dirs {
            let target = if dir.is_empty() {
                path.to_path_buf()
            } else {
                safe_path(path, Path::new(&dir))?
            };
            fs::create_dir_all(&target).map_err(|e| error(&target, e))?;
        }
    }
    for (key, image) in &before.files {
        let target = if key.is_empty() {
            path.to_path_buf()
        } else {
            safe_path(path, Path::new(key))?
        };
        if capture(&target)?.files.get("") == Some(image) {
            continue;
        }
        let parent = target
            .parent()
            .ok_or_else(|| error(&target, "missing image parent"))?;
        fs::create_dir_all(parent).map_err(|e| error(parent, e))?;
        let mut stage = tempfile::NamedTempFile::new_in(parent).map_err(|e| error(parent, e))?;
        stage
            .write_all(&image.bytes)
            .map_err(|e| error(&target, e))?;
        let mut permissions = stage
            .as_file()
            .metadata()
            .map_err(|e| error(&target, e))?
            .permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(image.mode);
        }
        #[cfg(not(unix))]
        permissions.set_readonly(image.readonly);
        stage
            .as_file()
            .set_permissions(permissions)
            .map_err(|e| error(&target, e))?;
        stage.as_file().sync_all().map_err(|e| error(&target, e))?;
        stage
            .persist(&target)
            .map_err(|e| error(&target, e.error))?;
        sync_directory(parent)?;
    }
    if capture(path)? != *before {
        return Err(error(path, "restored image does not match before-image"));
    }
    if let Some(parent) = path.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}

impl ConfigurationTransaction {
    /// Capture every target before publishing intent. Target lists come from
    /// the owning complete plan; directory presence does not grant deletion.
    pub fn begin(
        workspace: &Workspace,
        slots: &[PathBuf],
        operation: impl Into<String>,
    ) -> Result<Self, WorkspaceError> {
        let targets = targets(workspace, slots)?;
        let mut before = BTreeMap::new();
        for target in targets {
            // A slot already contains its own generated boot tree.
            if before
                .keys()
                .any(|parent: &PathBuf| target.starts_with(parent))
            {
                continue;
            }
            let path = safe_path(&workspace.root, &target)?;
            before.insert(target, capture(&path)?);
        }
        let parent = safe_path(&workspace.root, Path::new(RECOVERY_DIRECTORY))?;
        if parent.exists()
            && fs::read_dir(&parent)
                .map_err(|e| error(&parent, e))?
                .next()
                .is_some()
        {
            return Err(error(
                &parent,
                "pending configuration recovery requires explicit reopen/compensation before another apply",
            ));
        }
        fs::create_dir_all(&parent).map_err(|e| error(&parent, e))?;
        sync_directory(&workspace.root)?;
        let directory = tempfile::Builder::new()
            .prefix("operation-")
            .tempdir_in(&parent)
            .map_err(|e| error(&parent, e))?
            .keep();
        let transaction = Self {
            root: workspace.root.clone(),
            directory,
            journal: Journal {
                schema: 2,
                operation: operation.into(),
                phase: "prepared".into(),
                in_flight: false,
                external_effects_possible: false,
                checkpoint: before.clone(),
                before,
            },
        };
        transaction.persist()?;
        sync_directory(&parent)?;
        Ok(transaction)
    }

    /// Reopen only an explicitly selected operation in this workspace. The
    /// caller supplies the original admitted slot set; journal paths confer no
    /// authority. Old schemas and third states remain retained for inspection.
    pub fn reopen(
        workspace: &Workspace,
        slots: &[PathBuf],
        operation: &str,
    ) -> Result<Self, WorkspaceError> {
        let name = Path::new(operation);
        if name.components().count() != 1 || !operation.starts_with("operation-") {
            return Err(error(name, "invalid recovery operation name"));
        }
        let relative = Path::new(RECOVERY_DIRECTORY).join(name);
        let directory = safe_path(&workspace.root, &relative)?;
        let journal_path = safe_path(&directory, Path::new("journal.json"))?;
        let journal: Journal =
            serde_json::from_slice(&fs::read(&journal_path).map_err(|e| error(&journal_path, e))?)
                .map_err(|e| error(&journal_path, e))?;
        let expected = targets(workspace, slots)?;
        if journal.schema != 2
            || journal.before.keys().cloned().collect::<Vec<_>>() != expected
            || journal.checkpoint.keys().ne(journal.before.keys())
        {
            return Err(error(
                &journal_path,
                "recovery schema or admitted target set disagrees",
            ));
        }
        for (target, image) in journal.before.iter().chain(journal.checkpoint.iter()) {
            validate_image(image, target)?;
        }
        Ok(Self {
            root: workspace.root.clone(),
            directory,
            journal,
        })
    }

    /// Restore a proved local prefix, or resume an interrupted local restore.
    /// Boot publication stays with its role-restricted owner: a changed boot
    /// target requires that owner's compensation before this operation proceeds.
    /// Evidence and external-effect uncertainty survive successful restoration.
    pub fn restore_local_prefix(&mut self) -> Result<serde_json::Value, WorkspaceError> {
        let mut local = Vec::new();
        for (target, before) in &self.journal.before {
            let path = safe_path(&self.root, target)?;
            let current = capture(&path)?;
            let is_local = target
                .file_name()
                .is_some_and(|name| name == "vibe.toml" || name == "vibe.lock")
                || target.starts_with(vibe_core::layout::current_vibedeps_root());
            if is_local {
                validate_restore_state(&current, before, &self.journal.checkpoint[target], &path)?;
                local.push(target.clone());
            } else if current != *before {
                return Err(error(
                    &path,
                    "boot compensation must be completed by the boot journal owner before local restore",
                ));
            }
        }
        self.enter("restoring-local-prefix", false)?;
        for target in local {
            restore_image(
                &safe_path(&self.root, &target)?,
                &self.journal.before[&target],
                &self.journal.checkpoint[&target],
            )?;
        }
        self.journal.phase = "local-prefix-restored".into();
        self.journal.in_flight = false;
        self.persist()?;
        Ok(self.report())
    }

    pub fn recovery_path(&self) -> &Path {
        &self.directory
    }

    fn persist(&self) -> Result<(), WorkspaceError> {
        let mut staged = tempfile::NamedTempFile::new_in(&self.directory)
            .map_err(|e| error(&self.directory, e))?;
        serde_json::to_writer(staged.as_file_mut(), &self.journal)
            .map_err(|e| error(&self.directory, e))?;
        staged
            .as_file_mut()
            .flush()
            .map_err(|e| error(&self.directory, e))?;
        staged
            .as_file()
            .sync_all()
            .map_err(|e| error(&self.directory, e))?;
        staged
            .persist(self.directory.join("journal.json"))
            .map_err(|e| error(&self.directory, e.error))?;
        sync_directory(&self.directory)
    }

    /// Durable write-ahead marker, before a phase can mutate files or invoke a hook.
    pub fn enter(
        &mut self,
        phase: &str,
        external_effects_possible: bool,
    ) -> Result<(), WorkspaceError> {
        self.journal.phase = phase.into();
        self.journal.in_flight = true;
        self.journal.external_effects_possible |= external_effects_possible;
        self.persist()
    }

    /// Record only a phase whose caller proved successful. A failed phase stays
    /// in-flight with original before-images; recovery cannot assume its prefix.
    pub fn checkpoint(&mut self) -> Result<(), WorkspaceError> {
        let mut current = BTreeMap::new();
        for target in self.journal.before.keys() {
            current.insert(target.clone(), capture(&safe_path(&self.root, target)?)?);
        }
        self.journal.checkpoint = current;
        self.journal.in_flight = false;
        self.persist()?;
        inject_boundary_fault(&self.root, &self.journal.phase)
    }

    /// Read-only evidence for error/report adapters and recovery tools. The
    /// payload includes before-images, not a claim of complete local rollback.
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"recovery":self.directory,"phase":self.journal.phase,
            "in_flight":self.journal.in_flight,"external_effects_possible":self.journal.external_effects_possible,
            "complete_rollback":false})
    }

    /// Remove retained evidence only after the entire caller operation succeeded.
    pub fn complete(mut self) -> Result<(), WorkspaceError> {
        self.journal.phase = "complete".into();
        self.journal.in_flight = false;
        self.persist()?;
        fs::remove_dir_all(&self.directory).map_err(|e| error(&self.directory, e))?;
        sync_directory(self.directory.parent().expect("operation parent"))
    }
}

#[cfg(test)]
mod m_05_e_3_cases {
    use super::*;

    #[test]
    fn pending_recovery_blocks_new_apply_and_legacy_images_are_inspection_only() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("vibe.toml"),
            "[project]\nname='recovery'\nversion='1.0.0'\n",
        )
        .unwrap();
        let workspace = Workspace::discover(temp.path()).unwrap();
        let tx = ConfigurationTransaction::begin(&workspace, &[], "pending").unwrap();
        assert!(ConfigurationTransaction::begin(&workspace, &[], "retry").is_err());
        let operation = tx
            .recovery_path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let path = tx.recovery_path().join("journal.json");
        let mut wire: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        wire["schema"] = serde_json::json!(1);
        fs::write(&path, serde_json::to_vec(&wire).unwrap()).unwrap();
        assert!(ConfigurationTransaction::reopen(&workspace, &[], &operation).is_err());
        assert!(path.is_file());
    }

    #[test]
    fn reopen_restores_real_manifest_lock_slot_prefix_and_retains_external_uncertainty() {
        let temp = tempfile::tempdir().unwrap();
        let manifest = b"[project]\nname='recovery'\nversion='1.0.0'\n";
        fs::write(temp.path().join("vibe.toml"), manifest).unwrap();
        fs::write(temp.path().join("vibe.lock"), b"before lock").unwrap();
        let workspace = Workspace::discover(temp.path()).unwrap();
        let slot =
            PathBuf::from(vibe_core::layout::current_vibedeps_root()).join("org.fixture.app/1.0.0");
        fs::create_dir_all(temp.path().join(&slot)).unwrap();
        fs::write(temp.path().join(&slot).join("owned"), b"before slot").unwrap();
        let mut tx =
            ConfigurationTransaction::begin(&workspace, &[slot.clone()], "restore fixture")
                .unwrap();
        tx.enter("manifest", false).unwrap();
        fs::write(temp.path().join("vibe.toml"), b"after manifest").unwrap();
        tx.checkpoint().unwrap();
        tx.enter("slot", true).unwrap();
        fs::remove_dir_all(temp.path().join(&slot)).unwrap();
        tx.checkpoint().unwrap();
        tx.enter("lock", false).unwrap();
        fs::write(temp.path().join("vibe.lock"), b"after lock").unwrap();
        tx.checkpoint().unwrap();
        let operation = tx
            .recovery_path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        drop(tx);
        let mut tx =
            ConfigurationTransaction::reopen(&workspace, &[slot.clone()], &operation).unwrap();
        let report = tx.restore_local_prefix().unwrap();
        assert_eq!(fs::read(temp.path().join("vibe.toml")).unwrap(), manifest);
        assert_eq!(
            fs::read(temp.path().join("vibe.lock")).unwrap(),
            b"before lock"
        );
        assert_eq!(
            fs::read(temp.path().join(&slot).join("owned")).unwrap(),
            b"before slot"
        );
        assert_eq!(report["external_effects_possible"], true);
        assert_eq!(report["complete_rollback"], false);
        drop(tx);
        ConfigurationTransaction::reopen(&workspace, &[slot], &operation)
            .unwrap()
            .restore_local_prefix()
            .unwrap();
    }

    #[test]
    fn recovery_refuses_third_state_before_restoring_any_target() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("vibe.toml"),
            "[project]\nname='recovery'\nversion='1.0.0'\n",
        )
        .unwrap();
        let workspace = Workspace::discover(temp.path()).unwrap();
        let mut tx = ConfigurationTransaction::begin(&workspace, &[], "third state").unwrap();
        fs::write(temp.path().join("vibe.toml"), b"planned manifest").unwrap();
        tx.checkpoint().unwrap();
        fs::write(temp.path().join("vibe.lock"), b"unrecorded user state").unwrap();
        assert!(
            tx.restore_local_prefix()
                .unwrap_err()
                .to_string()
                .contains("third state")
        );
        assert_eq!(
            fs::read(temp.path().join("vibe.toml")).unwrap(),
            b"planned manifest"
        );
        assert_eq!(
            fs::read(temp.path().join("vibe.lock")).unwrap(),
            b"unrecorded user state"
        );
        assert!(tx.recovery_path().join("journal.json").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn capture_refuses_backslash_and_non_utf8_before_publishing_intent() {
        use std::os::unix::ffi::OsStringExt;
        for name in [
            std::ffi::OsString::from("a\\b"),
            std::ffi::OsString::from_vec(vec![0xff]),
        ] {
            let temp = tempfile::tempdir().unwrap();
            fs::write(
                temp.path().join("vibe.toml"),
                "[project]\nname='recovery'\nversion='1.0.0'\n",
            )
            .unwrap();
            let workspace = Workspace::discover(temp.path()).unwrap();
            let slot = PathBuf::from(vibe_core::layout::current_vibedeps_root())
                .join("org.fixture.app/1.0.0");
            fs::create_dir_all(temp.path().join(&slot).join("a")).unwrap();
            fs::write(temp.path().join(&slot).join("a/b"), b"recorded").unwrap();
            fs::write(temp.path().join(&slot).join(&name), b"user").unwrap();
            assert!(
                ConfigurationTransaction::begin(&workspace, &[slot.clone()], "filename refusal")
                    .is_err()
            );
            assert!(!temp.path().join(RECOVERY_DIRECTORY).exists());
            assert_eq!(
                fs::read(temp.path().join(slot).join(name)).unwrap(),
                b"user"
            );
        }
    }

    #[test]
    fn boundary_prefixes_retain_before_images_and_hook_uncertainty() {
        for phase in ["manifest", "slot", "boot", "lock", "post-install"] {
            let temp = tempfile::tempdir().unwrap();
            fs::write(
                temp.path().join("vibe.toml"),
                "[project]\nname='recovery'\nversion='1.0.0'\n",
            )
            .unwrap();
            let workspace = Workspace::discover(temp.path()).unwrap();
            let mut transaction =
                ConfigurationTransaction::begin(&workspace, &[], "fault-matrix").unwrap();
            transaction.enter(phase, phase == "post-install").unwrap();
            let path = transaction.recovery_path().join("journal.json");
            drop(transaction); // simulated interrupted boundary; no Drop cleanup
            let journal: Journal = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
            assert_eq!(journal.phase, phase);
            assert!(journal.in_flight);
            assert_eq!(journal.external_effects_possible, phase == "post-install");
            assert!(
                journal.before[Path::new("vibe.toml")].files[""]
                    .bytes
                    .starts_with(b"[project]")
            );
        }
    }
}
