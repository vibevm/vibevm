//! The one library that composes a session's PATH and `VIBE_APP_USE`, so
//! `vibe shell --use`, `vibe app env --use` and `vibe app exec` cannot drift
//! (PROP-059 `#session-selection`, PROP-075 `#ACTIVATION-ENV-OUTPUT`,
//! `#VERSION-EXEC-SPELLING`).
//!
//! The invariant PROP-075 asks a test to assert directly is structural here:
//! all three surfaces call [`selection`] for the directories and
//! [`compose_path`] for the order, so one selection cannot produce two
//! different PATHs.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#session-selection");

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vibe_app::{SESSION_OFF, SessionChoices, SettingsRoot};

/// What one selection contributes to a child environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Selection {
    /// The real payload directories of the chosen providers, in priority
    /// order: what goes in front of the child's PATH so the session reaches
    /// that provider directly instead of through the slot's link.
    pub bins: Vec<PathBuf>,
    /// The slot link directories a `SLOT=none` takes out of the child's
    /// PATH, so a lookup falls through to the system program.
    pub withdrawn: Vec<PathBuf>,
    /// The `VIBE_APP_USE` entries this selection carries.
    pub choices: Vec<String>,
}

/// Resolve `uses` (`SLOT=KEY` entries, `SLOT=none` included) against the
/// homes in priority order. Every surface that applies a session choice goes
/// through here.
pub(crate) fn selection(homes: &[SettingsRoot], uses: &[String]) -> Result<Selection> {
    let mut resolved = Selection::default();
    for entry in uses {
        let (slot, key) = entry
            .split_once('=')
            .map(|(slot, key)| (slot.trim(), key.trim()))
            .unwrap_or((entry.trim(), ""));
        for settings in homes {
            let index = vibe_app::Index::load(settings)?;
            if key == SESSION_OFF {
                resolved
                    .withdrawn
                    .extend(vibe_app::slot_link_dirs(settings, &index, slot));
                continue;
            }
            let storage = vibe_app::Storage::load(settings)?;
            resolved.bins.extend(vibe_app::slot_session_dirs(
                settings, &index, &storage, key,
            )?);
        }
        resolved.choices.push(entry.clone());
    }
    Ok(resolved)
}

/// The canonical `VIBE_APP_USE` value of `inherited` plus `entries`, a later
/// entry winning per slot.
pub(crate) fn choices(inherited: Option<&str>, entries: &[String]) -> Result<SessionChoices> {
    let mut choices = match inherited {
        Some(value) => SessionChoices::parse(value)?,
        None => SessionChoices::default(),
    };
    for entry in entries {
        choices.apply(entry)?;
    }
    Ok(choices)
}

/// Put each directory of `front` at the head of `inherited`, in priority
/// order, unless it is already listed (compared canonically), and leave out
/// every directory a `SLOT=none` withdrew.
pub(crate) fn compose_path(
    front: &[PathBuf],
    inherited: Option<&OsStr>,
    withdrawn: &[PathBuf],
) -> Result<OsString> {
    let same = |left: &Path, right: &Path| {
        left == right
            || matches!(
                (std::fs::canonicalize(left), std::fs::canonicalize(right)),
                (Ok(left), Ok(right)) if left == right
            )
    };
    let kept = |entry: &PathBuf| !withdrawn.iter().any(|gone| same(gone, entry));
    let existing: Vec<PathBuf> = inherited
        .map(|value| std::env::split_paths(value).filter(kept).collect())
        .unwrap_or_default();
    let mut head: Vec<PathBuf> = Vec::new();
    for dir in front.iter().filter(|dir| kept(dir)) {
        if existing.iter().any(|entry| same(dir, entry)) {
            continue;
        }
        if head.iter().any(|entry| same(dir, entry)) {
            continue;
        }
        head.push(dir.clone());
    }
    std::env::join_paths(head.into_iter().chain(existing))
        .context("composing the session PATH: a directory contains the PATH separator")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opt_bin_goes_first_unless_already_listed() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("opt/bin");
        std::fs::create_dir_all(&bin).unwrap();
        let inherited = std::env::join_paths(["/usr/bin", "/bin"]).unwrap();
        let composed = compose_path(std::slice::from_ref(&bin), Some(&inherited), &[]).unwrap();
        let entries: Vec<PathBuf> = std::env::split_paths(&composed).collect();
        assert_eq!(entries[0], bin);
        assert_eq!(entries.len(), 3);

        let spelled = dir.path().join("opt/./bin");
        let listed = std::env::join_paths([Path::new("/usr/bin"), spelled.as_path()]).unwrap();
        let composed = compose_path(std::slice::from_ref(&bin), Some(&listed), &[]).unwrap();
        assert_eq!(composed, listed, "an already listed home is not repeated");

        let alone = compose_path(std::slice::from_ref(&bin), None, &[]).unwrap();
        assert_eq!(PathBuf::from(alone), bin);
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#slot-off")]
    fn a_withdrawn_slot_link_directory_is_left_out_of_the_child_path() {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("opt/paths/node/0");
        let bin = dir.path().join("opt/bin");
        std::fs::create_dir_all(&link).unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        let inherited = std::env::join_paths([link.as_path(), Path::new("/usr/bin")]).unwrap();
        // `--use node=none`: the slot's link directory goes, everything else
        // stays in order, and `opt/bin` still leads.
        let composed = compose_path(
            std::slice::from_ref(&bin),
            Some(&inherited),
            std::slice::from_ref(&link),
        )
        .unwrap();
        let entries: Vec<PathBuf> = std::env::split_paths(&composed).collect();
        assert_eq!(entries, vec![bin.clone(), PathBuf::from("/usr/bin")]);
        // A withdrawn directory is not put back at the front either.
        let composed = compose_path(
            &[link.clone(), bin.clone()],
            Some(&inherited),
            std::slice::from_ref(&link),
        )
        .unwrap();
        let entries: Vec<PathBuf> = std::env::split_paths(&composed).collect();
        assert_eq!(entries, vec![bin, PathBuf::from("/usr/bin")]);
    }

    #[test]
    fn a_repeated_directory_is_put_in_front_once() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let composed = compose_path(&[bin.clone(), bin.clone()], None, &[]).unwrap();
        assert_eq!(std::env::split_paths(&composed).count(), 1);
    }

    #[test]
    fn a_later_use_wins_per_slot_over_the_inherited_value() {
        let choices = choices(
            Some("node=org.example/node@20.0.0;ruby=org.example/ruby@3.3.0"),
            &["node=org.example/node@22.0.0".to_owned()],
        )
        .unwrap();
        assert_eq!(
            choices.render(),
            "node=org.example/node@22.0.0;ruby=org.example/ruby@3.3.0"
        );
    }
}
