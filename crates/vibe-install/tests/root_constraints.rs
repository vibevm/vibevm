//! Consumer constraints are conjunctive; package identities remain unique.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INSTALL-CONSUMER-CONSTRAINTS"
);

#[path = "root_constraints/fixture.rs"]
mod fixture;
#[path = "support/retained.rs"]
mod retained;
#[path = "root_constraints/review_cases.rs"]
mod review_cases;
use fixture::*;
use vibe_core::manifest::Manifest;
use vibe_install::Plan;

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INSTALL-CONSUMER-CONSTRAINTS"
)]
fn incompatible_siblings_refuse_before_fetch_and_preserve_bytes() {
    let mut f = Fixture::new(
        "",
        &[("a", &requires("x", "^1")), ("b", &requires("x", "^2"))],
    );
    f.add("x", "1.5.0", "");
    f.add("x", "2.5.0", "");
    let before = files(&f.root);
    let error = match f.plan(".", &[]) {
        Err(error) => error,
        Ok(_) => panic!("both consumers must constrain the solve"),
    };
    let text = error.to_string();
    assert!(
        text.contains("a") && text.contains("b") && text.contains("org.constraints/x"),
        "{text}"
    );
    assert!(f.source.fetches.borrow().is_empty());
    assert_eq!(files(&f.root), before);
}

#[test]
fn overlapping_siblings_select_intersection_independent_of_order() {
    for (a, b) in [(">=1, <3", "<2"), ("<2", ">=1, <3")] {
        let mut f = Fixture::new("", &[("a", &requires("x", a)), ("b", &requires("x", b))]);
        f.add("x", "1.5.0", "");
        f.add("x", "2.5.0", "");
        assert_eq!(version(&f.plan(".", &[]).unwrap(), "x"), "1.5.0");
    }
}

#[test]
fn explicit_member_request_cannot_replace_sibling_constraint() {
    let mut f = Fixture::new(&requires("x", "^1"), &[("a", &requires("x", "^1"))]);
    f.add("x", "1.5.0", "");
    f.add("x", "2.5.0", "");
    f.apply(".", &[]);
    let before = files(&f.root);
    assert!(f.plan("a", &["org.constraints/x@=2.5.0"]).is_err());
    assert_eq!(files(&f.root), before);
}

#[test]
fn explicit_update_moves_named_coordinate_preserves_sibling_and_unrelated_pin() {
    let mut f = Fixture::new(
        "",
        &[
            ("a", &requires("x", "^1")),
            ("b", &requires("x", ">=1, <3")),
            ("c", &requires("y", "^1")),
        ],
    );
    f.add("x", "1.0.0", "");
    f.add("y", "1.0.0", "");
    f.apply(".", &[]);
    f.add("x", "2.0.0", "");
    f.add("y", "1.5.0", "");
    let sibling = std::fs::read(f.root.join("b/vibe.toml")).unwrap();
    let lock = f.apply("a", &["org.constraints/x@=2.0.0"]);
    assert_eq!(
        lock.packages
            .iter()
            .find(|p| p.name == "x")
            .unwrap()
            .version
            .to_string(),
        "2.0.0"
    );
    assert_eq!(
        lock.packages
            .iter()
            .find(|p| p.name == "y")
            .unwrap()
            .version
            .to_string(),
        "1.0.0"
    );
    assert_eq!(std::fs::read(f.root.join("b/vibe.toml")).unwrap(), sibling);
    assert!(
        Manifest::read(f.root.join("vibe.toml"))
            .unwrap()
            .requires
            .packages
            .is_empty()
    );
    assert_eq!(lock.meta.root_dependencies.len(), 2);
    assert!(matches!(f.plan(".", &[]).unwrap(), Plan::Fresh));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INSTALL-CONSTRAINT-RETRIES"
)]
fn conditional_resolve_preserves_base_consumer_intersection() {
    let root = requires("trigger", "^1");
    let mut f = Fixture::new(
        &root,
        &[
            ("a", &requires("x", ">=1, <3")),
            ("b", &requires("x", "<2")),
        ],
    );
    f.add("x", "1.5.0", "");
    f.add("x", "2.5.0", "");
    f.add("y", "1.0.0", "");
    f.add(
        "trigger",
        "1.0.0",
        "[target.\"context(flow:trigger)\".dependencies.packages]\n'org.constraints/y'='^1'\n",
    );
    let plan = f.plan(".", &[]).unwrap();
    assert_eq!(version(&plan, "x"), "1.5.0");
    assert_eq!(version(&plan, "y"), "1.0.0");
    assert!(f.source.solves.borrow().len() >= 2);
}

