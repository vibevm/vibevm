//! A shim directory vibe shares with somebody else's tools (PROP-019
//! `#layout`, `#install-root`): on a network that allows executables only
//! under one directory, `<base>/opt/bin` is the directory the operator
//! already keeps their tools in. Vibe must publish its shims there without
//! ever deleting, overwriting or claiming a file it did not write.
//!
//! Driven through the real CLI in an isolated settings home, install root
//! and HOME, with no network.

#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#layout");

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::UserScratch;
use sha2::{Digest, Sha256};

/// A foreign `vibe` the operator put on PATH long before vibe arrived.
const FOREIGN_VIBE: &str = "#!/bin/sh\n# the operator's own launcher\nexec /opt/corp/vibe \"$@\"\n";

/// An unrelated tenant of the shared directory.
const FOREIGN_TOOL: &str = "#!/bin/sh\necho corp-tool\n";

/// One isolated machine whose install base holds a shared shim directory.
struct Machine {
    user: UserScratch,
    base: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Machine {
    fn new() -> Self {
        let machine = Machine {
            user: UserScratch::new(),
            base: tempfile::tempdir().expect("install base"),
            home: tempfile::tempdir().expect("home"),
        };
        fs::create_dir_all(machine.shim_dir()).expect("the shared shim directory");
        machine
    }

    fn shim_dir(&self) -> PathBuf {
        self.base.path().join("opt").join("bin")
    }

    fn inventory(&self) -> PathBuf {
        self.base
            .path()
            .join("opt")
            .join("vibevm")
            .join("shims.toml")
    }

    fn cmd(&self) -> assert_cmd::Command {
        let mut command = self.user.vibe();
        command
            .env("VIBEVM_INSTALL_ROOT", self.base.path())
            .env("HOME", self.home.path())
            .env("SHELL", "/bin/bash")
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.shim_dir().display()),
            )
            .env_remove("VIBEVM_HOME")
            .env_remove("VIBEVM_SHELL_HOME")
            .env_remove("VIBE_UNATTENDED")
            .env_remove("VIBEVM_RELEASE_ORIGIN")
            .env_remove("VIBEVM_USER_CONFIG")
            .env_remove("VIBE_OFFLINE");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.cmd().args(args).output().expect("running vibe")
    }

    /// `vibe self import <tiny payload> --use` — the shim-writing operation.
    fn import(&self, tag: &str) -> Output {
        let payload = self.base.path().join(format!("ready-vibe-{tag}"));
        fs::write(&payload, "#!/bin/sh\nexit 0\n").expect("the payload");
        fs::set_permissions(&payload, fs::Permissions::from_mode(0o755)).expect("payload mode");
        let mut command = self.cmd();
        command
            .args(["--unattended", "self", "import"])
            .arg(&payload)
            .args(["--tag", tag, "--use"]);
        command.output().expect("running vibe self import")
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn ok(output: &Output) -> String {
    let joined = text(output);
    assert!(
        output.status.success(),
        "status={}: {joined}",
        output.status
    );
    joined
}

fn failed(output: &Output) -> String {
    let joined = text(output);
    assert!(
        !output.status.success(),
        "expected a refusal: status={}: {joined}",
        output.status
    );
    joined
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("reading `{}`: {error}", path.display()))
}

/// Every assertion a refusal owes the operator: the path, whose file it is
/// not, the requirement, and what to do.
fn refusal_names_path_spec_and_fix(message: &str, path: &Path) {
    assert!(
        message.contains(&path.display().to_string()),
        "names the file: {message}"
    );
    assert!(
        message.contains("is not a file vibe wrote"),
        "says whose file it is not: {message}"
    );
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-019#layout"),
        "cites the requirement: {message}"
    );
    assert!(message.contains("fix: "), "offers a fix: {message}");
}

#[test]
fn a_foreign_file_at_a_shim_name_refuses_the_write_and_is_left_byte_identical() {
    let machine = Machine::new();
    let foreign_vibe = machine.shim_dir().join("vibe");
    let foreign_tool = machine.shim_dir().join("corp-tool");
    fs::write(&foreign_vibe, FOREIGN_VIBE).expect("the operator's own vibe");
    fs::write(&foreign_tool, FOREIGN_TOOL).expect("the operator's own tool");

    let message = failed(&machine.import("1.2.3"));
    refusal_names_path_spec_and_fix(&message, &foreign_vibe);

    // Nothing in the directory moved, and no other shim was written first:
    // the whole write is refused before its first byte.
    assert_eq!(read(&foreign_vibe), FOREIGN_VIBE);
    assert_eq!(read(&foreign_tool), FOREIGN_TOOL);
    assert!(
        !machine.shim_dir().join("vibe-index").exists(),
        "a refused write publishes no shim at all"
    );
    assert!(
        !machine.inventory().exists(),
        "a refused write claims nothing"
    );

    // Once the operator moves their own file aside, the same command works.
    fs::rename(&foreign_vibe, machine.shim_dir().join("corp-vibe")).expect("moving it aside");
    ok(&machine.import("1.2.4"));
    assert!(read(&machine.shim_dir().join("vibe")).contains("vibevm (VVM) shim"));
    assert_eq!(read(&foreign_tool), FOREIGN_TOOL);
}

