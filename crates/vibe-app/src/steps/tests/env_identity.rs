//! Programmatically supplied source and plan inputs cannot bypass export checks.
use super::*;
use crate::operations::{Command, OperationContext};
use crate::steps::run::{Run, RunFlags};
use crate::steps::{ParamValue, Start, StepPaths};

#[test]
fn programmatic_option_refuses_before_feature_prompts() {
    let root = tempfile::tempdir().unwrap();
    let mut resolved = package(root.path(), BUILD);
    resolved.options[0].features[0].id = "a-b".into();
    resolved.options[0].features[1].id = "a.b".into();
    let never =
        |_: &str, _: &[crate::select::Choice], _: &[bool]| -> Result<Option<Vec<usize>>, String> {
            panic!("invalid projection must refuse before prompting")
        };
    let mut attended = host();
    attended.attended = true;
    let error = run(root.path(), &resolved, &attended, Some(&never)).unwrap_err();
    let text = error.to_string();
    assert!(
        text.contains("a-b") && text.contains("a.b") && text.contains("VIBE_FEATURE_A_B"),
        "{text}"
    );
    assert!(!root.path().join("settings").exists());
}

#[test]
fn runtime_refuses_programmatic_plan_collisions_without_values_or_work_publication() {
    let root = tempfile::tempdir().unwrap();
    let resolved = package(root.path(), BUILD);
    let base = run(root.path(), &resolved, &host(), None).unwrap();
    let settings = SettingsRoot::new(&root.path().join("settings"));
    let progress = vibe_core::progress::Progress::default();
    let ctx = OperationContext {
        http_client: None,
        import_scope: None,
        settings: &settings,
        json: false,
        offline: true,
        chooser: None,
        proxy_image: Path::new("unused-proxy"),
        facts: HostFacts::default(),
        feature_chooser: None,
        parameter_prompt: None,
        license_prompt: None,
        on_plan: None,
        on_notice: None,
        progress: &progress,
        quiet_children: true,
        panel: None,
        on_step_log: None,
        interrupt: None,
        agent: AgentHooks::default(),
        resolver: None,
        resolver_scope: crate::ResolverScope::Caller,
        source_reader: None,
        environment: None,
        search_path: None,
    };
    for parameters in [false, true] {
        let mut plan = base.clone();
        if parameters {
            plan.parameters = vec![
                ParamValue {
                    id: "a-b".into(),
                    value: "SYN_FIRST_SECRET".into(),
                    secret: true,
                },
                ParamValue {
                    id: "a.b".into(),
                    value: "SYN_SECOND_SECRET".into(),
                    secret: true,
                },
            ];
        } else {
            plan.declared[0].id = "a-b".into();
            plan.declared[1].id = "a.b".into();
        }
        let result = Run::new(
            &ctx,
            &resolved,
            &plan,
            RunFlags {
                operation: None,
                command: Command::Install,
                start: Start::Fresh,
                force: false,
                update_in_place: false,
                replaces: None,
                paths: StepPaths::default(),
            },
        );
        let error = match result {
            Err(e) => e,
            Ok(_) => panic!("ambiguous plan admitted"),
        };
        let text = format!("{error}\n{error:?}");
        assert!(
            text.contains("plan environment projection")
                && text.contains("a-b")
                && text.contains("a.b"),
            "{text}"
        );
        assert!(!text.contains("SYN_FIRST_SECRET") && !text.contains("SYN_SECOND_SECRET"));
        assert!(!settings.path().exists());
        for step in &mut plan.steps {
            step.step.kind = vibe_core::manifest::StepKind::Copy {
                from: "input".into(),
                to: "output".into(),
            };
        }
        assert!(
            Run::new(
                &ctx,
                &resolved,
                &plan,
                RunFlags {
                    operation: None,
                    command: Command::Install,
                    start: Start::Fresh,
                    force: false,
                    update_in_place: false,
                    replaces: None,
                    paths: StepPaths::default(),
                }
            )
            .is_ok(),
            "a plan without an environment consumer keeps literal IDs"
        );
    }
}
