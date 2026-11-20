//! Producer output is the actual bootstrap consumer subject, never a hand kit.
use super::*;
use std::io::{Cursor, Write};
use vibe_wire::behaviour::release_manifest::{
    BundleDistributionManifest, DistributionComponent, DistributionComponentName,
    DistributionSourceArchive,
};

fn zip(rows: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in rows {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
fn bundle(cli: &[u8]) -> Vec<u8> {
    versioned_bundle(cli, env!("CARGO_PKG_VERSION"), b"synthetic index")
}
fn versioned_bundle(cli: &[u8], version: &str, index: &[u8]) -> Vec<u8> {
    let source = zip(&[
        ("Cargo.toml", b"[workspace]\n"),
        ("crates/vibe-cli/src/main.rs", b"fn main() {}"),
    ]);
    let proxy = b"synthetic proxy";
    let components = [
        ("vibe", DistributionComponentName::Vibe, cli),
        (
            "vibe-app-proxy",
            DistributionComponentName::VibeAppProxy,
            proxy.as_slice(),
        ),
        ("vibe-index", DistributionComponentName::VibeIndex, index),
    ]
    .into_iter()
    .map(|(path, name, bytes)| DistributionComponent {
        name,
        path: path.into(),
        size: bytes.len() as u64,
        digest: format!("sha256:{}", digest(bytes)),
    })
    .collect();
    let manifest = BundleDistributionManifest::new(
        version.parse().unwrap(),
        "1".repeat(40),
        "x86_64-unknown-linux-gnu",
        components,
        DistributionSourceArchive {
            path: "vibevm-source.zip".into(),
            size: source.len() as u64,
            digest: format!("sha256:{}", digest(&source)),
            tree_oid: "2".repeat(40),
        },
    )
    .unwrap()
    .to_json_bytes()
    .unwrap();
    zip(&[
        ("DISTRIBUTION.json", &manifest),
        ("vibe", cli),
        ("vibe-app-proxy", proxy),
        ("vibe-index", index),
        ("vibevm-source.zip", &source),
        ("LICENSE.md", b"synthetic license"),
        ("README.md", b"synthetic distribution"),
    ])
}
#[test]
#[cfg(target_os = "linux")]
fn kit_producer_publishes_and_native_consumer_installs_exact_output() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let root = temp.path();
    let source = root.join("source");
    fs::create_dir(&source).unwrap();
    let output = root.join("produced kit");
    let user = UserScratch::new();
    let cli = fs::read(env!("CARGO_BIN_EXE_vibe")).unwrap();
    let installer = fs::read(
        std::env::var_os("NST_KIT_INSTALLER").expect("exact existing native installer artifact"),
    )
    .unwrap();
    let bytes = bundle(&cli);
    let profile = b"schema=1\nname='synthetic'\n[path]\nfix='never'\n";
    fs::write(source.join("bundle.zip"), &bytes).unwrap();
    fs::write(source.join("installer"), &installer).unwrap();
    fs::write(source.join("profile.toml"), profile).unwrap();
    let recipe = format!(
        "schema_version=1\n[kit]\nname='synthetic'\ntarget='x86_64-unknown-linux-gnu'\nversion='{}'\n[bundle]\npath='bundle.zip'\nsha256='{}'\n[installer]\npath='installer'\nsha256='{}'\n[site_profile]\npath='profile.toml'\nsha256='{}'\n",
        env!("CARGO_PKG_VERSION"),
        digest(&bytes),
        digest(&installer),
        digest(profile)
    );
    let file = source.join("recipe.toml");
    fs::write(&file, &recipe).unwrap();
    let build = |out: &Path| {
        command(
            &user,
            root,
            &[
                "kit",
                "build",
                "--file",
                file.to_str().unwrap(),
                "--out",
                out.to_str().unwrap(),
            ],
        )
    };
    let unsafe_parent = root.join("readable-parent");
    fs::create_dir(&unsafe_parent).unwrap();
    fs::set_permissions(&unsafe_parent, fs::Permissions::from_mode(0o755)).unwrap();
    bad(build(&unsafe_parent.join("output")));
    assert_eq!(
        fs::metadata(&unsafe_parent).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(fs::read_dir(&unsafe_parent).unwrap().count(), 0);
    good(build(&output));
    let sums = fs::read_to_string(output.join("SHA256SUMS.txt")).unwrap();
    for (path, hash) in vibe_bootstrap::kit::parse_checksums(&sums).unwrap() {
        assert_eq!(digest(&fs::read(output.join(path)).unwrap()), hash);
    }
    let before: Vec<_> = fs::read_dir(&output)
        .unwrap()
        .map(|x| {
            let p = x.unwrap().path();
            (p.file_name().unwrap().to_owned(), fs::read(p).unwrap())
        })
        .collect();
    bad(build(&output));
    bad(build(&source.join("overlapping-output")));
    assert!(!source.join("overlapping-output").exists());
    for (name, bytes) in &before {
        assert_eq!(fs::read(output.join(name)).unwrap(), *bytes);
    }
    for world in ["corrupt", "missing", "version"] {
        let refused = root.join(format!("refused-{world}"));
        if world == "corrupt" {
            fs::write(source.join("bundle.zip"), b"corrupt").unwrap();
        }
        if world == "missing" {
            fs::remove_file(source.join("bundle.zip")).unwrap();
        }
        if world == "version" {
            fs::write(source.join("bundle.zip"), &bytes).unwrap();
            fs::write(
                &file,
                recipe.replace(
                    &format!("version='{}'", env!("CARGO_PKG_VERSION")),
                    "version='9.9.9'",
                ),
            )
            .unwrap();
        }
        bad(build(&refused));
        assert!(!refused.exists());
    }
    fs::write(source.join("bundle.zip"), &bytes).unwrap();
    fs::write(&file, &recipe).unwrap();
    let home = root.join("consumer home");
    let settings = root.join("consumer settings");
    let install = root.join("consumer install");
    fs::create_dir(&home).unwrap();
    let native = output.join("vibe-installer-x86_64-unknown-linux-gnu");
    let consume = |dry: bool| {
        let mut c = std::process::Command::new(&native);
        c.env_clear()
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("VIBE_SETTINGS", &settings)
            .env("VIBEVM_INSTALL_ROOT", &install)
            .env("PATH", "/usr/bin:/bin")
            .current_dir(root)
            .args([
                "--kit",
                output.to_str().unwrap(),
                "--destination",
                install.to_str().unwrap(),
                "--settings",
                settings.to_str().unwrap(),
                "--unattended",
                "--no-path",
            ]);
        if dry {
            c.arg("--dry-run");
        }
        c.output().unwrap()
    };
    good(consume(true));
    assert!(!install.exists());
    assert!(!settings.exists());
    good(consume(false));
    let shim = install.join("opt/bin/vibe");
    assert!(shim.exists(), "actual consumer must publish native shim");
    let out = std::process::Command::new(&shim)
        .env_clear()
        .env("HOME", &home)
        .env("VIBE_SETTINGS", &settings)
        .env("VIBEVM_INSTALL_ROOT", &install)
        .env("PATH", "/usr/bin:/bin")
        .args(["self", "current", "--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
#[cfg(target_os = "linux")]
fn produced_kit_native_upgrade_preserves_existing_install_on_corrupt_input() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let root = temp.path();
    let source = root.join("source");
    fs::create_dir(&source).unwrap();
    let user = UserScratch::new();
    let cli = fs::read(env!("CARGO_BIN_EXE_vibe")).unwrap();
    let old = std::env::var_os("NST_KIT_OLD_CLI").expect("exact historical 1.0.11 artifact");
    let old_bytes = fs::read(&old).unwrap();
    let old_dir = root.join("historical A");
    fs::create_dir(&old_dir).unwrap();
    let old_exe = old_dir.join("vibe");
    fs::write(&old_exe, &old_bytes).unwrap();
    fs::set_permissions(&old_exe, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(old_dir.join("vibe-index"), b"owned index A").unwrap();
    let installer = fs::read(std::env::var_os("NST_KIT_INSTALLER").unwrap()).unwrap();
    let profile = b"schema=1\nname='upgrade-synthetic'\n[path]\nfix='never'\n";
    let bundle = versioned_bundle(&cli, env!("CARGO_PKG_VERSION"), b"owned index B");
    for (name, bytes) in [
        ("bundle.zip", bundle.as_slice()),
        ("installer", installer.as_slice()),
        ("profile.toml", profile.as_slice()),
    ] {
        fs::write(source.join(name), bytes).unwrap();
    }
    let recipe = format!(
        "schema_version=1\n[kit]\nname='synthetic'\ntarget='x86_64-unknown-linux-gnu'\nversion='{}'\n[bundle]\npath='bundle.zip'\nsha256='{}'\n[installer]\npath='installer'\nsha256='{}'\n[site_profile]\npath='profile.toml'\nsha256='{}'\n",
        env!("CARGO_PKG_VERSION"),
        digest(&bundle),
        digest(&installer),
        digest(profile)
    );
    fs::write(source.join("recipe.toml"), recipe).unwrap();
    let output = root.join("produced B");
    good(command(
        &user,
        root,
        &[
            "kit",
            "build",
            "--file",
            source.join("recipe.toml").to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
        ],
    ));
    let home = root.join("one home");
    fs::create_dir(&home).unwrap();
    let settings = root.join("one settings");
    let install = root.join("one install");
    let invoke = |exe: &Path, args: &[&str]| {
        std::process::Command::new(exe)
            .env_clear()
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("VIBE_SETTINGS", &settings)
            .env("VIBEVM_INSTALL_ROOT", &install)
            .env("PATH", "/usr/bin:/bin")
            .current_dir(root)
            .args(args)
            .output()
            .unwrap()
    };
    good(invoke(
        &old_exe,
        &[
            "--unattended",
            "self",
            "import",
            old_exe.to_str().unwrap(),
            "--tag",
            "1.0.11",
            "--use",
        ],
    ));
    let shim = install.join("opt/bin/vibe");
    let current = || {
        let out = invoke(&shim, &["self", "current", "--json"]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap()
    };
    let version = || {
        let out = invoke(&shim, &["--version"]);
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    let which = || {
        let out = invoke(&shim, &["self", "which", "vibe", "--json"]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        std::path::PathBuf::from(v["path"].as_str().unwrap())
    };
    assert_eq!(version(), "vibe 1.0.11");
    let before = current();
    assert_eq!(before["active_selector"], "tag:1.0.11#1");
    assert_eq!(before["active"]["selector"], "tag:1.0.11#1");
    let old_index = which();
    assert_eq!(digest(&fs::read(&old_index).unwrap()), digest(&old_bytes));
    let old_hash = digest(&fs::read(&old_index).unwrap());
    fs::write(install.join("foreign-marker"), b"unrelated preserved").unwrap();
    let native = output.join("vibe-installer-x86_64-unknown-linux-gnu");
    let consume = |dry: bool| {
        let mut args = vec![
            "--kit",
            output.to_str().unwrap(),
            "--destination",
            install.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "--unattended",
            "--no-path",
            "--upgrade",
        ];
        if dry {
            args.push("--dry-run");
        }
        invoke(&native, &args)
    };
    let bundle_path = output.join(format!(
        "vibevm-{}-x86_64-unknown-linux-gnu.zip",
        env!("CARGO_PKG_VERSION")
    ));
    fs::write(&bundle_path, b"corrupt B").unwrap();
    assert!(!consume(false).status.success());
    assert_eq!(version(), "vibe 1.0.11");
    assert_eq!(current(), before);
    assert_eq!(digest(&fs::read(which()).unwrap()), old_hash);
    assert_eq!(
        fs::read(install.join("foreign-marker")).unwrap(),
        b"unrelated preserved"
    );
    fs::write(&bundle_path, bundle).unwrap();
    let inventory = old_index
        .ancestors()
        .find(|p| p.join("state.toml").is_file())
        .unwrap()
        .to_path_buf();
    let state_before = fs::read(inventory.join("state.toml")).unwrap();
    let pointer_before = fs::read(inventory.join("current")).unwrap();
    let pending = inventory.join("activation.pending.toml");
    fs::write(&pending, b"synthetic pending journal; must not recover").unwrap();
    assert!(!consume(true).status.success());
    assert_eq!(
        fs::read(&pending).unwrap(),
        b"synthetic pending journal; must not recover"
    );
    assert_eq!(
        fs::read(inventory.join("state.toml")).unwrap(),
        state_before
    );
    assert_eq!(fs::read(inventory.join("current")).unwrap(), pointer_before);
    fs::remove_file(&pending).unwrap();
    good(consume(true));
    assert_eq!(
        fs::read(inventory.join("state.toml")).unwrap(),
        state_before
    );
    assert_eq!(fs::read(inventory.join("current")).unwrap(), pointer_before);
    assert_eq!(version(), "vibe 1.0.11");
    assert_eq!(current(), before);
    assert_eq!(digest(&fs::read(which()).unwrap()), old_hash);
    good(consume(false));
    assert_eq!(
        version(),
        format!("vibe {}", env!("CARGO_PKG_VERSION")),
        "native rerun exited successfully without upgrading A"
    );
    let after = current();
    assert_eq!(
        after["active_selector"],
        format!("tag:{}#2", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(after["active"]["selector"], after["active_selector"]);
    assert_eq!(digest(&fs::read(which()).unwrap()), digest(&cli));
    assert_ne!(digest(&fs::read(which()).unwrap()), old_hash);
    assert_eq!(
        digest(&fs::read(&old_index).unwrap()),
        old_hash,
        "immutable A must remain available"
    );
    assert!(
        !consume(false).status.success(),
        "same-version upgrade must refuse"
    );
    assert_eq!(current(), after);
    assert_eq!(
        fs::read(install.join("foreign-marker")).unwrap(),
        b"unrelated preserved"
    );
}
