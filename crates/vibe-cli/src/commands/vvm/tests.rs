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
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#surface", r = 5)]
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

/// The refusal the whole posture rests on: online it is not a refusal at
/// all, and offline it hands the operator both halves of what happened —
/// the verb that wanted the network, spelled the way they typed it, and the
/// address it wanted — plus the rule it is obeying.
#[test]
fn the_offline_refusal_names_the_verb_the_address_and_the_rule() {
    let online = VvmEnv::default();
    assert!(
        online
            .refuse_offline("self:update", "https://example.invalid/x")
            .is_ok()
    );

    let offline = VvmEnv {
        offline: true,
        ..VvmEnv::default()
    };
    let error = offline
        .refuse_offline(
            "self:reinstall",
            "https://example.invalid/DISTRIBUTIONS.json",
        )
        .unwrap_err()
        .to_string();
    // The report label is `self:reinstall`; what the operator typed is
    // `vibe self reinstall`, and that is what the message says.
    assert!(error.contains("`vibe self reinstall`"), "{error}");
    assert!(
        error.contains("https://example.invalid/DISTRIBUTIONS.json"),
        "{error}"
    );
    assert!(
        error.contains("violates spec://org.vibevm.core/vibevm/common/PROP-019#surface"),
        "{error}"
    );
    // A refusal that did not say how to get past it would be half a
    // message: the posture has three rungs and the operator set one.
    assert!(error.contains("--offline"), "{error}");
    assert!(error.contains("VIBE_OFFLINE"), "{error}");
    assert!(error.contains("[net] offline"), "{error}");
}

/// The source lane's own refusal. A build that needs the managed mirror
/// needs a clone or a fetch, and under the posture it is stopped before git
/// runs — naming the mirror it would have gone to, whether that mirror was
/// asked for by name or is the default this run would have picked.
#[test]
fn the_offline_posture_refuses_the_source_lane_before_git_reaches_a_mirror() {
    let temp = tempfile::tempdir().unwrap();
    let env = VvmEnv {
        root: Some(temp.path().join("opt")),
        offline: true,
        ..VvmEnv::default()
    };
    let ctx = output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Auto);

    // An explicit `--mirror` names that mirror; no flag names the default
    // the non-interactive path would have taken.
    for (requested, expected) in [
        (Some("github"), "https://github.com/vibevm/vibevm.git"),
        (None, "https://gitverse.ru/vibevm/vibevm.git"),
    ] {
        let args = VvmInstallArgs {
            // Not `latest`: `latest` prefers the checkout this test process
            // is running out of, which needs no mirror and no network.
            selector: "1.2.3".to_string(),
            kind: ForcedKind {
                tag: false,
                branch: false,
                commit: false,
            },
            profile: None,
            release: false,
            mirror: requested.map(str::to_string),
            force: false,
        };
        let error = run_install_cmd(&ctx, &env, args, "self:update")
            .unwrap_err()
            .to_string();
        assert!(error.contains("`vibe self update`"), "{error}");
        assert!(error.contains(expected), "{error}");
    }
    // The clone never happened, so the managed mirror was never created.
    assert!(
        !VersionStore::new(temp.path().join("opt"))
            .mirror_dir()
            .exists()
    );
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
