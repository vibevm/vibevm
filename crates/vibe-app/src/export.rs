//! The system projection of the launcher catalog
//! (`PROP-059#launcher-runtime`): `vibe app launchers --export-system`
//! writes the platform's own menu entries for the launchers a client would
//! show — `menu = true` and not hidden by the user — and
//! `<home>/applications/exported.json` records every path it wrote, so a
//! later export rewrites and prunes, and `--remove-system` or an uninstall
//! removes exactly that. The projection is derived and disposable: nothing
//! is ever read back from it.
//!
//! The entries start a launcher through `vibe app start
//! <group>/<name>#<launcher id>`, a key without the version, so a shortcut
//! survives an update; `X-Vibe-Key` / `VibeKey` keep the exact key it was
//! written for.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime");

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_wire::generated::application_exported as wire;
use vibe_wire::generated::application_launchers_all as all_wire;

#[path = "export/entries.rs"]
mod entries;
#[path = "export/plan.rs"]
mod plan;

use crate::catalog::LauncherCatalog;
use crate::error::{AppError, StateError};
use crate::error_catalog::CatalogError;
use crate::index::{Index, Installation};
use crate::settings::{SettingsRoot, io, write_atomic};
use crate::storage::Storage;

pub use wire::{ApplicationExported, SystemPlatform};

use plan::{Body, Planned};

/// The protocol tag of the export answers and of `exported.json`.
pub const EXPORTED_PROTOCOL: &str = "vibe-app-exported/1";

/// Where the platform's entries go and how they start a launcher. The CLI
/// fills it from the environment, so this crate reads none.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub struct SystemTargets {
    pub platform: SystemPlatform,
    /// Linux: the XDG data home (`~/.local/share`); Windows: the Start
    /// Menu's `Programs` folder; macOS: `~/Applications`.
    pub root: PathBuf,
    /// The command line that starts one launcher through vibe; the launcher
    /// spelling is appended as its last word.
    pub start: Vec<String>,
}

impl SystemTargets {
    /// The platform this build runs on.
    pub fn host_platform() -> SystemPlatform {
        if cfg!(windows) {
            SystemPlatform::Windows
        } else if cfg!(target_os = "macos") {
            SystemPlatform::Macos
        } else {
            SystemPlatform::Linux
        }
    }
}

/// One `--export-system` call.
#[derive(Debug, Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub struct ExportRequest<'a> {
    pub settings: &'a SettingsRoot,
    pub targets: &'a SystemTargets,
    /// Only the launchers of this collection.
    pub collection: Option<&'a str>,
    pub project_root: Option<&'a Path>,
    /// Answer what would be written and removed, changing nothing.
    pub dry_run: bool,
}

/// Read the record of a home; an absent or damaged one records nothing, and
/// then nothing is pruned, which only ever leaves files behind.
fn load(settings: &SettingsRoot) -> Vec<wire::Entry> {
    fs::read(settings.exported_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ApplicationExported>(&bytes).ok())
        .map(|record| record.entries)
        .unwrap_or_default()
}

fn save(
    settings: &SettingsRoot,
    platform: SystemPlatform,
    entries: &[wire::Entry],
) -> Result<(), StateError> {
    let path = settings.exported_path();
    if entries.is_empty() {
        return match fs::remove_file(&path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(io("removing", &path, error))
            }
            _ => Ok(()),
        };
    }
    let record = document(settings, platform, false, entries.to_vec(), Vec::new());
    let mut bytes = serde_json::to_vec_pretty(&record).map_err(|error| StateError::Io {
        action: "encoding",
        path: path.display().to_string(),
        source: std::io::Error::other(error),
    })?;
    bytes.push(b'\n');
    write_atomic(&path, &bytes)
}

fn document(
    settings: &SettingsRoot,
    platform: SystemPlatform,
    dry_run: bool,
    entries: Vec<wire::Entry>,
    removed: Vec<String>,
) -> ApplicationExported {
    ApplicationExported {
        protocol: EXPORTED_PROTOCOL.into(),
        home: settings.path().display().to_string(),
        platform,
        dry_run,
        entries,
        removed,
    }
}

/// `vibe app launchers --export-system`: write the entries of every
/// launcher a client would show, prune what an earlier export wrote and
/// this one does not, and record the result.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub fn export_system(request: &ExportRequest<'_>) -> Result<ApplicationExported, AppError> {
    let settings = request.settings;
    let _lock = if request.dry_run {
        None
    } else {
        Some(settings.lock()?)
    };
    let catalog = LauncherCatalog::load(settings)?;
    if let Some(name) = request.collection
        && catalog.layer().collection(name).is_none()
    {
        return Err(CatalogError::CollectionMissing {
            name: name.to_owned(),
            home: settings.home_name(),
        }
        .into());
    }
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let listing =
        crate::launch::listing_all(&index, &storage, &catalog, request.project_root, true);
    let planned = plan::plan(&listing, request);
    let entries: Vec<wire::Entry> = planned.iter().map(Planned::entry).collect();
    let written: BTreeSet<&String> = entries.iter().flat_map(|entry| &entry.paths).collect();
    let removed: Vec<String> = load(settings)
        .iter()
        .flat_map(|entry| entry.paths.clone())
        .filter(|path| !written.contains(path))
        .collect();
    let platform = request.targets.platform.clone();
    if request.dry_run {
        return Ok(document(settings, platform, true, entries, removed));
    }
    for entry in &planned {
        write(entry, request.targets)?;
    }
    for path in &removed {
        remove(Path::new(path), Some(request.targets));
    }
    save(settings, platform.clone(), &entries)?;
    Ok(document(settings, platform, false, entries, removed))
}

