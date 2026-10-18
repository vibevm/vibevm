//! G2 command exposure: actual isolated vibe child, never thread-local faults.
mod common;
#[path = "../../vibe-install/tests/support/g2.rs"]
mod fixture;
use common::hook_slot::{self, Fixture as HookFixture};
use std::fs;
use vibe_core::manifest::{Lockfile, Manifest};

fn classified_hooks(registry: &std::path::Path, mode: HookFixture) {
    let package = registry.join("org.example/hooked/v0.1.0");
    let path = package.join("vibe.toml");
    let text = fs::read_to_string(&path).unwrap();
    let mut text = text.split_once("\n[hooks]").unwrap().0.to_owned();
    let hooks: &[(&str, &str)] = match mode {
        HookFixture::Timing => &[("pre", "pre"), ("post", "post")],
        HookFixture::PreFail => &[("pre", "fail")],
        HookFixture::PostFail => &[("post", "fail")],
    };
    for (point, script) in hooks {
        text.push_str(&format!("\n[[extension]]\nid='g2-{point}'\npoint='slot:{point}-install'\nhandler={{kind='script',base='hooks/{script}'}}\neffects=['build-artifact']\napplies_to={{packages=['org.example/hooked']}}\n"));
    }
    fs::write(path, text).unwrap();
    fs::write(
        package.join("boot/generated.md"),
        "# Generated fixture boot\n",
    )
    .unwrap();
}

fn install(user: &common::UserScratch, f: &fixture::Fixture) -> std::process::Output {
    user.vibe()
        .args(["--json", "--offline", "install", "--assume-yes", "--path"])
        .arg(&f.root)
        .arg("--registry")
        .arg(&f.registry)
        .output()
        .unwrap()
}

#[test]
fn cli_no_input_is_typed_and_leaves_product_state_unchanged() {
    let f = fixture::Fixture::new(None);
    let user = common::UserScratch::new();
    let manifest = fs::read(f.root.join("vibe.toml")).unwrap();
    let output = install(&user, &f);
    assert!(!output.status.success());
    let wire = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(wire.contains("continuity"), "{wire}");
    assert_eq!(fs::read(f.root.join("vibe.toml")).unwrap(), manifest);
    assert!(!f.root.join("vibe.lock").exists());
    assert!(!f.root.join(common::deps_root()).exists());
}

#[test]
fn cli_configured_removal_without_new_authority_refuses_and_retains_shared_roots() {
    for branch in ["mup", "wal"] {
        let f = fixture::Fixture::new(Some(branch));
        let user = common::UserScratch::new();
        let output = install(&user, &f);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let lock = Lockfile::read(f.root.join("vibe.lock")).unwrap();
        assert_eq!(lock.meta.root_dependencies.len(), 3);
        let before = fixture::image(&f.root);
        let output = user
            .vibe()
            .args([
                "--json",
                "uninstall",
                "org.vibevm.world/wal",
                "--assume-yes",
                "--path",
            ])
            .arg(f.root.join("member"))
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("configured removal requires a newly admitted authored request")
        );
        assert_eq!(fixture::image(&f.root), before);
        let lock = Lockfile::read(f.root.join("vibe.lock")).unwrap();
        assert!(
            lock.packages
                .iter()
                .any(|p| p.name == "multi-user-planning")
        );
        assert!(lock.packages.iter().any(|p| p.name == "wal"));
        let root = Manifest::read(f.root.join("vibe.toml")).unwrap();
        assert_eq!(
            root.consumer_configuration[0].selections["continuity"].members,
            Some(vec![branch.into()])
        );
        let index = fixture::boot_text(&f.root);
        assert!(index.contains("12-flow-multi-user-planning"));
        assert!(index.contains("10-flow-wal"));
    }
}

#[test]
fn cli_hook_success_activates_real_contribution_before_boot_and_after_lock() {
    let (_outer, user, project, registry) = hook_slot::setup(HookFixture::Timing);
    classified_hooks(&registry, HookFixture::Timing);
    let output = hook_slot::install(&user, project.path(), &registry);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let slot = project
        .path()
        .join(common::slot_dir("org.example.hooked", "0.1.0"));
    assert_eq!(
        fs::read_to_string(slot.join("hook-order.txt")).unwrap(),
        "pre\npost\n"
    );
    assert!(
        fs::read_to_string(project.path().join(common::index_rel()))
            .unwrap()
            .contains("generated.md")
    );
    let docs = hook_slot::documents(&output.stdout);
    let rows = hook_slot::slot_outcomes(&docs);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r["status"] == "ok"));
    let recovery = project
        .path()
        .join(vibe_workspace::configuration_transaction::RECOVERY_DIRECTORY);
    assert_eq!(fs::read_dir(recovery).unwrap().count(), 0);
}

