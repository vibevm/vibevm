//! Public Linux fixtures for user applications (PROP-059): install → list →
//! run → uninstall for the shell, zip and tar.gz methods, hook ordering,
//! `--force` on a changed owned payload and unattended selection errors.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::UserScratch;
use serde_json::Value;
use sha2::{Digest, Sha256};

const TOOL: &str = "#!/bin/sh\necho \"TOOL_RAN $*\"\n";

/// A local registry with one app package `org.example/<name>@1.0.0`.
struct Fixture {
    root: tempfile::TempDir,
    registry: PathBuf,
    package: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        let package = registry.join(format!("org.example/{name}/v1.0.0"));
        fs::create_dir_all(&package).unwrap();
        Self {
            root,
            registry,
            package,
        }
    }

    fn manifest(&self, name: &str, body: &str) {
        fs::write(
            self.package.join("vibe.toml"),
            format!(
                "[package]\nname = \"{name}\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n[application]\nid = \"tool\"\n\n{body}"
            ),
        )
        .unwrap();
    }

    fn script(&self, name: &str, body: &str) {
        let path = self.package.join(name);
        fs::write(&path, body).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn vibe(&self, user: &UserScratch, args: &[&str]) -> Output {
        let mut command = user.vibe();
        command.current_dir(self.root.path()).args(args);
        command.output().unwrap()
    }

    fn install(&self, user: &UserScratch, package: &str, extra: &[&str]) -> Output {
        let registry = self.registry.to_str().unwrap();
        let mut args = vec!["install", "-g", package, "--registry", registry];
        args.extend_from_slice(extra);
        self.vibe(user, &args)
    }
}

fn option_header(id: &str) -> String {
    format!(
        "[[installer_options]]\nid = \"{id}\"\nname = \"Option {id}\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n",
        arch = std::env::consts::ARCH
    )
}

const LAUNCHER: &str = "[[installer_options.launchers]]\nid = \"main\"\nname = \"Tool\"\nexecutable = \"$app_bin/tool\"\nargs = [\"fixed\"]\ndefault = true\n\n[[installer_options.public_commands]]\nname = \"tool\"\nlauncher = \"main\"\n";

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

fn failed(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "unexpected success: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn json(output: &Output) -> Value {
    serde_json::from_str(&ok(output)).unwrap()
}

fn index(user: &UserScratch) -> Value {
    serde_json::from_slice(&fs::read(user.settings.join("applications/index.json")).unwrap())
        .unwrap()
}

