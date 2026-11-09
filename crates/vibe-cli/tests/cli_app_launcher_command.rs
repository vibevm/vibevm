//! Public Linux fixture for a launcher that names a command instead of a
//! payload file (PROP-065 `#launcher-command`): the plan warns and installs
//! anyway when the command is absent, the proxy resolves it on PATH when it
//! is there, the proxy never resolves to itself, and `app launchers` reports
//! both states.

#![cfg(target_os = "linux")]

mod common;

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::UserScratch;
use sha2::{Digest, Sha256};

/// One local registry holding the fixture packages.
struct Registry {
    root: tempfile::TempDir,
}

impl Registry {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }

    fn path(&self) -> PathBuf {
        self.root.path().join("registry")
    }

    /// `org.example/<name>@1.0.0` in slot `<name>`, whose payload holds
    /// `share/program` and whose one launcher runs the command
    /// `<interpreter>` on it, claiming the public command `<command>`.
    fn package(&self, name: &str, interpreter: &str, command: &str) {
        let dir = self.path().join(format!("org.example/{name}/v1.0.0"));
        fs::create_dir_all(&dir).unwrap();
        let mut zip = zip::ZipWriter::new(fs::File::create(dir.join("app.zip")).unwrap());
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o644);
        zip.start_file("share/program", options).unwrap();
        zip.write_all(b"PROGRAM\n").unwrap();
        zip.finish().unwrap();
        let sha = format!(
            "{:x}",
            Sha256::digest(fs::read(dir.join("app.zip")).unwrap())
        );
        fs::write(
            dir.join("vibe.toml"),
            format!(
                "[package]\nname = \"{name}\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
                 [compatibility]\nmin_vibe_version = \"1.0.12\"\n\n\
                 [application]\nid = \"{name}\"\n\n\
                 [[installer_options]]\nid = \"zip\"\nname = \"Zip\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\n\
                 [installer_options.installation]\ntype = \"zip\"\nsource = \"app.zip\"\nsha256 = \"{sha}\"\n\n\
                 [[installer_options.launchers]]\nid = \"main\"\nname = \"App\"\n\
                 executable = \"command:{interpreter}\"\nargs = [\"$app_root/share/program\"]\nkind = \"command\"\ndefault = true\n\n\
                 [[installer_options.public_commands]]\nname = \"{command}\"\nlauncher = \"main\"\n",
                arch = std::env::consts::ARCH,
            ),
        )
        .unwrap();
    }

    /// `org.example/<name>@1.0.0` in slot `<name>` whose launcher is an
    /// ordinary payload file printing `<marker> <args>`, claiming `<command>`.
    fn plain_package(&self, name: &str, command: &str, marker: &str) {
        let dir = self.path().join(format!("org.example/{name}/v1.0.0"));
        fs::create_dir_all(&dir).unwrap();
        let mut zip = zip::ZipWriter::new(fs::File::create(dir.join("app.zip")).unwrap());
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        zip.start_file("bin/tool", options).unwrap();
        zip.write_all(format!("#!/bin/sh\necho \"{marker} $*\"\n").as_bytes())
            .unwrap();
        zip.finish().unwrap();
        let sha = format!(
            "{:x}",
            Sha256::digest(fs::read(dir.join("app.zip")).unwrap())
        );
        fs::write(
            dir.join("vibe.toml"),
            format!(
                "[package]\nname = \"{name}\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
                 [application]\nid = \"{name}\"\n\n\
                 [[installer_options]]\nid = \"zip\"\nname = \"Zip\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\n\
                 [installer_options.installation]\ntype = \"zip\"\nsource = \"app.zip\"\nsha256 = \"{sha}\"\n\n\
                 [[installer_options.launchers]]\nid = \"main\"\nname = \"App\"\n\
                 executable = \"$app_bin/tool\"\nkind = \"command\"\ndefault = true\n\n\
                 [[installer_options.public_commands]]\nname = \"{command}\"\nlauncher = \"main\"\n",
                arch = std::env::consts::ARCH,
            ),
        )
        .unwrap();
    }

    fn install(&self, user: &UserScratch, package: &str, path: &OsString) -> Output {
        let registry = self.path();
        user.vibe()
            .current_dir(self.root.path())
            .args(["install", "-g", package, "--registry"])
            .arg(&registry)
            .arg("--yes")
            .env("PATH", path)
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

/// A directory holding a shell script named `name` that prints a marker and
/// every argument it was given.
fn stub(dir: &Path, name: &str, marker: &str) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\necho \"{marker} $*\"\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn path_of(dirs: &[&Path]) -> OsString {
    let mut value = OsString::new();
    for dir in dirs {
        if !value.is_empty() {
            value.push(":");
        }
        value.push(dir.as_os_str());
    }
    // `/usr/bin` and `/bin` are the host's own; the stubs decide the test.
    value.push(":/usr/bin:/bin");
    value
}

