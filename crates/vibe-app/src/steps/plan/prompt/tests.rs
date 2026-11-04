//! Prompt preparation, source observation and preview/execution continuity.
use super::*;
use crate::agents::{AgentHooks, AgentNotice, AgentsConfig, select_agent};
use crate::index::PackageCoordinate;
use crate::resolve::from_manifest;
use crate::settings::SettingsRoot;
use crate::steps::features::FeatureRequest;
use crate::steps::params::ParamValue;
use crate::steps::plan::{HostFacts, PlanInput, Prompts, plan};
use vibe_core::manifest::{Manifest, StepPhase};

#[cfg(test)]
fn manifest(extra: &str) -> Manifest {
    Manifest::parse_str(&format!(
        "[package]\nname='prompt'\ngroup='org.example'\nkind='app'\nversion='1.0.0'\n\
         [application]\nid='prompt'\n\
         [[installer_options]]\nid='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'\nresult={{external='/tmp/prompt-output'}}\n\
         [[installer_options.features]]\nid='off'\nname='Off'\n\
         [[installer_options.parameters]]\nid='token'\nname='Token'\ntype='string'\nsecret=true\n\
         [[installer_options.parameters]]\nid='public'\nname='Public'\ntype='string'\n\
         [[installer_options.steps]]\nid='agent'\ntitle='Agent'\nkind='agent'\nprompt_file='prompt.md'\n{extra}\n"
    )).unwrap()
}

#[cfg(test)]
fn planned(root: &Path, extra: &str, hosted: bool) -> Result<crate::steps::Plan, AppError> {
    let resolved = from_manifest(
        manifest(extra),
        root.into(),
        PackageCoordinate {
            group: "org.example".into(),
            name: "prompt".into(),
            version: "1.0.0".into(),
        },
    )
    .unwrap();
    let settings = SettingsRoot::new(&root.join("settings"));
    plan(&PlanInput {
        agent_modes: None,
        configuration: None,
        resolved: &resolved,
        option: &resolved.options[0],
        request: &FeatureRequest::default(),
        parameters: &[("token".into(), "SYN_SECRET_PROMPT_VALUE".into())],
        accept_license: false,
        previous: None,
        settings: &settings,
        facts: HostFacts::default(),
        offline: true,
        attended: false,
        prompts: Prompts::default(),
        agent: AgentHooks {
            hosted,
            ..Default::default()
        },
        requirements: None,
        required_by: None,
    })
}

#[test]
fn active_files_are_checked_but_skipped_or_agentless_files_are_not_read() {
    let root = tempfile::tempdir().unwrap();
    assert!(
        planned(root.path(), "", true)
            .unwrap_err()
            .to_string()
            .contains("does not exist")
    );
    for (extra, hosted) in [("", false), ("when={feature='off'}", true)] {
        let p = planned(root.path(), extra, hosted).unwrap();
        assert!(!p.steps[0].run);
        assert!(p.steps[0].prepared_prompt.is_none());
    }
    std::fs::write(root.path().join("prompt.md"), "Use $param:token").unwrap();
    let e = planned(root.path(), "", true).unwrap_err();
    assert!(e.to_string().contains("secret"));
    assert!(!e.to_string().contains("SYN_SECRET_PROMPT_VALUE"));
    assert!(!root.path().join("settings").exists());
}

#[test]
fn file_prose_and_inline_prose_share_control_and_reference_validation() {
    let root = tempfile::tempdir().unwrap();
    let option = &manifest("").installer_options[0];
    for text in [
        "bad\u{7}",
        "$param:missing",
        "${unterminated",
        "$param:token",
    ] {
        assert!(option.validate_prompt_template(text).is_err());
        std::fs::write(root.path().join("prompt.md"), text).unwrap();
        assert!(planned(root.path(), "", true).is_err(), "{text:?}");
    }
    std::fs::write(root.path().join("prompt.md"), b"\xff").unwrap();
    assert!(
        planned(root.path(), "", true)
            .unwrap_err()
            .to_string()
            .contains("not UTF-8")
    );
    std::fs::write(root.path().join("prompt.md"), "line\n\r\t$$param:token").unwrap();
    assert!(planned(root.path(), "", true).is_ok());
}

#[test]
fn preview_retains_only_finish_root_placeholders_and_never_masks_other_errors() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("prompt.md"), "$app_root $app_bin").unwrap();
    assert!(planned(root.path(), "", true).is_err());
    let p = planned(root.path(), "phase='finish'", true).unwrap();
    let vars = Vars::planning(root.path(), root.path(), root.path(), &[], &p.parameters);
    assert_eq!(
        template::preview(
            p.steps[0].prepared_prompt.as_deref().unwrap(),
            &vars,
            "agent",
            StepPhase::Finish
        )
        .unwrap(),
        "$app_root $app_bin"
    );
    assert!(crate::steps::agent::prompt_text(&p.steps[0], &vars).is_err());
    std::fs::write(root.path().join("prompt.md"), "$app_root $param:public").unwrap();
    let error = planned(root.path(), "phase='finish'", true).unwrap_err();
    assert!(error.to_string().contains("has no value"));
}

