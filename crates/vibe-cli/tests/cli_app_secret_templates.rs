//! Public local install preflight rejects secret prompt-file interpolation.
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#SECRET-TEMPLATES");

#[cfg(test)]
mod common;

use common::app_registry::AppRegistry;

#[test]
fn global_install_refuses_secret_prompt_file_before_earlier_steps_or_envelope() {
    let registry = AppRegistry::new();
    let sentinel = registry.root.path().join("SENTINEL");
    let dependency_sentinel = registry.root.path().join("DEPENDENCY_SENTINEL");
    let dependency = registry.package("requirement", "1.0.0", "requirement", &format!(
        "[[installer_options.steps]]\nid='copy'\ntitle='Copy'\nkind='copy'\nfrom='input'\nto='input'\n\
         [[installer_options.steps]]\nid='mark'\ntitle='Mark dependency execution'\nkind='shell'\ncommand=\"printf DEPENDENCY > '{}'\"\n", dependency_sentinel.display()));
    std::fs::write(dependency.join("input"), "DEPENDENCY_INPUT").unwrap();
    let package = registry.package("template-check", "1.0.0", "template-check", &format!(
        "requires=['org.example/requirement@1']\n[[installer_options.parameters]]\nid='token'\nname='Token'\ntype='string'\nsecret=true\n\
         [[installer_options.steps]]\nid='copy'\ntitle='Copy'\nkind='copy'\nfrom='input'\nto='input'\n\
         [[installer_options.steps]]\nid='earlier'\ntitle='Earlier'\nkind='shell'\ncommand=\"printf SENTINEL > '{}'\"\n\
         [[installer_options.steps]]\nid='agent'\ntitle='Agent'\nkind='agent'\nprompt_file='prompt.md'\n", sentinel.display()));
    std::fs::write(package.join("input"), "PUBLIC_INPUT").unwrap();
    std::fs::write(package.join("prompt.md"), "Use $param:token").unwrap();
    let output = registry.app(
        &["install", "-g", "--agent-mode", "agent"],
        "org.example/template-check",
        &[
            "--offline",
            "--unattended",
            "--json",
            "--set",
            "token=SYN_PROMPT_CANARY",
        ],
    );
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "{text}\nearlier_step_ran={} outbox_published={}",
        sentinel.exists(),
        registry.settings().join("agentic/outbox").exists()
    );
    assert!(text.contains("secret"), "{text}");
    assert!(!text.contains("SYN_PROMPT_CANARY"), "{text}");
    assert!(!sentinel.exists(), "an earlier app step already ran");
    assert!(
        !dependency_sentinel.exists(),
        "a dependency installer already ran"
    );
    assert!(!registry.settings().join("cache/app-work").exists());
    assert!(!registry.settings().join("agentic/outbox").exists());
    assert!(!registry.settings().join("applications/index.json").exists());
}

#[test]
fn global_install_uses_the_preflight_prompt_after_an_earlier_step_replaces_its_file() {
    let registry = AppRegistry::new();
    let package = registry.package("prompt-snapshot", "1.0.0", "prompt-snapshot",
        "[[installer_options.steps]]\nid='change'\ntitle='Change prompt after planning'\nkind='recipe'\nscript='change.sh'\n\
         [[installer_options.steps]]\nid='agent'\ntitle='Agent'\nkind='agent'\nprompt_file='prompt.md'\n");
    std::fs::write(package.join("prompt.md"), "ADMITTED_PROMPT").unwrap();
    std::fs::write(package.join("change.sh"), "mkdir -p \"$VIBE_OUT\"\nprintf REPLACEMENT_PROMPT > \"$VIBE_PACKAGE/prompt.md\"\ncp \"$VIBE_PACKAGE/prompt.md\" \"$VIBE_OUT/replacement-proof\"\n").unwrap();
    let output = registry.app(
        &["install", "-g", "--agent-mode", "agent"],
        "org.example/prompt-snapshot",
        &["--offline", "--unattended", "--json"],
    );
    let text = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(75), "{text}");
    let work = registry
        .settings()
        .join("cache/app-work/org.example.prompt-snapshot@1.0.0");
    assert_eq!(
        std::fs::read_to_string(work.join("out/replacement-proof")).unwrap(),
        "REPLACEMENT_PROMPT"
    );
    assert_eq!(
        std::fs::read_to_string(
            std::path::Path::new(
                common::fake_agent::refused(&output).1["run"]
                    .as_str()
                    .unwrap()
            )
            .join("prompt.md")
        )
        .unwrap(),
        "ADMITTED_PROMPT"
    );
}
