//! `vibe storage add | attach | detach | list`: row edits of `storage.toml`
//! under the mutation lock, and the inventory of every library with its
//! availability and installations.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#availability");

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::manifest::is_portable_token;
use vibe_wire::generated::application_storage_list as wire;

use super::{Availability, DEFAULT_LIBRARY, Library, Storage, create_library, read_marker, save};
use crate::error::{AppError, StateError, StorageError};
use crate::index::{Index, Location};
use crate::launch;
use crate::settings::{SettingsRoot, io};

/// Create a fresh library in a new or empty directory and list it. Without
/// `id`, the directory's own name is the id when it is a portable token.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(&dir.path().join("home"));
/// let usb = dir.path().join("usb");
/// let library = vibe_app::storage_add(&settings, &usb, None).unwrap();
/// assert_eq!(library.id, "usb");
/// assert!(usb.join(vibe_app::LIBRARY_MARKER).is_file());
/// assert!(vibe_app::storage_add(&settings, &usb, Some("again")).is_err());
/// ```
pub fn add(settings: &SettingsRoot, path: &Path, id: Option<&str>) -> Result<Library, AppError> {
    let _lock = settings.lock()?;
    let storage = Storage::load(settings)?;
    let path = absolute(path)?;
    let id = match id {
        Some(id) => id.to_owned(),
        None => path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| is_portable_token(name))
            .map(str::to_owned)
            .ok_or_else(|| StorageError::InvalidId {
                id: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                reason: "the directory name is not a portable token, so it cannot be the id",
            })?,
    };
    check_new_id(&storage, &id)?;
    check_unlisted(&storage, &path)?;
    if path.exists() {
        let empty = path.is_dir()
            && fs::read_dir(&path)
                .map_err(|source| io("reading", &path, source))?
                .next()
                .is_none();
        if !empty {
            return Err(StorageError::NotEmpty {
                path: path.display().to_string(),
            }
            .into());
        }
    }
    create_library(&path, &id)?;
    let library = Library { id, path };
    let mut config = storage.config().clone();
    config.libraries.push(library.clone());
    save(settings, &config)?;
    Ok(library)
}

/// List an existing library: its marker id must not be configured yet.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(&dir.path().join("home"));
/// let disk = dir.path().join("disk");
/// vibe_app::create_library(&disk, "disk").unwrap();
/// assert_eq!(vibe_app::storage_attach(&settings, &disk).unwrap().id, "disk");
/// assert!(vibe_app::storage_attach(&settings, &disk).is_err(), "already configured");
/// ```
pub fn attach(settings: &SettingsRoot, path: &Path) -> Result<Library, AppError> {
    let _lock = settings.lock()?;
    let storage = Storage::load(settings)?;
    let path = absolute(path)?;
    let not_a_library = |reason: String| StorageError::NotALibrary {
        path: path.display().to_string(),
        reason,
    };
    if !path.is_dir() {
        return Err(not_a_library("it is not a directory".into()).into());
    }
    let marker = read_marker(&path).map_err(not_a_library)?;
    check_new_id(&storage, &marker.id)?;
    check_unlisted(&storage, &path)?;
    let library = Library {
        id: marker.id,
        path,
    };
    let mut config = storage.config().clone();
    config.libraries.push(library.clone());
    save(settings, &config)?;
    Ok(library)
}

/// Unlist one library; its directory and files are left alone, and its
/// installations become unavailable until it is attached again.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(&dir.path().join("home"));
/// let usb = dir.path().join("usb");
/// vibe_app::storage_add(&settings, &usb, None).unwrap();
/// assert_eq!(vibe_app::storage_detach(&settings, "usb").unwrap().path, usb);
/// assert!(usb.join(vibe_app::LIBRARY_MARKER).is_file(), "files stay");
/// assert!(vibe_app::storage_detach(&settings, vibe_app::DEFAULT_LIBRARY).is_err());
/// ```
pub fn detach(settings: &SettingsRoot, id: &str) -> Result<Library, AppError> {
    if id == DEFAULT_LIBRARY {
        return Err(StorageError::DetachDefault.into());
    }
    let _lock = settings.lock()?;
    let storage = Storage::load(settings)?;
    let mut config = storage.config().clone();
    let position = config
        .libraries
        .iter()
        .position(|library| library.id == id)
        .ok_or_else(|| StateError::UnknownLibrary { id: id.into() })?;
    let library = config.libraries.remove(position);
    save(settings, &config)?;
    Ok(library)
}

