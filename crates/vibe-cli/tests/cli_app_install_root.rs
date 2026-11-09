//! User applications follow vibe's own install root (PROP-019
//! `#install-root`): with `$VIBEVM_INSTALL_ROOT` (or `[self] install_root`)
//! their PATH proxies are written to `<root>/opt/bin` beside vibe's shims
//! and new payloads to `<root>/opt/apps`; applications installed under the
//! settings root before keep working where they are until moved.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::UserScratch;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// A local registry holding `org.example/<name>@1.0.0` app packages whose
/// tar.gz payload carries `bin/<command>`.
struct Fixture {
    root: tempfile::TempDir,
    registry: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        Self { root, registry }
    }

    fn package(&self, name: &str, command: &str) {
        let package = self.registry.join(format!("org.example/{name}/v1.0.0"));
        fs::create_dir_all(&package).unwrap();
        let archive = package.join("payload.tar.gz");
        let tool = format!("#!/bin/sh\necho \"{command} RAN $*\"\n");
        let encoder = flate2::write::GzEncoder::new(
            fs::File::create(&archive).unwrap(),
            flate2::Compression::fast(),
        );
        let mut tar = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(tool.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, format!("bin/{command}"), tool.as_bytes())
            .unwrap();
        tar.into_inner().unwrap().finish().unwrap();
        let sha = format!("{:x}", Sha256::digest(fs::read(&archive).unwrap()));
        fs::write(
            package.join("vibe.toml"),
            format!(
                "[package]\nname = \"{name}\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
                 [application]\nid = \"{command}\"\n\n\
                 [[installer_options]]\nid = \"tgz\"\nname = \"Tgz\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\
                 [installer_options.installation]\ntype = \"tar_gz\"\nsource = \"payload.tar.gz\"\nsha256 = \"{sha}\"\n\n\
                 [[installer_options.launchers]]\nid = \"main\"\nname = \"{command}\"\nexecutable = \"$app_bin/{command}\"\nkind = \"command\"\ndefault = true\n\n\
                 [[installer_options.public_commands]]\nname = \"{command}\"\nlauncher = \"main\"\n",
                arch = std::env::consts::ARCH
            ),
        )
        .unwrap();
    }

    fn vibe(&self, user: &UserScratch, root: Option<&Path>, args: &[&str]) -> Output {
        let mut command = user.vibe();
        command.current_dir(self.root.path()).args(args);
        match root {
            Some(root) => command.env("VIBEVM_INSTALL_ROOT", root),
            None => command.env_remove("VIBEVM_INSTALL_ROOT"),
        };
        command.output().unwrap()
    }

    fn install(&self, user: &UserScratch, root: Option<&Path>, package: &str) -> Output {
        let registry = self.registry.to_str().unwrap();
        self.vibe(
            user,
            root,
            &["install", "-g", package, "--registry", registry],
        )
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

/// Run a PATH proxy directly, with nothing of vibe's in its environment.
fn run(proxy: &Path) -> String {
    let output = Command::new(proxy)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    ok(&output)
}

fn index(user: &UserScratch) -> Value {
    serde_json::from_slice(&fs::read(user.settings.join("applications/index.json")).unwrap())
        .unwrap()
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#install-root")]
fn applications_follow_the_install_root_and_earlier_ones_keep_working() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    fixture.package("old-app", "oldtool");
    fixture.package("new-app", "newtool");
    let base = fixture.root.path().join("Work");
    let store = base.join("opt");
    let bin = store.join("bin");
    let legacy = user.settings.join("opt/bin");

    // Installed before any install root: under the settings root.
    ok(&fixture.install(&user, None, "org.example/old-app"));
    assert!(run(&legacy.join("oldtool")).contains("oldtool RAN"));

    // The operator's own file in the permitted directory is never touched.
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("operator-notes"), b"mine").unwrap();

    let listed = ok(&fixture.vibe(&user, Some(&base), &["app", "list"]));
    assert!(
        listed.contains(&format!("commands: {}", bin.display())),
        "{listed}"
    );
    assert!(
        listed.contains(&format!(
            "new installations: {} (library `install-root`)",
            store.join("apps").display()
        )),
        "{listed}"
    );
    assert!(
        listed.contains("1 installation(s) stay in library `default`"),
        "{listed}"
    );

    let shown = ok(&fixture.vibe(&user, Some(&base), &["self", "apply", "--show"]));
    assert!(
        shown.contains(&format!(
            "applications: commands in {}, payloads in {}",
            bin.display(),
            store.join("apps").display()
        )),
        "{shown}"
    );

    ok(&fixture.install(&user, Some(&base), "org.example/new-app"));
    let payload = store.join("apps/newtool/org.example.new-app@1.0.0");
    assert!(payload.join("bin/newtool").is_file());
    let record = &index(&user)["installations"]["org.example/new-app@1.0.0"];
    assert_eq!(record["library"], "install-root");
    let storage = fs::read_to_string(user.settings.join("storage.toml")).unwrap();
    assert!(storage.contains("install-root"), "{storage}");
    assert!(store.join("vibe-library.toml").is_file());
    let pointer = fs::read_to_string(store.join("vibe-app-home.toml")).unwrap();
    assert!(
        pointer.contains(
            &fs::canonicalize(&user.settings)
                .unwrap()
                .display()
                .to_string()
        ),
        "{pointer}"
    );
    assert!(
        !user.settings.join("opt/bin/newtool").exists(),
        "no new proxy goes to the settings root"
    );
    assert!(run(&bin.join("newtool")).contains("newtool RAN"));
    // Every reserved command is in the permitted directory now; the earlier
    // installation did not move, and its earlier proxy still runs too.
    assert!(run(&bin.join("oldtool")).contains("oldtool RAN"));
    assert!(run(&legacy.join("oldtool")).contains("oldtool RAN"));
    assert_eq!(
        index(&user)["installations"]["org.example/old-app@1.0.0"]["library"],
        "default"
    );
    assert_eq!(fs::read(bin.join("operator-notes")).unwrap(), b"mine");

    // An explicit move brings the earlier one along.
    ok(&fixture.vibe(
        &user,
        Some(&base),
        &[
            "storage",
            "move",
            "org.example/old-app@1.0.0",
            "--to",
            "install-root",
        ],
    ));
    assert!(
        store
            .join("apps/oldtool/org.example.old-app@1.0.0")
            .is_dir()
    );
    assert!(run(&bin.join("oldtool")).contains("oldtool RAN"));

    // Uninstalling releases the name in both places it was written.
    ok(&fixture.vibe(
        &user,
        Some(&base),
        &["uninstall", "-g", "org.example/old-app"],
    ));
    assert!(!bin.join("oldtool").exists());
    assert!(!legacy.join("oldtool").exists());
    assert!(run(&bin.join("newtool")).contains("newtool RAN"));
    assert_eq!(fs::read(bin.join("operator-notes")).unwrap(), b"mine");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#install-root")]
fn an_install_root_that_moved_refuses_new_installations_instead_of_mixing() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    fixture.package("first-app", "first");
    fixture.package("second-app", "second");
    let first = fixture.root.path().join("First");
    let second = fixture.root.path().join("Second");
    ok(&fixture.install(&user, Some(&first), "org.example/first-app"));
    let output = fixture.install(&user, Some(&second), "org.example/second-app");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("PROP-019#install-root"), "{stderr}");
    assert!(stderr.contains("vibe storage move"), "{stderr}");
    assert!(!second.join("opt/apps").exists());
    // The first root still serves what it holds.
    assert!(run(&first.join("opt/bin/first")).contains("first RAN"));
}
