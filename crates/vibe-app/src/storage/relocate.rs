//! `vibe storage move` and `vibe storage gc`. A move is copy, rename,
//! rewrite the index, delete; an interruption leaves at most one orphan
//! directory, and the sweep lists (and on request removes) every directory
//! under an available library's `apps/` that no index record references.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#move");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#portability");

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_wire::generated::application_storage_gc as wire;

use super::{Availability, Storage};
use crate::error::{AppError, StorageError};
use crate::index::{Index, Location};
use crate::launch;
use crate::operations::installed;
use crate::settings::SettingsRoot;

/// The result of one move: where the installation lived and lives now.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#move")]
pub struct Moved {
    pub key: String,
    pub from: String,
    pub to: String,
    pub old_app_root: PathBuf,
    pub new_app_root: PathBuf,
    pub message: String,
}

/// Relocate one portable owned installation into library `to`: copy its
/// directory under a temporary name, rename it into place, rewrite the index
/// record, then delete the source. Non-portable, external and unavailable
/// installations, and an unavailable destination, are refused before any
/// copy. The proxy set is unchanged.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let error = vibe_app::storage_move(&settings, "org.example/tool@1.0.0", "default", None);
/// assert!(error.unwrap_err().to_string().contains("not_installed"));
/// ```
pub fn relocate(
    settings: &SettingsRoot,
    spelling: &str,
    to: &str,
    environment: Option<&dyn crate::environment::UserEnvironment>,
) -> Result<Moved, AppError> {
    let held = settings.lock()?;
    let lock = &held;
    let mut index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let (key, record) = installed(settings, &index, spelling)?;
    let key = key.to_owned();
    let Location::Owned { library, directory } = &record.location else {
        return Err(StorageError::NotPortable {
            key,
            reason: "external installations are never portable",
        }
        .into());
    };
    if !record.portable {
        return Err(StorageError::NotPortable {
            key,
            reason: "its package or installer option sets portable = false",
        }
        .into());
    }
    let (from, directory) = (library.clone(), directory.clone());
    let source = match launch::app_root(record, &storage) {
        Ok(root) if root.is_dir() => root,
        root => {
            let reason = launch::unavailable_reason(&root).unwrap_or_default();
            return Err(AppError::Unavailable { key, reason });
        }
    };
    if to == from {
        return Err(StorageError::SameLibrary { key, library: from }.into());
    }
    let target = crate::launch::library_path(&storage.usable(to)?, &directory);
    if fs::symlink_metadata(&target).is_ok() {
        return Err(StorageError::DestinationOccupied {
            path: target.display().to_string(),
        }
        .into());
    }
    let parent = target.parent().unwrap_or(Path::new("."));
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temporary = parent.join(format!(".moving-{name}-{}", std::process::id()));
    fs::create_dir_all(parent).map_err(|source| io("creating", parent, source))?;
    crate::fs_util::copy_tree(&source, &temporary)
        .map_err(|failure| io(failure.action, &failure.path, failure.source))?;
    fs::rename(&temporary, &target).map_err(|source| io("placing", &target, source))?;
    if let Some(record) = index.installations.get_mut(&key) {
        record.location = Location::Owned {
            library: to.to_owned(),
            directory,
        };
    }
    index.save_locked(
        settings,
        &crate::catalog::HomeEvent::about(crate::catalog::EventKind::Updated, key.clone()),
        lock,
    )?;
    // The payload moved, so every link that exposes one of its directories
    // on PATH is repointed (`PROP-059#path-directories`).
    crate::paths::apply_locked(
        settings,
        &mut index,
        &Storage::load(settings)?,
        environment,
        lock,
    )?;
    let mut message = format!("moved {key} from library `{from}` to `{to}`");
    if let Err(error) = fs::remove_dir_all(&source) {
        message.push_str(&format!(
            "; the source directory `{}` could not be removed ({error}) and `vibe storage gc` reports it",
            source.display()
        ));
    }
    Ok(Moved {
        key,
        from,
        to: to.to_owned(),
        old_app_root: source,
        new_app_root: target,
        message,
    })
}

/// The `vibe-app-storage-gc/1` sweep: every directory under
/// `apps/<slot>/` of an available library that no index record references,
/// leftover `.staging-*` and `.moving-*` directories included; with
/// `delete`, each is removed. Unavailable libraries are skipped and named.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// vibe_app::create_library(&dir.path().join("opt"), "default").unwrap();
/// std::fs::create_dir_all(dir.path().join("opt/apps/tool/stray")).unwrap();
/// let report = vibe_app::storage_gc(&settings, true).unwrap();
/// assert_eq!(report.orphans.len(), 1);
/// assert!(!dir.path().join("opt/apps/tool/stray").exists());
/// ```
pub fn gc(settings: &SettingsRoot, delete: bool) -> Result<wire::ApplicationStorageGc, AppError> {
    let _lock = settings.lock()?;
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let mut orphans = Vec::new();
    let mut skipped = Vec::new();
    for library in storage.libraries() {
        if let Availability::Unavailable(reason) = storage.availability(&library.id) {
            skipped.push(wire::Skipped {
                library: library.id,
                reason,
            });
            continue;
        }
        let referenced: BTreeSet<PathBuf> = index
            .installations
            .values()
            .filter_map(|record| match &record.location {
                Location::Owned {
                    library: owner,
                    directory,
                } if owner == &library.id => Some(PathBuf::from(directory)),
                _ => None,
            })
            .collect();
        for orphan in unreferenced(&library.path, &referenced)? {
            if delete {
                fs::remove_dir_all(&orphan).map_err(|source| io("removing", &orphan, source))?;
            }
            orphans.push(wire::Orphan {
                library: library.id.clone(),
                path: orphan.display().to_string(),
            });
        }
    }
    Ok(wire::ApplicationStorageGc {
        protocol: "vibe-app-storage-gc/1".into(),
        deleted: delete,
        orphans,
        skipped,
    })
}

