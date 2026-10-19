//! Local, cross-platform application installer and launcher CLI coverage.

mod common;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::UserScratch;
use serde_json::Value;
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;

const PACKAGE: &str = "org.example/choice-app";

#[test]
fn command_installer_options_are_listed_and_selected_by_os_and_arch() {
    let fixture = Fixture::new(true);
    let user = UserScratch::new();

    let listed = fixture.json(
        &user,
        &[
            "app",
            "installers",
            PACKAGE,
            "--registry",
            fixture.registry_text(),
            "--json",
        ],
    );
    let _: vibe_wire::generated::application_installer_options::ApplicationInstallerOptions =
        serde_json::from_value(listed.clone()).unwrap();
    let platforms = listed["platforms"].as_array().unwrap();
    assert_eq!(platforms.len(), 2);
    assert!(
        platforms
            .iter()
            .any(|row| row["id"] == fixture.os && row["name"] == fixture.os)
    );
    assert!(
        platforms
            .iter()
            .any(|row| row["id"] == "plan9" && row["name"] == "Plan 9")
    );
    let options: Vec<&Value> = platforms
        .iter()
        .flat_map(|platform| platform["architectures"].as_array().unwrap())
        .flat_map(|arch| arch["installers"].as_array().unwrap())
        .collect();
    assert_eq!(options.len(), 3);
    assert!(options.iter().any(|row| row["id"] == "host"));
    assert!(options.iter().any(|row| row["id"] == "other-cpu"));
    assert!(options.iter().any(|row| row["id"] == "other-os"));

    // An explicitly named, non-detected target is selectable; the helper is
    // deliberately the same local binary, so no foreign program is executed.
    let installed = fixture.output(
        &user,
        &[
            "install",
            "-g",
            PACKAGE,
            "--registry",
            fixture.registry_text(),
            "--platform",
            fixture.os,
            "--arch",
            fixture.other_arch,
            "--installer",
            "other-cpu",
            "--json",
        ],
    );
    assert_success(&installed);
    assert!(String::from_utf8_lossy(&installed.stderr).contains("INSTALL_STDOUT"));
    assert!(String::from_utf8_lossy(&installed.stderr).contains("INSTALL_STDERR"));
    assert!(fixture.other_root.join("bin").join(helper_name()).is_file());

    let index: Value =
        serde_json::from_slice(&fs::read(user.settings.join("applications/index.json")).unwrap())
            .unwrap();
    assert_eq!(
        index["applications"]["choice"]["selectedOptionId"],
        "other-cpu"
    );
    assert_eq!(
        index["applications"]["choice"]["appRoot"],
        fixture.other_root_text()
    );
}

#[test]
fn unknown_platform_can_be_selected_explicitly() {
    let fixture = Fixture::new(true);
    let user = UserScratch::new();
    let installed = fixture.output(
        &user,
        &[
            "install",
            "-g",
            PACKAGE,
            "--registry",
            fixture.registry_text(),
            "--platform",
            "plan9",
            "--arch",
            fixture.arch,
            "--installer",
            "other-os",
            "--json",
        ],
    );
    assert_success(&installed);
    assert!(String::from_utf8_lossy(&installed.stderr).contains("INSTALL_STDOUT"));
    assert!(fixture.other_root.join("bin").join(helper_name()).is_file());
    let listing = fixture.json(&user, &["app", "launchers", PACKAGE, "--json"]);
    let _: vibe_wire::generated::application_launchers::ApplicationLaunchers =
        serde_json::from_value(listing.clone()).unwrap();
    assert_eq!(listing["launchers"][0]["id"], "alien");
    assert_eq!(listing["app_root"], fixture.other_root_text());
}

