//! The commands that edit the user layer (`PROP-059#launcher-catalog`):
//! `vibe app collection create|rename|delete|add|remove|order|list`,
//! `vibe app pin|unpin` and `vibe app launcher`. Every one of them takes the
//! home's mutation lock, refuses a key that names no launcher of the home,
//! and answers the whole layer as `vibe-app-collections/1` so a client sees
//! what its edit did to the rest of the arrangement.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog");

use std::path::Path;

use specmark::spec;
use vibe_wire::generated::application_collections as wire;

use super::events::{EventKind, HomeEvent};
use super::user::{UserCollection, UserLauncher, UserLaunchers};
use crate::error::AppError;
use crate::error_catalog::CatalogError;
use crate::index::Index;
use crate::settings::SettingsRoot;

pub use super::user::PinTarget;

/// What `vibe app launcher KEY` changes about one launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub enum LauncherChange {
    /// `--hide`: keep it out of a client's menus; it stays in the catalog
    /// marked `hidden_by: "user"`.
    Hide,
    /// `--show`: undo a hide.
    Show,
    /// `--name "…"`: a name of the user's own, surviving update.
    Name(String),
    /// `--icon PATH`: an icon file of the user's own, surviving update.
    Icon(String),
    /// `--reset`: forget the whole override row.
    Reset,
}

/// `vibe app collection list`: the layer as it stands, changing nothing.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let layer = vibe_app::collections(&settings).unwrap();
/// assert_eq!(layer.protocol, "vibe-app-collections/1");
/// assert!(layer.collections.is_empty() && layer.pins.taskbar.is_empty());
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn collections(settings: &SettingsRoot) -> Result<wire::ApplicationCollections, AppError> {
    Ok(UserLaunchers::load(settings)?.to_wire(settings))
}

/// One edit of the layer: take the lock, read the index for key checks, let
/// `change` edit the layer, save it and answer the whole layer.
fn edit(
    settings: &SettingsRoot,
    change: impl FnOnce(&mut UserLaunchers, &Index) -> Result<(), AppError>,
) -> Result<wire::ApplicationCollections, AppError> {
    edit_as(
        settings,
        &HomeEvent::new(EventKind::CollectionsChanged),
        change,
    )
}

/// [`edit`] recording another event than a change of the arrangement.
fn edit_as(
    settings: &SettingsRoot,
    event: &HomeEvent,
    change: impl FnOnce(&mut UserLaunchers, &Index) -> Result<(), AppError>,
) -> Result<wire::ApplicationCollections, AppError> {
    let _lock = settings.lock()?;
    let index = Index::load(settings)?;
    let mut layer = UserLaunchers::load(settings)?;
    change(&mut layer, &index)?;
    layer.save(settings, event)?;
    Ok(layer.to_wire(settings))
}

/// Refuse a key that names no launcher of this home.
fn known(index: &Index, settings: &SettingsRoot, keys: &[String]) -> Result<(), AppError> {
    for key in keys {
        if !super::names_a_launcher(index, key) {
            return Err(CatalogError::LauncherMissing {
                key: key.clone(),
                home: settings.home_name(),
            }
            .into());
        }
    }
    Ok(())
}

fn missing(name: &str, settings: &SettingsRoot) -> AppError {
    CatalogError::CollectionMissing {
        name: name.to_owned(),
        home: settings.home_name(),
    }
    .into()
}

/// `vibe app collection create NAME [--icon PATH]`.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn collection_create(
    settings: &SettingsRoot,
    name: &str,
    icon: Option<&Path>,
) -> Result<wire::ApplicationCollections, AppError> {
    edit(settings, |layer, _| {
        if layer.collection(name).is_some() {
            return Err(CatalogError::CollectionExists {
                name: name.to_owned(),
            }
            .into());
        }
        layer.collections.push(UserCollection {
            name: name.to_owned(),
            icon: icon.map(|path| path.display().to_string()),
            ..Default::default()
        });
        Ok(())
    })
}

/// `vibe app collection rename OLD NEW`. The renamed collection keeps its
/// place, its keys and its removals, and stops being a seeded one: the user
/// has taken it over, so a later install of the same `menu_group` seeds a
/// fresh collection rather than filling this one.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn collection_rename(
    settings: &SettingsRoot,
    old: &str,
    new: &str,
) -> Result<wire::ApplicationCollections, AppError> {
    edit(settings, |layer, _| {
        if layer.collection(new).is_some() {
            return Err(CatalogError::CollectionExists {
                name: new.to_owned(),
            }
            .into());
        }
        let collection = layer
            .collection_mut(old)
            .ok_or_else(|| missing(old, settings))?;
        collection.name = new.to_owned();
        collection.seeded = false;
        Ok(())
    })
}

