//! The launcher arrangement inside the portable set (`PROP-059#portable-set`):
//! the collections, pinned lists and overrides of the user layer, exported
//! with the words `launchers.toml` uses and merged back after an import's
//! installations, so a machine that already has an arrangement keeps it and
//! gains what the set adds.

use vibe_wire::generated::apps_manifest as wire;

use crate::catalog::{
    EventKind, HomeEvent, PinTarget, UserCollection, UserLauncher, UserLaunchers, names_a_launcher,
};
use crate::error::AppError;
use crate::index::Index;
use crate::settings::SettingsRoot;

/// Copy the user layer into `set`. Seeded collections travel too: the
/// importing machine merges them by name with what its own seeding made.
pub(super) fn export_into(layer: &UserLaunchers, set: &mut wire::AppsManifest) {
    set.collection = layer
        .collections
        .iter()
        .map(|collection| wire::Collection {
            name: collection.name.clone(),
            launchers: collection.launchers.clone(),
        })
        .collect();
    set.pin = [
        (wire::PinTarget::Taskbar, &layer.pins.taskbar),
        (wire::PinTarget::Favorites, &layer.pins.favorites),
    ]
    .into_iter()
    .filter(|(_, launchers)| !launchers.is_empty())
    .map(|(target, launchers)| wire::Pin {
        target,
        launchers: launchers.clone(),
    })
    .collect();
    set.launcher = layer
        .launchers
        .iter()
        .filter(|row| !row.is_empty())
        .map(|row| wire::Launcher {
            key: row.key.clone(),
            name: row.name.clone(),
            hidden: row.hidden,
            icon: row.icon.clone(),
        })
        .collect();
}

/// Merge the arrangement of `set` into the home's user layer under its
/// mutation lock, after the installations: a collection is created or
/// extended by name, a pinned list gains the keys it lacks in the set's
/// order, an override sets the fields the set carries. A key naming no
/// launcher of this home (its package was refused or skipped) is left out;
/// the answer lists each such key once with the reason.
///
/// ```
/// use vibe_wire::generated::apps_manifest::{AppsManifest, AppsManifestHome, Collection};
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let set = AppsManifest {
///     protocol: "vibe-apps/1".into(),
///     exported: "2026-11-01T12:00:00Z".into(),
///     home: AppsManifestHome::User,
///     platform: "linux".into(),
///     arch: "x86_64".into(),
///     application: vec![],
///     collection: vec![Collection {
///         name: "Writing".into(),
///         launchers: vec!["org.example/vim@9.1.0#vim".into()],
///     }],
///     pin: vec![],
///     launcher: vec![],
/// };
/// let skipped = vibe_app::restore_arrangement(&settings, &set).unwrap();
/// assert_eq!(skipped[0].0, "org.example/vim@9.1.0#vim");
/// let layer = vibe_app::UserLaunchers::load(&settings).unwrap();
/// assert!(layer.collection("Writing").unwrap().launchers.is_empty());
/// ```
pub fn restore_arrangement(
    settings: &SettingsRoot,
    set: &wire::AppsManifest,
) -> Result<Vec<(String, String)>, AppError> {
    if set.collection.is_empty() && set.pin.is_empty() && set.launcher.is_empty() {
        return Ok(Vec::new());
    }
    let _lock = settings.lock()?;
    let index = Index::load(settings)?;
    let mut layer = UserLaunchers::load(settings)?;
    let mut skipped: Vec<(String, String)> = Vec::new();
    let mut usable = |key: &String| {
        if names_a_launcher(&index, key) {
            return true;
        }
        if !skipped.iter().any(|(seen, _)| seen == key) {
            skipped.push((
                key.clone(),
                "names no launcher of this home; its package is not installed".into(),
            ));
        }
        false
    };
    for collection in &set.collection {
        let keys: Vec<&String> = collection
            .launchers
            .iter()
            .filter(|key| usable(key))
            .collect();
        let position = match layer
            .collections
            .iter()
            .position(|held| held.name == collection.name)
        {
            Some(position) => position,
            None => {
                layer.collections.push(UserCollection {
                    name: collection.name.clone(),
                    ..Default::default()
                });
                layer.collections.len() - 1
            }
        };
        let held = &mut layer.collections[position];
        for key in keys {
            held.removed.retain(|removed| removed != key);
            if !held.launchers.contains(key) {
                held.launchers.push(key.clone());
            }
        }
    }
    for pin in &set.pin {
        let target = match pin.target {
            wire::PinTarget::Taskbar => PinTarget::Taskbar,
            wire::PinTarget::Favorites => PinTarget::Favorites,
        };
        let keys: Vec<&String> = pin.launchers.iter().filter(|key| usable(key)).collect();
        let list = layer.pin_list_mut(target);
        for key in keys {
            if !list.contains(key) {
                list.push(key.clone());
            }
        }
    }
    for row in &set.launcher {
        if !usable(&row.key) {
            continue;
        }
        let position = match layer.launchers.iter().position(|held| held.key == row.key) {
            Some(position) => position,
            None => {
                layer.launchers.push(UserLauncher {
                    key: row.key.clone(),
                    ..Default::default()
                });
                layer.launchers.len() - 1
            }
        };
        let held = &mut layer.launchers[position];
        held.hidden |= row.hidden;
        if row.name.is_some() {
            held.name.clone_from(&row.name);
        }
        if row.icon.is_some() {
            held.icon.clone_from(&row.icon);
        }
    }
    layer.launchers.retain(|row| !row.is_empty());
    layer.save(settings, &HomeEvent::new(EventKind::CollectionsChanged))?;
    Ok(skipped)
}
