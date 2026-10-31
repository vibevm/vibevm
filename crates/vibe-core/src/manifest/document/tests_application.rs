//! User-application manifest grammar (`PROP-059#declaration`).

use super::Manifest;
use crate::manifest::{InstallerMethod, ShellInterpreter};

const PACKAGE: &str = r#"
[package]
name = "demo"
group = "org.vibevm"
kind = "app"
version = "1.1.0"

[application]
id = "demo"
portable = false
"#;

#[test]
fn strict_application_declaration_round_trips() {
    let manifest = Manifest::parse_str(PACKAGE).unwrap();
    let application = manifest.application.as_ref().unwrap();
    assert_eq!(application.id, "demo");
    assert!(!application.portable);
    assert!(manifest.installer_options.is_empty());
    assert_eq!(
        Manifest::parse_str(&toml::to_string(&manifest).unwrap()).unwrap(),
        manifest
    );
    let default = Manifest::parse_str(&PACKAGE.replace("portable = false\n", "")).unwrap();
    assert!(default.application.unwrap().portable);
}

#[test]
fn retired_application_fields_are_not_parsed() {
    for retired in [
        "installer_package = \"org.vibevm/demo@=1.0.0\"\n",
        "runtime = \"node\"\n",
        "entry = \"tooling/application.mjs\"\n",
        "commands = [\"demo\"]\n",
    ] {
        let text = format!("{PACKAGE}{retired}");
        assert!(Manifest::parse_str(&text).is_err(), "{text}");
    }
    let bridge = format!(
        "{PACKAGE}[application_source]\nkind = \"git\"\nurl = \"https://example.com/demo.git\"\ntracked_ref = \"refs/heads/main\"\nregistry_path = \"vibevm/vibepacks\"\n"
    );
    assert!(Manifest::parse_str(&bridge).is_err());
}

#[test]
fn application_is_package_only_and_strict() {
    let project = PACKAGE.replacen(
        "[package]\nname = \"demo\"\ngroup = \"org.vibevm\"\nkind = \"app\"\nversion = \"1.1.0\"",
        "[project]\nname = \"demo\"\nversion = \"1.0.0\"",
        1,
    );
    assert!(
        Manifest::parse_str(&project)
            .unwrap_err()
            .to_string()
            .contains("package-role")
    );
    let invalid = PACKAGE.replacen("id = \"demo\"", "id = \"Bad Id\"", 1);
    assert!(
        Manifest::parse_str(&invalid)
            .unwrap_err()
            .to_string()
            .contains("portable lowercase token")
    );
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
interpreter = "posix_sh"
[[installer_options.launchers]]
id = "main"
name = "Demo"
executable = "$app_root/bin/demo"
working_directory = "."
args = ["${project_root}", "$$literal"]
default = true
[[installer_options.public_commands]]
name = "demo"
launcher = "main"
[[installer_options]]
id = "ubuntu-zip"
name = "Portable archive"
platform = "ubuntu"
platform_name = "Ubuntu"
arch = "x86_64"
arch_name = "64-bit x86"
portable = false
[installer_options.installation]
type = "zip"
source = "releases/demo.zip"
sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
payload_root = "demo"
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
        Some(InstallerMethod::Shell {
            interpreter: Some(ShellInterpreter::PosixSh),
            ..
        })
    ));
    assert!(matches!(
        manifest.installer_options[1].installation,
        Some(InstallerMethod::Zip { .. })
    ));
    assert_eq!(manifest.installer_options[1].portable, Some(false));
    assert_eq!(manifest.installer_options[0].portable, None);
    assert_eq!(manifest.installer_options[0].launchers[0].name, "Demo");
    assert_eq!(
        manifest.installer_options[0].public_commands[0].name,
        "demo"
    );
    assert!(manifest.installer_options[1].public_commands.is_empty());
    assert_eq!(
        Manifest::parse_str(&toml::to_string(&manifest).unwrap()).unwrap(),
        manifest
    );
}

