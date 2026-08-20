//! Unit tests for `vibe self` dispatch and selector resolution. Split out
//! of `mod.rs` so the production file stays inside the file-length budget
//! (DISCIPLINE-SWEEP §1a tests-out); included via `#[path]` from `mod.rs`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#surface");

use super::*;

#[test]
fn default_root_is_under_dot_vibe_and_not_the_legacy_home_opt() {
    let home = PathBuf::from("C:/Users/tester");
    let root = resolve_root(None, None, Some(home.clone())).unwrap();
    assert_eq!(root, home.join(".vibe").join("opt"));
    assert_ne!(root, home.join("opt"));
}

#[test]
fn install_root_override_remains_the_install_base() {
    let override_base = PathBuf::from("D:/vvm-test");
    let root = resolve_root(
        None,
        Some(override_base.clone()),
        Some(PathBuf::from("C:/Users/tester")),
    )
    .unwrap();
    assert_eq!(root, override_base.join("opt"));
}
use crate::commands::vvm::model::{
    InstallRecord, Kind, Origin, Profile, Selector, State, VersionId,
};
use specmark::verifies;

fn rec(kind: Kind, id: &str, instance: u64) -> InstallRecord {
    InstallRecord {
        kind,
        id: id.into(),
        instance,
        commit: "c".into(),
        toolchain: "t".into(),
        profile: Profile::Debug,
        installed_at: "now".into(),
        origin: Origin::Managed,
        source_path: None,
        payload_sha256: None,
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#selectors", r = 1)]
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
