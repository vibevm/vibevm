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
        application.installer_package.as_ref().unwrap().to_string(),
        "org.vibevm.zap/lens@=0.1.0"
    );
    assert_eq!(
        application.entry.as_ref().unwrap().to_string_lossy(),
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

#[test]
fn installer_options_parse_and_round_trip() {
    let manifest = Manifest::parse_str(
        r#"
[package]
name = "demo"
group = "org.vibevm"
kind = "app"
version = "1.0.0"
[application]
id = "demo"
[[installer_options]]
id = "ubuntu-shell"
name = "Ubuntu package manager"
platform = "ubuntu"
platform_name = "Ubuntu"
arch = "x86_64"
arch_name = "64-bit x86"
pre_install = "echo prepare"
post_install = "echo finish"
[installer_options.installation]
type = "shell"
app_root = "$vibe_home/opt/apps/demo"
install = "apt-get install demo"
force_install = "apt-get --reinstall install demo"
update = "apt-get upgrade demo"
uninstall = "apt-get remove demo"
force_uninstall = "apt-get purge demo"
[[installer_options.launchers]]
id = "main"
name = "Demo"
executable = "$app_root/bin/demo"
working_directory = "."
args = ["${project_root}", "$$literal"]
default = true
[[installer_options]]
id = "ubuntu-zip"
name = "Portable archive"
platform = "ubuntu"
platform_name = "Ubuntu"
arch = "x86_64"
arch_name = "64-bit x86"
[installer_options.installation]
type = "zip"
source = "releases/demo.zip"
sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
payload_root = "demo"
[[installer_options.installation.runners]]
path = "bin/demo"
name = "demo"
[[installer_options.launchers]]
id = "main"
name = "Demo archive"
executable = "bin/demo"
"#,
    )
    .unwrap();
    assert_eq!(manifest.installer_options.len(), 2);
    assert_eq!(
        manifest.installer_options[0].pre_install.as_deref(),
        Some("echo prepare")
    );
    assert_eq!(
        manifest.installer_options[0].post_install.as_deref(),
        Some("echo finish")
    );
    assert!(matches!(
        manifest.installer_options[0].installation,
        super::super::InstallerMethod::Shell { .. }
    ));
    assert!(matches!(
        manifest.installer_options[1].installation,
        super::super::InstallerMethod::Zip { .. }
    ));
    assert_eq!(manifest.installer_options[0].launchers[0].name, "Demo");
    assert_eq!(
        Manifest::parse_str(&toml::to_string(&manifest).unwrap()).unwrap(),
        manifest
    );
}

#[test]
fn installer_method_kinds_and_validation() {
    let prefix = r#"
[package]
name = "demo"
group = "org.vibevm"
kind = "app"
version = "1.0.0"
[application]
id = "demo"
"#;
    let option = |kind: &str, fields: &str, platform: &str| {
        format!(
            r#"{prefix}
[[installer_options]]
id = "main"
name = "Main installer"
platform = "{platform}"
platform_name = "Platform"
arch = "aarch64"
[installer_options.installation]
type = "{kind}"
{fields}
"#
        )
    };
    let digest = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    for (kind, fields, platform) in [
        (
            "tar_gz",
            format!("source = \"demo.tar.gz\"\nsha256 = \"{digest}\""),
            "freebsd",
        ),
        (
            "exe",
            format!(
                "source = \"setup.exe\"\nsha256 = \"{digest}\"\napp_root = \"C:\\\\Program Files\\\\Demo\"\nforce_arguments = [\"/force\", \"two words\"]\nforce_uninstall = \"uninstall.exe /force\""
            ),
            "windows",
        ),
        (
            "msi",
            format!(
                "source = \"setup.msi\"\nsha256 = \"{digest}\"\napp_root = \"$vibe_home/opt/apps/demo\""
            ),
            "windows",
        ),
    ] {
        let text = option(kind, &fields, platform);
        assert!(Manifest::parse_str(&text).is_ok(), "{text}");
    }
    for (text, expected) in [
        (
            prefix.to_string(),
            "requires nonempty [[installer_options]]",
        ),
        (
            option(
                "shell",
                "app_root = \"/opt/demo\"\ninstall = \"echo install\"\nuninstall = \"echo uninstall\"\nforce_uninstall = \"echo force\"",
                "linux",
            ),
            "force_install",
        ),
        (
            option(
                "zip",
                "source = \"../escape.zip\"\nsha256 = \"bad\"",
                "linux",
            ),
            "source",
        ),
        (
            option("zip", "source = \"demo.zip\"\nsha256 = \"bad\"", "linux"),
            "sha256",
        ),
        (
            option(
                "exe",
                &format!("source = \"setup.exe\"\nsha256 = \"{digest}\"\napp_root = \"/opt/demo\""),
                "linux",
            ),
            "platform = windows",
        ),
        (
            option(
                "shell",
                "app_root = \"$project_root/demo\"\ninstall = \"echo demo\"\nforce_install = \"echo force\"\nuninstall = \"echo uninstall\"\nforce_uninstall = \"echo force uninstall\"",
                "linux",
            ),
            "app_root",
        ),
    ] {
        assert!(
            Manifest::parse_str(&text)
                .unwrap_err()
                .to_string()
                .contains(expected),
            "{text}"
        );
    }
}

#[test]
fn forced_and_lifecycle_shell_commands_are_strict() {
    let base = r#"
[package]
name = "demo"
group = "org.vibevm"
kind = "app"
version = "1.0.0"
[application]
id = "demo"
[[installer_options]]
id = "linux-shell"
name = "Linux shell"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"
pre_install = "echo prepare"
post_install = "echo done"
[installer_options.installation]
type = "shell"
app_root = "/opt/demo"
install = "echo install"
force_install = "echo force install"
uninstall = "echo uninstall"
force_uninstall = "echo force uninstall"
"#;
    let parsed = Manifest::parse_str(base).unwrap();
    assert_eq!(
        parsed.installer_options[0].pre_install.as_deref(),
        Some("echo prepare")
    );
    assert_eq!(
        Manifest::parse_str(&toml::to_string(&parsed).unwrap()).unwrap(),
        parsed
    );
    for (from, to, expected) in [
        (
            "force_install = \"echo force install\"\n",
            "",
            "force_install",
        ),
        (
            "force_uninstall = \"echo force uninstall\"\n",
            "",
            "force_uninstall",
        ),
        (
            "pre_install = \"echo prepare\"",
            "pre_install = \"   \"",
            "pre_install",
        ),
        (
            "post_install = \"echo done\"",
            "post_install = \"\"",
            "post_install",
        ),
        (
            "force_install = \"echo force install\"",
            "force_install = \"echo\\nunsafe\"",
            "force_install",
        ),
    ] {
        let invalid = base.replacen(from, to, 1);
        assert!(
            Manifest::parse_str(&invalid)
                .unwrap_err()
                .to_string()
                .contains(expected),
            "{invalid}"
        );
    }
}

#[test]
fn installer_option_identity_and_labels_are_consistent() {
    let base = r#"
[package]
name = "demo"
group = "org.vibevm"
kind = "app"
version = "1.0.0"
[application]
id = "demo"
[[installer_options]]
id = "first"
name = "First"
platform = "ubuntu"
platform_name = "Ubuntu"
arch = "x86_64"
arch_name = "x86 64"
[installer_options.installation]
type = "shell"
app_root = "/opt/demo"
install = "echo install"
force_install = "echo force install"
uninstall = "echo uninstall"
force_uninstall = "echo force uninstall"
[[installer_options]]
id = "second"
name = "Second"
platform = "ubuntu"
platform_name = "Ubuntu"
arch = "x86_64"
arch_name = "x86 64"
[installer_options.installation]
type = "shell"
app_root = "/opt/demo"
install = "echo install"
force_install = "echo force install"
uninstall = "echo uninstall"
force_uninstall = "echo force uninstall"
"#;
    assert!(Manifest::parse_str(base).is_ok());
    let wrong_kind = base.replacen("kind = \"app\"", "kind = \"flow\"", 1);
    assert!(
        Manifest::parse_str(&wrong_kind)
            .unwrap_err()
            .to_string()
            .contains("kind = app")
    );
    let duplicate = base.replacen("id = \"second\"", "id = \"first\"", 1);
    assert!(
        Manifest::parse_str(&duplicate)
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );
    let inconsistent = base.replacen(
        "platform_name = \"Ubuntu\"",
        "platform_name = \"Ubuntu Linux\"",
        1,
    );
    assert!(
        Manifest::parse_str(&inconsistent)
            .unwrap_err()
            .to_string()
            .contains("inconsistent platform_name")
    );
    let inconsistent_arch =
        base.replacen("arch_name = \"x86 64\"", "arch_name = \"64-bit x86\"", 1);
    assert!(
        Manifest::parse_str(&inconsistent_arch)
            .unwrap_err()
            .to_string()
            .contains("inconsistent arch_name")
    );
}