#[test]
fn cli_failed_pre_and_post_hooks_retain_typed_effect_dispositions() {
    for pre in [true, false] {
        let (_outer, user, project, registry) = hook_slot::setup(if pre {
            HookFixture::PreFail
        } else {
            HookFixture::PostFail
        });
        classified_hooks(
            &registry,
            if pre {
                HookFixture::PreFail
            } else {
                HookFixture::PostFail
            },
        );
        let before_image = fixture::image(project.path());
        let lock_before = fs::read(project.path().join("vibe.lock")).unwrap();
        let output = hook_slot::install(&user, project.path(), &registry);
        assert_eq!(output.status.success(), !pre);
        let docs = hook_slot::documents(&output.stdout);
        let rows = hook_slot::slot_outcomes(&docs);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["status"], "fail");
        assert!(rows[0]["stdout"].as_str().unwrap().contains("SOFT-STDOUT"));
        assert!(rows[0]["stderr"].as_str().unwrap().contains("SOFT-STDERR"));
        assert_eq!(rows[0]["slot_target"]["name"], "hooked");
        assert_eq!(
            rows[0]["point"],
            if pre {
                "slot:pre-install"
            } else {
                "slot:post-install"
            }
        );
        if pre {
            assert_eq!(
                fs::read(project.path().join("vibe.lock")).unwrap(),
                lock_before
            );
        } else {
            assert_eq!(rows[0]["flagged"], true);
        }
        eprintln!(
            "CLI pre={pre}: exit={:?}, typed rows={rows:?}",
            output.status.code()
        );
        let recovery = project
            .path()
            .join(vibe_workspace::configuration_transaction::RECOVERY_DIRECTORY);
        let retained = fs::read_dir(&recovery)
            .map(|entries| entries.count())
            .unwrap_or(0);
        if retained == 0 {
            // Preserve the REAL failed fixture and observations. This file is
            // not a fabricated transaction journal or a recovery authority.
            let image = fixture::image(project.path());
            let kept = project.keep();
            let observation = serde_json::json!({
                "observation_kind":"missing-retained-hook-failure-journal",
                "pre":pre,"exit":output.status.code(),"documents":docs,
                "stderr":String::from_utf8_lossy(&output.stderr),
                "before_lock":lock_before,"before_image":before_image,"current_image":image,
                "recovery_entries":retained,
            });
            fs::write(
                kept.join("g2-failure-observation.json"),
                serde_json::to_vec_pretty(&observation).unwrap(),
            )
            .unwrap();
            panic!(
                "failed hook has no retained configuration journal; actual fixture and CLI evidence kept at {}",
                kept.display()
            );
        }
        let (_, journal_path, journal) = fixture::journal(project.path());
        assert_eq!(
            journal["phase"],
            if pre { "slot" } else { "post-install-failed" }
        );
        assert_eq!(journal["external_effects_possible"], true);
        assert_eq!(journal["in_flight"], true);
        let state = hook_slot::lifecycle_state(project.path());
        let key = hook_slot::state_key(if pre { "g2-pre" } else { "g2-post" });
        assert_eq!(
            state.execution[&key].status,
            vibe_wire::generated::lifecycle_state::ExecutionRecordStatus::Fail
        );
        assert!(journal_path.exists());
    }
}

#[test]
fn cli_unclassified_legacy_sugar_refuses_with_recovery_evidence() {
    let (_outer, user, project, registry) = hook_slot::setup(HookFixture::Timing);
    let lock = fs::read(project.path().join("vibe.lock")).unwrap();
    let output = hook_slot::install(&user, project.path(), &registry);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported-effect-classification"));
    assert_eq!(fs::read(project.path().join("vibe.lock")).unwrap(), lock);
    let (_, path, journal) = fixture::journal(project.path());
    assert_eq!(journal["phase"], "slot");
    assert_eq!(journal["in_flight"], true);
    assert!(path.is_file());
}

#[test]
fn cli_supported_removal_retains_shared_dependency_until_last_consumer() {
    let f = fixture::Fixture::new(Some("mup"));
    let mut root = Manifest::read(f.root.join("vibe.toml")).unwrap();
    root.consumer_configuration.clear();
    root.requires.packages = vec!["org.vibevm.world/wal@=1.1.0".parse().unwrap()];
    root.write(f.root.join("vibe.toml")).unwrap();
    let user = common::UserScratch::new();
    let output = install(&user, &f);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for member in [true, false] {
        let output = user
            .vibe()
            .args([
                "--json",
                "uninstall",
                "org.vibevm.world/wal",
                "--assume-yes",
                "--path",
            ])
            .arg(if member {
                f.root.join("member")
            } else {
                f.root.clone()
            })
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let lock = Lockfile::read(f.root.join("vibe.lock")).unwrap();
        assert_eq!(lock.packages.iter().any(|p| p.name == "wal"), member);
        assert!(
            lock.packages
                .iter()
                .any(|p| p.name == "multi-user-planning")
        );
        assert_eq!(fixture::boot_text(&f.root).contains("10-flow-wal"), member);
        assert!(fixture::boot_text(&f.root).contains("12-flow-multi-user-planning"));
    }
}