#[test]
fn local_zip_option_installs_verified_payload_and_launcher() {
    let fixture = Fixture::new(true);
    let user = UserScratch::new();
    let package_dir = fixture.manifest.parent().unwrap();
    let archive = package_dir.join("payload.zip");
    let file = fs::File::create(&archive).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file(
        format!("bin/{}", helper_name()),
        SimpleFileOptions::default().unix_permissions(0o755),
    )
    .unwrap();
    zip.write_all(&fs::read(&fixture.helper).unwrap()).unwrap();
    zip.finish().unwrap();
    let digest = format!("{:x}", Sha256::digest(fs::read(&archive).unwrap()));
    fs::write(
        &fixture.manifest,
        format!(
            "[package]\nname='choice-app'\ngroup='org.example'\nkind='app'\nversion='1.0.0'\n\
             [application]\nid='choice'\ncommands=[]\n\
             [[installer_options]]\nid='local-zip'\nname='Local ZIP'\nplatform={platform}\nplatform_name={platform}\narch={arch}\n\
             [installer_options.installation]\ntype='zip'\nsource='payload.zip'\nsha256='{digest}'\n\
             [[installer_options.launchers]]\nid='main'\nname='ZIP Main'\nexecutable='bin/{helper}'\nargs=['zip-fixed']\ndefault=true\n",
            platform = toml_string(fixture.os),
            arch = toml_string(fixture.arch),
            helper = helper_name(),
        ),
    )
    .unwrap();

    let installed = fixture.output(
        &user,
        &[
            "install",
            "-g",
            PACKAGE,
            "--registry",
            fixture.registry_text(),
            "--platform",
            fixture.os,
            "--arch",
            fixture.arch,
            "--installer",
            "local-zip",
            "--json",
        ],
    );
    assert_success(&installed);
    let report: Value = serde_json::from_slice(&installed.stdout).unwrap();
    assert_eq!(report["selectedMode"], "zip");
    let listing = fixture.json(&user, &["app", "launchers", PACKAGE, "--json"]);
    assert_eq!(listing["default_launcher_id"], "main");
    assert_eq!(listing["launchers"][0]["status"], "available");
    let started = fixture.output(&user, &["app", "start", PACKAGE]);
    assert_success(&started);
    assert!(String::from_utf8_lossy(&started.stdout).contains("LAUNCH_ARG=zip-fixed"));
}

#[test]
fn failed_zip_post_install_keeps_prior_index_payload_and_launcher() {
    let fixture = Fixture::new(true);
    let user = UserScratch::new();
    let package_dir = fixture.manifest.parent().unwrap();
    let initial_archive = package_dir.join("payload-initial.zip");
    write_zip_payload(&initial_archive, &fixture.helper, false);
    let initial_digest = format!("{:x}", Sha256::digest(fs::read(&initial_archive).unwrap()));
    write_zip_manifest(&fixture, "payload-initial.zip", &initial_digest, None);

    let installed = fixture.output(
        &user,
        &[
            "install",
            "-g",
            PACKAGE,
            "--registry",
            fixture.registry_text(),
            "--platform",
            fixture.os,
            "--arch",
            fixture.arch,
            "--installer",
            "local-zip",
            "--json",
        ],
    );
    assert_success(&installed);
    let index_path = user.settings.join("applications/index.json");
    let prior_index = fs::read(&index_path).unwrap();
    let prior_record: Value = serde_json::from_slice(&prior_index).unwrap();
    let prior_root = PathBuf::from(
        prior_record["applications"]["choice"]["appRoot"]
            .as_str()
            .unwrap(),
    );
    let prior_executable = prior_root.join("bin").join(helper_name());
    let prior_payload = fs::read(&prior_executable).unwrap();
    let prior_listing = fixture.json(&user, &["app", "launchers", PACKAGE, "--json"]);

    let replacement_archive = package_dir.join("payload-replacement.zip");
    write_zip_payload(&replacement_archive, &fixture.helper, true);
    let replacement_digest = format!(
        "{:x}",
        Sha256::digest(fs::read(&replacement_archive).unwrap())
    );
    let failing_hook = shell_command(&fixture.helper, "fail-post-install", &fixture.host_root);
    write_zip_manifest(
        &fixture,
        "payload-replacement.zip",
        &replacement_digest,
        Some(&failing_hook),
    );

    let failed = fixture.output(
        &user,
        &[
            "update",
            "-g",
            PACKAGE,
            "--registry",
            fixture.registry_text(),
            "--platform",
            fixture.os,
            "--arch",
            fixture.arch,
            "--installer",
            "local-zip",
        ],
    );
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("post_install"));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("POST_INSTALL_FAILED"));
    assert_eq!(fs::read(&index_path).unwrap(), prior_index);
    assert_eq!(fs::read(&prior_executable).unwrap(), prior_payload);
    let after = fixture.json(&user, &["app", "launchers", PACKAGE, "--json"]);
    assert_eq!(after, prior_listing);
    let started = fixture.output(&user, &["app", "start", PACKAGE]);
    assert_success(&started);
    assert!(String::from_utf8_lossy(&started.stdout).contains("LAUNCH_ARG=zip-fixed"));
}

