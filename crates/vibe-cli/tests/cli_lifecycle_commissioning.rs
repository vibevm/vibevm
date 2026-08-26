//! R2.8 commissioning: an installed phase announcer composes with a real
//! selected-stack preset, freshness, disable controls, and exhaustive query.

mod common;
#[path = "lifecycle_commissioning/support.rs"]
mod support;
#[path = "lifecycle_commissioning/world.rs"]
mod world;

use std::collections::BTreeMap;
use std::fs;

use common::UserScratch;
use specmark::verifies;
use support::*;
use vibe_core::manifest::Manifest;
use vibe_wire::generated::extensions_report::{
    Handler, ManifestKind, PackageKind, ProviderSource, SelectorSubjectKind, State, Tier,
};
use world::*;

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-LOG-PLUGIN")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
fn phase_announcer_owner_scenario_is_green_end_to_end() {
    let epoch_a_user = UserScratch::new();
    let scenario = tempfile::tempdir().unwrap();
    let epoch_a_registry = trusted_registry(scenario.path(), "epoch-a-registry");
    let epoch_a = create_epoch_a(&epoch_a_user, scenario.path(), &epoch_a_registry);
    let registry = fresh_epoch_b_registry(&epoch_a, scenario.path());

    let user = UserScratch::new();
    assert_fresh_user(&user);
    let project = create_epoch_b(&epoch_a, scenario.path());
    assert_fresh_user(&user);
    let project_name = Manifest::read(project.join("vibe.toml"))
        .unwrap()
        .project
        .unwrap()
        .name;

    let first = user
        .vibe()
        .args(["test", "--path"])
        .arg(&project)
        .arg("--registry")
        .arg(&registry)
        .output()
        .unwrap();
    assert!(
        first.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    let human = String::from_utf8(first.stdout).unwrap();
    let narration = [
        format!(
            "  → will run `{CARGO_BUILD}` — point=phase:build, handler=script, provider={STACK} tier=preset"
        ),
        format!(
            "  → will run `{ANNOUNCE}` — point=phase:build, handler=builtin:log, provider={ANNOUNCER} tier=dependency"
        ),
        format!(
            "  → will run `{CARGO_TEST}` — point=phase:test, handler=script, provider={STACK} tier=preset"
        ),
        format!(
            "  → will run `{ANNOUNCE_TEST}` — point=phase:test, handler=builtin:log, provider={ANNOUNCER} tier=dependency"
        ),
    ];
    let outcomes = [
        format!("  → log [{ANNOUNCER}]: hello from build in {project_name} by {ANNOUNCER}"),
        format!("  → log [{ANNOUNCER}]: hello from test in {project_name} by {ANNOUNCER}"),
    ];
    let narration_at = narration
        .each_ref()
        .map(|line| exact_line_index(&human, line));
    let outcome_at = outcomes
        .each_ref()
        .map(|line| exact_line_index(&human, line));
    assert!(narration_at.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(narration_at[3] < outcome_at[0]);
    assert!(outcome_at[0] < outcome_at[1]);
    assert_real_binary(&project);
    assert_eq!(
        fs::read_to_string(project.join("target/commissioning-test-ran.txt")).unwrap(),
        "ran"
    );

    let (fresh_plan, fresh) = lifecycle_json(&user, &project, &registry);
    assert_plan_rows(&fresh_plan, &EXPECTED_FOUR);
    assert_report_rows(&fresh, &EXPECTED_FOUR);
    assert_eq!(
        steps(&fresh),
        [
            ("validate", "ok"),
            ("install", "fresh"),
            ("generate", "no-op"),
            ("build", "fresh"),
            ("test", "fresh"),
        ]
    );
    for row in &fresh.contributions {
        assert_eq!(row.status, "fresh");
        assert_optional_shape(row, None, None);
    }

    append_manifest(
        &project,
        &format!("\n[extensions]\ndisable = [\"{ANNOUNCE}\"]\n"),
    );
    let (disabled_plan, disabled_run) = lifecycle_json(&user, &project, &registry);
    let surviving = [EXPECTED_FOUR[0], EXPECTED_FOUR[2], EXPECTED_FOUR[3]];
    assert_plan_rows(&disabled_plan, &surviving);
    assert_report_rows(&disabled_run, &surviving);
    assert_eq!(
        steps(&disabled_run),
        [
            ("validate", "ok"),
            ("install", "fresh"),
            ("generate", "no-op"),
            ("build", "ok"),
            ("test", "ok"),
        ]
    );
    assert!(
        !disabled_run
            .contributions
            .iter()
            .any(|row| row.key == ANNOUNCE)
    );
    for row in &disabled_run.contributions {
        assert_eq!(row.status, "ok");
        let expected_message = (row.key == ANNOUNCE_TEST)
            .then(|| format!("hello from test in {project_name} by {ANNOUNCER}"));
        let expected_stdout = (row.key == CARGO_TEST).then_some("test result: ok");
        assert_optional_shape(row, expected_message.as_deref(), expected_stdout);
    }

    let before = machine_snapshot(&user, scenario.path(), &project);
    let extensions = query(&user, &project);
    let after = machine_snapshot(&user, scenario.path(), &project);
    assert_eq!(after, before);
    assert_eq!(extensions.command, "extensions");
    assert!(extensions.ok);
    assert_eq!(extensions.project.identity, "__host__/owner-scenario");
    assert_eq!(extensions.project.manifest_kind, ManifestKind::Project);
    assert_eq!(extensions.project.version, "0.0.1");
    assert_eq!(
        extensions.project.root,
        vibe_core::machine_json_path(&project)
    );
    assert_eq!(extensions.project.effective_stack.as_deref(), Some(STACK));
    assert_eq!(
        extensions.selector_subject.kind,
        SelectorSubjectKind::Unscoped
    );
    assert!(extensions.selector_subject.package.is_none());
    assert!(extensions.selector_subject.path.is_none());
    assert_eq!(extensions.count, 4);
    assert_eq!(extensions.effective_count, 3);
    assert!(extensions.notices.is_empty());
    assert_eq!(
        extensions
            .declarations
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        [CARGO_BUILD, CARGO_TEST, ANNOUNCE, ANNOUNCE_TEST]
    );

    let stack_root = vibe_core::machine_json_path(&project.join(common::slot_dir(
        "org.vibevm.fixture.lifecycle-rust-stack",
        "0.1.0",
    )));
    for (sequence, key, id, point, declaration, base, inputs) in [
        (
            0,
            CARGO_BUILD,
            "cargo-build",
            "phase:build",
            0,
            "scripts/build",
            vec!["Cargo.toml", "src/**"],
        ),
        (
            1,
            CARGO_TEST,
            "cargo-test",
            "phase:test",
            1,
            "scripts/test",
            vec!["Cargo.toml", "src/**", "tests/**"],
        ),
    ] {
        let row = report_row(&extensions, key);
        assert_eq!(
            (row.id.as_str(), row.point.as_str(), row.sequence),
            (id, point, sequence)
        );
        assert_eq!(
            (
                row.order.provider,
                row.order.declaration,
                row.order.activation
            ),
            (Some(1), declaration, None)
        );
        assert_eq!(
            (
                row.natural_tier.clone(),
                row.tier.clone(),
                row.state.clone()
            ),
            (Tier::Preset, Tier::Preset, State::Effective)
        );
        let Handler::Script(handler) = &row.handler else {
            panic!("expected script: {row:?}")
        };
        assert_eq!(handler.base, base);
        assert_eq!(
            row.inputs
                .as_ref()
                .map(|items| items.iter().map(String::as_str).collect::<Vec<_>>()),
            Some(inputs)
        );
        assert!(row.authored_config.is_none() && row.effective_config.is_none());
        assert_eq!(
            (
                row.authored_auto,
                row.auto,
                row.activated,
                row.disabled,
                row.selector_matches,
                row.effective
            ),
            (None, true, false, false, true, true)
        );
        assert!(
            row.applies_to.is_none()
                && !row.compiler_internals
                && row.pass.is_none()
                && row.when.is_none()
                && row.native.is_none()
        );
        assert_eq!(
            (
                row.provider.source.clone(),
                row.provider.identity.as_str(),
                row.provider.kind.clone(),
                row.provider.version.as_str()
            ),
            (
                ProviderSource::Dependency,
                STACK,
                Some(PackageKind::Stack),
                "0.1.0"
            )
        );
        assert_eq!(row.provider.root.as_deref(), Some(stack_root.as_str()));
        assert!(row.provider.content_hash.is_some());
    }

    let announcer_root = vibe_core::machine_json_path(&project.join(common::slot_dir(
        "org.vibevm.fixture.phase-announcer",
        "0.1.0",
    )));
    let expected_config = BTreeMap::from([(
        "message".to_string(),
        Some(serde_json::Value::String(MESSAGE.to_string())),
    )]);
    for (sequence, key, id, point, declaration, disabled, state, effective) in [
        (
            2,
            ANNOUNCE,
            "announce",
            "phase:build",
            0,
            true,
            State::Disabled,
            false,
        ),
        (
            3,
            ANNOUNCE_TEST,
            "announce-test",
            "phase:test",
            1,
            false,
            State::Effective,
            true,
        ),
    ] {
        let row = report_row(&extensions, key);
        assert_eq!(
            (row.id.as_str(), row.point.as_str(), row.sequence),
            (id, point, sequence)
        );
        assert_eq!(
            (
                row.order.provider,
                row.order.declaration,
                row.order.activation
            ),
            (Some(0), declaration, None)
        );
        assert_eq!(
            (
                row.natural_tier.clone(),
                row.tier.clone(),
                row.state.clone()
            ),
            (Tier::Dependency, Tier::Dependency, state)
        );
        let Handler::Builtin(handler) = &row.handler else {
            panic!("expected builtin: {row:?}")
        };
        assert_eq!(handler.name, "log");
        assert_eq!(row.authored_config.as_ref(), Some(&expected_config));
        assert_eq!(row.effective_config.as_ref(), Some(&expected_config));
        assert_eq!(
            (
                row.authored_auto,
                row.auto,
                row.activated,
                row.disabled,
                row.selector_matches,
                row.effective
            ),
            (None, true, false, disabled, true, effective)
        );
        assert!(
            row.inputs.is_none()
                && row.applies_to.is_none()
                && !row.compiler_internals
                && row.pass.is_none()
                && row.when.is_none()
                && row.native.is_none()
        );
        assert_eq!(
            (
                row.provider.source.clone(),
                row.provider.identity.as_str(),
                row.provider.kind.clone(),
                row.provider.version.as_str()
            ),
            (
                ProviderSource::Dependency,
                ANNOUNCER,
                Some(PackageKind::Tool),
                "0.1.0"
            )
        );
        assert_eq!(row.provider.root.as_deref(), Some(announcer_root.as_str()));
        assert!(row.provider.content_hash.is_some());
    }
}
