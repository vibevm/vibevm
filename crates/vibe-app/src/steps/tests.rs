//! Planner proofs: features, conditions, needs and the up-front refusals.

use std::cell::RefCell;
use std::fs;
use std::path::Path;

use vibe_core::manifest::Manifest;

use super::features::FeatureRequest;
use super::plan::{FeatureChooser, HostFacts, PlanInput, Previous, Prompts, plan};
use crate::agents::{AgentHooks, AgentsConfig, SelectedAgent, select_agent};
use crate::error::{AppError, StepError};
use crate::index::PackageCoordinate;
use crate::resolve::{ResolvedPackage, from_manifest};
use crate::settings::SettingsRoot;

fn package(dir: &Path, steps: &str) -> ResolvedPackage {
    let text = format!(
        r#"[package]
name = "tool"
group = "org.example"
kind = "app"
version = "1.0.0"
[application]
id = "tool"
[[installer_options]]
id = "main"
name = "Main"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"
[[installer_options.features]]
id = "fixes"
name = "Fixes"
[[installer_options.features]]
id = "docs"
name = "Docs"
default = true
{steps}"#
    );
    let manifest = Manifest::parse_str(&text).unwrap();
    let coordinate = PackageCoordinate {
        group: "org.example".into(),
        name: "tool".into(),
        version: "1.0.0".into(),
    };
    from_manifest(manifest, dir.to_path_buf(), coordinate).unwrap()
}

const BUILD: &str = "[[installer_options.steps]]\nid = \"build\"\ntitle = \"Build\"\nkind = \"recipe\"\nscript = \"b.sh\"\n";

fn on_path(name: &str) -> bool {
    name == "make"
}

fn env(name: &str) -> bool {
    name == "CI"
}

struct Host {
    offline: bool,
    attended: bool,
    request: FeatureRequest,
    previous: Option<Vec<String>>,
    agent: Option<SelectedAgent>,
    hosted: bool,
}

fn host() -> Host {
    Host {
        offline: false,
        attended: false,
        request: FeatureRequest::default(),
        previous: None,
        agent: None,
        hosted: false,
    }
}

fn run(
    dir: &Path,
    resolved: &ResolvedPackage,
    host: &Host,
    chooser: Option<FeatureChooser<'_>>,
) -> Result<super::Plan, AppError> {
    let settings = SettingsRoot::new(&dir.join("settings"));
    let parameters = Default::default();
    plan(&PlanInput {
        resolved,
        option: &resolved.options[0],
        request: &host.request,
        parameters: &[],
        accept_license: false,
        previous: host.previous.as_deref().map(|features| Previous {
            features,
            parameters: &parameters,
            license: None,
            ask: false,
        }),
        settings: &settings,
        facts: HostFacts {
            command_present: &on_path,
            env_present: &env,
            ..HostFacts::default()
        },
        offline: host.offline,
        attended: host.attended,
        prompts: Prompts {
            features: chooser,
            ..Prompts::default()
        },
        agent: AgentHooks {
            selected: host.agent.as_ref(),
            hosted: host.hosted,
            ..AgentHooks::default()
        },
        requirements: None,
    })
}

fn actions(plan: &super::Plan) -> Vec<(String, bool, Option<String>)> {
    plan.steps
        .iter()
        .map(|step| (step.step.id.clone(), step.run, step.reason.clone()))
        .collect()
}

#[test]
fn conditions_decide_run_or_skip_with_the_deciding_condition() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    let steps = format!(
        "{BUILD}\
         [[installer_options.steps]]\nid = \"fix\"\ntitle = \"Fix\"\nkind = \"shell\"\ncommand = \"x\"\nwhen = {{ feature = \"fixes\" }}\n\
         [[installer_options.steps]]\nid = \"doc\"\ntitle = \"Doc\"\nkind = \"copy\"\nfrom = \"docs\"\nwhen = [{{ feature = \"docs\" }}, {{ file_present = \"$package/docs\" }}, {{ command_present = \"make\" }}]\n\
         [[installer_options.steps]]\nid = \"ci\"\ntitle = \"CI\"\nkind = \"shell\"\ncommand = \"x\"\nunless = {{ env_present = \"CI\" }}\n\
         [[installer_options.steps]]\nid = \"net\"\ntitle = \"Net\"\nkind = \"shell\"\ncommand = \"x\"\nunless = [{{ offline = true }}, {{ agent_configured = true }}]\n\
         [[installer_options.steps]]\nid = \"gone\"\ntitle = \"Gone\"\nkind = \"shell\"\ncommand = \"x\"\nwhen = {{ file_present = \"$work/missing\" }}\n"
    );
    let resolved = package(dir.path(), &steps);
    let plan = run(dir.path(), &resolved, &host(), None).unwrap();
    assert_eq!(plan.features, ["docs"]);
    assert_eq!(
        actions(&plan),
        [
            ("build".into(), true, None),
            (
                "fix".into(),
                false,
                Some("when `feature = \"fixes\"` does not hold".into())
            ),
            ("doc".into(), true, None),
            (
                "ci".into(),
                false,
                Some("unless `env_present = \"CI\"` holds".into())
            ),
            ("net".into(), true, None),
            (
                "gone".into(),
                false,
                Some("when `file_present = \"$work/missing\"` does not hold".into())
            ),
        ]
    );
    let offline = Host {
        offline: true,
        ..host()
    };
    let plan = run(dir.path(), &resolved, &offline, None).unwrap();
    assert!(!plan.steps[4].run, "offline = true holds");
    let wire = plan.to_wire();
    assert_eq!(wire.protocol, "vibe-app-plan/1");
    assert!(wire.features[1].chosen);
    assert!(
        plan.render()
            .contains("2. skip  fix (shell)\n              Fix\n              skipped: when `feature = \"fixes\"` does not hold")
    );
}