#[test]
fn local_tar_gz_option_installs_verified_payload() {
    let fixture = Fixture::new(true);
    let user = UserScratch::new();
    let package_dir = fixture.manifest.parent().unwrap();
    let archive = package_dir.join("payload.tar.gz");
    let output = fs::File::create(&archive).unwrap();
    let encoder = flate2::write::GzEncoder::new(output, flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);
    let bytes = fs::read(&fixture.helper).unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    tar.append_data(
        &mut header,
        format!("bin/{}", helper_name()),
        bytes.as_slice(),
    )
    .unwrap();
    tar.into_inner().unwrap().finish().unwrap();
    let digest = format!("{:x}", Sha256::digest(fs::read(&archive).unwrap()));
    fs::write(
        &fixture.manifest,
        format!(
            "[package]\nname='choice-app'\ngroup='org.example'\nkind='app'\nversion='1.0.0'\n\
             [application]\nid='choice'\ncommands=[]\n\
             [[installer_options]]\nid='local-tar'\nname='Local tar.gz'\nplatform={platform}\nplatform_name={platform}\narch={arch}\n\
             [installer_options.installation]\ntype='tar_gz'\nsource='payload.tar.gz'\nsha256='{digest}'\n\
             [[installer_options.launchers]]\nid='main'\nname='tar Main'\nexecutable='bin/{helper}'\nargs=['tar-fixed']\ndefault=true\n",
            platform = toml_string(fixture.os),
            arch = toml_string(fixture.arch),
            helper = helper_name(),
        ),
    )
    .unwrap();
    let installed = fixture.output(
        &user,
        &[
            "install",
            "-g",
            PACKAGE,
            "--registry",
            fixture.registry_text(),
            "--platform",
            fixture.os,
            "--arch",
            fixture.arch,
            "--installer",
            "local-tar",
            "--json",
        ],
    );
    assert_success(&installed);
    let report: Value = serde_json::from_slice(&installed.stdout).unwrap();
    assert_eq!(report["selectedMode"], "tar_gz");
    let listing = fixture.json(&user, &["app", "launchers", PACKAGE, "--json"]);
    assert_eq!(listing["launchers"][0]["status"], "available");
    let started = fixture.output(&user, &["app", "start", PACKAGE]);
    assert_success(&started);
    assert!(String::from_utf8_lossy(&started.stdout).contains("LAUNCH_ARG=tar-fixed"));
}

