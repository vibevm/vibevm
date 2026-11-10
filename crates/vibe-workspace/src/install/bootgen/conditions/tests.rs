//! Generation-time predicate resolution — which gates survive into
//! `INDEX.md` and which are decided here.
//!
//! The one invariant worth a test of its own: a `target:` gate is *never*
//! residual. It names the platform the product addresses, which is a
//! property of the project and never of a session, so it is resolved while
//! artifacts are generated — exactly as `installed:` always has been — and
//! `prepare_index_body`'s `kind` expression therefore keeps the meaning it
//! has today (PROP-076 `#SYS-HOST-AND-TARGET`).
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HOST-AND-TARGET",
    r = 1
);

use vibe_core::manifest::{Manifest, TargetOs, TargetSelection};

use super::*;

/// A world over an empty resolution, so only the target axis is in play.
fn world(targets: TargetSelection) -> ConditionWorld {
    ConditionWorld::new(&[], targets)
}

fn resolved(when: &str, targets: TargetSelection) -> Option<Option<WhenCondition>> {
    let condition: WhenCondition = when.parse().expect("the test's own condition parses");
    resolve_condition(Some(&condition), &world(targets))
}

#[test]
fn an_absent_target_selection_includes_every_target_row() {
    // The compatibility condition the owner stated: a project that declares
    // nothing sees no `target:` row dropped and no row gated.
    for os in ["windows", "macos", "linux"] {
        assert_eq!(
            resolved(&format!("target:{os}"), TargetSelection::AllPlatforms),
            Some(None),
            "`target:{os}` is included unconditionally under an absent selection"
        );
    }
}

#[test]
fn a_declared_selection_includes_its_own_and_drops_the_rest() {
    let only_windows = TargetSelection::Only(vec![TargetOs::Windows]);
    assert_eq!(
        resolved("target:windows", only_windows.clone()),
        Some(None),
        "a selected platform's row becomes unconditional"
    );
    assert_eq!(
        resolved("target:linux", only_windows),
        None,
        "an unselected platform's row is omitted entirely, not gated"
    );
}

#[test]
fn a_target_gate_is_never_residual_whatever_the_selection() {
    // Whether included or dropped, the verdict is never `Some(Some(_))`:
    // no `target:` string can reach a generated INDEX row.
    for targets in [
        TargetSelection::AllPlatforms,
        TargetSelection::Only(vec![TargetOs::Linux]),
        TargetSelection::Only(vec![TargetOs::Windows, TargetOs::Macos]),
    ] {
        for os in ["windows", "macos", "linux"] {
            let verdict = resolved(&format!("target:{os}"), targets.clone());
            assert!(
                !matches!(verdict, Some(Some(_))),
                "`target:{os}` must not survive as a residual condition: {verdict:?}"
            );
        }
    }
}

#[test]
fn a_session_gate_still_survives_for_the_reading_session() {
    // The other half of the owner decision: `os:` is about the machine the
    // session runs on, so generation keeps it rather than deciding it.
    assert_eq!(
        resolved("os:windows", TargetSelection::Only(vec![TargetOs::Linux])),
        Some(Some(WhenCondition::Os(TargetOs::Windows))),
        "an `os:` row is preserved verbatim, and the product's target does \
         not answer it"
    );
}

#[test]
fn an_unconditional_contribution_is_untouched() {
    assert_eq!(
        resolve_condition(None, &world(TargetSelection::AllPlatforms)),
        Some(None)
    );
}

#[test]
fn the_project_target_selection_comes_from_the_manifest_and_defaults_to_every_os() {
    // Read from the node's own authored declaration, never observed — which
    // is why baking a platform is not volatile content.
    let bare = Manifest::parse_str("[project]\nname='host'\nversion='1.0.0'\n").unwrap();
    assert_eq!(project_targets(&bare), TargetSelection::AllPlatforms);

    let declared = Manifest::parse_str(
        "[project]\nname='host'\nversion='1.0.0'\n\
         [system_prompt]\ntarget = ['windows', 'linux']\n",
    )
    .unwrap();
    assert_eq!(
        project_targets(&declared),
        TargetSelection::Only(vec![TargetOs::Windows, TargetOs::Linux]),
        "the author's own order is preserved"
    );
}
