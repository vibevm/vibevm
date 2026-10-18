//! G2 public plan/apply, exact replay and retained recovery matrix.
#[path = "support/g2.rs"]
mod fixture;
use fixture::Fixture;
use std::collections::BTreeSet;
use vibe_core::manifest::{Lockfile, Manifest, SpecFormat};
use vibe_install::{InstallSource, NullObserver};
use vibe_workspace::configuration_transaction::{
    ConfigurationTransaction, with_before_boundary_fault, with_boundary_fault,
};

// The prepared replay entry may inspect retained authority, but must never
// ask the solver/provider to acquire a different graph or payload.
struct ReplayOnly<'a>(&'a dyn InstallSource);
impl InstallSource for ReplayOnly<'_> {
    fn normalized_request(
        &self,
    ) -> Option<&vibe_resolver::resolvo_engine::request::NormalizedRequest> {
        self.0.normalized_request()
    }
    fn catalog_inspection(&self) -> Option<&vibe_resolver::resolvo_engine::CatalogInspection> {
        self.0.catalog_inspection()
    }
    fn solve(
        &self,
        _: &[vibe_core::PackageRef],
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        panic!("exact replay must not solve")
    }
    fn manifest_of(
        &self,
        _: &vibe_core::PackageRef,
    ) -> Result<Manifest, vibe_resolver::SolveError> {
        panic!("exact replay must not query provider metadata")
    }
    fn solve_masked(
        &self,
        _: &[vibe_core::PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        panic!("exact replay must not solve masked")
    }
    fn resolve_and_fetch(
        &self,
        _: &vibe_core::PackageRef,
        _: &std::path::Path,
        _: Option<&str>,
    ) -> Result<vibe_registry::CachedPackage, vibe_registry::RegistryError> {
        panic!("exact replay must not fetch")
    }
    fn materialise_in_place(
        &self,
        _: &vibe_core::PackageRef,
        _: &std::path::Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, vibe_registry::RegistryError> {
        panic!("exact replay must not mutate in place")
    }
}

#[test]
fn redbook_coexists_switches_dormant_contributions_and_replays_exactly() {
    let _store = fixture::store_guard();
    let _home = vibe_test_support::isolated_home().unwrap();
    let f = Fixture::new(Some("mup"));
    let original_definition =
        std::fs::read(f.registry.join("org.vibevm.world/redbook/v1.1.0/vibe.toml")).unwrap();
    let mut initial = None;
    for branch in ["mup", "wal", "mup"] {
        let mut m = Manifest::read(f.root.join("vibe.toml")).unwrap();
        m.consumer_configuration[0]
            .selections
            .get_mut("continuity")
            .unwrap()
            .members = Some(vec![branch.into()]);
        m.write(f.root.join("vibe.toml")).unwrap();
        let src = f.source();
        let p = vibe_install::evolution::plan_reconfiguration(
            &src,
            &f.root,
            fixture::request(),
            SpecFormat::Mixed,
            &NullObserver,
        )
        .unwrap();
        let mut expected = fixture::packages::expected_names(
            branch,
            if branch == "mup" {
                &[
                    "health-audit",
                    "manual-tests",
                    "comparative-research",
                    "dev-runtime-docs",
                    "discovery-prompt",
                ]
            } else {
                &[
                    "health-audit",
                    "manual-tests",
                    "comparative-research",
                    "dev-runtime-docs",
                    "wal-specspaces",
                ]
            },
        );
        expected.extend(["wal".into(), "multi-user-planning".into()]);
        assert_eq!(
            p.resolution
                .iter()
                .map(|p| p.name.clone())
                .collect::<BTreeSet<_>>(),
            expected
        );
        let redbook = p.resolution.iter().find(|p| p.name == "redbook").unwrap();
        for (member, selected) in [
            ("wal", branch == "wal"),
            ("multi-user-planning", branch == "mup"),
        ] {
            assert_eq!(redbook.requires.iter().any(|(_, n)| n == member), selected);
            assert_eq!(
                p.selected_provenance
                    .iter()
                    .any(|e| e.selected.parent.name == "redbook"
                        && e.selected.provider.name == member),
                selected
            );
        }
        let retained = p.clone();
        let bound = src
            .bind_request(p.normalized_request.clone().unwrap())
            .unwrap();
        let record =
            vibe_install::requested_record(p.normalized_request.as_ref().unwrap()).unwrap();
        assert!(record.defaults.is_empty());
        if branch == "mup" {
            if let Some(old) = &initial {
                assert_eq!(old, &record);
            } else {
                initial = Some(record);
            }
        }
        fixture::apply(&src, p).unwrap();
        let lock = Lockfile::read(f.root.join("vibe.lock")).unwrap();
        assert_eq!(lock.meta.root_dependencies.len(), 3);
        assert!(
            lock.packages
                .iter()
                .all(|p| p.version == semver::Version::new(1, 1, 0))
        );
        let (_, resolved) = vibe_wire::behaviour::lock_configuration::decode(
            lock.meta.configuration.as_ref().unwrap(),
        )
        .unwrap();
        assert!(
            serde_json::to_string(&resolved.configurations[0].dormant)
                .unwrap()
                .contains(if branch == "mup" {
                    "wal-specspaces"
                } else {
                    "discovery-prompt"
                })
        );
        let index = fixture::boot_text(&f.root);
        for active in [
            "10-flow-wal",
            "12-flow-multi-user-planning",
            "03-flow-redbook",
        ] {
            assert!(index.contains(active), "{active}: {index}");
        }
        assert_eq!(
            index.contains("/boot/03b-member-wal-specspaces"),
            branch == "wal"
        );
        let replay = vibe_install::evolution::PreparedReinstall::from_planned(
            &lock,
            &ReplayOnly(&bound),
            &retained,
            bound.catalog_inspection().unwrap(),
        )
        .unwrap();
        let mut missing_capture = bound.catalog_inspection().unwrap().clone();
        missing_capture.candidates.clear();
        assert!(
            vibe_install::evolution::PreparedReinstall::from_planned(
                &lock,
                &ReplayOnly(&bound),
                &retained,
                &missing_capture
            )
            .is_err()
        );
        replay.validate(&lock).unwrap();
        replay.validate_slots(&f.root, SpecFormat::Mixed).unwrap();
        assert_eq!(
            replay.report.decision,
            vibe_install::evolution::Decision::ExactReuse
        );
        let replay_edges = replay
            .resolution()
            .into_iter()
            .map(|p| (p.name, p.requires))
            .collect::<Vec<_>>();
        assert_eq!(
            replay_edges,
            retained
                .resolution
                .iter()
                .map(|p| (p.name.clone(), p.requires.clone()))
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(
        std::fs::read(f.registry.join("org.vibevm.world/redbook/v1.1.0/vibe.toml")).unwrap(),
        original_definition
    );
    assert!(!f.registry.join("org.vibevm.world/redbook/v1.1.1").exists());
}

#[test]
fn missing_and_conflicting_selection_refuse_before_product_writes() {
    let _store = fixture::store_guard();
    let _home = vibe_test_support::isolated_home().unwrap();
    for conflicting in [false, true] {
        let f = Fixture::new(None);
        if conflicting {
            let mut m = Manifest::read(f.root.join("vibe.toml")).unwrap();
            let mut s = m.consumer_configuration[0].selections["additions"].clone();
            s.members = Some(vec!["mup".into(), "wal".into()]);
            m.consumer_configuration[0]
                .selections
                .insert("continuity".into(), s);
            m.write(f.root.join("vibe.toml")).unwrap();
        }
        let before = fixture::image(&f.root);
        let error = vibe_install::plan(&f.source(), &f.root, fixture::request(), &NullObserver)
            .unwrap_err();
        let vibe_install::Error::Solve(mut solve) = error else {
            panic!("{error:?}")
        };
        while let vibe_resolver::SolveError::RootConstraints { conflict, .. } = solve {
            solve = *conflict;
        }
        let vibe_resolver::SolveError::Configuration(configuration) = &solve else {
            panic!("{solve:?}")
        };
        use vibe_resolver::resolvo_engine::configuration::ConfigurationSolveError;
        assert!(
            if conflicting {
                matches!(
                    configuration.as_ref(),
                    ConfigurationSolveError::Conflict { .. }
                )
            } else {
                matches!(
                    configuration.as_ref(),
                    ConfigurationSolveError::NeedsSelection { .. }
                )
            },
            "{solve:?}"
        );
        assert!(solve.to_string().contains("continuity"), "{solve}");
        assert_eq!(fixture::image(&f.root), before);
    }
}

#[test]
fn independent_wal_mup_without_redbook_activate_both_real_boot_contributions() {
    let _store = fixture::store_guard();
    let _home = vibe_test_support::isolated_home().unwrap();
    let f = Fixture::new(Some("mup"));
    let mut manifest = Manifest::read(f.root.join("vibe.toml")).unwrap();
    manifest.requires.packages.clear();
    manifest.consumer_configuration.clear();
    manifest.write(f.root.join("vibe.toml")).unwrap();
    let p = f.plan();
    assert_eq!(
        p.resolution
            .iter()
            .map(|p| p.name.as_str())
            .collect::<BTreeSet<_>>(),
        ["wal", "multi-user-planning"].into()
    );
    fixture::apply(&f.source(), p).unwrap();
    let index = fixture::boot_text(&f.root);
    assert!(index.contains("10-flow-wal") && index.contains("12-flow-multi-user-planning"));
    let lock = Lockfile::read(f.root.join("vibe.lock")).unwrap();
    assert_eq!(lock.meta.root_dependencies.len(), 2);
}

#[test]
fn every_before_after_prefix_retains_identity_images_and_partial_recovery() {
    let _store = fixture::store_guard();
    let _home = vibe_test_support::isolated_home().unwrap();
    for phase in ["manifest", "slot", "boot", "lock", "post-install"] {
        for before_phase in [true, false] {
            eprintln!("starting {phase} before={before_phase}");
            let f = Fixture::new(Some("mup"));
            let src = f.source();
            let mut request = fixture::request();
            request.roots = vec!["org.vibevm.world/wal@=1.1.0".parse().unwrap()];
            let vibe_install::Plan::Ready(p) =
                vibe_install::plan(&src, &f.root, request, &NullObserver).unwrap()
            else {
                panic!("ready")
            };
            let p = *p;
            eprintln!("planned {phase} before={before_phase}");
            let slots = Fixture::slots(&p);
            let workspace = vibe_workspace::Workspace::discover(&f.root).unwrap();
            let before = fixture::image(&f.root);
            let call = || fixture::apply(&src, p);
            let error = if before_phase {
                with_before_boundary_fault(&f.root, phase, call)
            } else {
                with_boundary_fault(&f.root, phase, call)
            }
            .unwrap_err();
            eprintln!("interrupted {phase} before={before_phase}");
            assert!(
                matches!(error, vibe_install::Error::Workspace(_)),
                "{error:?}"
            );
            assert!(
                error.to_string().contains("injected interruption"),
                "{error}"
            );
            let (operation, journal_path, journal) = fixture::journal(&f.root);
            assert_eq!(journal["operation"], "install");
            assert_eq!(journal["phase"], phase);
            assert_eq!(journal["in_flight"], before_phase);
            assert_eq!(
                journal["before"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .collect::<Vec<_>>(),
                journal["checkpoint"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .collect::<Vec<_>>()
            );
            assert!(journal["before"].get("vibe.toml").is_some());
            let post = fixture::image(&f.root);
            if phase == "manifest" && before_phase {
                assert_eq!(before, post);
            }
            if phase == "manifest" && !before_phase {
                assert_ne!(
                    before[std::path::Path::new("vibe.toml")],
                    post[std::path::Path::new("vibe.toml")]
                );
            }
            assert!(ConfigurationTransaction::reopen(&workspace, &[], &operation).is_err());
            assert!(
                ConfigurationTransaction::reopen(&workspace, &slots, "operation-wrong").is_err()
            );
            let mut tx = ConfigurationTransaction::reopen(&workspace, &slots, &operation).unwrap();
            assert_eq!(tx.report()["complete_rollback"], false);
            assert_eq!(
                tx.report()["external_effects_possible"],
                phase != "manifest"
            );
            // Pending recovery cannot silently start another apply.
            let fresh = vibe_install::evolution::plan_reconfiguration(
                &src,
                &f.root,
                fixture::request(),
                SpecFormat::Mixed,
                &NullObserver,
            )
            .unwrap();
            assert!(fixture::apply(&src, fresh).is_err());
            assert_eq!(fixture::image(&f.root), post);
            for _ in 0..2 {
                match tx.restore_local_prefix() {
                    Ok(report) => {
                        assert_eq!(report["complete_rollback"], false);
                        assert_eq!(fixture::image(&f.root), before);
                    }
                    Err(error) => {
                        assert!(
                            error.to_string().contains("boot compensation"),
                            "{phase}/{before_phase}: {error}"
                        );
                        assert_eq!(fixture::image(&f.root), post);
                    }
                }
                assert!(journal_path.exists());
            }
            eprintln!(
                "{phase} before={before_phase}: retained {} disposition={}",
                operation,
                tx.report()
            );
        }
    }
}

#[test]
fn stale_workspace_and_third_state_edits_are_never_overwritten() {
    let _store = fixture::store_guard();
    let _home = vibe_test_support::isolated_home().unwrap();
    for target in ["vibe.toml", "member/vibe.toml", "vibe.lock"] {
        let f = Fixture::new(Some("wal"));
        let p = f.plan();
        let src = f.source();
        std::fs::write(f.root.join(target), b"third state writer").unwrap();
        let current = fixture::image(&f.root);
        let error = fixture::apply(&src, p).unwrap_err();
        assert!(matches!(error, vibe_install::Error::Workspace(_)));
        assert!(error.to_string().contains("changed since planning"));
        assert_eq!(fixture::image(&f.root), current);
    }
    let f = Fixture::new(Some("wal"));
    let p = f.plan();
    let slots = Fixture::slots(&p);
    let workspace = vibe_workspace::Workspace::discover(&f.root).unwrap();
    assert!(with_boundary_fault(&f.root, "slot", || fixture::apply(&f.source(), p)).is_err());
    let (operation, path, _) = fixture::journal(&f.root);
    let edited = f.root.join(&slots[0]).join("user-edit.txt");
    std::fs::write(&edited, b"third state slot").unwrap();
    let current = fixture::image(&f.root);
    let mut tx = ConfigurationTransaction::reopen(&workspace, &slots, &operation).unwrap();
    for _ in 0..2 {
        assert!(
            tx.restore_local_prefix()
                .unwrap_err()
                .to_string()
                .contains("third state")
        );
        assert_eq!(fixture::image(&f.root), current);
        assert!(path.exists());
    }
}

#[test]
fn offline_replay_refuses_changed_lock_source_payload_definition_and_slot() {
    let _store = fixture::store_guard();
    let _home = vibe_test_support::isolated_home().unwrap();
    for corruption in ["lock", "payload", "definition", "source", "slot"] {
        let f = Fixture::new(Some("mup"));
        let src = f.source();
        let p = f.plan();
        let bound = src
            .bind_request(p.normalized_request.clone().unwrap())
            .unwrap();
        fixture::apply(&src, p.clone()).unwrap();
        let mut lock = Lockfile::read(f.root.join("vibe.lock")).unwrap();
        let replay = vibe_install::evolution::PreparedReinstall::from_planned(
            &lock,
            &bound,
            &p,
            bound.catalog_inspection().unwrap(),
        )
        .unwrap();
        let captured = replay
            .resolution()
            .into_iter()
            .find(|p| p.name == "redbook")
            .unwrap();
        let definition_before = std::fs::read(captured.content_dir.join("vibe.toml")).unwrap();
        assert!(!captured.content_dir.join("tampered.txt").exists());
        match corruption {
            "lock" => lock.meta.generated_by.push_str(" changed"),
            "payload" => std::fs::write(
                captured.content_dir.join("tampered.txt"),
                b"changed payload",
            )
            .unwrap(),
            "definition" => std::fs::write(
                captured.content_dir.join("vibe.toml"),
                b"corrupt definition",
            )
            .unwrap(),
            "source" => {
                lock.packages
                    .iter_mut()
                    .find(|p| p.name == "redbook")
                    .unwrap()
                    .source_ref = Some("substituted".into())
            }
            "slot" => {
                // Replace a captured OWNED file, preserving the cache inode.
                // An additional unowned user file is not payload corruption.
                let slot = vibe_workspace::vibedeps::slot_abs_path(
                    &f.root,
                    &captured.group,
                    &captured.name,
                    &captured.version,
                );
                std::fs::write(
                    slot.join("g2-replacement.toml"),
                    b"third state owned manifest",
                )
                .unwrap();
                std::fs::rename(slot.join("g2-replacement.toml"), slot.join("vibe.toml")).unwrap();
            }
            _ => unreachable!(),
        }
        let current = fixture::image(&f.root);
        let result = if corruption == "slot" {
            replay.validate_slots(&f.root, SpecFormat::Mixed)
        } else {
            replay.validate(&lock)
        };
        let error = result.unwrap_err();
        eprintln!("{corruption}: retained replay refusal: {error}");
        assert_eq!(fixture::image(&f.root), current);
        // Explicit cleanup of this test's isolated cache mutation, AFTER the
        // refusal and unchanged-product oracle. This is not product recovery:
        // no apply succeeds against the corrupt bytes and no journal is cleared.
        if corruption == "payload" {
            std::fs::remove_file(captured.content_dir.join("tampered.txt")).unwrap();
        } else if corruption == "definition" {
            std::fs::write(captured.content_dir.join("vibe.toml"), definition_before).unwrap();
        }
    }
}

#[test]
fn source_corruption_between_plan_and_apply_refuses_without_product_writes() {
    let _store = fixture::store_guard();
    let _home = vibe_test_support::isolated_home().unwrap();
    let f = Fixture::new(Some("mup"));
    let src = f.source();
    let p = f.plan();
    let before = fixture::image(&f.root);
    std::fs::write(
        f.registry.join("org.vibevm.world/redbook/v1.1.0/vibe.toml"),
        b"corrupt source",
    )
    .unwrap();
    let error = fixture::apply(&src, p).unwrap_err();
    assert!(
        matches!(
            error,
            vibe_install::Error::Solve(_) | vibe_install::Error::Registry(_)
        ),
        "{error:?}"
    );
    assert_eq!(fixture::image(&f.root), before);
}

struct HookBoundary {
    before: bool,
    point: String,
}
impl vibe_install::SlotLifecycleObserver for HookBoundary {
    fn observe(&self, _: &vibe_install::SlotLifecyclePlan) -> Result<(), String> {
        if self.before && self.point == "pre-install" {
            Err("G2 before pre hook".into())
        } else {
            Ok(())
        }
    }
    fn outcome(&self, report: &vibe_install::SlotLifecycleReport) -> Result<(), String> {
        if !self.before
            && report.point == format!("slot:{}", self.point)
            && report.reference.starts_with("org.g2/hooked#")
        {
            Err(format!("G2 after {} hook", self.point))
        } else {
            Ok(())
        }
    }
}

#[test]
fn actual_pre_post_hook_prefixes_keep_external_effect_uncertainty() {
    let _store = fixture::store_guard();
    use std::sync::Arc;
    let _home = vibe_test_support::isolated_home().unwrap();
    for point in ["pre-install", "post-install"] {
        for before in [true, false] {
            let f = Fixture::new(Some("mup"));
            f.add_hook();
            let p = f.plan();
            let slots = Fixture::slots(&p);
            let workspace = vibe_workspace::Workspace::discover(&f.root).unwrap();
            let lease = Arc::new(vibe_lifecycle::LifecycleLease::acquire(&f.root).unwrap());
            let run = vibe_lifecycle::RunMetadata {
                requested: "install".into(),
                chain: vec!["install".into()],
                offline: true,
                assume_yes: true,
                agent_mode: vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Cli,
                force: false,
                trace_compile: false,
                run_id: vibe_lifecycle::process::allocate_run_id(&f.root).unwrap(),
                started: vibe_core::timestamp::now_utc(),
                selected: ".".into(),
            };
            let action = || {
                vibe_install::apply_with_spec_format_and_lifecycle_observed(
                    &f.source(),
                    p,
                    vibe_core::user_config::SlotIntegrity::TrustPresence,
                    SpecFormat::Mixed,
                    run,
                    vibe_lifecycle::process::StreamMode::Capture,
                    vibe_install::SlotLifecycleSeams {
                        observer: Arc::new(HookBoundary {
                            before,
                            point: point.into(),
                        }),
                        agent: Arc::new(vibe_lifecycle::NoAgentBackend),
                    },
                    lease,
                )
            };
            let result = if before && point == "post-install" {
                with_before_boundary_fault(&f.root, point, action)
            } else {
                action()
            };
            let error = result.unwrap_err();
            eprintln!("hook {point} before={before}: {error}");
            assert!(
                error
                    .to_string()
                    .contains(if before && point == "post-install" {
                        "injected interruption"
                    } else if before {
                        "G2 before pre hook"
                    } else if point == "pre-install" {
                        "G2 after pre-install hook"
                    } else {
                        "G2 after post-install hook"
                    }),
                "{error}"
            );
            assert!(
                matches!(
                    error,
                    vibe_install::Error::Workspace(_)
                        | vibe_install::Error::Lifecycle(_)
                        | vibe_install::Error::SlotFailed { .. }
                ),
                "{error:?}"
            );
            let (_, _, journal) = fixture::journal(&f.root);
            assert_eq!(
                journal["phase"],
                if point == "pre-install" {
                    "slot"
                } else {
                    "post-install"
                }
            );
            assert_eq!(journal["in_flight"], true);
            assert_eq!(journal["external_effects_possible"], true);
            let marker = f.root.join(if point == "pre-install" {
                "pre-effect"
            } else {
                "post-effect"
            });
            assert_eq!(marker.exists(), !before, "{point}/{before}");
            let (operation, path, _) = fixture::journal(&f.root);
            let mut tx = ConfigurationTransaction::reopen(&workspace, &slots, &operation).unwrap();
            match tx.restore_local_prefix() {
                Ok(report) => assert_eq!(report["complete_rollback"], false),
                Err(error) => assert!(
                    error.to_string().contains("third state")
                        || error.to_string().contains("boot compensation"),
                    "{error}"
                ),
            }
            assert_eq!(tx.report()["complete_rollback"], false);
            assert_eq!(tx.report()["external_effects_possible"], true);
            assert_eq!(marker.exists(), !before);
            assert!(path.is_file());
        }
    }
}