#[test]
fn command_install_launch_and_update_keep_literal_args_and_current_paths() {
    let fixture = Fixture::new(true);
    let user = UserScratch::new();
    let installed = fixture.output(
        &user,
        &[
            "install",
            "-g",
            PACKAGE,
            "--registry",
            fixture.registry_text(),
            "--platform",
            fixture.os,
            "--arch",
            fixture.arch,
            "--installer",
            "host",
        ],
    );
    assert_success(&installed);
    assert!(String::from_utf8_lossy(&installed.stdout).contains("INSTALL_STDOUT"));
    assert!(String::from_utf8_lossy(&installed.stderr).contains("INSTALL_STDERR"));

    let listing = fixture.json(&user, &["app", "launchers", PACKAGE, "--json"]);
    assert_eq!(listing["protocol"], "vibe-app-launchers/1");
    assert_eq!(listing["default_launcher_id"], "main");
    assert_eq!(listing["app_root"], fixture.host_root_text());
    assert_eq!(listing["launchers"].as_array().unwrap().len(), 2);
    assert_eq!(listing["launchers"][0]["id"], "main");
    assert_eq!(listing["launchers"][0]["status"], "available");
    assert_eq!(
        listing["launchers"][0]["executable"]["template"],
        format!("bin/{}", helper_name())
    );
    assert_eq!(listing["launchers"][1]["id"], "secondary");

    let literal = "a b;$HOME$(echo untouched)";
    let launched = fixture.output(&user, &["app", "run", PACKAGE, "secondary", "--", literal]);
    assert_success(&launched);
    let stdout = String::from_utf8_lossy(&launched.stdout);
    assert!(stdout.contains("LAUNCH_ARG=secondary-fixed"), "{stdout}");
    assert!(
        stdout.contains(&format!("LAUNCH_ARG={literal}")),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!("LAUNCH_CWD={}", fixture.host_root_text())),
        "{stdout}"
    );

    let started = fixture.output(&user, &["app", "start", PACKAGE]);
    assert_success(&started);
    assert!(String::from_utf8_lossy(&started.stdout).contains("LAUNCH_ARG=main-fixed"));

    fixture.write_manifest(false);
    let updated = fixture.output(
        &user,
        &[
            "update",
            "-g",
            PACKAGE,
            "--registry",
            fixture.registry_text(),
            "--platform",
            fixture.os,
            "--arch",
            fixture.arch,
            "--installer",
            "host",
        ],
    );
    assert_success(&updated);
    assert!(String::from_utf8_lossy(&updated.stdout).contains("UPDATE_STDOUT"));
    assert!(String::from_utf8_lossy(&updated.stderr).contains("UPDATE_STDERR"));
    let after = fixture.json(&user, &["app", "launchers", PACKAGE, "--json"]);
    assert_eq!(after["default_launcher_id"], Value::Null);
    assert_eq!(after["launchers"].as_array().unwrap().len(), 1);
    assert_eq!(after["launchers"][0]["id"], "only");
    let start_without_default = fixture.output(&user, &["app", "start", PACKAGE]);
    assert!(!start_without_default.status.success());
    assert!(String::from_utf8_lossy(&start_without_default.stderr).contains("selection_required"));
}

#[test]
fn shell_hooks_run_in_order_and_force_uses_distinct_commands() {
    let fixture = Fixture::new(true);
    let user = UserScratch::new();
    let installed = fixture.output(
        &user,
        &[
            "install",
            "-g",
            PACKAGE,
            "--registry",
            fixture.registry_text(),
            "--platform",
            fixture.os,
            "--arch",
            fixture.arch,
            "--installer",
            "host",
        ],
    );
    assert_success(&installed);
    assert_markers_in_order(
        &installed.stdout,
        &[
            "PRE_INSTALL_STDOUT",
            "INSTALL_STDOUT",
            "POST_INSTALL_STDOUT",
        ],
    );

    let forced = fixture.output(
        &user,
        &[
            "install",
            "-g",
            PACKAGE,
            "--force",
            "--registry",
            fixture.registry_text(),
            "--platform",
            fixture.os,
            "--arch",
            fixture.arch,
            "--installer",
            "host",
        ],
    );
    assert_success(&forced);
    assert_markers_in_order(
        &forced.stdout,
        &[
            "PRE_INSTALL_STDOUT",
            "FORCE_INSTALL_STDOUT",
            "POST_INSTALL_STDOUT",
        ],
    );
    assert!(
        !String::from_utf8_lossy(&forced.stdout)
            .lines()
            .any(|line| line == "INSTALL_STDOUT")
    );

    let removed = fixture.output(&user, &["uninstall", "-g", PACKAGE, "--force"]);
    assert_success(&removed);
    assert!(String::from_utf8_lossy(&removed.stdout).contains("FORCE_UNINSTALL_STDOUT"));
}

