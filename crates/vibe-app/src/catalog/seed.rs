//! Seeding and unseeding the user layer from the inventory
//! (`PROP-059#launcher-catalog`). An install, modify or update seeds the
//! collection a package's `menu_group` names; an uninstall drops that
//! package's keys from every collection, both pinned lists, the overrides
//! and the usage facts.
//!
//! Seeding is deliberately timid: it creates a collection it does not find,
//! appends the menu launchers it does not see, and nothing else. It never
//! reorders what the user arranged and never re-adds a key the user took
//! out — that is what each collection's `removed` list is for.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog");

use specmark::spec;

use super::usage::Usage;
use super::user::{UserCollection, UserLaunchers};
use crate::error::StateError;
use crate::index::Installation;
use crate::settings::SettingsRoot;

/// Seed the collection of `installation`'s `menu_group` with its
/// `menu = true` launchers. A package without a `menu_group` seeds nothing:
/// each of its entries stands alone (`PROP-059#menu`). The caller holds the
/// home's mutation lock.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn seed_package(
    settings: &SettingsRoot,
    installation: &Installation,
) -> Result<(), StateError> {
    let Some(group) = installation.menu.menu_group.clone() else {
        return Ok(());
    };
    let package = installation.package.key();
    let wanted: Vec<String> = installation
        .launchers
        .iter()
        .filter(|launcher| launcher.shows_in_menu())
        .map(|launcher| super::launcher_key(&package, &launcher.id))
        .collect();
    let mut layer = UserLaunchers::load(settings)?;
    let mut changed = false;
    let position = match layer
        .collections
        .iter()
        .position(|collection| collection.name == group)
    {
        Some(position) => position,
        None => {
            layer.collections.push(UserCollection {
                name: group.clone(),
                seeded: true,
                ..Default::default()
            });
            changed = true;
            layer.collections.len() - 1
        }
    };
    let collection = &mut layer.collections[position];
    for key in wanted {
        if collection.launchers.contains(&key) || collection.removed.contains(&key) {
            continue;
        }
        collection.launchers.push(key);
        changed = true;
    }
    if changed {
        layer.save(settings)?;
    }
    Ok(())
}

/// Drop every launcher key of `installation` from the user layer and the
/// usage facts, and drop a seeded collection the removal left empty and the
/// user never touched. The caller holds the home's mutation lock.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn drop_package(
    settings: &SettingsRoot,
    installation: &Installation,
) -> Result<(), StateError> {
    let package = installation.package.key();
    let keys: Vec<String> = installation
        .launchers
        .iter()
        .map(|launcher| super::launcher_key(&package, &launcher.id))
        .collect();
    if keys.is_empty() {
        return Ok(());
    }
    let mut layer = UserLaunchers::load(settings)?;
    let before = layer.clone();
    layer.forget(&keys);
    layer
        .collections
        .retain(|collection| !untouched_and_empty(collection));
    if layer != before {
        layer.save(settings)?;
    }
    let mut usage = Usage::load(settings)?;
    let counts = usage.launchers.len();
    usage.forget(&keys);
    if usage.launchers.len() != counts {
        usage.save(settings)?;
    }
    Ok(())
}

/// A collection seeding created, holding nothing, that the user never
/// edited: `seeded` stays true only until the first `collection` command
/// touches it, so nothing of the user's is lost by dropping it.
fn untouched_and_empty(collection: &UserCollection) -> bool {
    collection.seeded && collection.launchers.is_empty()
}
