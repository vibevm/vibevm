use super::*;
use crate::manifest::RunCommandHandler;

#[test]
fn a_project_declares_one_typed_run_command() {
    let manifest = Manifest::parse_str(
        r#"[project]
name = "demo"
version = "0.1.0"

[[command]]
id = "preview"
description = "Build and serve the preview"
handler = { kind = "script", base = "tooling/preview" }
"#,
    )
    .expect("project command");
    assert_eq!(manifest.run_commands.len(), 1);
    assert!(matches!(
        &manifest.run_commands[0].handler,
        RunCommandHandler::Script { base } if base == std::path::Path::new("tooling/preview")
    ));
}

#[test]
fn package_commands_and_escaping_bases_are_refused() {
    let package = Manifest::parse_str(
        r#"[package]
group = "org.example"
name = "demo"
kind = "app"
version = "0.1.0"

[[command]]
id = "preview"
handler = { kind = "script", base = "tooling/preview" }
"#,
    )
    .expect_err("a dependency must not inject host commands");
    assert!(package.to_string().contains("host-owned"));

    let escaping = Manifest::parse_str(
        r#"[project]
name = "demo"
version = "0.1.0"

[[command]]
id = "preview"
handler = { kind = "script", base = "../outside" }
"#,
    )
    .expect_err("command paths stay in the host");
    assert!(escaping.to_string().contains("project-relative path"));
}
