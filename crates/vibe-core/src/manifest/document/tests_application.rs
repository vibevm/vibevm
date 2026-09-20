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

[application.distribution]
repository = "vibevm/zap"
release_tag = "v1.0.0"
index_asset = "DISTRIBUTIONS.json"
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
    assert_eq!(
        application.distribution.as_ref().unwrap().repository,
        "vibevm/zap"
    );
    let rendered = toml::to_string(&manifest).expect("render");
    Manifest::parse_str(&rendered).expect("round trip");
}

#[test]
fn application_source_proxy_is_strict_and_bridge_may_carry_binary_metadata() {
    let proxy = r#"
[package]
name = "zap"
group = "org.vibevm.zap"
kind = "flow"
version = "1.0.0"
bridge = true

[application_source]
kind = "git"
url = "https://github.com/vibevm/zap.git"
tracked_ref = "refs/heads/1.0.0"
registry_path = "vibevm/vibepacks"
"#;
    let manifest = Manifest::parse_str(proxy).expect("valid application proxy");
    let source = manifest.application_source.as_ref().unwrap();
    assert_eq!(source.tracked_ref, "refs/heads/1.0.0");
    Manifest::parse_str(&toml::to_string(&manifest).unwrap()).expect("round trip");

    for (from, to, expected) in [
        (
            "https://github.com/vibevm/zap.git",
            "https://token@github.com/vibevm/zap.git",
            "credential-free",
        ),
        ("refs/heads/1.0.0", "main", "refs/heads"),
        ("vibevm/vibepacks", "../outside", "portable"),
    ] {
        let error = Manifest::parse_str(&proxy.replacen(from, to, 1))
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
    let both = format!(
        "{proxy}\n[application]{}",
        PACKAGE.split_once("[application]").unwrap().1
    );
    let combined = Manifest::parse_str(&both).expect("bridge carries binary and source routes");
    assert!(combined.application.is_some());
    assert!(combined.application_source.is_some());

    let non_bridge = both.replace("bridge = true\n", "");
    assert!(
        Manifest::parse_str(&non_bridge)
            .unwrap_err()
            .to_string()
            .contains("only on a bridge package")
    );
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