#[test]
fn failed_pre_install_hook_preserves_last_successful_index() {
    let fixture = Fixture::new(true);
    let user = UserScratch::new();
    let installed = fixture.output(
        &user,
        &[
            "install",
            "-g",
            PACKAGE,
            "--registry",
            fixture.registry_text(),
            "--platform",
            fixture.os,
            "--arch",
            fixture.arch,
            "--installer",
            "host",
        ],
    );
    assert_success(&installed);
    let index_path = user.settings.join("applications/index.json");
    let before = fs::read(&index_path).unwrap();
    fixture.fail_pre_install_hook();

    let failed = fixture.output(
        &user,
        &[
            "update",
            "-g",
            PACKAGE,
            "--registry",
            fixture.registry_text(),
            "--platform",
            fixture.os,
            "--arch",
            fixture.arch,
            "--installer",
            "host",
        ],
    );
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("PRE_INSTALL_FAILED"));
    assert!(!String::from_utf8_lossy(&failed.stdout).contains("UPDATE_STDOUT"));
    assert_eq!(fs::read(&index_path).unwrap(), before);
}

struct Fixture {
    _root: tempfile::TempDir,
    registry: PathBuf,
    manifest: PathBuf,
    helper: PathBuf,
    host_root: PathBuf,
    other_root: PathBuf,
    os: &'static str,
    arch: &'static str,
    other_arch: &'static str,
}

impl Fixture {
    fn new(default: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        let package = registry.join("org.example/choice-app/v1.0.0");
        fs::create_dir_all(&package).unwrap();
        let helper = root.path().join(helper_name());
        compile_helper(&helper);
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let other_arch = if arch == "aarch64" {
            "x86_64"
        } else {
            "aarch64"
        };
        let host_root = root.path().join("External App Ω");
        let other_root = root.path().join("Other CPU Ω");
        let fixture = Self {
            _root: root,
            registry,
            manifest: package.join("vibe.toml"),
            helper,
            host_root,
            other_root,
            os,
            arch,
            other_arch,
        };
        fixture.write_manifest(default);
        fixture
    }

