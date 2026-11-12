//! The refusals and the provenance words of `[versions] select`
//! (PROP-075 `#VERSIONS-SELECT`). The policy's own arithmetic is pinned
//! in `vibe_core::version_select`; what is checked here is that each
//! refusal names the four things an operator needs — the policy, the
//! requirement, the version to carry in, and where to put it.

use std::path::PathBuf;

use super::{Origin, no_drop, redirect_note};

/// Every layer prints the word the rest of the configuration surface
/// already uses for it, so one `vibe show config` reads consistently.
#[test]
fn each_layer_is_named_the_way_every_other_layered_value_names_it() {
    assert_eq!(Origin::Environment.label(), "env");
    assert_eq!(Origin::UserConfig.label(), "config.toml");
    assert_eq!(Origin::SiteProfile.label(), "site profile");
    assert_eq!(Origin::BuiltIn.label(), "default");
}

/// The override is a `VIBE_*` name, which is what lets `[env]` in
/// `config.toml` supply it as a fallback like every other such variable,
/// and it is read at the composition root with its siblings.
#[test]
fn the_environment_override_is_a_promotable_vibe_name() {
    assert!(crate::output::VERSIONS_SELECT_VAR.starts_with("VIBE_"));
    assert_eq!(crate::output::VERSIONS_SELECT_VAR, "VIBE_VERSIONS_SELECT");
}

/// `drop-only` with no drop declared at all refuses by naming the key to
/// write, the command that prints what to fill it with, and the policy
/// that would install anyway.
#[test]
fn no_drop_at_all_names_the_key_the_probe_and_the_other_policy() {
    let message = no_drop("org.vibevm.node/node", "org.vibevm.node/node@22").to_string();
    assert!(message.contains("drop-only"), "{message}");
    assert!(message.contains("[artifacts]"), "{message}");
    assert!(message.contains("packages = "), "{message}");
    assert!(
        message.contains("vibe artifacts where org.vibevm.node/node"),
        "{message}"
    );
    assert!(message.contains("prefer-drop"), "{message}");
    assert!(
        message.contains("PROP-075#VERSIONS-SELECT"),
        "every refusal names the record it enforces: {message}"
    );
    assert!(message.contains("fix:"), "{message}");
}

/// A requirement the domain cannot satisfy at all is the registry's own
/// refusal, so the policy's message points at the listing rather than
/// inventing a version to carry in.
#[test]
fn an_unsatisfiable_requirement_points_at_the_listing_and_names_no_version() {
    let message = super::nothing_in_drop_message(
        "org.vibevm.node/node",
        "org.vibevm.node/node@23",
        None,
        &[PathBuf::from("/srv/drop")],
        None,
    );
    assert!(message.contains("matches no version"), "{message}");
    assert!(
        message.contains("vibe app versions org.vibevm.node/node"),
        "{message}"
    );
}

/// The ordinary `drop-only` refusal: the newest matching version by
/// number, the exact path its payload belongs at, and the fix that
/// installs today instead.
#[test]
fn a_drop_holding_nothing_names_the_version_and_where_it_belongs() {
    let message = super::nothing_in_drop_message(
        "org.vibevm.node/node",
        "org.vibevm.node/node@22",
        Some(&"22.23.5".parse().unwrap()),
        &[PathBuf::from("/srv/drop")],
        Some("/srv/drop/org.vibevm.node/node/22.23.5/node-v22.23.5-linux-x64.tar.xz".to_owned()),
    );
    assert!(message.contains("22.23.5"), "{message}");
    assert!(
        message.contains("/srv/drop/org.vibevm.node/node/22.23.5/node-v22.23.5-linux-x64.tar.xz"),
        "{message}"
    );
    assert!(message.contains("prefer-drop"), "{message}");
}

/// With no file name a skeleton can settle, the refusal still has to say
/// where to look: the drop's own directory, never nothing.
#[test]
fn an_unknown_file_name_falls_back_to_naming_the_drop_itself() {
    let message = super::nothing_in_drop_message(
        "org.vibevm.node/node",
        "org.vibevm.node/node@22",
        Some(&"22.23.5".parse().unwrap()),
        &[PathBuf::from("/srv/drop")],
        None,
    );
    assert!(message.contains("/srv/drop"), "{message}");
}

/// The note of a redirected resolution names both versions and the ground
/// that made the picked one available — `in drop` for a payload on disk,
/// `installed` for a version this home already holds. Announcing the
/// second as the first would be a sentence about a file that is not there.
#[test]
fn the_note_names_both_versions_and_the_ground_that_applied() {
    let picked = "22.23.4".parse().unwrap();
    let newer = "22.23.5".parse().unwrap();
    assert_eq!(
        redirect_note("node@22", &picked, &newer, true),
        "node@22 → 22.23.4 (in drop; 22.23.5 is newer but not in the drop)"
    );
    assert_eq!(
        redirect_note("node@22", &picked, &newer, false),
        "node@22 → 22.23.4 (installed; 22.23.5 is newer but not in the drop)"
    );
}