#[test]
fn needs_refuse_up_front_or_skip_optional_steps() {
    let dir = tempfile::tempdir().unwrap();
    let need = |needs: &str, optional: bool| {
        format!(
            "{BUILD}[[installer_options.steps]]\nid = \"x\"\ntitle = \"X\"\nkind = \"shell\"\ncommand = \"x\"\nneeds = [{needs}]\noptional = {optional}\n"
        )
    };
    let refused = |needs: &str, host: &Host| {
        let resolved = package(dir.path(), &need(needs, false));
        match run(dir.path(), &resolved, host, None) {
            Err(AppError::Step(StepError::UnmetNeed { step, need, .. })) => {
                format!("{step}:{need}")
            }
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(refused("\"command:cmake\"", &host()), "x:command:cmake");
    assert_eq!(refused("\"terminal\"", &host()), "x:terminal");
    assert_eq!(refused("\"agent\"", &host()), "x:agent");
    let offline = Host {
        offline: true,
        ..host()
    };
    assert_eq!(refused("\"network\"", &offline), "x:network");
    let met = package(dir.path(), &need("\"command:make\", \"network\"", false));
    assert!(run(dir.path(), &met, &host(), None).unwrap().steps[1].run);
    let attended = Host {
        attended: true,
        ..host()
    };
    let terminal = package(dir.path(), &need("\"terminal\"", false));
    assert!(run(dir.path(), &terminal, &attended, None).unwrap().steps[1].run);
    let optional = package(dir.path(), &need("\"command:cmake\"", true));
    let plan = run(dir.path(), &optional, &host(), None).unwrap();
    assert_eq!(
        plan.steps[1].reason.as_deref(),
        Some("unmet need `command:cmake`")
    );

    let cached = format!(
        "[[installer_options.steps]]\nid = \"src\"\ntitle = \"S\"\nkind = \"sources\"\nneeds = [\"network\"]\n\
         [[installer_options.steps.sources]]\nname = \"up\"\nurl = \"https://example.com/up.txt\"\nsha256 = \"{}\"\n{BUILD}",
        crate::installer::source_build::tests_digest(b"up")
    );
    let resolved = package(dir.path(), &cached);
    assert!(
        run(dir.path(), &resolved, &offline, None).is_err(),
        "no cache hit"
    );
    let settings = SettingsRoot::new(&dir.path().join("settings"));
    let file = settings
        .app_sources_cache()
        .join(crate::installer::source_build::tests_digest(b"up"))
        .join("up.txt");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, b"up").unwrap();
    assert!(run(dir.path(), &resolved, &offline, None).unwrap().steps[0].run);
}

#[test]
fn agents_are_skipped_with_one_warning_unless_required_and_the_result_must_be_produced() {
    let dir = tempfile::tempdir().unwrap();
    let agent = |required: bool| {
        format!(
            "[[installer_options.steps]]\nid = \"fix\"\ntitle = \"Fix\"\ndescription = \"the sources need porting\"\nkind = \"agent\"\nprompt = \"port\"\nrequired = {required}\n{BUILD}"
        )
    };
    let resolved = package(dir.path(), &agent(false));
    let plan = run(dir.path(), &resolved, &host(), None).unwrap();
    assert_eq!(plan.steps[0].reason.as_deref(), Some("no agent configured"));
    assert_eq!(plan.warnings.len(), 1);
    assert!(plan.warnings[0].contains("`fix`") && plan.warnings[0].contains("agents.toml"));
    let resolved = package(dir.path(), &agent(true));
    match run(dir.path(), &resolved, &host(), None) {
        Err(AppError::Step(StepError::AgentRequired { reason, .. })) => {
            assert_eq!(reason, "the sources need porting");
        }
        other => panic!("{other:?}"),
    }
    let gated = BUILD.replace(
        "script = \"b.sh\"",
        "script = \"b.sh\"\nwhen = { feature = \"fixes\" }",
    );
    let resolved = package(dir.path(), &gated);
    assert!(matches!(
        run(dir.path(), &resolved, &host(), None),
        Err(AppError::Step(StepError::NoResultStep { .. }))
    ));
}

#[test]
fn a_chosen_or_hosting_agent_runs_agent_steps_and_interactive_ones_need_a_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let steps = format!(
        "[[installer_options.steps]]\nid = \"fix\"\ntitle = \"Fix\"\nkind = \"agent\"\nprompt = \"port\"\nmode = \"interactive\"\n\
         [[installer_options.steps]]\nid = \"tidy\"\ntitle = \"Tidy\"\nkind = \"shell\"\ncommand = \"x\"\nneeds = [\"agent\"]\n\
         when = {{ agent_configured = true }}\n{BUILD}"
    );
    let resolved = package(dir.path(), &steps);
    let none = run(dir.path(), &resolved, &host(), None).unwrap();
    assert_eq!(
        actions(&none)[..2],
        [
            ("fix".into(), false, Some("no agent configured".into())),
            (
                "tidy".into(),
                false,
                Some("when `agent_configured = true` does not hold".into())
            )
        ]
    );
    let chosen = select_agent(&AgentsConfig::default(), Some("codex"), None).unwrap();
    let attended = Host {
        attended: true,
        agent: chosen.clone(),
        ..host()
    };
    let plan = run(dir.path(), &resolved, &attended, None).unwrap();
    assert!(
        plan.steps.iter().all(|step| step.run),
        "{:?}",
        actions(&plan)
    );
    assert!(plan.warnings.is_empty());
    let headless = Host {
        agent: chosen,
        ..host()
    };
    match run(dir.path(), &resolved, &headless, None) {
        Err(AppError::Step(StepError::UnmetNeed { step, need, .. })) => {
            assert_eq!((step.as_str(), need.as_str()), ("fix", "terminal"));
        }
        other => panic!("{other:?}"),
    }
    let hosted = Host {
        hosted: true,
        ..host()
    };
    let plan = run(dir.path(), &resolved, &hosted, None).unwrap();
    assert!(plan.steps.iter().all(|step| step.run), "parked steps run");
}