#[test]
fn a_command_launcher_installs_without_the_command_and_runs_once_it_is_on_path() {
    let registry = Registry::new();
    let user = UserScratch::new();
    let host = tempfile::tempdir().unwrap();
    let hostbin = host.path().join("hostbin");
    registry.package("alpha", "fixture-interp", "alpha");

    let proxies = user.settings.join("opt/bin");
    let bare = path_of(&[]);

    // The plan says the command is missing and installs anyway. The plan
    // and its warning are progress output on stderr; the outcome line is
    // the command's own stdout.
    let installed = registry.install(&user, "org.example/alpha", &bare);
    let report = ok(&installed);
    let progress = String::from_utf8_lossy(&installed.stderr);
    assert!(
        progress.contains("runs the command fixture-interp, which is not on PATH"),
        "{progress}"
    );
    assert!(
        report.contains("installed org.example/alpha@1.0.0"),
        "{report}"
    );

    // Running it without the command is one clear line and exit 127.
    let output = Command::new("alpha")
        .env("PATH", path_of(&[&proxies]))
        .env_remove("VIBE_APP_USE")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(127), "{stderr}");
    assert!(
        stderr.contains("command `fixture-interp` is not on PATH"),
        "{stderr}"
    );
    assert!(stderr.contains("launcher `main`"), "{stderr}");
    assert!(stderr.contains("vibe install -g"), "{stderr}");

    // `app launchers` says the same, as a row state.
    let listing = ok(&user
        .vibe()
        .args(["app", "launchers", "org.example/alpha"])
        .env("PATH", bare.clone())
        .output()
        .unwrap());
    assert!(
        listing.contains("unavailable: command `fixture-interp` is not on PATH"),
        "{listing}"
    );
    assert!(listing.contains("command:fixture-interp"), "{listing}");

    // With the command on PATH the launcher runs it on the payload file.
    stub(&hostbin, "fixture-interp", "HOST");
    let with = path_of(&[&proxies, &hostbin]);
    let output = Command::new("alpha")
        .arg("extra")
        .env("PATH", &with)
        .env_remove("VIBE_APP_USE")
        .output()
        .unwrap();
    let stdout = ok(&output);
    let payload = user
        .settings
        .join("opt/apps/alpha/org.example.alpha@1.0.0/share/program");
    assert_eq!(
        stdout,
        format!("HOST {} extra\n", payload.display()),
        "the command runs the payload file with the caller's arguments"
    );

    // And the listing resolves the located file, not a payload path.
    let listing = ok(&user
        .vibe()
        .args(["app", "launchers", "org.example/alpha"])
        .env("PATH", &with)
        .output()
        .unwrap());
    assert!(listing.contains("App · available"), "{listing}");
    assert!(
        listing.contains(hostbin.join("fixture-interp").to_str().unwrap()),
        "{listing}"
    );
    assert!(listing.contains("\"base\":\"path\""), "{listing}");
}

/// A proxy whose own command name is the command its launcher runs falls
/// through to the host's program instead of re-entering itself, and the
/// proxies of the other installed applications in that same directory are
/// found like any other PATH entry.
#[test]
fn a_command_launcher_skips_only_its_own_proxy() {
    let registry = Registry::new();
    let user = UserScratch::new();
    let host = tempfile::tempdir().unwrap();
    let hostbin = host.path().join("hostbin");
    // `beta` claims the command `fixture-interp` and its launcher runs
    // `fixture-interp`: the proxy must not find itself.
    registry.package("beta", "fixture-interp", "fixture-interp");
    // `gamma`'s launcher runs `beta-run`, which `delta` provides as an
    // ordinary payload launcher — its proxy sits beside gamma's own.
    registry.package("gamma", "beta-run", "gamma");
    registry.plain_package("delta", "beta-run", "DELTA");

    let proxies = user.settings.join("opt/bin");
    stub(&hostbin, "fixture-interp", "HOST");
    let with = path_of(&[&proxies, &hostbin]);

    ok(&registry.install(&user, "org.example/beta", &with));
    let output = Command::new("fixture-interp")
        .env("PATH", &with)
        .env_remove("VIBE_APP_USE")
        .output()
        .unwrap();
    let payload = user
        .settings
        .join("opt/apps/beta/org.example.beta@1.0.0/share/program");
    assert_eq!(
        ok(&output),
        format!("HOST {}\n", payload.display()),
        "the proxy named `fixture-interp` runs the host's `fixture-interp`, not itself"
    );

    // A sibling proxy in the same directory is a perfectly good provider:
    // `gamma` finds `beta-run` through delta's proxy, which the exclusion
    // of gamma's own image does not touch.
    ok(&registry.install(&user, "org.example/delta", &with));
    ok(&registry.install(&user, "org.example/gamma", &with));
    let output = Command::new("gamma")
        .env("PATH", &with)
        .env_remove("VIBE_APP_USE")
        .output()
        .unwrap();
    let gamma = user
        .settings
        .join("opt/apps/gamma/org.example.gamma@1.0.0/share/program");
    assert_eq!(
        ok(&output),
        format!("DELTA {}\n", gamma.display()),
        "gamma's launcher reached `beta-run` through delta's proxy beside its own"
    );
}