#[test]
fn conflicting_explicit_refs_refuse_and_identical_refs_coalesce() {
    let mut f = Fixture::new("", &[]);
    f.add("x", "1.0.0", "");
    f.add("x", "2.0.0", "");
    assert!(
        f.plan(".", &["org.constraints/x@^1", "org.constraints/x@^2"])
            .is_err()
    );
    assert!(
        f.source.solves.borrow().is_empty(),
        "conflicting request must fail before source calls"
    );
    let lock = f.apply(".", &["org.constraints/x@^1", "org.constraints/x@^1"]);
    assert_eq!(lock.meta.root_dependencies.len(), 1);
}

#[test]
fn held_pin_conflict_retries_every_authored_requirement() {
    let mut f = Fixture::new(
        "",
        &[
            ("a", &requires("x", "^1")),
            ("b", &requires("y", ">=1, <3")),
            ("c", &requires("y", "<2")),
        ],
    );
    f.add("x", "1.0.0", "");
    f.add("y", "1.0.0", "");
    f.apply(".", &[]);
    f.add("x", "2.0.0", &requires("y", ">=1.5"));
    f.add("y", "1.5.0", "");
    f.add("y", "2.5.0", "");
    f.source.solves.borrow_mut().clear();
    let plan = f.plan("a", &["org.constraints/x@=2.0.0"]).unwrap();
    assert_eq!(version(&plan, "y"), "1.5.0");
    let attempts = f.source.solves.borrow();
    assert_eq!(attempts.len(), 2, "held solve then full authored retry");
    assert_eq!(attempts[1].iter().filter(|p| p.name == "y").count(), 2);
}

#[test]
fn fresh_lock_checks_all_consumers_and_never_seeds_empty_root() {
    let mut f = Fixture::new(
        "",
        &[
            ("a", &requires("x", ">=1, <3")),
            ("b", &requires("x", "^1")),
        ],
    );
    f.add("x", "1.5.0", "");
    f.add("x", "2.5.0", "");
    let root_before = std::fs::read(f.root.join("vibe.toml")).unwrap();
    f.apply(".", &[]);
    f.source.solves.borrow_mut().clear();
    assert!(matches!(f.plan(".", &[]).unwrap(), Plan::Fresh));
    assert!(f.source.solves.borrow().is_empty());
    assert_eq!(
        std::fs::read(f.root.join("vibe.toml")).unwrap(),
        root_before
    );
    write(
        &f.root.join("b/vibe.toml"),
        &format!(
            "[project]\nname='b'\nversion='1.0.0'\n{}",
            requires("x", "^2")
        ),
    );
    let plan = f.plan(".", &[]).unwrap();
    assert_eq!(version(&plan, "x"), "2.5.0");
    assert!(!f.source.solves.borrow().is_empty());
    assert_eq!(
        std::fs::read(f.root.join("vibe.toml")).unwrap(),
        root_before
    );
}

#[test]
fn nested_version_vars_constrain_solve_and_only_selected_var_is_replaced() {
    let vars = "[workspace.versions]\ncore='>=1, <3'\n";
    let variable = "[requires.packages]\n'org.constraints/x'={version.var='core'}\n";
    let nested = "[workspace]\nmembers=['child']\n[workspace.versions]\ncore='<2'\n";
    let mut f = Fixture::new(vars, &[("a", nested), ("b", variable)]);
    write(
        &f.root.join("a/child/vibe.toml"),
        &format!("[project]\nname='child'\nversion='1.0.0'\ngroup='{GROUP}'\n{variable}"),
    );
    f.add("x", "1.5.0", "");
    f.add("x", "2.5.0", "");
    let root_before = std::fs::read(f.root.join("vibe.toml")).unwrap();
    let nested_before = std::fs::read(f.root.join("a/vibe.toml")).unwrap();
    let sibling_before = std::fs::read(f.root.join("b/vibe.toml")).unwrap();
    assert_eq!(version(&f.plan(".", &[]).unwrap(), "x"), "1.5.0");
    let lock = f.apply("a/child", &["org.constraints/x@=2.5.0"]);
    let child = Manifest::read(f.root.join("a/child/vibe.toml")).unwrap();
    assert!(child.requires.var_packages.is_empty());
    assert_eq!(child.requires.packages[0].version.to_string(), "=2.5.0");
    assert_eq!(lock.meta.root_dependencies.len(), 1);
    assert_eq!(lock.meta.root_dependencies[0].version.to_string(), "=2.5.0");
    assert_eq!(
        std::fs::read(f.root.join("vibe.toml")).unwrap(),
        root_before
    );
    assert_eq!(
        std::fs::read(f.root.join("a/vibe.toml")).unwrap(),
        nested_before
    );
    assert_eq!(
        std::fs::read(f.root.join("b/vibe.toml")).unwrap(),
        sibling_before
    );
    assert!(matches!(f.plan(".", &[]).unwrap(), Plan::Fresh));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INSTALL-CONSUMER-RECORDING"
)]
fn snapshot_is_unique_conservative_and_roundtrips_prerelease() {
    for (a, b, expected) in [
        ("^1.0", "^1.0", "^1.0"),
        (">=1, <3", "<2", "=1.5.0"),
        (">=1.5.0-alpha.1, <2", "=1.5.0-alpha.2", "=1.5.0-alpha.2"),
    ] {
        let mut f = Fixture::new("", &[("a", &requires("x", a)), ("b", &requires("x", b))]);
        f.add("x", "1.5.0-alpha.2", "");
        f.add("x", "1.5.0", "");
        let lock = f.apply(".", &[]);
        assert_eq!(lock.meta.root_dependencies.len(), 1);
        assert_eq!(lock.meta.root_dependencies[0].version.to_string(), expected);
        assert!(matches!(f.plan(".", &[]).unwrap(), Plan::Fresh));
    }
}