/// Directories `apps/<slot>/<entry>` under `root` whose relative path is not
/// in `referenced`, sorted. Files and symbolic links are never candidates.
fn unreferenced(root: &Path, referenced: &BTreeSet<PathBuf>) -> Result<Vec<PathBuf>, StorageError> {
    let apps = root.join("apps");
    let mut found = Vec::new();
    for slot in directories(&apps)? {
        let slot_name = slot.file_name().map(PathBuf::from).unwrap_or_default();
        for entry in directories(&slot)? {
            let entry_name = entry.file_name().map(PathBuf::from).unwrap_or_default();
            let relative = Path::new("apps").join(&slot_name).join(entry_name);
            if !referenced.contains(&relative) {
                found.push(entry);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// The real subdirectories of `dir`; an absent `dir` has none.
fn directories(dir: &Path) -> Result<Vec<PathBuf>, StorageError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(io("reading", dir, source)),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| io("reading", dir, source))?;
        let kind = entry
            .file_type()
            .map_err(|source| io("reading", &entry.path(), source))?;
        if kind.is_dir() {
            found.push(entry.path());
        }
    }
    Ok(found)
}

fn io(action: &'static str, path: &Path, source: std::io::Error) -> StorageError {
    StorageError::Io {
        action,
        path: path.display().to_string(),
        source,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::index::{AuthoredCommands, Installation, Method, PackageCoordinate};
    use crate::storage::{DEFAULT_LIBRARY, add, create_library, detach};

    const KEY: &str = "org.example/a@1.0.0";

    pub(crate) fn owned_record(library: &str, directory: &str) -> Installation {
        Installation {
            source_evidence: None,
            secret_origins: Default::default(),
            configuration: None,
            pending: None,
            slot: "tool".into(),
            package: PackageCoordinate {
                group: "org.example".into(),
                name: "a".into(),
                version: "1.0.0".into(),
            },
            option: "zip".into(),
            platform: "linux".into(),
            arch: "x86_64".into(),
            method: Method::Zip,
            location: Location::Owned {
                library: library.into(),
                directory: directory.into(),
            },
            portable: true,
            launchers: Vec::new(),
            public_commands: Vec::new(),
            path_dirs: Vec::new(),
            version_file: Vec::new(),
            template: None,
            migrations: Vec::new(),
            migrated: Vec::new(),
            commands: AuthoredCommands::default(),
            features: Vec::new(),
            parameters: Default::default(),
            license: None,
            icon: None,
            menu: Default::default(),
            steps: Vec::new(),
            lifecycle: crate::index::Lifecycle::default(),
        }
    }

    /// A settings root with library `usb` holding the installed payload of
    /// `KEY` at `apps/tool/a`, and the default library created.
    fn installed_in_usb(record: Installation) -> (tempfile::TempDir, SettingsRoot) {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(&dir.path().join("home"));
        add(&settings, &dir.path().join("usb"), None).unwrap();
        create_library(&settings.default_library_path(), DEFAULT_LIBRARY).unwrap();
        let payload = dir.path().join("usb/apps/tool/a/bin");
        fs::create_dir_all(&payload).unwrap();
        fs::write(payload.join("tool"), b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("tool", payload.join("alias")).unwrap();
        let mut index = Index::default();
        index.installations.insert(KEY.into(), record);
        index
            .save(
                &settings,
                &crate::catalog::HomeEvent::new(crate::catalog::EventKind::Updated),
            )
            .unwrap();
        (dir, settings)
    }

    #[test]
    fn a_move_copies_renames_rewrites_and_deletes() {
        let (dir, settings) = installed_in_usb(owned_record("usb", "apps/tool/a"));
        let moved = relocate(&settings, KEY, DEFAULT_LIBRARY, None).unwrap();
        assert_eq!(moved.old_app_root, dir.path().join("usb/apps/tool/a"));
        assert_eq!(moved.new_app_root, dir.path().join("home/opt/apps/tool/a"));
        assert!(moved.new_app_root.join("bin/tool").is_file());
        #[cfg(unix)]
        assert_eq!(
            fs::read_link(moved.new_app_root.join("bin/alias")).unwrap(),
            Path::new("tool")
        );
        assert!(!moved.old_app_root.exists());
        let record = &Index::load(&settings).unwrap().installations[KEY];
        assert_eq!(
            record.location,
            Location::Owned {
                library: DEFAULT_LIBRARY.into(),
                directory: "apps/tool/a".into(),
            }
        );
        assert!(gc(&settings, false).unwrap().orphans.is_empty());
        assert!(matches!(
            relocate(&settings, KEY, DEFAULT_LIBRARY, None),
            Err(AppError::Storage(StorageError::SameLibrary { .. }))
        ));
    }

    #[test]
    fn moves_are_refused_before_any_copy() {
        let mut record = owned_record("usb", "apps/tool/a");
        record.portable = false;
        let (dir, settings) = installed_in_usb(record);
        let opt_apps = dir.path().join("home/opt/apps");
        assert!(matches!(
            relocate(&settings, KEY, DEFAULT_LIBRARY, None),
            Err(AppError::Storage(StorageError::NotPortable { .. }))
        ));
        assert!(fs::read_dir(&opt_apps).unwrap().next().is_none());

        let mut external = owned_record("usb", "apps/tool/a");
        external.method = Method::Shell;
        external.portable = false;
        external.location = Location::External {
            app_root: dir.path().join("usb/apps/tool/a"),
        };
        let mut index = Index::default();
        index.installations.insert(KEY.into(), external);
        index
            .save(
                &settings,
                &crate::catalog::HomeEvent::new(crate::catalog::EventKind::Updated),
            )
            .unwrap();
        assert!(matches!(
            relocate(&settings, KEY, DEFAULT_LIBRARY, None),
            Err(AppError::Storage(StorageError::NotPortable { .. }))
        ));

        let mut index = Index::default();
        index
            .installations
            .insert(KEY.into(), owned_record("usb", "apps/tool/a"));
        index
            .save(
                &settings,
                &crate::catalog::HomeEvent::new(crate::catalog::EventKind::Updated),
            )
            .unwrap();
        assert!(matches!(
            relocate(&settings, KEY, "nowhere", None),
            Err(AppError::State(
                crate::error::StateError::UnknownLibrary { .. }
            ))
        ));
        fs::create_dir_all(opt_apps.join("tool/a")).unwrap();
        assert!(matches!(
            relocate(&settings, KEY, DEFAULT_LIBRARY, None),
            Err(AppError::Storage(StorageError::DestinationOccupied { .. }))
        ));
        fs::remove_dir_all(opt_apps.join("tool")).unwrap();
        detach(&settings, "usb").unwrap();
        assert!(matches!(
            relocate(&settings, KEY, DEFAULT_LIBRARY, None),
            Err(AppError::Unavailable { .. })
        ));
        assert!(fs::read_dir(&opt_apps).unwrap().next().is_none());
        assert!(dir.path().join("usb/apps/tool/a/bin/tool").is_file());
    }

    #[test]
    fn gc_lists_only_unreferenced_directories_of_available_libraries() {
        let (dir, settings) = installed_in_usb(owned_record("usb", "apps/tool/a"));
        let usb = dir.path().join("usb");
        for leftover in [
            "apps/tool/.moving-a-1",
            "apps/tool/.staging-a-2",
            "apps/other/b",
        ] {
            fs::create_dir_all(usb.join(leftover)).unwrap();
        }
        fs::write(
            usb.join("apps/tool/note.txt"),
            b"a file is never a candidate",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("home/opt/apps/tool/stray")).unwrap();

        let report = gc(&settings, false).unwrap();
        let found: Vec<(&str, &str)> = report
            .orphans
            .iter()
            .map(|orphan| (orphan.library.as_str(), orphan.path.as_str()))
            .collect();
        let path = |relative: &str| dir.path().join(relative).display().to_string();
        assert_eq!(
            found,
            [
                ("default", path("home/opt/apps/tool/stray").as_str()),
                ("usb", path("usb/apps/other/b").as_str()),
                ("usb", path("usb/apps/tool/.moving-a-1").as_str()),
                ("usb", path("usb/apps/tool/.staging-a-2").as_str()),
            ]
        );
        assert!(!report.deleted);
        assert!(usb.join("apps/other/b").is_dir(), "listing deletes nothing");

        let report = gc(&settings, true).unwrap();
        assert_eq!(report.orphans.len(), 4);
        assert!(gc(&settings, false).unwrap().orphans.is_empty());
        assert!(usb.join("apps/tool/a/bin/tool").is_file());
        assert!(usb.join("apps/tool/note.txt").is_file());

        fs::create_dir_all(usb.join("apps/tool/again")).unwrap();
        let away = dir.path().join("usb-away");
        fs::rename(&usb, &away).unwrap();
        let report = gc(&settings, true).unwrap();
        assert!(report.orphans.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].library, "usb");
        assert!(report.skipped[0].reason.contains("missing"));
        assert!(away.join("apps/tool/again").is_dir());
    }
}
