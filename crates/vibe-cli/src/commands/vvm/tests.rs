//! Unit tests for `vibe self` dispatch and selector resolution. Split out
//! of `mod.rs` so the production file stays inside the file-length budget
//! (DISCIPLINE-SWEEP §1a tests-out); included via `#[path]` from `mod.rs`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#surface");

use super::*;

#[test]
fn default_root_is_under_dot_vibe_and_not_the_legacy_home_opt() {
    let home = PathBuf::from(if cfg!(windows) {
        "C:/Users/tester"
    } else {
        "/home/tester"
    });
    let root = resolve_root(None, None, Some(home.clone())).unwrap();
    assert_eq!(root, home.join(".vibe").join("opt"));
    assert_ne!(root, home.join("opt"));
}

#[test]
fn install_root_override_remains_the_install_base() {
    let override_base = PathBuf::from(if cfg!(windows) {
        "D:/vvm-test"
    } else {
        "/tmp/vvm-test"
    });
    let root = resolve_root(
        None,
        Some(override_base.clone()),
        Some(PathBuf::from(if cfg!(windows) {
            "C:/Users/tester"
        } else {
            "/home/tester"
        })),
    )
    .unwrap();
    assert_eq!(root, override_base.join("opt"));
}

#[test]
fn relative_install_root_is_absolutized_and_parent_components_are_normalized() {
    let cwd = PathBuf::from(if cfg!(windows) {
        r"C:\work\repo"
    } else {
        "/work/repo"
    });
    let root = absolute_lexical(Path::new("../owner/.vibe"), &cwd).unwrap();
    assert!(root.is_absolute());
    assert_eq!(
        root,
        if cfg!(windows) {
            PathBuf::from(r"C:\work\owner\.vibe")
        } else {
            PathBuf::from("/work/owner/.vibe")
        }
    );
}
use crate::commands::vvm::model::{
    InstallRecord, InstanceId, Kind, Origin, Profile, Selector, State, VersionId,
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
        distribution_manifest_sha256: None,
    }
}

/// On a binary execution the release lane answers three selectors and hands
/// everything else to the source lane (PROP-019 `##CMD-INSTALL`,
/// `##CMD-UPDATE`, `##SEL-STABLE`).
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#surface", r = 3)]
fn a_binary_execution_routes_stable_to_the_newest_release_and_latest_nowhere() {
    let parse = |raw: &str| model::Selector::parse(raw, None).unwrap();
    assert_eq!(
        binary_lane(&parse("1.2.3")),
        Some(BinaryLane::Version("1.2.3".to_string()))
    );
    assert_eq!(
        binary_lane(&parse("v1.2.3")),
        Some(BinaryLane::Version("1.2.3".to_string())),
        "a `v`-prefixed tag names the same release"
    );
    // `stable` is the newest release, so it takes `self update`'s path
    // rather than falling through to a source build.
    assert_eq!(binary_lane(&parse("stable")), Some(BinaryLane::Newest));
    assert_eq!(binary_lane(&parse("latest")), Some(BinaryLane::NoRelease));
    // A branch or commit has no release to fetch: the source lane keeps it.
    assert_eq!(binary_lane(&parse("main")), None);
    assert_eq!(binary_lane(&parse("branch:main")), None);
}

/// The refusal a binary execution meets on `latest` has to leave the user
/// holding all three release verbs, not just the one that existed before
/// `update` learned to move forward.
#[test]
fn the_latest_refusal_names_every_release_verb() {
    let text = VvmError::BinaryFetchUnavailable.to_string();
    for verb in [
        "vibe self update",
        "vibe self install X.Y.Z",
        "vibe self reinstall",
        "--mirror",
    ] {
        assert!(text.contains(verb), "`{verb}` missing from: {text}");
    }
}

#[test]
fn use_revalidation_rejects_an_inventoried_instance_deleted_before_activation() {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path());
    let record = rec(Kind::Branch, "main", 1);
    store.record_install(record.clone()).unwrap();
    let error = ensure_activatable(&store, &record).unwrap_err().to_string();
    assert!(error.contains("incomplete or corrupt"), "{error}");
    assert!(store.read_current().unwrap().is_none());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#selectors", r = 2)]
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
    assert_eq!(
        resolve_installed(
            &state,
            &Selector::Exact(InstanceId::new(VersionId::new(Kind::Branch, "main"), 1)),
            "branch:main#1",
        )
        .unwrap()
        .instance,
        1
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