/// The `vibe-app-storage-list/1` inventory: every configured library (the
/// default first) with its availability and installations, then every
/// library id the index still references but `storage.toml` no longer lists.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let listing = vibe_app::storage_list(&settings).unwrap();
/// assert_eq!(listing.libraries.len(), 1);
/// assert_eq!(listing.libraries[0].id, vibe_app::DEFAULT_LIBRARY);
/// assert!(listing.libraries[0].default);
/// ```
pub fn listing(settings: &SettingsRoot) -> Result<wire::ApplicationStorageList, AppError> {
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let libraries = storage.libraries();
    let configured: BTreeSet<&str> = libraries
        .iter()
        .map(|library| library.id.as_str())
        .collect();
    let detached: BTreeSet<&str> = index
        .installations
        .values()
        .filter_map(|record| match &record.location {
            Location::Owned { library, .. } if !configured.contains(library.as_str()) => {
                Some(library.as_str())
            }
            _ => None,
        })
        .collect();
    let mut rows: Vec<wire::Library> = libraries
        .iter()
        .map(|library| row(&index, &storage, &library.id, Some(&library.path)))
        .collect();
    rows.extend(
        detached
            .into_iter()
            .map(|id| row(&index, &storage, id, None)),
    );
    Ok(wire::ApplicationStorageList {
        protocol: "vibe-app-storage-list/1".into(),
        libraries: rows,
    })
}

fn row(index: &Index, storage: &Storage, id: &str, path: Option<&Path>) -> wire::Library {
    let reason = match storage.availability(id) {
        Availability::Available => None,
        Availability::Unavailable(reason) => Some(reason),
    };
    let installations = index
        .installations
        .iter()
        .filter(|(_, record)| {
            matches!(&record.location, Location::Owned { library, .. } if library == id)
        })
        .map(|(key, record)| {
            let reason = launch::unavailable_reason(&launch::app_root(record, storage));
            wire::Installation {
                key: key.clone(),
                slot: record.slot.clone(),
                status: status(&reason),
                reason,
            }
        })
        .collect();
    wire::Library {
        id: id.to_owned(),
        path: path.map(|path| path.display().to_string()),
        default: id == DEFAULT_LIBRARY,
        status: status(&reason),
        reason,
        installations,
    }
}

fn status(reason: &Option<String>) -> wire::Status {
    if reason.is_none() {
        wire::Status::Ready
    } else {
        wire::Status::Unavailable
    }
}

fn absolute(path: &Path) -> Result<PathBuf, StorageError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Err(StorageError::RelativePath {
            path: path.display().to_string(),
        })
    }
}

fn check_new_id(storage: &Storage, id: &str) -> Result<(), StorageError> {
    let reason = if !is_portable_token(id) {
        "it is not a lowercase portable token"
    } else if id == DEFAULT_LIBRARY {
        "`default` names the implicit library under the settings root"
    } else if storage.path_of(id).is_some() {
        "a configured library already uses it"
    } else {
        return Ok(());
    };
    Err(StorageError::InvalidId {
        id: id.into(),
        reason,
    })
}