    fn write_manifest(&self, default: bool) {
        let host_launcher = if default {
            format!(
                "[[installer_options.launchers]]\nid='main'\nname='Main'\nexecutable='bin/{}'\nargs=['main-fixed']\ndefault=true\n\
                 [[installer_options.launchers]]\nid='secondary'\nname='Secondary'\nexecutable='bin/{}'\nargs=['secondary-fixed']\n",
                helper_name(),
                helper_name(),
            )
        } else {
            format!(
                "[[installer_options.launchers]]\nid='only'\nname='Only'\nexecutable='bin/{}'\nargs=['only-fixed']\n",
                helper_name(),
            )
        };
        let content = format!(
            "[package]\nname='choice-app'\ngroup='org.example'\nkind='app'\nversion='1.0.0'\n\
             [application]\nid='choice'\ncommands=[]\n\
             [[installer_options]]\nid='host'\nname='Host Installer'\nplatform={os}\nplatform_name={os}\narch={arch}\npre_install={host_pre}\npost_install={host_post}\n\
             [installer_options.installation]\ntype='shell'\napp_root={host_root}\n\
             install={host_install}\nforce_install={host_force_install}\nupdate={host_update}\nuninstall={host_uninstall}\nforce_uninstall={host_force_uninstall}\n\
             {host_launcher}\n\
             [[installer_options]]\nid='other-cpu'\nname='Other CPU Installer'\nplatform={os}\nplatform_name={os}\narch={other_arch}\n\
             [installer_options.installation]\ntype='shell'\napp_root={other_root}\n\
             install={other_install}\nforce_install={other_force_install}\nuninstall={other_uninstall}\nforce_uninstall={other_force_uninstall}\n\
             [[installer_options.launchers]]\nid='other'\nname='Other CPU'\nexecutable='bin/{helper_file}'\n\
             [[installer_options]]\nid='other-os'\nname='Other OS Installer'\nplatform='plan9'\nplatform_name='Plan 9'\narch={arch}\n\
             [installer_options.installation]\ntype='shell'\napp_root={other_root}\n\
             install={other_install}\nforce_install={other_force_install}\nuninstall={other_uninstall}\nforce_uninstall={other_force_uninstall}\n\
             [[installer_options.launchers]]\nid='alien'\nname='Other OS'\nexecutable='bin/{helper_file}'\n",
            os = toml_string(self.os),
            arch = toml_string(self.arch),
            other_arch = toml_string(self.other_arch),
            host_root = toml_string(self.host_root_text()),
            other_root = toml_string(self.other_root_text()),
            host_pre = toml_string(&shell_command(&self.helper, "pre-install", &self.host_root)),
            host_post = toml_string(&shell_command(
                &self.helper,
                "post-install",
                &self.host_root
            )),
            host_install = toml_string(&shell_command(&self.helper, "install", &self.host_root)),
            host_force_install = toml_string(&shell_command(
                &self.helper,
                "force-install",
                &self.host_root
            )),
            host_update = toml_string(&shell_command(&self.helper, "update", &self.host_root)),
            host_uninstall =
                toml_string(&shell_command(&self.helper, "uninstall", &self.host_root)),
            host_force_uninstall = toml_string(&shell_command(
                &self.helper,
                "force-uninstall",
                &self.host_root
            )),
            other_install = toml_string(&shell_command(&self.helper, "install", &self.other_root)),
            other_force_install = toml_string(&shell_command(
                &self.helper,
                "force-install",
                &self.other_root
            )),
            other_uninstall =
                toml_string(&shell_command(&self.helper, "uninstall", &self.other_root)),
            other_force_uninstall = toml_string(&shell_command(
                &self.helper,
                "force-uninstall",
                &self.other_root
            )),
            helper_file = helper_name(),
        );
        fs::write(&self.manifest, content).unwrap();
    }

    fn fail_pre_install_hook(&self) {
        let original = fs::read_to_string(&self.manifest).unwrap();
        let prior = format!(
            "pre_install={}",
            toml_string(&shell_command(&self.helper, "pre-install", &self.host_root))
        );
        let failing = format!(
            "pre_install={}",
            toml_string(&shell_command(
                &self.helper,
                "fail-pre-install",
                &self.host_root
            ))
        );
        assert!(original.contains(&prior));
        fs::write(&self.manifest, original.replacen(&prior, &failing, 1)).unwrap();
    }

    fn registry_text(&self) -> &str {
        self.registry.to_str().unwrap()
    }
    fn host_root_text(&self) -> &str {
        self.host_root.to_str().unwrap()
    }
    fn other_root_text(&self) -> &str {
        self.other_root.to_str().unwrap()
    }

    fn output(&self, user: &UserScratch, args: &[&str]) -> Output {
        user.vibe()
            .current_dir(self._root.path())
            .args(args)
            .output()
            .unwrap()
    }

