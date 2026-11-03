//! Public path compatibility and post-plan declaration changes at apply.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INSTALL-CONSUMER-RECORDING"
);

use super::fixture::*;
use vibe_core::manifest::Lockfile;
use vibe_core::user_config::SlotIntegrity;
use vibe_install::{Plan, PlannedInstall};
use vibe_workspace::hooks::HookPolicy;

fn apply(source: &Source, plan: Box<PlannedInstall>) -> Result<(), vibe_install::Error> {
    vibe_install::apply(
        source,
        *plan,
        SlotIntegrity::Verify,
        &HookPolicy {
            allowed_groups: vec![],
            allow_hooks: false,
        },
    )
    .map(|_| ())
}

#[test]
fn relative_selected_member_completes_plan_apply_and_reload() {
    let mut f = Fixture::new("", &[]);
    f.add("x", "1.0.0", "");
    let cwd = std::env::current_dir().unwrap();
    let owned = tempfile::tempdir_in(&cwd).unwrap();
    let root = owned.path().join("project");
    write(
        &root.join("vibe.toml"),
        "[project]\nname='root'\nversion='1.0.0'\n[workspace]\nmembers=['member']\n",
    );
    let member = root.join("member");
    write(
        &member.join("vibe.toml"),
        "[project]\nname='member'\nversion='1.0.0'\n",
    );
    let relative = member.strip_prefix(&cwd).unwrap();
    assert!(relative.is_relative());
    let Plan::Ready(plan) = vibe_install::plan(
        &f.source,
        relative,
        request(&["org.constraints/x@^1"]),
        &vibe_install::NullObserver,
    )
    .unwrap() else {
        panic!("explicit install must plan")
    };
    apply(&f.source, plan).unwrap();
    let lock = Lockfile::read(root.join("vibe.lock")).unwrap();
    assert_eq!(lock.packages.len(), 1);
    assert!(matches!(
        vibe_install::plan(
            &f.source,
            relative,
            request(&[]),
            &vibe_install::NullObserver
        )
        .unwrap(),
        Plan::Fresh
    ));
    assert_eq!(std::env::current_dir().unwrap(), cwd);
}

#[cfg(unix)]
#[test]
fn native_symlink_alias_keeps_the_selected_member() {
    let mut f = Fixture::new("", &[("a", &requires("x", "^1"))]);
    f.add("x", "1.0.0", "");
    let aliases = tempfile::tempdir().unwrap();
    let alias = aliases.path().join("workspace");
    std::os::unix::fs::symlink(&f.root, &alias).unwrap();
    let Plan::Ready(plan) = vibe_install::plan(
        &f.source,
        &alias.join("a"),
        request(&["org.constraints/x@=1.0.0"]),
        &vibe_install::NullObserver,
    )
    .unwrap() else {
        panic!("explicit install must plan")
    };
    apply(&f.source, plan).unwrap();
    let member = vibe_core::manifest::Manifest::read(f.root.join("a/vibe.toml")).unwrap();
    assert_eq!(member.requires.packages[0].version.to_string(), "=1.0.0");
    assert_eq!(f.lock().packages.len(), 1);
}

#[test]
fn changed_sibling_version_or_kind_refuses_apply_and_preserves_current_bytes() {
    for changed in [
        "'org.constraints/x'='^1'",
        "'stack:org.constraints/x'='>=1, <3'",
    ] {
        let mut f = Fixture::new(
            "",
            &[
                ("a", &requires("x", "^1")),
                ("b", &requires("x", ">=1, <3")),
            ],
        );
        f.add("x", "1.0.0", "");
        f.apply(".", &[]);
        f.add("x", "2.0.0", "");
        let Plan::Ready(plan) = f.plan("a", &["org.constraints/x@=2.0.0"]).unwrap() else {
            panic!("explicit update must plan")
        };
        write(
            &f.root.join("b/vibe.toml"),
            &format!("[project]\nname='b'\nversion='1.0.0'\n[requires.packages]\n{changed}\n"),
        );
        let before = files(&f.root);
        let error = apply(&f.source, plan)
            .expect_err("the already-read sibling declaration must match the selected resolution");
        assert!(
            matches!(error, vibe_install::Error::ConsumerConstraints { .. }),
            "{error}"
        );
        assert!(error.to_string().contains("b/vibe.toml"), "{error}");
        assert_eq!(
            files(&f.root),
            before,
            "preserve edited sibling and old selected manifest/lock/slots/boot"
        );
    }
}
