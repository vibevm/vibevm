//! Unit tests for `vibe man` dispatch and selector resolution. Split out
//! of `mod.rs` so the production file stays inside the file-length budget
//! (DISCIPLINE-SWEEP §1a tests-out); included via `#[path]` from `mod.rs`.

specmark::scope!("spec://vibevm/common/PROP-019#surface");

use super::*;
use crate::commands::man::model::{InstallRecord, Kind, Origin, Selector, State, VersionId};
use specmark::verifies;

fn rec(kind: Kind, id: &str, instance: u64) -> InstallRecord {
    InstallRecord {
        kind,
        id: id.into(),
        instance,
        commit: "c".into(),
        toolchain: "t".into(),
        profile: "debug".into(),
        installed_at: "now".into(),
        origin: Origin::Managed,
        source_path: None,
    }
}

#[test]
#[verifies("spec://vibevm/common/PROP-019#selectors", r = 1)]
fn resolve_installed_picks_the_newest_instance_per_selector() {
    let state = State {
        next_instance: 9,
        installs: vec![
            rec(Kind::Branch, "main", 1),
            rec(Kind::Branch, "main", 5),
            rec(Kind::Tag, "1.2.0", 2),
            rec(Kind::Tag, "1.10.0", 3),
        ],
    };
    // latest → newest instance of branch:main.
    let r = resolve_installed(&state, &Selector::Latest, "latest").unwrap();
    assert_eq!(r.version_id(), VersionId::new(Kind::Branch, "main"));
    assert_eq!(r.instance, 5);
    // stable → highest semver tag.
    assert_eq!(
        resolve_installed(&state, &Selector::Stable, "stable")
            .unwrap()
            .version_id(),
        VersionId::new(Kind::Tag, "1.10.0")
    );
    // bare name → branch precedence.
    assert_eq!(
        resolve_installed(&state, &Selector::Ambiguous("main".into()), "main")
            .unwrap()
            .instance,
        5
    );
    // not installed → error.
    assert!(
        resolve_installed(
            &state,
            &Selector::Explicit(VersionId::new(Kind::Tag, "9.9.9")),
            "9.9.9"
        )
        .is_err()
    );
}