    fn json(&self, user: &UserScratch, args: &[&str]) -> Value {
        let output = self.output(user, args);
        assert_success(&output);
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

fn write_zip_payload(archive: &Path, helper: &Path, replacement: bool) {
    let file = fs::File::create(archive).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file(
        format!("bin/{}", helper_name()),
        SimpleFileOptions::default().unix_permissions(0o755),
    )
    .unwrap();
    zip.write_all(&fs::read(helper).unwrap()).unwrap();
    if replacement {
        zip.start_file("replacement.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"new generation").unwrap();
    }
    zip.finish().unwrap();
}

fn write_zip_manifest(fixture: &Fixture, source: &str, digest: &str, post_install: Option<&str>) {
    let post = post_install.map_or_else(String::new, |command| {
        format!("post_install={}\n", toml_string(command))
    });
    fs::write(
        &fixture.manifest,
        format!(
            "[package]\nname='choice-app'\ngroup='org.example'\nkind='app'\nversion='1.0.0'\n\
             [application]\nid='choice'\ncommands=[]\n\
             [[installer_options]]\nid='local-zip'\nname='Local ZIP'\nplatform={platform}\nplatform_name={platform}\narch={arch}\n{post}\
             [installer_options.installation]\ntype='zip'\nsource={source}\nsha256='{digest}'\n\
             [[installer_options.launchers]]\nid='main'\nname='ZIP Main'\nexecutable='bin/{helper}'\nargs=['zip-fixed']\ndefault=true\n",
            platform = toml_string(fixture.os),
            arch = toml_string(fixture.arch),
            source = toml_string(source),
            helper = helper_name(),
        ),
    )
    .unwrap();
}

fn helper_name() -> &'static str {
    if cfg!(windows) {
        "fixture-app.exe"
    } else {
        "fixture-app"
    }
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap()
}

fn shell_command(helper: &Path, operation: &str, app_root: &Path) -> String {
    format!(
        "{} {operation} {}",
        shell_word(helper),
        shell_word(app_root)
    )
}

fn shell_word(path: &Path) -> String {
    let value = path.to_str().unwrap();
    if cfg!(windows) {
        format!("\"{value}\"")
    } else {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn assert_markers_in_order(output: &[u8], markers: &[&str]) {
    let output = String::from_utf8_lossy(output);
    let mut offset = 0;
    for marker in markers {
        let relative = output[offset..]
            .find(marker)
            .unwrap_or_else(|| panic!("missing marker `{marker}` in {output}"));
        offset += relative + marker.len();
    }
}

fn compile_helper(path: &Path) {
    let source = path.with_extension("rs");
    fs::write(&source, HELPER_SOURCE).unwrap();
    let output = Command::new("rustc")
        .arg("--edition=2021")
        .arg(&source)
        .arg("-o")
        .arg(path)
        .output()
        .unwrap();
    assert_success(&output);
}

const HELPER_SOURCE: &str = r#"
use std::{env, fs, path::PathBuf};
fn main() {
    let mut args = env::args_os().skip(1);
    let operation = args.next().and_then(|arg| arg.to_str().map(str::to_owned));
    match operation.as_deref() {
        Some("install" | "force-install" | "update") => {
            let root = PathBuf::from(args.next().expect("payload root"));
            let bin = root.join("bin");
            fs::create_dir_all(&bin).unwrap();
            let destination = bin.join(if cfg!(windows) { "fixture-app.exe" } else { "fixture-app" });
            fs::copy(env::current_exe().unwrap(), &destination).unwrap();
            #[cfg(unix)] {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&destination, fs::Permissions::from_mode(0o755)).unwrap();
            }
            let marker = match operation.as_deref() {
                Some("install") => "INSTALL",
                Some("force-install") => "FORCE_INSTALL",
                _ => "UPDATE",
            };
            println!("{marker}_STDOUT");
            eprintln!("{marker}_STDERR");
        }
        Some("pre-install") => println!("PRE_INSTALL_STDOUT"),
        Some("post-install") => println!("POST_INSTALL_STDOUT"),
        Some("fail-pre-install") => {
            eprintln!("PRE_INSTALL_FAILED");
            std::process::exit(17);
        }
        Some("fail-post-install") => {
            eprintln!("POST_INSTALL_FAILED");
            std::process::exit(19);
        }
        Some("uninstall") => { println!("UNINSTALL_STDOUT"); }
        Some("force-uninstall") => { println!("FORCE_UNINSTALL_STDOUT"); }
        Some(first) => {
            println!("LAUNCH_ARG={first}");
            for arg in args { println!("LAUNCH_ARG={}", arg.to_string_lossy()); }
            println!("LAUNCH_CWD={}", env::current_dir().unwrap().display());
        }
        None => panic!("operation or launcher argument is required"),
    }
}
"#;