/// Refuse a directory that is already one of the libraries, the default
/// included, comparing canonical paths when both exist.
fn check_unlisted(storage: &Storage, path: &Path) -> Result<(), StorageError> {
    let canonical = fs::canonicalize(path).ok();
    for library in storage.libraries() {
        let same = library.path == path
            || canonical.is_some() && fs::canonicalize(&library.path).ok() == canonical;
        if same {
            return Err(StorageError::AlreadyListed {
                path: path.display().to_string(),
                id: library.id,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::relocate::tests::retry_locked;

    fn add(settings: &SettingsRoot, path: &Path, id: Option<&str>) -> Result<Library, AppError> {
        retry_locked(|| super::add(settings, path, id))
    }

    fn attach(settings: &SettingsRoot, path: &Path) -> Result<Library, AppError> {
        retry_locked(|| super::attach(settings, path))
    }

    fn detach(settings: &SettingsRoot, id: &str) -> Result<Library, AppError> {
        retry_locked(|| super::detach(settings, id))
    }

    #[test]
    fn add_derives_or_takes_an_id_and_refuses_bad_targets() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(&dir.path().join("home"));
        let usb = dir.path().join("usb");
        assert_eq!(add(&settings, &usb, None).unwrap().id, "usb");
        let named = dir.path().join("Big Disk");
        assert!(matches!(
            add(&settings, &named, None),
            Err(AppError::Storage(StorageError::InvalidId { .. }))
        ));
        assert_eq!(add(&settings, &named, Some("big")).unwrap().id, "big");
        let config = Storage::load(&settings).unwrap();
        let ids: Vec<&str> = config
            .config()
            .libraries
            .iter()
            .map(|library| library.id.as_str())
            .collect();
        assert_eq!(ids, ["usb", "big"]);

        let busy = dir.path().join("busy");
        fs::create_dir_all(&busy).unwrap();
        fs::write(busy.join("file"), b"x").unwrap();
        assert!(matches!(
            add(&settings, &busy, Some("busy")),
            Err(AppError::Storage(StorageError::NotEmpty { .. }))
        ));
        let file = dir.path().join("plain-file");
        fs::write(&file, b"x").unwrap();
        assert!(matches!(
            add(&settings, &file, Some("file")),
            Err(AppError::Storage(StorageError::NotEmpty { .. }))
        ));
        for id in ["default", "usb", "Upper", "a..b"] {
            assert!(
                matches!(
                    add(&settings, &dir.path().join("fresh"), Some(id)),
                    Err(AppError::Storage(StorageError::InvalidId { .. }))
                ),
                "{id}"
            );
        }
        assert!(matches!(
            add(&settings, &dir.path().join("home/opt"), Some("opt")),
            Err(AppError::Storage(StorageError::AlreadyListed { .. }))
        ));
        assert!(matches!(
            add(&settings, Path::new("relative"), Some("rel")),
            Err(AppError::Storage(StorageError::RelativePath { .. }))
        ));
        assert!(
            !dir.path().join("fresh").exists(),
            "refused before creation"
        );
    }

    #[test]
    fn attach_and_detach_edit_only_the_rows() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(&dir.path().join("home"));
        let usb = dir.path().join("usb");
        add(&settings, &usb, Some("usb")).unwrap();
        assert!(matches!(
            attach(&settings, &usb),
            Err(AppError::Storage(StorageError::InvalidId { .. }))
        ));
        let removed = detach(&settings, "usb").unwrap();
        assert_eq!(removed.path, usb);
        assert!(usb.join("apps").is_dir(), "detach leaves files alone");
        assert!(
            Storage::load(&settings)
                .unwrap()
                .config()
                .libraries
                .is_empty()
        );
        assert!(matches!(
            detach(&settings, "usb"),
            Err(AppError::State(StateError::UnknownLibrary { .. }))
        ));
        assert!(matches!(
            detach(&settings, DEFAULT_LIBRARY),
            Err(AppError::Storage(StorageError::DetachDefault))
        ));

        assert_eq!(attach(&settings, &usb).unwrap().id, "usb");
        let storage = Storage::load(&settings).unwrap();
        assert_eq!(storage.availability("usb"), Availability::Available);

        let plain = dir.path().join("plain");
        fs::create_dir_all(&plain).unwrap();
        assert!(matches!(
            attach(&settings, &plain),
            Err(AppError::Storage(StorageError::NotALibrary { .. }))
        ));
        let foreign_default = dir.path().join("other-home-opt");
        create_library(&foreign_default, DEFAULT_LIBRARY).unwrap();
        assert!(matches!(
            attach(&settings, &foreign_default),
            Err(AppError::Storage(StorageError::InvalidId { .. }))
        ));
    }

    #[test]
    fn listing_shows_configured_then_detached_libraries() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(&dir.path().join("home"));
        let usb = dir.path().join("usb");
        add(&settings, &usb, None).unwrap();
        let mut index = Index::default();
        let record = crate::storage::relocate::tests::owned_record("usb", "apps/tool/a");
        index
            .installations
            .insert("org.example/a@1.0.0".into(), record);
        index
            .save(
                &settings,
                &crate::catalog::HomeEvent::new(crate::catalog::EventKind::Updated),
            )
            .unwrap();

        let listed = listing(&settings).unwrap();
        let ids: Vec<&str> = listed.libraries.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["default", "usb"]);
        let usb_row = &listed.libraries[1];
        assert_eq!(usb_row.status, wire::Status::Ready);
        assert_eq!(usb_row.installations.len(), 1);
        assert_eq!(usb_row.installations[0].status, wire::Status::Unavailable);
        assert!(
            usb_row.installations[0]
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("missing"))
        );

        detach(&settings, "usb").unwrap();
        let listed = listing(&settings).unwrap();
        let detached = &listed.libraries[1];
        assert_eq!(detached.id, "usb");
        assert_eq!(detached.path, None);
        assert_eq!(detached.status, wire::Status::Unavailable);
        assert_eq!(detached.installations.len(), 1);
    }
}
