//! The user's arrangement across versions (`PROP-059#launcher-runtime`):
//! an update from `org/pkg@1.0.0` to `org/pkg@1.1.0` rewrites every
//! collection membership, pin, override and usage count keyed
//! `org/pkg@1.0.0#id` to `org/pkg@1.1.0#id` when the new version still has
//! a launcher `id`, and drops the keys whose launcher vanished; a rollback
//! carries them back the same way. Without it every update would empty the
//! user's shelves of that package, because a launcher key names a version.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime");

use super::events::{EventKind, HomeEvent};
use super::usage::{LauncherUsage, Usage};
use super::user::UserLaunchers;
use crate::error::StateError;
use crate::index::Installation;
use crate::settings::SettingsRoot;

/// Old launcher key → its new key, or `None` when the launcher vanished.
type Rewrite = Vec<(String, Option<String>)>;

fn rewrite_of(from: &Installation, to: &Installation) -> Rewrite {
    let (old, new) = (from.package.key(), to.package.key());
    from.launchers
        .iter()
        .map(|launcher| {
            let kept = to.launchers.iter().any(|other| other.id == launcher.id);
            (
                super::launcher_key(&old, &launcher.id),
                kept.then(|| super::launcher_key(&new, &launcher.id)),
            )
        })
        .collect()
}

/// Rewrite one ordered list of keys, keeping the first place of a key that
/// now appears twice: seeding may already have appended the new key behind
/// the user's own place for the old one.
fn rewrite_list(list: &mut Vec<String>, rewrite: &Rewrite) {
    let mut out: Vec<String> = Vec::with_capacity(list.len());
    for key in list.drain(..) {
        let key = match rewrite.iter().find(|(old, _)| *old == key) {
            Some((_, Some(new))) => new.clone(),
            Some((_, None)) => continue,
            None => key,
        };
        if !out.contains(&key) {
            out.push(key);
        }
    }
    *list = out;
}

/// Apply `rewrite` to the layer and the usage facts; whether anything
/// changed.
fn apply(layer: &mut UserLaunchers, usage: &mut Usage, rewrite: &Rewrite) -> bool {
    let (layer_before, usage_before) = (layer.clone(), usage.clone());
    for collection in &mut layer.collections {
        rewrite_list(&mut collection.launchers, rewrite);
        rewrite_list(&mut collection.removed, rewrite);
    }
    rewrite_list(&mut layer.pins.taskbar, rewrite);
    rewrite_list(&mut layer.pins.favorites, rewrite);
    let mut overrides = Vec::with_capacity(layer.launchers.len());
    for mut row in layer.launchers.drain(..) {
        match rewrite.iter().find(|(old, _)| *old == row.key) {
            Some((_, Some(new))) => row.key = new.clone(),
            Some((_, None)) => continue,
            None => {}
        }
        // A row the user already has for the new key is the newer word.
        if !overrides
            .iter()
            .any(|kept: &super::user::UserLauncher| kept.key == row.key)
        {
            overrides.push(row);
        }
    }
    layer.launchers = overrides;
    for (old, new) in rewrite {
        let Some(facts) = usage.launchers.remove(old) else {
            continue;
        };
        if let Some(new) = new {
            let merged = usage.launchers.entry(new.clone()).or_default();
            *merged = LauncherUsage {
                starts: merged.starts.saturating_add(facts.starts),
                last_started: merged.last_started.clone().max(facts.last_started),
            };
        }
    }
    *layer != layer_before || *usage != usage_before
}

/// Carry the arrangement of `from`'s launchers over to `to`'s, writing the
/// layer and the usage facts and bumping the revision once with one
/// `launchers-changed` event. The caller holds the home's mutation lock.
pub(crate) fn carry_over(
    settings: &SettingsRoot,
    from: &Installation,
    to: &Installation,
) -> Result<(), StateError> {
    if from.package.key() == to.package.key() {
        return Ok(());
    }
    let rewrite = rewrite_of(from, to);
    crate::export::rekey(settings, &rewrite)?;
    let mut layer = UserLaunchers::load(settings)?;
    let mut usage = Usage::load(settings)?;
    if apply(&mut layer, &mut usage, &rewrite) {
        layer.write(settings)?;
        usage.write(settings)?;
        super::bump_revision(
            settings,
            &HomeEvent::about(EventKind::LaunchersChanged, to.package.key()),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::user::{UserCollection, UserLauncher, UserPins};
    use super::*;

    const OLD_EDIT: &str = "org.example/editor@1.0.0#edit";
    const OLD_TOOL: &str = "org.example/editor@1.0.0#tool";
    const NEW_EDIT: &str = "org.example/editor@1.1.0#edit";

    fn rewrite() -> Rewrite {
        vec![
            (OLD_EDIT.into(), Some(NEW_EDIT.into())),
            (OLD_TOOL.into(), None),
        ]
    }

    #[test]
    fn memberships_pins_overrides_and_counts_follow_the_launcher_and_vanished_ones_go() {
        let mut layer = UserLaunchers {
            pins: UserPins {
                taskbar: vec![OLD_TOOL.into(), OLD_EDIT.into()],
                favorites: vec!["other/x@1.0.0#x".into()],
            },
            collections: vec![UserCollection {
                name: "Editor Suite".into(),
                seeded: true,
                // Seeding already appended the new key behind the old one.
                launchers: vec![OLD_EDIT.into(), OLD_TOOL.into(), NEW_EDIT.into()],
                removed: vec![OLD_TOOL.into()],
                ..Default::default()
            }],
            launchers: vec![UserLauncher {
                key: OLD_EDIT.into(),
                name: Some("My editor".into()),
                ..Default::default()
            }],
        };
        let mut usage = Usage::default();
        usage.launchers.insert(
            OLD_EDIT.into(),
            LauncherUsage {
                starts: 3,
                last_started: Some("2026-10-31T00:00:00Z".into()),
            },
        );
        usage.launchers.insert(
            OLD_TOOL.into(),
            LauncherUsage {
                starts: 1,
                last_started: None,
            },
        );

        assert!(apply(&mut layer, &mut usage, &rewrite()));
        assert_eq!(layer.pins.taskbar, vec![NEW_EDIT.to_owned()]);
        assert_eq!(layer.pins.favorites, vec!["other/x@1.0.0#x".to_owned()]);
        assert_eq!(
            layer.collections[0].launchers,
            vec![NEW_EDIT.to_owned()],
            "the user's place, once"
        );
        assert!(layer.collections[0].removed.is_empty());
        assert_eq!(layer.launchers[0].key, NEW_EDIT);
        assert_eq!(layer.launchers[0].name.as_deref(), Some("My editor"));
        assert_eq!(usage.of(NEW_EDIT).starts, 3);
        assert!(!usage.launchers.contains_key(OLD_EDIT));
        assert!(!usage.launchers.contains_key(OLD_TOOL));

        // Carrying again changes nothing: the old keys are gone.
        assert!(!apply(&mut layer, &mut usage, &rewrite()));
    }
}