/// `vibe app collection delete NAME`.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn collection_delete(
    settings: &SettingsRoot,
    name: &str,
) -> Result<wire::ApplicationCollections, AppError> {
    edit(settings, |layer, _| {
        if layer.collection(name).is_none() {
            return Err(missing(name, settings));
        }
        layer
            .collections
            .retain(|collection| collection.name != name);
        Ok(())
    })
}

/// `vibe app collection add NAME KEY…`: append the keys it does not hold
/// yet, in the order given, and forget that the user had removed them.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn collection_add(
    settings: &SettingsRoot,
    name: &str,
    keys: &[String],
) -> Result<wire::ApplicationCollections, AppError> {
    edit(settings, |layer, index| {
        known(index, settings, keys)?;
        let collection = layer
            .collection_mut(name)
            .ok_or_else(|| missing(name, settings))?;
        collection.seeded = false;
        for key in keys {
            collection.removed.retain(|removed| removed != key);
            if !collection.launchers.contains(key) {
                collection.launchers.push(key.clone());
            }
        }
        Ok(())
    })
}

/// `vibe app collection remove NAME KEY…`: take the keys out and record them
/// as removed, so seeding never puts them back.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn collection_remove(
    settings: &SettingsRoot,
    name: &str,
    keys: &[String],
) -> Result<wire::ApplicationCollections, AppError> {
    edit(settings, |layer, index| {
        known(index, settings, keys)?;
        let collection = layer
            .collection_mut(name)
            .ok_or_else(|| missing(name, settings))?;
        collection.seeded = false;
        collection.launchers.retain(|held| !keys.contains(held));
        for key in keys {
            if !collection.removed.contains(key) {
                collection.removed.push(key.clone());
            }
        }
        Ok(())
    })
}

/// `vibe app collection order NAME KEY…`: the full order. The collection
/// becomes exactly the given sequence; a key it held and the sequence omits
/// counts as removed, so seeding does not reinstate it.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn collection_order(
    settings: &SettingsRoot,
    name: &str,
    keys: &[String],
) -> Result<wire::ApplicationCollections, AppError> {
    edit(settings, |layer, index| {
        known(index, settings, keys)?;
        let collection = layer
            .collection_mut(name)
            .ok_or_else(|| missing(name, settings))?;
        collection.seeded = false;
        let dropped: Vec<String> = collection
            .launchers
            .iter()
            .filter(|held| !keys.contains(held))
            .cloned()
            .collect();
        collection.launchers = keys.to_vec();
        collection.launchers.dedup();
        for key in dropped {
            if !collection.removed.contains(&key) {
                collection.removed.push(key);
            }
        }
        collection.removed.retain(|key| !keys.contains(key));
        Ok(())
    })
}

/// `vibe app pin KEY [--to taskbar|favorites]`: pinned lists keep order, and
/// a repeated pin moves the key to the end.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn pin(
    settings: &SettingsRoot,
    key: &str,
    target: PinTarget,
) -> Result<wire::ApplicationCollections, AppError> {
    edit(settings, |layer, index| {
        known(index, settings, &[key.to_owned()])?;
        let list = layer.pin_list_mut(target);
        list.retain(|pinned| pinned != key);
        list.push(key.to_owned());
        Ok(())
    })
}

/// `vibe app unpin KEY [--from taskbar|favorites]`.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn unpin(
    settings: &SettingsRoot,
    key: &str,
    target: PinTarget,
) -> Result<wire::ApplicationCollections, AppError> {
    edit(settings, |layer, index| {
        known(index, settings, &[key.to_owned()])?;
        layer.pin_list_mut(target).retain(|pinned| pinned != key);
        Ok(())
    })
}

/// `vibe app launcher KEY --hide | --show | --name "…" | --icon PATH |
/// --reset`: one override row per key, dropped again when it says nothing.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn launcher_override(
    settings: &SettingsRoot,
    key: &str,
    change: LauncherChange,
) -> Result<wire::ApplicationCollections, AppError> {
    edit_as(
        settings,
        &HomeEvent::about(EventKind::LaunchersChanged, key),
        |layer, index| {
            known(index, settings, &[key.to_owned()])?;
            if change == LauncherChange::Reset {
                layer.launchers.retain(|row| row.key != key);
                return Ok(());
            }
            let position = match layer.launchers.iter().position(|row| row.key == key) {
                Some(position) => position,
                None => {
                    layer.launchers.push(UserLauncher {
                        key: key.to_owned(),
                        ..Default::default()
                    });
                    layer.launchers.len() - 1
                }
            };
            let row = &mut layer.launchers[position];
            match change {
                LauncherChange::Hide => row.hidden = true,
                LauncherChange::Show => row.hidden = false,
                LauncherChange::Name(name) => row.name = Some(name),
                LauncherChange::Icon(path) => row.icon = Some(path),
                LauncherChange::Reset => unreachable!("handled above"),
            }
            if row.is_empty() {
                layer.launchers.retain(|row| row.key != key);
            }
            Ok(())
        },
    )
}
