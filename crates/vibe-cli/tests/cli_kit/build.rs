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
    let source = zip(&[
        ("Cargo.toml", b"[workspace]\n"),
        ("crates/vibe-cli/src/main.rs", b"fn main() {}"),
    ]);
    let proxy = b"synthetic proxy";
    let index = b"synthetic index";
    let components = [
        ("vibe", DistributionComponentName::Vibe, cli),
        (
            "vibe-app-proxy",
            DistributionComponentName::VibeAppProxy,
            proxy.as_slice(),
        ),
        (
            "vibe-index",
            DistributionComponentName::VibeIndex,
            index.as_slice(),
        ),
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
        env!("CARGO_PKG_VERSION").parse().unwrap(),
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