#[test]
fn explicit_latest_keeps_caret_and_exact_recording() {
    for (exact, expected) in [(false, "^1.2.3"), (true, "=1.2.3")] {
        let mut f = Fixture::new("", &[]);
        f.add("x", "1.2.3", "");
        let mut req = request(&["org.constraints/x"]);
        req.exact = exact;
        let lock = f.apply_request(".", req);
        let authored = Manifest::read(f.root.join("vibe.toml")).unwrap();
        assert_eq!(authored.requires.packages[0].version.to_string(), expected);
        assert_eq!(lock.meta.root_dependencies, authored.requires.packages);
        assert!(matches!(f.plan(".", &[]).unwrap(), Plan::Fresh));
    }
}

#[test]
fn missing_selected_node_refuses_before_legacy_migration() {
    let f = Fixture::new("", &[]);
    let mut lock = vibe_core::manifest::Lockfile::empty("test", "2026-11-04T00:00:00Z");
    lock.meta.root_dependencies = vec!["org.constraints/x@^1".parse().unwrap()];
    lock.write(f.root.join("vibe.lock")).unwrap();
    let mut workspace = vibe_workspace::Workspace::discover(&f.root).unwrap();
    let mut manifest = Manifest::read(f.root.join("vibe.toml")).unwrap();
    let before = files(&f.root);
    let result = vibe_install::plan_prepared_with_spec_format(
        &f.source,
        &f.root.join("missing"),
        &mut manifest,
        &mut workspace,
        request(&[]),
        vibe_core::manifest::SpecFormat::Mixed,
        &vibe_install::NullObserver,
    );
    assert!(matches!(
        result,
        Err(vibe_install::Error::SelectedConsumerMissing { .. })
    ));
    assert_eq!(files(&f.root), before);
    assert!(manifest.requires.packages.is_empty());
}

#[test]
fn ambiguous_historical_snapshot_never_seeds_last_wins() {
    let f = Fixture::new("", &[]);
    let mut lock = vibe_core::manifest::Lockfile::empty("test", "2026-11-04T00:00:00Z");
    lock.meta.root_dependencies = vec![
        "org.constraints/x@^1".parse().unwrap(),
        "org.constraints/x@^2".parse().unwrap(),
    ];
    lock.write(f.root.join("vibe.lock")).unwrap();
    let before = files(&f.root);
    assert!(matches!(
        f.plan(".", &[]),
        Err(vibe_install::Error::AmbiguousRootSnapshot)
    ));
    assert_eq!(files(&f.root), before);
    assert!(f.source.solves.borrow().is_empty());
}

#[test]
fn every_kind_assertion_is_checked_on_solve_and_fresh_paths() {
    let mut f = Fixture::new(&requires("x", "^1"), &[("a", &requires("x", "^1"))]);
    f.add("x", "1.0.0", "");
    f.apply(".", &[]);
    write(
        &f.root.join("a/vibe.toml"),
        "[project]\nname='a'\nversion='1.0.0'\n[requires.packages]\n'stack:org.constraints/x'='^1'\n",
    );
    let before = files(&f.root);
    for refs in [vec![], vec!["org.constraints/x@^1"]] {
        let error = match f.plan(".", &refs) {
            Err(error) => error,
            Ok(_) => panic!("a later consumer's kind must survive fresh and solved paths"),
        };
        assert!(matches!(
            error,
            vibe_install::Error::ConsumerConstraints {
                source: vibe_resolver::SolveError::KindMismatch { .. },
                ..
            }
        ));
        assert!(
            std::error::Error::source(&error)
                .unwrap()
                .is::<vibe_resolver::SolveError>(),
            "the CLI exit-code mapper must still see the typed solver cause"
        );
    }
    assert_eq!(files(&f.root), before);
}