#[test]
fn the_shim_write_keeps_the_other_tenants_and_records_exactly_what_it_wrote() {
    let machine = Machine::new();
    let foreign_tool = machine.shim_dir().join("corp-tool");
    let foreign_named_like_a_tool = machine.shim_dir().join("vibe-tree");
    fs::write(&foreign_tool, FOREIGN_TOOL).expect("the operator's own tool");
    fs::write(&foreign_named_like_a_tool, FOREIGN_TOOL).expect("a vibe-ish name vibe never writes");

    ok(&machine.import("1.2.3"));

    // The shims vibe publishes on this platform, recorded by their digests.
    let inventory = read(&machine.inventory());
    assert!(inventory.contains("schema = 1"), "{inventory}");
    for name in ["vibe", "vibe-index"] {
        let shim = machine.shim_dir().join(name);
        let bytes = fs::read(&shim).expect("the published shim");
        assert!(
            shim.metadata().expect("shim metadata").permissions().mode() & 0o111 != 0,
            "`{name}` is executable"
        );
        assert!(
            inventory.contains(&format!("name = \"{name}\"")),
            "{inventory}"
        );
        assert!(inventory.contains(&digest(&bytes)), "{inventory}");
    }

    // Names vibe does not publish are not claimed, and no tenant changed.
    assert!(!inventory.contains("vibe-tree"), "{inventory}");
    assert_eq!(read(&foreign_tool), FOREIGN_TOOL);
    assert_eq!(read(&foreign_named_like_a_tool), FOREIGN_TOOL);

    // Re-publishing is idempotent and still leaves the tenants alone.
    ok(&machine.import("1.2.4"));
    assert_eq!(read(&machine.inventory()), inventory);
    assert_eq!(read(&foreign_tool), FOREIGN_TOOL);
}

#[test]
fn doctor_fix_repairs_vibes_own_shim_but_refuses_a_foreign_one() {
    let machine = Machine::new();
    let foreign_tool = machine.shim_dir().join("corp-tool");
    fs::write(&foreign_tool, FOREIGN_TOOL).expect("the operator's own tool");
    ok(&machine.import("1.2.3"));
    let published = read(&machine.shim_dir().join("vibe"));

    // A shim of vibe's own that lost its content is repaired: it is still
    // recognisable as vibe's, so `--fix` may rewrite it.
    let damaged = format!("{published}# edited by hand\n");
    fs::write(machine.shim_dir().join("vibe"), &damaged).expect("damaging vibe's own shim");
    let repaired =
        text(&machine.run(["--unattended", "self", "doctor", "--fix", "--yes"].as_ref()));
    assert!(
        repaired.contains("repair completed"),
        "the fix ran: {repaired}"
    );
    assert_eq!(read(&machine.shim_dir().join("vibe")), published);
    assert_eq!(read(&foreign_tool), FOREIGN_TOOL);

    // A foreign file put at a shim's name is refused instead, and `--fix`
    // changes nothing at all.
    let hijacked = machine.shim_dir().join("vibe-index");
    fs::write(&hijacked, FOREIGN_VIBE).expect("somebody else's vibe-index");
    let refused =
        failed(&machine.run(["--unattended", "self", "doctor", "--fix", "--yes"].as_ref()));
    refusal_names_path_spec_and_fix(&refused, &hijacked);
    assert_eq!(read(&hijacked), FOREIGN_VIBE);
    assert_eq!(read(&machine.shim_dir().join("vibe")), published);
    assert_eq!(read(&foreign_tool), FOREIGN_TOOL);
}

#[test]
fn removal_and_apply_remove_leave_every_file_in_the_shim_directory_alone() {
    let machine = Machine::new();
    let foreign_tool = machine.shim_dir().join("corp-tool");
    fs::write(&foreign_tool, FOREIGN_TOOL).expect("the operator's own tool");
    ok(&machine.import("1.2.3"));
    ok(&machine.import("1.2.4"));
    let published = read(&machine.shim_dir().join("vibe"));
    let inventory = read(&machine.inventory());

    // A site profile, so `self apply --remove` has a receipt to undo.
    let profile = machine.base.path().join("corp.toml");
    fs::write(
        &profile,
        "schema = 1\nname = \"corp\"\n\n[path]\nfix = \"never\"\n",
    )
    .expect("the site profile");
    let mut apply = machine.cmd();
    apply
        .args(["--unattended", "self", "apply"])
        .arg(&profile)
        .arg("--no-fix-path");
    ok(&apply.output().expect("running vibe self apply"));
    let stored = machine.user.settings.join("site-profile.toml");
    assert!(stored.is_file(), "the profile was stored");

    // Uninstalling every version, collecting garbage and undoing the
    // profile all leave the shared directory exactly as it is.
    for args in [
        ["--unattended", "self", "gc", "--prune-others", "--yes"].as_slice(),
        [
            "--unattended",
            "self",
            "uninstall",
            "--all",
            "--yes",
            "--force",
        ]
        .as_slice(),
        ["--unattended", "self", "gc", "--build"].as_slice(),
        ["--unattended", "self", "apply", "--remove"].as_slice(),
    ] {
        machine.run(args);
        assert_eq!(read(&foreign_tool), FOREIGN_TOOL, "after {args:?}");
        assert_eq!(
            read(&machine.shim_dir().join("vibe")),
            published,
            "after {args:?}"
        );
        assert_eq!(read(&machine.inventory()), inventory, "after {args:?}");
    }
    assert!(!stored.exists(), "`--remove` undid its own files");
    assert!(
        machine.shim_dir().is_dir(),
        "the shared directory itself survives"
    );
}
