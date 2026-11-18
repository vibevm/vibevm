//! Profile-requested settings home: explicit durable environment, no migration.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");
use super::super::env::{Persisted, Shell, settings_paths_equal};
use super::super::{VvmEnv, make_persister};
use anyhow::Result;
use vibe_core::site_profile::SiteProfile;

pub(in crate::commands::vvm) enum Action {
    Apply,
    Inspect,
    Remove,
}

pub(in crate::commands::vvm) fn step(
    profile: &SiteProfile,
    env: &VvmEnv,
    action: Action,
    dry_run: bool,
    notes: &mut Vec<String>,
) -> Result<bool> {
    let expanded = profile.with_home(env.home.as_deref());
    let Some(requested) = expanded.self_source.settings_dir.as_deref() else {
        return Ok(false);
    };
    if !requested.is_absolute() {
        notes.push(format!(
            "settings_dir {} cannot be used on this host; VIBE_SETTINGS was not changed",
            requested.display()
        ));
        return Ok(false);
    }
    let persister = make_persister(env, Shell::detect(env.shell.as_deref()))?;
    let target = requested.display().to_string();
    let actual = vibe_core::settings::settings_dir();
    notes.push(format!(
        "settings directory: {}; profile requests: {}",
        actual
            .as_deref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "unavailable".into()),
        target
    ));
    if actual.as_deref().is_none_or(|path| {
        !settings_paths_equal(&path.display().to_string(), &target, cfg!(windows))
    }) {
        notes.push(format!("this process still uses its original settings directory; set VIBE_SETTINGS to {target} and re-run apply there. No existing settings were moved or deleted"));
    }
    let current = persister.settings_home()?;
    let matches = current
        .as_deref()
        .is_some_and(|value| settings_paths_equal(value, &target, cfg!(windows)));
    match action {
        Action::Inspect => {
            notes.push(format!(
                "persistent VIBE_SETTINGS: {} ({})",
                current.as_deref().unwrap_or("not set"),
                if matches {
                    "matches profile"
                } else {
                    "differs from profile"
                }
            ));
            Ok(false)
        }
        Action::Apply if dry_run => {
            notes.push(format!(
                "persistent VIBE_SETTINGS: {}",
                if matches {
                    "already set"
                } else {
                    "would be set from profile"
                }
            ));
            Ok(false)
        }
        Action::Apply => {
            let changed = persister.set_settings_home(requested)? == Persisted::Changed;
            notes.push(format!(
                "persistent VIBE_SETTINGS: {}; {}",
                if changed { "updated" } else { "already set" },
                persister.activation_hint()
            ));
            Ok(changed)
        }
        Action::Remove if dry_run => {
            notes.push(format!(
                "persistent VIBE_SETTINGS: {}",
                if matches {
                    "would remove matching profile value"
                } else {
                    "different value retained"
                }
            ));
            Ok(false)
        }
        Action::Remove => {
            let changed = persister.remove_settings_home(requested)? == Persisted::Changed;
            notes.push(format!(
                "persistent VIBE_SETTINGS: {}",
                if changed {
                    "matching profile value removed"
                } else {
                    "absent or different value retained"
                }
            ));
            Ok(changed)
        }
    }
}
