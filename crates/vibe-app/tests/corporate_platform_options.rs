//! Canonical supplier option selection, not foreign-native execution proof.
use vibe_app::{InstallChoice, select_option};
use vibe_core::manifest::{InstallerMethod, Manifest};

#[test]
fn pi_exact_supplier_platform_selection_preserves_linux_and_refuses_mismatch() {
    let manifest = Manifest::parse_str(include_str!(
        "../../../vibevm/vibepacks/org.vibevm.agents/pi/v1.5.2/vibe.toml"
    ))
    .unwrap();
    assert_eq!(manifest.installer_options.len(), 2);
    for (platform, arch, id, asset, digest) in [
        (
            "macos",
            "aarch64",
            "macos-arm64-standalone",
            "pi-darwin-arm64.tar.gz",
            "56b5e194d485ced555667b8ba3276157a8272e9ecde9e001e428af80985bdd9b",
        ),
        (
            "linux",
            "x86_64",
            "linux-x64-standalone",
            "pi-linux-x64.tar.gz",
            "811c6f2c9e7aaee252b5404252e556fb713aa209c1f7f924226aa77d4ef70cb5",
        ),
    ] {
        let choice = InstallChoice {
            platform: Some(platform.into()),
            arch: Some(arch.into()),
            installer: None,
        };
        let selected = select_option(&manifest.installer_options, &choice, None).unwrap();
        assert_eq!(selected.id, id);
        let Some(InstallerMethod::TarGz {
            source,
            sha256,
            payload_root,
        }) = selected.installation
        else {
            panic!("expected exact supplier tar.gz owner");
        };
        assert_eq!(
            source,
            format!("https://github.com/earendil-works/pi/releases/download/v1.5.2/{asset}")
        );
        assert_eq!(sha256, digest);
        assert_eq!(payload_root.as_deref(), Some(std::path::Path::new("pi")));
        assert_eq!(selected.launchers.len(), 1);
        assert_eq!(selected.launchers[0].executable, "$app_root/pi");
        assert_eq!(selected.public_commands.len(), 1);
        assert_eq!(selected.public_commands[0].name, "pi");
        assert_eq!(selected.public_commands[0].launcher, "main");
    }
    for (platform, arch, installer) in [
        ("windows", "x86_64", None),
        ("macos", "x86_64", None),
        ("macos", "aarch64", Some("linux-x64-standalone")),
        ("linux", "x86_64", Some("macos-arm64-standalone")),
    ] {
        assert!(
            select_option(
                &manifest.installer_options,
                &InstallChoice {
                    platform: Some(platform.into()),
                    arch: Some(arch.into()),
                    installer: installer.map(str::to_owned),
                },
                None,
            )
            .is_err()
        );
    }
}
