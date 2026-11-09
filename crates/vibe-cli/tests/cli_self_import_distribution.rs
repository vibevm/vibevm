//! `vibe self import` takes the whole distribution beside the payload
//! (PROP-019 `#provenance`): a release unpacked by hand is imported member by
//! member, and a bare payload brings the companions beside it. Either way
//! `vibe install -g` through the imported shim then installs an application —
//! which needs `vibe-app-proxy` in the instance — and the foreign tools that
//! share the unpacked directory are left alone.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::UserScratch;
use sha2::{Digest, Sha256};
use vibe_publish::release_manifest::{
    BundleDistributionManifest, DISTRIBUTION_MANIFEST_FILENAME,
    DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, DistributionComponent, DistributionComponentName,
    DistributionSourceArchive,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
/// A tenant of the directory the release is unpacked into.
const FOREIGN: (&str, &[u8]) = ("corp-tool", b"#!/bin/sh\necho corp\n");

/// One isolated machine: its install root, a home, and a registry holding
/// one shell-method application.
struct Machine {
    user: UserScratch,
    temp: tempfile::TempDir,
}

impl Machine {
    fn new() -> Self {
        let machine = Self {
            user: UserScratch::new(),
            temp: tempfile::tempdir().unwrap(),
        };
        fs::create_dir_all(machine.home()).unwrap();
        machine.write_app();
        machine
    }

    fn base(&self) -> PathBuf {
        self.temp.path().join("base")
    }

    fn home(&self) -> PathBuf {
        self.temp.path().join("home")
    }

    fn registry(&self) -> PathBuf {
        self.temp.path().join("registry")
    }

    fn shim(&self) -> PathBuf {
        self.base().join("opt").join("bin").join("vibe")
    }

    fn write_app(&self) {
        let package = self.registry().join("org.example/shell-app/v1.0.0");
        fs::create_dir_all(&package).unwrap();
        let app_root = self.temp.path().join("app-root");
        let install = package.join("install.sh");
        fs::write(
            &install,
            "#!/bin/sh\nset -e\nmkdir -p \"$VIBE_APP_ROOT/bin\"\nprintf '#!/bin/sh\\necho TOOL_RAN\\n' > \"$VIBE_APP_ROOT/bin/tool\"\nchmod 755 \"$VIBE_APP_ROOT/bin/tool\"\n",
        )
        .unwrap();
        fs::set_permissions(&install, fs::Permissions::from_mode(0o755)).unwrap();
        let script = install.display();
        fs::write(
            package.join("vibe.toml"),
            format!(
                "[package]\nname = \"shell-app\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
                 [application]\nid = \"tool\"\n\n\
                 [[installer_options]]\nid = \"shell\"\nname = \"Option shell\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\
                 [installer_options.installation]\ntype = \"shell\"\napp_root = \"{root}\"\ninstall = \"sh '{script}'\"\nforce_install = \"sh '{script}'\"\nuninstall = \"true\"\nforce_uninstall = \"true\"\ninterpreter = \"posix_sh\"\n\n\
                 [[installer_options.launchers]]\nid = \"main\"\nname = \"Tool\"\nexecutable = \"$app_bin/tool\"\nkind = \"command\"\ndefault = true\n\n\
                 [[installer_options.public_commands]]\nname = \"tool\"\nlauncher = \"main\"\n",
                arch = std::env::consts::ARCH,
                root = app_root.display(),
            ),
        )
        .unwrap();
    }

    fn command(&self, program: &Path) -> Command {
        let mut command = Command::new(program);
        command
            .env(vibe_core::settings::SETTINGS_DIR_ENV, &self.user.settings)
            .env(vibe_test_support::REGISTRY_CACHE_ENV, &self.user.cache)
            .env(vibe_test_support::SEARCH_CACHE_ENV, &self.user.search_cache)
            .env("VIBEVM_INSTALL_ROOT", self.base())
            .env("HOME", self.home())
            .env("SHELL", "/bin/bash")
            .env("PATH", "/usr/bin:/bin")
            .env_remove("VIBEVM_HOME")
            .env_remove("VIBEVM_SHELL_HOME")
            .env_remove("VIBE_UNATTENDED")
            .env_remove("VIBEVM_RELEASE_ORIGIN")
            .env_remove("VIBEVM_USER_CONFIG")
            .env_remove("VIBE_OFFLINE")
            .current_dir(self.temp.path());
        command
    }

    /// `<dir>/vibe self import <dir>/vibe --tag <VERSION> --use`, run by the
    /// unpacked binary itself, as the owner does it.
    fn import_from(&self, payload: &Path) -> Output {
        self.command(payload)
            .args(["--unattended", "self", "import"])
            .arg(payload)
            .args(["--tag", VERSION, "--use"])
            .output()
            .unwrap()
    }

    /// `vibe install -g org.example/shell-app` through the imported shim.
    fn install_app(&self) -> Output {
        self.command(&self.shim())
            .args(["--unattended", "install", "-g", "org.example/shell-app"])
            .arg("--registry")
            .arg(self.registry())
            .output()
            .unwrap()
    }
}

fn text(output: &Output) -> String {
    format!(
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn ok(output: &Output) {
    assert!(output.status.success(), "{}", text(output));
}

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn copy_executable(from: &str, to: &Path) {
    fs::copy(from, to).unwrap();
    fs::set_permissions(to, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The executables a bare unpacked directory holds, beside a foreign tool.
fn bare_dir(dir: &Path, with_proxy: bool) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let vibe = dir.join("vibe");
    copy_executable(env!("CARGO_BIN_EXE_vibe"), &vibe);
    if with_proxy {
        copy_executable(
            env!("CARGO_BIN_EXE_vibe-app-proxy"),
            &dir.join("vibe-app-proxy"),
        );
    }
    fs::write(dir.join(FOREIGN.0), FOREIGN.1).unwrap();
    vibe
}

fn source_zip() -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        ("Cargo.toml", b"[workspace]\n".as_slice()),
        (
            "crates/vibe-cli/Cargo.toml",
            b"[package]\nname='vibe-cli'\n",
        ),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// A release ZIP unpacked by hand: the real `vibe` and `vibe-app-proxy`,
/// a stand-in `vibe-index`, the packaged source and its `DISTRIBUTION.json`.
fn unpacked_release(dir: &Path) -> PathBuf {
    let vibe = bare_dir(dir, true);
    fs::write(dir.join("vibe-index"), b"index").unwrap();
    fs::set_permissions(dir.join("vibe-index"), fs::Permissions::from_mode(0o755)).unwrap();
    let source = source_zip();
    fs::write(dir.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME), &source).unwrap();
    let component = |name, path: &str| {
        let bytes = fs::read(dir.join(path)).unwrap();
        DistributionComponent {
            name,
            path: path.to_string(),
            size: bytes.len() as u64,
            digest: sha(&bytes),
        }
    };
    let manifest = BundleDistributionManifest::new(
        semver::Version::parse(VERSION).unwrap(),
        COMMIT,
        "x86_64-unknown-linux-gnu",
        vec![
            component(DistributionComponentName::Vibe, "vibe"),
            component(DistributionComponentName::VibeAppProxy, "vibe-app-proxy"),
            component(DistributionComponentName::VibeIndex, "vibe-index"),
        ],
        DistributionSourceArchive {
            path: DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.to_string(),
            size: source.len() as u64,
            digest: sha(&source),
            tree_oid: "a".repeat(40),
        },
    )
    .unwrap();
    fs::write(
        dir.join(DISTRIBUTION_MANIFEST_FILENAME),
        manifest.to_json_bytes().unwrap(),
    )
    .unwrap();
    vibe
}

fn listing(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().into_owned(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect();
    entries.sort();
    entries
}

#[test]
fn an_unpacked_release_imports_whole_and_installs_an_application_through_the_shim() {
    if !cfg!(all(target_arch = "x86_64", target_env = "gnu")) {
        return;
    }
    let machine = Machine::new();
    let dir = machine.temp.path().join("unpacked");
    let vibe = unpacked_release(&dir);
    let before = listing(&dir);

    ok(&machine.import_from(&vibe));
    assert_eq!(listing(&dir), before, "the unpacked directory is untouched");
    let instance = fs::read_to_string(machine.base().join("opt/vibevm/current")).unwrap();
    let instance = PathBuf::from(instance.trim());
    assert_eq!(
        fs::read(instance.join("bin/vibe-app-proxy")).unwrap(),
        fs::read(env!("CARGO_BIN_EXE_vibe-app-proxy")).unwrap()
    );
    assert!(instance.join(DISTRIBUTION_MANIFEST_FILENAME).is_file());
    assert!(!instance.join(FOREIGN.0).exists());
    assert!(!instance.join("bin").join(FOREIGN.0).exists());

    ok(&machine.install_app());
}

#[test]
fn a_tampered_member_refuses_the_import_by_name() {
    if !cfg!(all(target_arch = "x86_64", target_env = "gnu")) {
        return;
    }
    let machine = Machine::new();
    let dir = machine.temp.path().join("unpacked");
    let vibe = unpacked_release(&dir);
    fs::write(dir.join("vibe-index"), b"tampered").unwrap();

    let refused = machine.import_from(&vibe);
    assert!(!refused.status.success(), "{}", text(&refused));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains(&dir.join("vibe-index").display().to_string()),
        "{stderr}"
    );
    assert!(!machine.base().join("opt/vibevm/current").exists());
}

#[test]
fn a_bare_payload_brings_the_proxy_beside_it_and_reports_it_when_absent() {
    let machine = Machine::new();
    let alone = bare_dir(&machine.temp.path().join("alone"), false);
    let imported = machine.import_from(&alone);
    ok(&imported);
    let stderr = String::from_utf8_lossy(&imported.stderr);
    assert!(
        stderr.contains("vibe-app-proxy") && stderr.contains("proxy_image_missing"),
        "{stderr}"
    );
    let refused = machine.install_app();
    assert!(!refused.status.success(), "{}", text(&refused));
    assert!(
        text(&refused).contains("proxy_image_missing"),
        "{}",
        text(&refused)
    );

    let dir = machine.temp.path().join("with-proxy");
    let vibe = bare_dir(&dir, true);
    let before = listing(&dir);
    let imported = machine.import_from(&vibe);
    ok(&imported);
    assert!(
        !String::from_utf8_lossy(&imported.stderr).contains("`vibe-app-proxy` is not beside"),
        "{}",
        text(&imported)
    );
    assert_eq!(listing(&dir), before);
    ok(&machine.install_app());
}