#[test]
fn retained_literal_is_used_by_consent_preview_and_execution_expansion() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("prompt.md"), "ADMITTED $app_root").unwrap();
    let p = planned(root.path(), "phase='finish'", true).unwrap();
    std::fs::write(root.path().join("prompt.md"), "REPLACED $param:token").unwrap();
    let vars = Vars::planning(root.path(), root.path(), root.path(), &[], &p.parameters);
    let selected = select_agent(&AgentsConfig::default(), Some("codex"), None).unwrap();
    let shown = std::cell::RefCell::new(Vec::new());
    let notify = |notice: &AgentNotice<'_>| {
        if let AgentNotice::Prompt(prompt) = notice {
            shown.borrow_mut().push(prompt.prompt.to_owned());
        }
    };
    let hooks = AgentHooks {
        selected: selected.as_ref(),
        assume_yes: true,
        notify: Some(&notify),
        ..Default::default()
    };
    crate::steps::agent::admit(
        &hooks,
        &p,
        p.steps.iter(),
        &|_| false,
        &vars,
        false,
        &|id| crate::steps::envelope::Envelope::at(&vars.work, id),
    )
    .unwrap();
    assert_eq!(*shown.borrow(), ["ADMITTED $app_root"]);
    let actual_root = root.path().join("installed");
    let vars = Vars {
        app_root: Some(&actual_root),
        ..vars
    };
    assert_eq!(
        crate::steps::agent::prompt_text(&p.steps[0], &vars).unwrap(),
        format!("ADMITTED {}", actual_root.display())
    );
}

#[cfg(unix)]
#[test]
fn shared_hardlinks_are_observed_without_copying_or_exclusive_ownership() {
    use std::os::unix::fs::MetadataExt;
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("shared");
    let prompt = root.path().join("prompt.md");
    std::fs::write(&source, "SHARED_PROMPT").unwrap();
    std::fs::hard_link(&source, &prompt).unwrap();
    let a = source.metadata().unwrap();
    let b = prompt.metadata().unwrap();
    assert_eq!((a.dev(), a.ino()), (b.dev(), b.ino()));
    assert!(a.nlink() >= 2);
    let p = planned(root.path(), "", true).unwrap();
    assert_eq!(p.steps[0].prepared_prompt.as_deref(), Some("SHARED_PROMPT"));
    assert_eq!(prompt.metadata().unwrap().ino(), a.ino());
}

#[cfg(unix)]
#[test]
fn explicit_root_alias_is_admitted_but_descendant_symlinks_refuse() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let aliases = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("prompt.md"), "ADMITTED").unwrap();
    symlink(root.path(), aliases.path().join("root")).unwrap();
    assert!(planned(&aliases.path().join("root"), "", true).is_ok());
    std::fs::remove_file(root.path().join("prompt.md")).unwrap();
    std::fs::write(aliases.path().join("outside"), "OUTSIDE").unwrap();
    symlink(
        aliases.path().join("outside"),
        root.path().join("prompt.md"),
    )
    .unwrap();
    assert!(planned(root.path(), "", true).is_err());
    let option = manifest("").installer_options.remove(0);
    let mut step = option.steps[0].clone();
    if let StepKind::Agent { prompt_file, .. } = &mut step.kind {
        *prompt_file = Some("nested/outside".into());
    }
    symlink(aliases.path(), root.path().join("nested")).unwrap();
    let vars = Vars::planning(root.path(), root.path(), root.path(), &[], &[]);
    assert!(prepare(&option, &step, true, root.path(), &vars).is_err());
}

#[test]
fn programmatic_plan_cannot_bypass_runtime_secret_or_missing_preparation_errors() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("prompt.md"), "PUBLIC").unwrap();
    let mut p = planned(root.path(), "", true).unwrap();
    let params = [ParamValue {
        id: "token".into(),
        value: "SYN_RUNTIME_SECRET".into(),
        secret: true,
    }];
    let vars = Vars::planning(root.path(), root.path(), root.path(), &[], &params);
    p.steps[0].prepared_prompt = Some("$param:token".into());
    let e = crate::steps::agent::prompt_text(&p.steps[0], &vars).unwrap_err();
    assert!(e.to_string().contains("secret"));
    assert!(!e.to_string().contains("SYN_RUNTIME_SECRET"));
    p.steps[0].prepared_prompt = None;
    assert!(
        crate::steps::agent::prompt_text(&p.steps[0], &vars)
            .unwrap_err()
            .to_string()
            .contains("not prepared")
    );
}
