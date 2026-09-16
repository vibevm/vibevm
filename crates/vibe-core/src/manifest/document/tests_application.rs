//! User-application manifest grammar.

use super::Manifest;

const PACKAGE: &str = r#"
[package]
name = "zap"
group = "org.vibevm.zap"
kind = "flow"
version = "1.1.0"

[application]
id = "zap"
installer_package = "org.vibevm.zap/lens@=0.1.0"
runtime = "node"
entry = "tooling/source-install/application.mjs"
commands = ["zap-quicklens", "zap-server"]
"#;

#[test]
fn strict_application_declaration_round_trips() {
    let manifest = Manifest::parse_str(PACKAGE).expect("valid application package");
    let application = manifest.application.as_ref().expect("application");
    assert_eq!(application.id, "zap");
    assert_eq!(
        application.installer_package.to_string(),
        "org.vibevm.zap/lens@=0.1.0"
    );
    assert_eq!(
        application.entry.to_string_lossy(),
        "tooling/source-install/application.mjs"
    );
    assert_eq!(application.commands, ["zap-quicklens", "zap-server"]);
    let rendered = toml::to_string(&manifest).expect("render");
    Manifest::parse_str(&rendered).expect("round trip");
}

#[test]
fn application_is_package_only_and_strict() {
    let project = PACKAGE.replacen(
        "[package]\nname = \"zap\"\ngroup = \"org.vibevm.zap\"\nkind = \"flow\"\nversion = \"1.1.0\"",
        "[project]\nname = \"demo\"\nversion = \"1.0.0\"",
        1,
    );
    assert!(
        Manifest::parse_str(&project)
            .unwrap_err()
            .to_string()
            .contains("package-role")
    );
    for (from, to, expected) in [
        (
            "org.vibevm.zap/lens@=0.1.0",
            "org.vibevm.zap/lens",
            "exactly pinned",
        ),
        (
            "tooling/source-install/application.mjs",
            "../application.mjs",
            "provider-relative",
        ),
        (
            "commands = [\"zap-quicklens\", \"zap-server\"]",
            "commands = [\"zap-server\", \"zap-server\"]",
            "duplicate",
        ),
    ] {
        let invalid = PACKAGE.replacen(from, to, 1);
        assert!(
            Manifest::parse_str(&invalid)
                .unwrap_err()
                .to_string()
                .contains(expected),
            "{invalid}"
        );
    }
}
