//! The commands whose argument may be an **installed slot id** as well as
//! a package: `vibe app versions`, `use`, `alternatives` and
//! `env --use SLOT=VER`.
//!
//! **The precedence, stated once** (PROP-059 `#SHORT-NAME-SLOT-PRECEDENCE`):
//! an installed slot id wins, then an installed package's short name
//! standing for the one slot that package provides, then — for
//! `vibe app versions` alone — a registry short name. The order is this
//! way round so that **every spelling that works today keeps exactly the
//! meaning it has today**: a slot and an installation may share a word,
//! and the word must not change hands when a registry gains a package of
//! that name.

use anyhow::Result;
use vibe_app::{Index, SettingsRoot};

use super::render::render_collision;
use super::{Inputs, Lookup, Sources, from_registries_or_unchanged, installed_groups};
use crate::exit_code::InstallError;
use crate::output;

/// Whether a bare word already names a slot of this home — tier 1, and
/// the reason nothing that works today changes.
fn names_a_slot(index: &Index, word: &str) -> bool {
    index.slots.contains_key(word)
        || index
            .installations
            .values()
            .any(|record| record.slot == word)
}

/// The distinct slots this home's installations of a package named `word`
/// provide. Exactly one is a usable alias; none or several is today's
/// behaviour, unchanged.
fn slots_of_package(index: &Index, word: &str) -> Vec<String> {
    let mut slots: Vec<String> = index
        .installations
        .values()
        .filter(|record| record.package.name == word)
        .map(|record| record.slot.clone())
        .collect();
    slots.sort();
    slots.dedup();
    slots
}

/// The `SLOT` argument of `vibe app use` and `vibe app alternatives`,
/// with a bare installed package name read as the slot that package
/// provides.
///
/// A slot id keeps its meaning outright; the alias fires only for a word
/// that names no slot of this home at all, and only when exactly one
/// installed package of that name provides exactly one slot. A slot is
/// not a package, so there is no third tier here: a slot nothing
/// installed provides cannot be selected, and inventing one from a
/// registry would be a guess.
pub(crate) fn slot_target(
    ctx: &output::Context,
    command: &str,
    settings: &SettingsRoot,
    word: &str,
) -> Result<String> {
    let word = word.trim();
    let index = Index::load(settings)?;
    if names_a_slot(&index, word) {
        return Ok(word.to_owned());
    }
    let groups = installed_groups(&index, word);
    match (slots_of_package(&index, word).as_slice(), groups.as_slice()) {
        ([slot], [group]) => {
            ctx.decision(&format!("{word} → slot `{slot}` of {group}/{word}"));
            Ok(slot.clone())
        }
        // Two groups under one bare name is the PROP-008 §2.7 collision,
        // here as everywhere else.
        (_, many) if many.len() > 1 => Err(InstallError::AmbiguousPackage(render_collision(
            command,
            word,
            &Lookup::of_installed(groups),
        ))
        .into()),
        // Neither a slot nor an unambiguous installed package: the command
        // raises exactly the error it raises today.
        _ => Ok(word.to_owned()),
    }
}

/// The `PACKAGE|SLOT` argument of `vibe app versions`, with its `@VER`
/// suffix kept verbatim.
///
/// The first two tiers are already inside the library, so this boundary
/// only adds the third. `only_installed` is `--installed`, which answers
/// from this home and the template cache alone and so has no third tier
/// at all (PROP-075 `#VERSIONS-LIST-OUTPUT`).
pub(crate) fn versions_target(
    ctx: &output::Context,
    home: &SettingsRoot,
    target: &str,
    inputs: Inputs<'_>,
    only_installed: bool,
) -> Result<String> {
    let target = target.trim();
    let (head, suffix) = match target.split_once('@') {
        Some((head, tail)) => (head.trim(), Some(tail)),
        None => (target, None),
    };
    if head.contains('/') {
        return Ok(target.to_owned());
    }
    let index = Index::load(home)?;
    if names_a_slot(&index, head) || !installed_groups(&index, head).is_empty() || only_installed {
        return Ok(target.to_owned());
    }
    let settings = super::super::global::settings_root()?;
    let Some(qualified) = from_registries_or_unchanged(
        ctx,
        "vibe app versions",
        &Sources {
            home,
            settings: &settings,
            inputs,
        },
        head,
    )?
    else {
        // No candidate anywhere: this is the command's own
        // `unknown_target` refusal, which names the slots of this home —
        // the fact its reader needs and this boundary cannot know.
        return Ok(target.to_owned());
    };
    Ok(match suffix {
        Some(suffix) => format!("{qualified}@{suffix}"),
        None => qualified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The precedence rule's first tier: a slot id keeps today's meaning,
    /// and a word that names no slot falls through to the layers below.
    #[test]
    fn a_slot_id_stops_the_short_name_layer() {
        let mut index = Index::default();
        index
            .slots
            .insert("editor".into(), vibe_app::SlotState::default());
        assert!(names_a_slot(&index, "editor"));
        assert!(!names_a_slot(&index, "node"));
    }

    #[test]
    fn a_home_with_no_installation_offers_no_slot_alias() {
        assert!(slots_of_package(&Index::default(), "node").is_empty());
    }
}