#[test]
fn public_commands_are_local_to_an_installer_option() {
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
[installer_options.installation]
type = "shell"
app_root = "/opt/demo"
install = "echo install"
force_install = "echo force install"
uninstall = "echo uninstall"
force_uninstall = "echo force uninstall"
[[installer_options.launchers]]
id = "main"
name = "Demo"
executable = "bin/demo"
[[installer_options.public_commands]]
name = "demo"
launcher = "main"
"#;
    assert!(Manifest::parse_str(base).is_ok());
    // Toolchain spellings are commands too.
    for name in ["g++", "clang++", "python3.12", "x86_64-w64-mingw32-gcc"] {
        let extra = format!(
            "[[installer_options.public_commands]]\nname = \"{name}\"\nlauncher = \"main\"\n"
        );
        let parsed = Manifest::parse_str(&format!("{base}{extra}"));
        assert!(parsed.is_ok(), "{name}: {:?}", parsed.err());
    }
    for (extra, expected) in [
        (
            "[[installer_options.public_commands]]\nname = \"demo\"\nlauncher = \"main\"\n",
            "duplicated",
        ),
        (
            "[[installer_options.public_commands]]\nname = \"vibe\"\nlauncher = \"main\"\n",
            "other than `vibe`",
        ),
        (
            "[[installer_options.public_commands]]\nname = \"bad name\"\nlauncher = \"main\"\n",
            "command name",
        ),
        (
            "[[installer_options.public_commands]]\nname = \"+g\"\nlauncher = \"main\"\n",
            "command name",
        ),
        (
            "[[installer_options.public_commands]]\nname = \"other\"\nlauncher = \"missing\"\n",
            "unknown launcher",
        ),
    ] {
        let invalid = format!("{base}{extra}");
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
            option(
                "zip",
                "source = \"demo.zip\"\nsha256 = \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"\n[[installer_options.installation.runners]]\npath = \"bin/demo\"\nname = \"demo\"",
                "linux",
            ),
            "runners",
        ),
        (
            option(
                "shell",
                "app_root = \"/opt/demo\"\ninstall = \"i\"\nforce_install = \"f\"\nuninstall = \"u\"\nforce_uninstall = \"fu\"\ninterpreter = \"fish\"",
                "linux",
            ),
            "fish",
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

#[test]
fn application_without_installer_options_is_a_valid_product() {
    let manifest = Manifest::parse_str(PACKAGE).unwrap();
    assert!(manifest.installer_options.is_empty());
}

#[test]
fn source_build_model_is_validated() {
    let digest = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    let base = format!(
        r#"{PACKAGE}
[[installer_options]]
id = "build"
name = "Build from source"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"
[installer_options.installation]
type = "source_build"
[[installer_options.installation.sources]]
name = "upstream"
url = "http://127.0.0.1:8080/upstream.tar.gz"
sha256 = "{digest}"
[[installer_options.installation.sources]]
name = "patches"
path = "patches/fix.patch"
sha256 = "{digest}"
extract = false
[installer_options.installation.recipe]
interpreter = "posix_sh"
script = "recipes/build.sh"
"#
    );
    let manifest = Manifest::parse_str(&base).unwrap();
    let Some(InstallerMethod::SourceBuild { sources, recipe }) =
        &manifest.installer_options[0].installation
    else {
        panic!("source_build expected");
    };
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[1].extract, Some(false));
    assert_eq!(recipe.interpreter, Some(ShellInterpreter::PosixSh));
    assert!(
        manifest.installer_options[0]
            .installation
            .as_ref()
            .is_some_and(InstallerMethod::is_owned)
    );
    assert_eq!(
        Manifest::parse_str(&toml::to_string(&manifest).unwrap()).unwrap(),
        manifest
    );
    for (from, to, expected) in [
        ("name = \"patches\"", "name = \"upstream\"", "duplicated"),
        (
            "name = \"patches\"",
            "name = \"a/b\"",
            "portable directory name",
        ),
        (
            "path = \"patches/fix.patch\"",
            "path = \"patches/fix.patch\"\nurl = \"https://example.com/x\"",
            "exactly one of url or path",
        ),
        (
            "path = \"patches/fix.patch\"",
            "path = \"../fix.patch\"",
            "package-relative",
        ),
        (
            "url = \"http://127.0.0.1:8080/upstream.tar.gz\"",
            "url = \"ftp://example.com/x\"",
            "HTTP(S)",
        ),
        (
            "script = \"recipes/build.sh\"",
            "script = \"/abs/build.sh\"",
            "recipe.script",
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
    let empty = base
        .split("[[installer_options.installation.sources]]")
        .next()
        .unwrap()
        .to_string()
        + "sources = []\n[installer_options.installation.recipe]\nscript = \"build.sh\"\n";
    // No sources: the package tree itself is the input (PROP-059#source-build).
    let manifest = Manifest::parse_str(&empty).unwrap();
    let Some(InstallerMethod::SourceBuild { sources, .. }) =
        &manifest.installer_options[0].installation
    else {
        panic!("source_build expected");
    };
    assert!(sources.is_empty());
}
