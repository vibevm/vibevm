//! Exposure, marks, holds and rollback of an installed record: `vibe app
//! expose | hide` (`PROP-059#hidden-installations`), `vibe app mark`
//! (`PROP-065#uninstall-tree`), `vibe app hold | unhold | rollback`
//! (`PROP-065#update-and-rollback`). Each edits the index once under the
//! lock and synchronises the proxies; no installer runs.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#hidden-installations");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#update-and-rollback");

use std::path::Path;

use specmark::spec;

use crate::alternatives::{check_reservation, reserved_names};
use crate::error::AppError;
use crate::error_lifecycle::LifecycleError;
use crate::index::{Index, Mark};
use crate::proxies;
use crate::settings::SettingsRoot;

use super::listing::installed;
use super::normalize_slots;

/// Which state change a [`Change`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#hidden-installations")]
pub enum ChangeVerb {
    Expose,
    Hide,
    Mark,
    Hold,
    Unhold,
    Rollback,
}

/// The result of one state change, for the caller to render.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#hidden-installations")]
pub struct Change {
    pub verb: ChangeVerb,
    pub slot: String,
    pub key: String,
    pub slot_default: Option<String>,
    pub message: String,
}

/// Load, edit and save the index under the lock, then synchronise the
/// proxies with the new reserved command set.
fn edit(
    settings: &SettingsRoot,
    proxy_image: &Path,
    spelling: &str,
    verb: ChangeVerb,
    apply: impl FnOnce(&mut Index, &str) -> Result<String, AppError>,
) -> Result<Change, AppError> {
    edit_then(settings, proxy_image, spelling, verb, apply, |_, _| Ok(()))
}

/// [`edit`] with `after` run on the saved index, still under the lock.
fn edit_then(
    settings: &SettingsRoot,
    proxy_image: &Path,
    spelling: &str,
    verb: ChangeVerb,
    apply: impl FnOnce(&mut Index, &str) -> Result<String, AppError>,
    after: impl FnOnce(&Index, &str) -> Result<(), AppError>,
) -> Result<Change, AppError> {
    let _lock = settings.lock()?;
    let mut index = Index::load(settings)?;
    let (key, record) = installed(settings, &index, spelling)?;
    let (key, slot) = (key.to_owned(), record.slot.clone());
    let reserved = reserved_names(&index);
    let message = apply(&mut index, &key)?;
    normalize_slots(&mut index);
    index.save(
        settings,
        &crate::catalog::HomeEvent::about(crate::catalog::EventKind::Updated, key.clone()),
    )?;
    after(&index, &key)?;
    proxies::sync(settings, &reserved, &reserved_names(&index), proxy_image)?;
    Ok(Change {
        verb,
        slot_default: index.default_of(&slot).map(str::to_owned),
        slot,
        key,
        message,
    })
}

/// The first exposed provider of `slot` other than `except`, by key.
fn next_default(index: &Index, slot: &str, except: &str) -> Option<String> {
    index
        .installations
        .iter()
        .find(|(key, record)| record.slot == slot && *key != except && !record.lifecycle.hidden)
        .map(|(key, _)| key.clone())
}

/// Turn an installation into an exposed one under the ordinary slot rules:
/// it gains a place among the alternatives and becomes the default only of
/// a slot without one; it is marked `manual`.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let image = dir.path().join("vibe-app-proxy");
/// let error = vibe_app::expose(&settings, "org.example/gcc", &image).unwrap_err();
/// assert_eq!(error.code(), "not_installed");
/// ```
pub fn expose(
    settings: &SettingsRoot,
    spelling: &str,
    proxy_image: &Path,
) -> Result<Change, AppError> {
    edit(
        settings,
        proxy_image,
        spelling,
        ChangeVerb::Expose,
        |index, key| {
            let record = index.installations[key].clone();
            check_reservation(index, &[key], &record.slot, &record.public_commands)?;
            let was_hidden = record.lifecycle.hidden;
            if let Some(entry) = index.installations.get_mut(key) {
                entry.lifecycle.hidden = false;
                entry.lifecycle.mark = Mark::Manual;
                entry.lifecycle.superseded_by = None;
            }
            if index.default_of(&record.slot).is_none() {
                index
                    .slots
                    .insert(record.slot.clone(), Some(key.to_owned()));
            }
            Ok(if was_hidden {
                format!("exposed {key} in slot `{}`", record.slot)
            } else {
                format!("{key} was already exposed; marked manual")
            })
        },
    )
}

