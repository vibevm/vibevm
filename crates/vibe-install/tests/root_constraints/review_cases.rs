//! Public path compatibility and post-plan declaration changes at apply.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INSTALL-CONSUMER-RECORDING"
);

use super::fixture::*;
use vibe_core::manifest::{Lockfile, Manifest};
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

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn changed_sibling_common_input_refuses_apply_before_selected_publication() {
    let mut f = Fixture::new("", &[("member", "")]);
    f.add("x", "1.0.0", "");
    let Plan::Ready(planned) = f.plan(".", &["org.constraints/x"]).unwrap() else {
        panic!("actual plan")
    };
    let member = f.root.join("member/vibe.toml");
    write(
        &member,
        "[project]\nname='member'\nversion='1.0.0'\n[requires.packages.'org.constraints/x']\nversion='1'\nconfiguration={policy='require-selection'}\n",
    );
    let before = files(&f.root);
    let error = vibe_install::apply(
        &f.source,
        *planned,
        vibe_core::user_config::SlotIntegrity::Verify,
        &vibe_workspace::hooks::HookPolicy {
            allowed_groups: vec![],
            allow_hooks: false,
        },
    )
    .unwrap_err();
    assert!(
        matches!(
            &error,
            vibe_install::Error::ConfigurationInputChanged { consumer, dependency } if consumer=="member/vibe.toml" && dependency=="org.constraints/x"
        ),
        "{error}"
    );
    assert_eq!(
        files(&f.root),
        before,
        "refusal must precede publication, not write-and-restore"
    );
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn programmatic_resolution_common_definition_refuses_before_apply_effects() {
    let mut f = Fixture::new("", &[]);
    f.add("x", "1.0.0", "");
    let Plan::Ready(mut planned) = f.plan(".", &["org.constraints/x"]).unwrap() else {
        panic!("actual plan")
    };
    planned.resolution[0].manifest.configuration=Manifest::parse_str("[package]\ngroup='org.constraints'\nname='x'\nkind='flow'\nversion='1.0.0'\n[configuration]\n").unwrap().configuration;
    let before = files(&f.root);
    let error = vibe_install::apply(
        &f.source,
        *planned,
        vibe_core::user_config::SlotIntegrity::Verify,
        &vibe_workspace::hooks::HookPolicy {
            allowed_groups: vec![],
            allow_hooks: false,
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        vibe_install::Error::ConfigurationUnsupported { .. }
    ));
    assert_eq!(files(&f.root), before);
}

fn migration_fixture(target: &str) -> Fixture {
    let mut f = Fixture::new("", &[]);
    f.add("x", "1.0.0", "");
    f.apply(".", &["org.constraints/x@=1.0.0"]);
    let path = f.root.join("vibe.toml");
    let mut manifest = Manifest::read(&path).unwrap();
    manifest.requires.packages.clear();
    manifest.write(path).unwrap();
    let mut lock = f.lock();
    lock.meta.root_dependencies = vec![vibe_core::PackageRef::parse(target).unwrap()];
    lock.write(f.root.join("vibe.lock")).unwrap();
    f
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn legacy_seed_stays_in_memory_on_candidate_refusal_or_solver_failure() {
    for rich in [false, true] {
        let mut f = migration_fixture("org.constraints/x@=2.0.0");
        if rich {
            f.add("x", "2.0.0", "[configuration]\n");
        }
        let mut manifest = Manifest::read(f.root.join("vibe.toml")).unwrap();
        let original = manifest.clone();
        let mut workspace = vibe_workspace::Workspace::discover(&f.root).unwrap();
        let before = files(&f.root);
        let error = vibe_install::plan_prepared_with_spec_format(
            &f.source,
            &f.root,
            &mut manifest,
            &mut workspace,
            request(&[]),
            vibe_core::manifest::SpecFormat::Mixed,
            &vibe_install::NullObserver,
        )
        .unwrap_err();
        if rich {
            assert!(matches!(
                error,
                vibe_install::Error::ConfigurationUnsupported { .. }
            ));
        }
        assert_eq!(files(&f.root), before);
        assert_eq!(manifest, original);
        assert!(workspace.root_manifest.requires.packages.is_empty());
        assert_eq!(
            f.lock().meta.root_dependencies[0].to_string(),
            "org.constraints/x@=2.0.0"
        );
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn successful_legacy_seed_publishes_after_fresh_or_resolved_admission() {
    for fresh in [false, true] {
        let mut f = migration_fixture(if fresh {
            "org.constraints/x@=1.0.0"
        } else {
            "org.constraints/x@=2.0.0"
        });
        if !fresh {
            f.add("x", "2.0.0", "");
        }
        let solves = f.source.solves.borrow().len();
        let plan = f.plan(".", &[]).unwrap();
        if fresh {
            assert!(matches!(plan, Plan::Fresh));
            assert_eq!(
                f.source.solves.borrow().len(),
                solves,
                "Fresh migration must not resolve newer versions"
            );
        } else {
            assert!(matches!(plan, Plan::Ready(_)));
        }
        let manifest = Manifest::read(f.root.join("vibe.toml")).unwrap();
        assert_eq!(manifest.requires.packages.len(), 1);
        assert_eq!(manifest.requires.packages[0].name.as_str(), "x");
    }
}
