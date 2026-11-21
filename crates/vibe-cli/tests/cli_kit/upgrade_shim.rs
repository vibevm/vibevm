//! The installed shim follows an explicit kit upgrade to the new payload.
//!
//! The older installation is a stand-in executable that only reports its
//! version, imported through the ordinary import owner. The newer release is
//! the CLI under test, packed into a kit by `vibe kit build` and consumed by
//! the native installer library with `--upgrade`. The installer entry point is
//! reached by re-running this test binary with a cleared environment, so no
//! prebuilt installer artifact and no real user state take part.
//! The synthetic bundle targets x86_64 Linux, so the module is built only there.
use super::*;
use clap::Parser;

const INSTALLER_ARGUMENTS: &str = "VIBE_TEST_KIT_INSTALLER_ARGUMENTS";
const OLDER: &str = "1.0.11";

/// Acts as `vibe-installer` when the upgrade test re-runs this binary; an
/// ordinary test run has no arguments for it and does nothing.
#[test]
fn native_installer_entry() {
    let Ok(arguments) = std::env::var(INSTALLER_ARGUMENTS) else {
        return;
    };
    let arguments = std::iter::once("vibe-installer").chain(arguments.split('\n'));
    let code = match vibe_bootstrap::kit::run(vibe_bootstrap::kit::Args::parse_from(arguments)) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("vibe-installer: {error:#}");
            1
        }
    };
    std::process::exit(code);
}

#[test]
fn installed_shim_reports_new_version_after_explicit_kit_upgrade() {
    use std::os::unix::fs::PermissionsExt;
    let newer = env!("CARGO_PKG_VERSION");
    let temp = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let root = temp.path();
    let home = root.join("home");
    let settings = root.join("settings");
    let install = root.join("install");
    fs::create_dir(&home).unwrap();
    let isolated = |exe: &Path| {
        let mut command = std::process::Command::new(exe);
        command
            .env_clear()
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("VIBE_SETTINGS", &settings)
            .env("VIBEVM_INSTALL_ROOT", &install)
            .env("PATH", "/usr/bin:/bin")
            .current_dir(root);
        command
    };

    // Older installation A: a payload that can only say which version it is.
    let old_dir = root.join("older");
    fs::create_dir(&old_dir).unwrap();
    let old_exe = old_dir.join("vibe");
    let old_bytes = format!("#!/bin/sh\necho 'vibe {OLDER}'\n");
    fs::write(&old_exe, &old_bytes).unwrap();
    fs::set_permissions(&old_exe, fs::Permissions::from_mode(0o755)).unwrap();
    let cli_path = Path::new(env!("CARGO_BIN_EXE_vibe"));
    good(
        isolated(cli_path)
            .args(["--unattended", "self", "import"])
            .arg(&old_exe)
            .args(["--tag", OLDER, "--use", "--no-profile"])
            .output()
            .unwrap(),
    );
    let shim = install.join("opt/bin/vibe");
    let shim_version = || {
        let out = isolated(&shim).arg("--version").output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    // The payload the shim resolves, read the way the shim itself reads it.
    let active_payload = || {
        let pointer = fs::read_to_string(install.join("opt/vibevm/current")).unwrap();
        Path::new(pointer.trim()).join("bin/vibe")
    };
    assert_eq!(shim_version(), format!("vibe {OLDER}"));
    let old_payload = active_payload();
    assert_eq!(fs::read(&old_payload).unwrap(), old_bytes.as_bytes());
    let shim_bytes = fs::read(&shim).unwrap();

    // Newer release B: the CLI under test, published as a produced kit.
    let cli = fs::read(cli_path).unwrap();
    let bundle = build::versioned_bundle(&cli, newer, b"newer index");
    let installer = b"not executed: the installer library runs from the test binary";
    let profile = b"schema=1\nname='upgrade-shim'\n[path]\nfix='never'\n";
    let source = root.join("source");
    fs::create_dir(&source).unwrap();
    for (name, bytes) in [
        ("bundle.zip", bundle.as_slice()),
        ("installer", installer.as_slice()),
        ("profile.toml", profile.as_slice()),
    ] {
        fs::write(source.join(name), bytes).unwrap();
    }
    fs::write(
        source.join("recipe.toml"),
        format!(
            "schema_version=1\n[kit]\nname='upgrade-shim'\ntarget='x86_64-unknown-linux-gnu'\nversion='{newer}'\n[bundle]\npath='bundle.zip'\nsha256='{}'\n[installer]\npath='installer'\nsha256='{}'\n[site_profile]\npath='profile.toml'\nsha256='{}'\n",
            digest(&bundle),
            digest(installer),
            digest(profile)
        ),
    )
    .unwrap();
    let kit = root.join("kit");
    good(command(
        &UserScratch::new(),
        root,
        &[
            "kit",
            "build",
            "--file",
            source.join("recipe.toml").to_str().unwrap(),
            "--out",
            kit.to_str().unwrap(),
        ],
    ));

    let arguments = [
        "--kit",
        kit.to_str().unwrap(),
        "--destination",
        install.to_str().unwrap(),
        "--settings",
        settings.to_str().unwrap(),
        "--unattended",
        "--no-path",
        "--upgrade",
    ]
    .join("\n");
    good(
        isolated(&std::env::current_exe().unwrap())
            .env(INSTALLER_ARGUMENTS, arguments)
            .args([
                "--exact",
                "upgrade_shim::native_installer_entry",
                "--nocapture",
            ])
            .output()
            .unwrap(),
    );

    assert_eq!(
        shim_version(),
        format!("vibe {newer}"),
        "the installed shim still resolves the older payload"
    );
    assert_eq!(
        fs::read(&shim).unwrap(),
        shim_bytes,
        "the stable shim itself is not version-specific"
    );
    let new_payload = active_payload();
    assert_ne!(new_payload, old_payload);
    assert_eq!(digest(&fs::read(&new_payload).unwrap()), digest(&cli));
    let current = isolated(&shim)
        .args(["self", "current", "--json"])
        .output()
        .unwrap();
    assert!(current.status.success());
    let current: serde_json::Value = serde_json::from_slice(&current.stdout).unwrap();
    assert_eq!(current["active_selector"], format!("tag:{newer}#2"));
    assert_eq!(
        fs::read(&old_payload).unwrap(),
        old_bytes.as_bytes(),
        "the older immutable instance must remain available"
    );
}