/// Hide an installation: its proxies and its place among the alternatives
/// go, and the default passes to the next exposed provider when it held it.
pub fn hide(
    settings: &SettingsRoot,
    spelling: &str,
    proxy_image: &Path,
) -> Result<Change, AppError> {
    edit(
        settings,
        proxy_image,
        spelling,
        ChangeVerb::Hide,
        |index, key| {
            let slot = index.installations[key].slot.clone();
            if let Some(entry) = index.installations.get_mut(key) {
                entry.lifecycle.hidden = true;
            }
            if index.default_of(&slot) == Some(key) {
                let next = next_default(index, &slot, key);
                index.slots.insert(slot.clone(), next);
            }
            Ok(format!(
                "hid {key}; it stays installed and reachable with `vibe shell --with`"
            ))
        },
    )
}

/// Set the manual/auto mark (`vibe app mark manual|auto`).
pub fn mark(
    settings: &SettingsRoot,
    spelling: &str,
    mark: Mark,
    proxy_image: &Path,
) -> Result<Change, AppError> {
    edit(
        settings,
        proxy_image,
        spelling,
        ChangeVerb::Mark,
        |index, key| {
            if let Some(entry) = index.installations.get_mut(key) {
                entry.lifecycle.mark = mark;
            }
            Ok(format!("marked {key} {}", mark.as_str()))
        },
    )
}

/// Set or clear the `held` flag (`vibe app hold | unhold`).
pub fn hold(
    settings: &SettingsRoot,
    spelling: &str,
    held: bool,
    proxy_image: &Path,
) -> Result<Change, AppError> {
    let verb = if held {
        ChangeVerb::Hold
    } else {
        ChangeVerb::Unhold
    };
    edit(settings, proxy_image, spelling, verb, |index, key| {
        if let Some(entry) = index.installations.get_mut(key) {
            entry.lifecycle.held = held;
        }
        Ok(if held {
            format!("held {key}; a bulk update skips it")
        } else {
            format!("released the hold on {key}")
        })
    })
}

/// Expose the kept previous installation of the package, make it the slot
/// default again and hide the newer one, which becomes the kept one.
pub fn rollback(
    settings: &SettingsRoot,
    spelling: &str,
    proxy_image: &Path,
) -> Result<Change, AppError> {
    edit_then(
        settings,
        proxy_image,
        spelling,
        ChangeVerb::Rollback,
        |index, key| {
            let current = index.installations[key].clone();
            let previous = index
                .installations
                .iter()
                .filter(|(_, record)| {
                    record.slot == current.slot
                        && record.package.same_package(&current.package)
                        && record.lifecycle.superseded_by.as_deref() == Some(key)
                })
                .map(|(other, _)| other.clone())
                .next()
                .ok_or_else(|| LifecycleError::NoPrevious { key: key.into() })?;
            if let Some(entry) = index.installations.get_mut(&previous) {
                entry.lifecycle.hidden = current.lifecycle.hidden;
                entry.lifecycle.superseded_by = None;
            }
            if let Some(entry) = index.installations.get_mut(key) {
                entry.lifecycle.hidden = true;
                entry.lifecycle.superseded_by = Some(previous.clone());
            }
            let default_moves = index.default_of(&current.slot) == Some(key)
                || index.default_of(&current.slot).is_none();
            if default_moves && !current.lifecycle.hidden {
                index
                    .slots
                    .insert(current.slot.clone(), Some(previous.clone()));
            }
            Ok(format!(
                "rolled back {key} to {previous}; {key} is kept hidden until the next update"
            ))
        },
        // The arrangement goes back with the version, the way an update
        // carried it forward (`PROP-059#launcher-runtime`).
        |index, key| {
            let current = &index.installations[key];
            let previous = current
                .lifecycle
                .superseded_by
                .as_deref()
                .and_then(|previous| index.installations.get(previous));
            if let Some(previous) = previous {
                crate::catalog::carry_over(settings, current, previous)?;
            }
            Ok(())
        },
    )
}