#[test]
fn features_come_from_flags_then_the_question_then_defaults_and_update_reuses_them() {
    let dir = tempfile::tempdir().unwrap();
    let resolved = package(dir.path(), BUILD);
    let chosen = |host: &Host, chooser: Option<FeatureChooser<'_>>| {
        run(dir.path(), &resolved, host, chooser).map(|plan| plan.features)
    };
    assert_eq!(chosen(&host(), None).unwrap(), ["docs"]);
    let flags = Host {
        request: FeatureRequest {
            enable: vec!["fixes".into()],
            disable: vec!["docs".into()],
            profile: None,
        },
        previous: Some(vec!["docs".into()]),
        ..host()
    };
    assert_eq!(chosen(&flags, None).unwrap(), ["fixes"]);
    let unknown = Host {
        request: FeatureRequest {
            enable: vec!["gpu".into()],
            disable: Vec::new(),
            profile: None,
        },
        ..host()
    };
    assert!(matches!(
        chosen(&unknown, None),
        Err(AppError::Step(StepError::UnknownFeature { .. }))
    ));
    let update = Host {
        previous: Some(vec!["fixes".into(), "retired".into()]),
        ..host()
    };
    assert_eq!(chosen(&update, None).unwrap(), ["fixes"]);
    let asked = RefCell::new(Vec::new());
    let chooser = |_: &str, rows: &[crate::select::Choice], defaults: &[bool]| {
        asked.borrow_mut().push((rows.len(), defaults.to_vec()));
        Ok(Some(vec![0, 1]))
    };
    let attended = Host {
        attended: true,
        ..host()
    };
    assert_eq!(
        chosen(&attended, Some(&chooser)).unwrap(),
        ["fixes", "docs"]
    );
    assert_eq!(asked.borrow().as_slice(), [(2, vec![false, true])]);
    let cancel = |_: &str, _: &[crate::select::Choice], _: &[bool]| Ok(None);
    assert!(chosen(&attended, Some(&cancel)).is_err());
}