fn digest(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

fn write_zip(path: &Path, tool: &str) {
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
    zip.start_file("payload/bin/tool", options).unwrap();
    zip.write_all(tool.as_bytes()).unwrap();
    zip.start_file("payload/README", options).unwrap();
    zip.write_all(b"readme").unwrap();
    zip.finish().unwrap();
}

fn write_tar_gz(path: &Path, tool: &str) {
    let encoder =
        flate2::write::GzEncoder::new(fs::File::create(path).unwrap(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_size(tool.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    tar.append_data(&mut header, "bin/tool", tool.as_bytes())
        .unwrap();
    tar.into_inner().unwrap().finish().unwrap();
}

fn archive_option(id: &str, kind: &str, source: &str, sha256: &str, extra: &str) -> String {
    let payload_root = if kind == "zip" {
        "payload_root = \"payload\"\n"
    } else {
        ""
    };
    format!(
        "{header}{extra}[installer_options.installation]\ntype = \"{kind}\"\nsource = \"{source}\"\nsha256 = \"{sha256}\"\n{payload_root}\n{LAUNCHER}",
        header = option_header(id)
    )
}

#[test]
fn shell_method_installs_lists_runs_and_runs_the_authored_uninstall() {
    let fixture = Fixture::new("shell-app");
    let user = UserScratch::new();
    let app_root = fixture.root.path().join("external-root");
    fixture.script(
        "install.sh",
        &format!(
            "#!/bin/sh\nset -e\nmkdir -p \"$VIBE_APP_ROOT/bin\"\nprintf '%s' '{TOOL}' > \"$VIBE_APP_ROOT/bin/tool\"\nchmod 755 \"$VIBE_APP_ROOT/bin/tool\"\ntouch \"$VIBE_APP_ROOT/installed.marker\"\necho SHELL_INSTALLED\n"
        ),
    );
    fixture.script(
        "uninstall.sh",
        "#!/bin/sh\ntouch \"$VIBE_APP_ROOT/../uninstalled.marker\"\necho SHELL_UNINSTALLED\n",
    );
    let package_dir = fixture.package.display().to_string();
    fixture.manifest(
        "shell-app",
        &format!(
            "{header}[installer_options.installation]\ntype = \"shell\"\napp_root = \"{root}\"\ninstall = \"sh '{dir}/install.sh'\"\nforce_install = \"sh '{dir}/install.sh'\"\nuninstall = \"sh '{dir}/uninstall.sh'\"\nforce_uninstall = \"sh '{dir}/uninstall.sh'\"\ninterpreter = \"posix_sh\"\n\n{LAUNCHER}",
            header = option_header("shell"),
            root = app_root.display(),
            dir = package_dir,
        ),
    );

    let refused =
        failed(&fixture.install(&user, "org.example/shell-app", &["--library", "default"]));
    assert!(
        refused.contains("--library applies only to owned methods"),
        "{refused}"
    );
    let installed = fixture.install(&user, "org.example/shell-app", &["--json"]);
    let report = json(&installed);
    assert_eq!(report["selectedMode"], "shell");
    assert_eq!(report["installation"], "org.example/shell-app@1.0.0");
    assert_eq!(report["slotDefault"], "org.example/shell-app@1.0.0");
    assert!(String::from_utf8_lossy(&installed.stderr).contains("SHELL_INSTALLED"));
    assert!(app_root.join("installed.marker").is_file());
    let record = &index(&user)["installations"]["org.example/shell-app@1.0.0"];
    assert_eq!(record["method"], "shell");
    assert_eq!(record["appRoot"], app_root.display().to_string());
    assert_eq!(record["portable"], false);
    assert_eq!(record["publicCommands"][0]["name"], "tool");
    assert!(
        !user.settings.join("opt/bin").exists(),
        "PATH proxies are a later phase"
    );

    let listed = json(&fixture.vibe(&user, &["app", "list", "--json"]));
    let row = &listed["installations"][0];
    assert_eq!(row["status"], "ready");
    assert_eq!(row["default"], true);
    let launchers = json(&fixture.vibe(
        &user,
        &["app", "launchers", "org.example/shell-app", "--json"],
    ));
    let _: vibe_wire::generated::application_launchers::ApplicationLaunchers =
        serde_json::from_value(launchers.clone()).unwrap();
    assert_eq!(launchers["launchers"][0]["status"], "available");

    let ran = ok(&fixture.vibe(
        &user,
        &[
            "app",
            "run",
            "org.example/shell-app",
            "main",
            "--",
            "two words",
        ],
    ));
    assert!(ran.contains("TOOL_RAN fixed two words"), "{ran}");
    let started = ok(&fixture.vibe(&user, &["app", "start", "org.example/shell-app"]));
    assert!(started.contains("TOOL_RAN fixed"), "{started}");

    let removed = fixture.vibe(
        &user,
        &["uninstall", "-g", "org.example/shell-app", "--json"],
    );
    let report = json(&removed);
    assert!(report["slotDefault"].is_null());
    assert!(String::from_utf8_lossy(&removed.stderr).contains("SHELL_UNINSTALLED"));
    assert!(fixture.root.path().join("uninstalled.marker").is_file());
    assert!(app_root.is_dir(), "an external app_root is never deleted");
    assert_eq!(index(&user)["installations"], serde_json::json!({}));
}

#[test]
fn zip_and_tar_gz_payloads_land_in_the_default_library_and_are_removed() {
    let fixture = Fixture::new("archive-app");
    let user = UserScratch::new();
    write_zip(&fixture.package.join("tool.zip"), TOOL);
    write_tar_gz(&fixture.package.join("tool.tar.gz"), TOOL);
    let zip_sha = digest(&fixture.package.join("tool.zip"));
    let tar_sha = digest(&fixture.package.join("tool.tar.gz"));
    fixture.manifest(
        "archive-app",
        &format!(
            "{}{}",
            archive_option("zip", "zip", "tool.zip", &zip_sha, ""),
            archive_option("tgz", "tar_gz", "tool.tar.gz", &tar_sha, "")
        ),
    );
    let payload = user
        .settings
        .join("opt/apps/tool/org.example.archive-app@1.0.0");
    for installer in ["zip", "tgz"] {
        let report = json(&fixture.install(
            &user,
            "org.example/archive-app",
            &["--installer", installer, "--json"],
        ));
        assert_eq!(report["appRoot"], payload.display().to_string());
        assert!(payload.join("bin/tool").is_file(), "{installer}");
        assert!(user.settings.join("opt/vibe-library.toml").is_file());
        let record = &index(&user)["installations"]["org.example/archive-app@1.0.0"];
        assert_eq!(record["library"], "default");
        assert_eq!(
            record["directory"],
            "apps/tool/org.example.archive-app@1.0.0"
        );
        assert_eq!(record["portable"], true);
        assert_eq!(record["option"], installer);
        let ran = ok(&fixture.vibe(&user, &["app", "run", "org.example/archive-app", "main"]));
        assert!(ran.contains("TOOL_RAN fixed"), "{ran}");
        let leftovers: Vec<_> = fs::read_dir(user.settings.join("opt/apps/tool"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect();
        assert_eq!(leftovers.len(), 1, "{leftovers:?}");
        ok(&fixture.vibe(&user, &["uninstall", "-g", "org.example/archive-app"]));
        assert!(!payload.exists(), "{installer}");
    }
}

#[test]
fn hooks_run_in_order_and_a_failing_post_hook_keeps_the_previous_record() {
    let fixture = Fixture::new("hooked-app");
    let user = UserScratch::new();
    let log = fixture.root.path().join("hooks.log");
    let log_text = log.display().to_string();
    write_zip(&fixture.package.join("tool.zip"), TOOL);
    let first_sha = digest(&fixture.package.join("tool.zip"));
    let hooks = |post: &str| {
        format!(
            "pre_install = \"echo PRE >> '{log_text}'; test -n \\\"$VIBE_APP_ROOT\\\"\"\npost_install = \"{post}\"\n"
        )
    };
    let good_post = format!("test -x \\\"$VIBE_APP_ROOT/bin/tool\\\" && echo POST >> '{log_text}'");
    fixture.manifest(
        "hooked-app",
        &archive_option("zip", "zip", "tool.zip", &first_sha, &hooks(&good_post)),
    );
    ok(&fixture.install(&user, "org.example/hooked-app", &[]));
    assert_eq!(fs::read_to_string(&log).unwrap(), "PRE\nPOST\n");
    let before = index(&user);

    write_zip(&fixture.package.join("tool.zip"), "#!/bin/sh\necho NEW\n");
    let second_sha = digest(&fixture.package.join("tool.zip"));
    let bad_post = format!("echo POST_FAILED >> '{log_text}'; exit 3");
    fixture.manifest(
        "hooked-app",
        &archive_option("zip", "zip", "tool.zip", &second_sha, &hooks(&bad_post)),
    );
    let error = failed(&fixture.install(&user, "org.example/hooked-app", &[]));
    assert!(error.contains("post_install failed"), "{error}");
    assert_eq!(index(&user), before, "the previous record stays");
    let ran = ok(&fixture.vibe(&user, &["app", "run", "org.example/hooked-app", "main"]));
    assert!(
        ran.contains("TOOL_RAN"),
        "the previous payload is restored: {ran}"
    );
    assert_eq!(
        fs::read_to_string(&log).unwrap(),
        "PRE\nPOST\nPRE\nPOST_FAILED\n"
    );
}

#[test]
fn owned_payloads_are_replaced_and_removed_without_checks_and_force_is_narrow() {
    let fixture = Fixture::new("forced-app");
    let user = UserScratch::new();
    write_zip(&fixture.package.join("tool.zip"), TOOL);
    let sha = digest(&fixture.package.join("tool.zip"));
    fixture.manifest(
        "forced-app",
        &archive_option("zip", "zip", "tool.zip", &sha, ""),
    );
    let payload = user
        .settings
        .join("opt/apps/tool/org.example.forced-app@1.0.0");

    fs::create_dir_all(payload.join("bin")).unwrap();
    let error = failed(&fixture.install(&user, "org.example/forced-app", &[]));
    assert!(error.contains("no index record owns it"), "{error}");
    assert!(!user.settings.join("applications/index.json").exists());
    ok(&fixture.install(&user, "org.example/forced-app", &["--force"]));
    assert!(payload.join("bin/tool").is_file());

    fs::write(payload.join("README"), "edited by hand").unwrap();
    ok(&fixture.install(&user, "org.example/forced-app", &[]));
    assert_eq!(
        fs::read_to_string(payload.join("README")).unwrap(),
        "readme"
    );

    fs::write(payload.join("extra"), "new file").unwrap();
    ok(&fixture.vibe(&user, &["uninstall", "-g", "org.example/forced-app"]));
    assert!(
        !payload.exists(),
        "an owned payload is deleted unconditionally"
    );

    let shell = Fixture::new("shell-force");
    let app_root = shell.root.path().join("root");
    shell.manifest(
        "shell-force",
        &format!(
            "{header}[installer_options.installation]\ntype = \"shell\"\napp_root = \"{root}\"\ninstall = \"mkdir -p '{root}'\"\nforce_install = \"mkdir -p '{root}'\"\nuninstall = \"true\"\nforce_uninstall = \"true\"\n",
            header = option_header("shell"),
            root = app_root.display(),
        ),
    );
    ok(&shell.install(&user, "org.example/shell-force", &[]));
    let index_path = user.settings.join("applications/index.json");
    let mut document = index(&user);
    document["installations"]["org.example/shell-force@1.0.0"]["commands"]
        .as_object_mut()
        .unwrap()
        .remove("forceUninstall");
    fs::write(&index_path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    let error = failed(&shell.vibe(
        &user,
        &["uninstall", "-g", "org.example/shell-force", "--force"],
    ));
    assert!(error.contains("declares no force_uninstall"), "{error}");
    assert!(index(&user)["installations"]["org.example/shell-force@1.0.0"].is_object());
    ok(&shell.vibe(&user, &["uninstall", "-g", "org.example/shell-force"]));
}

#[test]
fn ambiguous_unattended_selection_is_an_error_before_any_mutation() {
    let fixture = Fixture::new("choice-app");
    let user = UserScratch::new();
    write_zip(&fixture.package.join("tool.zip"), TOOL);
    let sha = digest(&fixture.package.join("tool.zip"));
    fixture.manifest(
        "choice-app",
        &format!(
            "{}{}",
            archive_option("first", "zip", "tool.zip", &sha, ""),
            archive_option("second", "zip", "tool.zip", &sha, "")
        ),
    );
    let error = failed(&fixture.install(&user, "org.example/choice-app", &[]));
    assert!(error.contains("selection_required"), "{error}");
    assert!(
        error.contains("--installer") && error.contains("first, second"),
        "{error}"
    );
    assert!(!user.settings.join("applications/index.json").exists());
    assert!(!user.settings.join("opt").exists());

    let error = failed(&fixture.install(&user, "org.example/choice-app", &["--platform", "plan9"]));
    assert!(error.contains("no installer option"), "{error}");

    fixture.manifest("choice-app", "");
    let error = failed(&fixture.install(&user, "org.example/choice-app", &[]));
    assert!(error.contains("not globally installable"), "{error}");
}

#[test]
fn update_adds_an_owned_version_beside_and_reruns_a_shell_in_place() {
    let fixture = Fixture::new("archive-app");
    let user = UserScratch::new();
    write_zip(&fixture.package.join("tool.zip"), TOOL);
    let sha = digest(&fixture.package.join("tool.zip"));
    fixture.manifest(
        "archive-app",
        &archive_option("zip", "zip", "tool.zip", &sha, ""),
    );
    ok(&fixture.install(&user, "org.example/archive-app", &[]));
    let newer = fixture.registry.join("org.example/archive-app/v1.1.0");
    fs::create_dir_all(&newer).unwrap();
    fs::copy(fixture.package.join("tool.zip"), newer.join("tool.zip")).unwrap();
    fs::write(
        newer.join("vibe.toml"),
        fs::read_to_string(fixture.package.join("vibe.toml"))
            .unwrap()
            .replace("version = \"1.0.0\"", "version = \"1.1.0\""),
    )
    .unwrap();
    let registry = fixture.registry.to_str().unwrap();
    let report = json(&fixture.vibe(
        &user,
        &[
            "update",
            "-g",
            "org.example/archive-app",
            "--registry",
            registry,
            "--json",
        ],
    ));
    assert_eq!(report["installation"], "org.example/archive-app@1.1.0");
    assert_eq!(report["slotDefault"], "org.example/archive-app@1.1.0");
    let installations = &index(&user)["installations"];
    assert!(installations["org.example/archive-app@1.0.0"].is_object());
    assert!(installations["org.example/archive-app@1.1.0"].is_object());
    let removed = json(&fixture.vibe(
        &user,
        &[
            "uninstall",
            "-g",
            "org.example/archive-app@=1.1.0",
            "--json",
        ],
    ));
    assert_eq!(removed["slotDefault"], "org.example/archive-app@1.0.0");

    let shell = Fixture::new("shell-up");
    let app_root = shell.root.path().join("root");
    let log = shell.root.path().join("ops.log");
    let body = |version: &str| {
        format!(
            "[package]\nname = \"shell-up\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"{version}\"\n\n[application]\nid = \"up\"\n\n{header}[installer_options.installation]\ntype = \"shell\"\napp_root = \"{root}\"\ninstall = \"mkdir -p '{root}' && echo install-{version} >> '{log}'\"\nforce_install = \"echo force-{version} >> '{log}'\"\nupdate = \"echo update-{version} >> '{log}'\"\nuninstall = \"echo uninstall >> '{log}'\"\nforce_uninstall = \"echo force-uninstall >> '{log}'\"\n",
            header = option_header("shell"),
            root = app_root.display(),
            log = log.display(),
        )
    };
    fs::write(shell.package.join("vibe.toml"), body("1.0.0")).unwrap();
    ok(&shell.install(&user, "org.example/shell-up", &[]));
    let next = shell.registry.join("org.example/shell-up/v2.0.0");
    fs::create_dir_all(&next).unwrap();
    fs::write(next.join("vibe.toml"), body("2.0.0")).unwrap();
    let registry = shell.registry.to_str().unwrap();
    let report = json(&shell.vibe(
        &user,
        &[
            "update",
            "-g",
            "org.example/shell-up",
            "--registry",
            registry,
            "--json",
        ],
    ));
    assert_eq!(report["installation"], "org.example/shell-up@2.0.0");
    assert_eq!(report["slotDefault"], "org.example/shell-up@2.0.0");
    assert_eq!(
        fs::read_to_string(&log).unwrap(),
        "install-1.0.0\nupdate-2.0.0\n"
    );
    let installations = &index(&user)["installations"];
    assert!(installations["org.example/shell-up@1.0.0"].is_null());
    assert!(installations["org.example/shell-up@2.0.0"].is_object());
}