/// `vibe app launchers --remove-system`: remove everything the record names
/// and the record itself.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub fn remove_system(
    settings: &SettingsRoot,
    targets: &SystemTargets,
    dry_run: bool,
) -> Result<ApplicationExported, AppError> {
    let _lock = if dry_run {
        None
    } else {
        Some(settings.lock()?)
    };
    let removed: Vec<String> = load(settings)
        .into_iter()
        .flat_map(|entry| entry.paths)
        .collect();
    if !dry_run {
        for path in &removed {
            remove(Path::new(path), Some(targets));
        }
        save(settings, targets.platform.clone(), &[])?;
    }
    Ok(document(
        settings,
        targets.platform.clone(),
        dry_run,
        Vec::new(),
        removed,
    ))
}

/// Remove the system entries of an uninstalled installation's launchers.
/// The caller holds the home's mutation lock.
pub(crate) fn forget_installation(
    settings: &SettingsRoot,
    installation: &Installation,
) -> Result<(), StateError> {
    let package = installation.package.key();
    let keys: Vec<String> = installation
        .launchers
        .iter()
        .map(|launcher| crate::catalog::launcher_key(&package, &launcher.id))
        .collect();
    rekey(
        settings,
        &keys.into_iter().map(|key| (key, None)).collect::<Vec<_>>(),
    )
}

/// Follow launcher keys that changed (an update or a rollback): a key with
/// a successor keeps its files under the new key, so uninstalling the old
/// version no longer touches them; a key without one loses its files,
/// whose launcher no longer exists. The caller holds the mutation lock.
pub(crate) fn rekey(
    settings: &SettingsRoot,
    rewrite: &[(String, Option<String>)],
) -> Result<(), StateError> {
    let mut entries = load(settings);
    let before = entries.clone();
    let platform = fs::read(settings.exported_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ApplicationExported>(&bytes).ok())
        .map_or_else(SystemTargets::host_platform, |record| record.platform);
    entries.retain_mut(
        |entry| match rewrite.iter().find(|(old, _)| *old == entry.key) {
            Some((_, Some(new))) => {
                entry.key = new.clone();
                true
            }
            Some((_, None)) => {
                for path in &entry.paths {
                    remove(Path::new(path), None);
                }
                false
            }
            None => true,
        },
    );
    if entries != before {
        save(settings, platform, &entries)?;
    }
    Ok(())
}

/// Write one planned entry's files.
fn write(entry: &Planned, targets: &SystemTargets) -> Result<(), AppError> {
    if entry.bundle {
        // A wrapper is rewritten whole: what an earlier export put in it
        // and this one does not would otherwise stay.
        remove(&entry.record, Some(targets));
    }
    for (path, body) in &entry.files {
        let parent = path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|source| io("creating", parent, source))?;
        match body {
            Body::Text { text, executable } => {
                write_atomic(path, text.as_bytes())?;
                if *executable {
                    make_executable(path)?;
                }
            }
            Body::Copy(source) => {
                fs::copy(source, path).map_err(|error| io("copying", source, error))?;
            }
            Body::Shortcut(script) => plan::save_shortcut(path, script)?,
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), StateError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .map_err(|source| io("marking executable", path, source))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), StateError> {
    Ok(())
}

/// Remove one recorded path when it is still vibe's: a `.desktop` file must
/// carry `X-Vibe-Key`, a directory must be an `.app` wrapper whose
/// `Info.plist` carries `VibeKey`; anything else recorded (icon copies, a
/// shortcut) is named after its launcher by vibe alone. A Windows group
/// folder the removal emptied goes too.
fn remove(path: &Path, targets: Option<&SystemTargets>) {
    let ours = if path.is_dir() {
        path.extension().is_some_and(|ext| ext == "app")
            && fs::read_to_string(path.join("Contents/Info.plist"))
                .is_ok_and(|plist| plist.contains("<key>VibeKey</key>"))
    } else if path.extension().is_some_and(|ext| ext == "desktop") {
        fs::read_to_string(path).is_ok_and(|text| text.contains("\nX-Vibe-Key="))
    } else {
        true
    };
    if !ours {
        return;
    }
    let _ = if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    if let Some(targets) = targets
        && targets.platform == SystemPlatform::Windows
        && let Some(parent) = path.parent()
        && parent != targets.root
        && parent.starts_with(&targets.root)
    {
        // Fails, as wanted, while the folder holds anything else.
        let _ = fs::remove_dir(parent);
    }
}

/// The launcher rows of one listing an export projects.
pub(crate) fn exported_rows<'a>(
    listing: &'a all_wire::ApplicationLaunchersAll,
    collection: Option<&'a str>,
) -> impl Iterator<Item = (&'a all_wire::Installation, &'a all_wire::Launcher)> + 'a {
    listing
        .installations
        .iter()
        .flat_map(|installation| {
            installation
                .launchers
                .iter()
                .map(move |launcher| (installation, launcher))
        })
        .filter(|(_, launcher)| launcher.menu && launcher.hidden_by.is_none())
        .filter(move |(_, launcher)| {
            collection.is_none_or(|name| launcher.collections.iter().any(|held| held == name))
        })
}