#[test]
fn plain_version_recording_preserves_git_and_path_routes() {
    for source in [
        "{git='https://example.invalid/x', rev='abc', version='^1'}",
        "{path='../x', version='^1'}",
    ] {
        let mut manifest = Manifest::parse_str(&format!("[project]\nname='p'\nversion='1.0.0'\n[requires.packages]\n'org.constraints/x'={source}\n")).unwrap();
        let before = manifest.clone();
        assert!(!vibe_install::merge_manifest_requires(
            &mut manifest,
            &["org.constraints/x@=2.0.0".parse().unwrap()]
        ));
        assert_eq!(manifest.requires, before.requires);
    }
}

#[test]
fn unambiguous_legacy_snapshot_still_seeds_an_undeclared_workspace() {
    let mut f = Fixture::new("", &[]);
    f.add("x", "1.0.0", "");
    let mut lock = vibe_core::manifest::Lockfile::empty("test", "2026-11-04T00:00:00Z");
    let old: vibe_core::PackageRef = "flow:org.constraints/x@^1".parse().unwrap();
    lock.meta.root_dependencies = vec![old.clone(), old];
    lock.write(f.root.join("vibe.lock")).unwrap();
    let lock = f.apply(".", &[]);
    assert_eq!(lock.meta.root_dependencies.len(), 1);
    let manifest = Manifest::read(f.root.join("vibe.toml")).unwrap();
    assert_eq!(manifest.requires.packages, lock.meta.root_dependencies);
    assert!(matches!(f.plan(".", &[]).unwrap(), Plan::Fresh));
}

#[test]
fn conditional_pin_fallback_retains_every_consumer() {
    let mut f = Fixture::new(
        &requires("trigger", "^1"),
        &[
            ("a", &requires("x", ">=1, <3")),
            ("b", &requires("x", "<2")),
        ],
    );
    f.add("x", "1.0.0", "");
    f.add("trigger", "1.0.0", "");
    f.apply(".", &[]);
    f.add("x", "1.5.0", "");
    f.add("x", "2.5.0", "");
    f.add(
        "trigger",
        "2.0.0",
        "[target.\"context(flow:trigger)\".dependencies.packages]\n'org.constraints/y'='^1'\n",
    );
    f.add("y", "1.0.0", &requires("x", ">=1.5"));
    f.source.solves.borrow_mut().clear();
    let plan = f.plan(".", &["org.constraints/trigger@=2.0.0"]).unwrap();
    assert_eq!(version(&plan, "x"), "1.5.0");
    assert_eq!(version(&plan, "y"), "1.0.0");
    assert_eq!(
        f.source.solves.borrow().len(),
        3,
        "initial held success, conditional held conflict, complete free retry"
    );
}

#[test]
fn visibility_retry_retains_all_consumer_requirements() {
    let mut f = Fixture::new(
        &requires("bridge", "^1"),
        &[
            ("a", &requires("x", ">=1, <3")),
            ("b", &requires("x", "<2")),
        ],
    );
    f.add(
        "bridge",
        "1.0.0",
        "[requires.packages]\n'org.constraints/hidden'={version='^1', access='private'}\n",
    );
    f.add("hidden", "1.0.0", "");
    f.add("x", "1.5.0", "");
    f.add("x", "2.5.0", "");
    let plan = f.plan(".", &[]).unwrap();
    assert_eq!(version(&plan, "x"), "1.5.0");
    let attempts = f.source.solves.borrow();
    assert!(attempts.len() >= 2, "real visibility masking must re-solve");
    for roots in attempts.iter() {
        assert_eq!(roots.iter().filter(|r| r.name == "x").count(), 2);
    }
}

#[test]
fn source_declarations_prevent_legacy_seeding_without_registering_new_routes() {
    for source in [
        "{git='https://example.invalid/x', rev='abc'}",
        "{path='../x'}",
    ] {
        let f = Fixture::new(
            "",
            &[(
                "a",
                &format!("[requires.packages]\n'org.constraints/x'={source}\n"),
            )],
        );
        let mut lock = vibe_core::manifest::Lockfile::empty("test", "2026-11-04T00:00:00Z");
        lock.meta.root_dependencies = vec!["org.constraints/old@^1".parse().unwrap()];
        lock.write(f.root.join("vibe.lock")).unwrap();
        let before = files(&f.root);
        // The existing path-only empty-plan/source-registration debt is outside
        // this test; neither result permits copying the old roots into host.
        let _outcome = f.plan(".", &[]);
        assert_eq!(files(&f.root), before);
        assert!(
            Manifest::read(f.root.join("vibe.toml"))
                .unwrap()
                .requires
                .packages
                .is_empty()
        );
        assert!(
            f.source
                .solves
                .borrow()
                .iter()
                .flatten()
                .all(|r| r.name != "old")
        );
    }
}
