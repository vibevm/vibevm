//! `vibe app installers PACKAGE` lists every declared installer option on the
//! new model without selecting or running one (PROP-059 `#installer-options`).

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Output;

use common::UserScratch;
use serde_json::Value;
use vibe_wire::generated::application_installer_options::ApplicationInstallerOptions;

const PACKAGE: &str = "org.example/choice-app";

struct Fixture {
    root: tempfile::TempDir,
    registry: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        let package = registry.join("org.example/choice-app/v1.0.0");
        fs::create_dir_all(&package).unwrap();
        let host = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let digest = "a".repeat(64);
        let app_root = root.path().join("external");
        let manifest = format!(
            r#"[package]
name = "choice-app"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "choice"
portable = false

[[installer_options]]
id = "host"
name = "Host shell"
platform = "{host}"
platform_name = "{host}"
arch = "{arch}"
pre_install = "echo prepare"
[installer_options.installation]
type = "shell"
app_root = {app_root}
install = "echo install"
force_install = "echo force"
uninstall = "echo uninstall"
force_uninstall = "echo force uninstall"
interpreter = "posix_sh"
[[installer_options.launchers]]
id = "main"
name = "Main"
executable = "$app_bin/tool"
default = true
[[installer_options.public_commands]]
name = "choice"
launcher = "main"

[[installer_options]]
id = "other-cpu"
name = "Other CPU archive"
platform = "{host}"
platform_name = "{host}"
arch = "zz-cpu"
arch_name = "Imaginary CPU"
portable = true
[installer_options.installation]
type = "zip"
source = "choice.zip"
sha256 = "{digest}"
payload_root = "choice"

[[installer_options]]
id = "other-os"
name = "Plan 9 build"
platform = "plan9"
platform_name = "Plan 9"
arch = "{arch}"
[installer_options.installation]
type = "source_build"
[[installer_options.installation.sources]]
name = "upstream"
url = "https://example.invalid/choice.tar.gz"
sha256 = "{digest}"
[installer_options.installation.recipe]
script = "build.sh"
"#,
            app_root = serde_json::to_string(&app_root.display().to_string()).unwrap(),
        );
        fs::write(package.join("vibe.toml"), manifest).unwrap();
        Self { root, registry }
    }

    fn vibe(&self, user: &UserScratch, args: &[&str]) -> Output {
        user.vibe()
            .current_dir(self.root.path())
            .args(args)
            .output()
            .unwrap()
    }
}

fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn every_option_is_listed_grouped_with_the_detected_host_first() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    let registry = fixture.registry.to_str().unwrap();
    let output = fixture.vibe(
        &user,
        &[
            "app",
            "installers",
            PACKAGE,
            "--registry",
            registry,
            "--json",
        ],
    );
    let listed: Value = serde_json::from_str(&ok(&output)).unwrap();
    let typed: ApplicationInstallerOptions = serde_json::from_value(listed.clone()).unwrap();
    assert_eq!(typed.package, PACKAGE);
    let platforms = listed["platforms"].as_array().unwrap();
    assert_eq!(platforms.len(), 2);
    assert_eq!(platforms[0]["id"], std::env::consts::OS);
    assert_eq!(platforms[0]["detected"], true);
    assert_eq!(platforms[0]["architectures"][0]["detected"], true);
    assert_eq!(platforms[1]["name"], "Plan 9");
    let installers: Vec<&Value> = platforms
        .iter()
        .flat_map(|platform| platform["architectures"].as_array().unwrap())
        .flat_map(|arch| arch["installers"].as_array().unwrap())
        .collect();
    let by_id = |id: &str| {
        installers
            .iter()
            .find(|row| row["id"] == id)
            .copied()
            .unwrap()
    };
    assert_eq!(installers.len(), 3);
    assert_eq!(by_id("host")["installation"]["interpreter"], "posix_sh");
    assert_eq!(by_id("host")["public_commands"][0]["name"], "choice");
    assert_eq!(by_id("host")["pre_install"], "echo prepare");
    assert_eq!(by_id("other-cpu")["portable"], true);
    assert_eq!(by_id("other-cpu")["installation"]["payload_root"], "choice");
    assert_eq!(by_id("other-os")["installation"]["type"], "source_build");
    assert!(
        !user.settings.join("applications").exists(),
        "listing never mutates state"
    );

    let text = ok(&fixture.vibe(
        &user,
        &["app", "installers", PACKAGE, "--registry", registry],
    ));
    assert!(text.contains("choice-app: 3 installer(s)"), "{text}");
    assert!(text.contains("other-cpu: Other CPU archive"), "{text}");
}

#[test]
fn an_explicit_source_build_option_is_selected_and_refused_offline_on_a_cache_miss() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    let registry = fixture.registry.to_str().unwrap();
    let output = fixture.vibe(
        &user,
        &[
            "--offline",
            "install",
            "-g",
            PACKAGE,
            "--registry",
            registry,
            "--platform",
            "plan9",
        ],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--offline is set"), "{stderr}");
    assert!(!user.settings.join("applications/index.json").exists());
    assert!(!user.settings.join("cache/app-build").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn an_explicit_non_detected_option_is_selectable_and_listed_after_install() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    let registry = fixture.registry.to_str().unwrap();
    let app_root = fixture.root.path().join("external");
    fs::create_dir_all(app_root.join("bin")).unwrap();
    let output = fixture.vibe(
        &user,
        &[
            "install",
            "-g",
            PACKAGE,
            "--registry",
            registry,
            "--platform",
            std::env::consts::OS,
            "--installer",
            "host",
            "--json",
        ],
    );
    let report: Value = serde_json::from_str(&ok(&output)).unwrap();
    assert_eq!(report["applicationId"], "choice");
    assert!(String::from_utf8_lossy(&output.stderr).contains("install"));
    let listing: Value = serde_json::from_str(&ok(
        &fixture.vibe(&user, &["app", "launchers", PACKAGE, "--json"])
    ))
    .unwrap();
    assert_eq!(listing["default_launcher_id"], "main");
    assert_eq!(listing["launchers"][0]["status"], "unavailable");
    assert_eq!(listing["app_root"], app_root.display().to_string());
    let list: Value =
        serde_json::from_str(&ok(&fixture.vibe(&user, &["app", "list", "--json"]))).unwrap();
    assert_eq!(list["installations"][0]["portable"], false);
}
