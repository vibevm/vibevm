//! Canonical supplier option selection, not foreign-native execution proof.
use vibe_app::{InstallChoice, select_option};
use vibe_core::manifest::{InstallerMethod, Manifest};

#[test]
fn pi_exact_supplier_platform_selection_preserves_linux_and_refuses_mismatch() {
    let manifest = Manifest::parse_str(include_str!(
        "../../../vibevm/vibepacks/org.vibevm.agents/pi/v1.5.2/vibe.toml"
    ))
    .unwrap();
    assert_eq!(manifest.installer_options.len(), 3);
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
        ("windows", "aarch64", None),
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

#[test]
fn codex_and_qwen_exact_macos_options_preserve_linux_and_refuse_cross_target_ids() {
    for (text, name, source, digest, root, executable, linux_id) in [
        (
            include_str!("../../../vibevm/vibepacks/org.vibevm.agents/codex/v0.170.0/vibe.toml"),
            "codex",
            "https://github.com/openai/codex/releases/download/rust-v0.170.0/codex-aarch64-apple-darwin.tar.gz",
            "7c9f21d5cbae4860d14b379fdcf709ff399ce874fd714b58783e1b4199c2b9a9",
            None,
            "$app_root/codex-aarch64-apple-darwin",
            "linux-x64-musl",
        ),
        (
            include_str!("../../../vibevm/vibepacks/org.vibevm.agents/qwen/v0.28.0/vibe.toml"),
            "qwen",
            "https://github.com/QwenLM/qwen-code/releases/download/v0.28.0/qwen-code-darwin-arm64.tar.gz",
            "5355baa7b994da2641f9ed6bff6ac7b7d06cff2779635704b3aae26fdbfc412f",
            Some(std::path::Path::new("qwen-code")),
            "$app_bin/qwen",
            "linux-x64-standalone",
        ),
    ] {
        let manifest = Manifest::parse_str(text).unwrap();
        assert_eq!(manifest.installer_options.len(), 3);
        let choice = InstallChoice {
            platform: Some("macos".into()),
            arch: Some("aarch64".into()),
            installer: None,
        };
        let selected = select_option(&manifest.installer_options, &choice, None).unwrap();
        assert_eq!(selected.id, "macos-arm64-standalone");
        let Some(InstallerMethod::TarGz {
            source: actual,
            sha256,
            payload_root,
        }) = selected.installation
        else {
            panic!("expected exact supplier tar.gz owner");
        };
        assert_eq!(actual, source);
        assert_eq!(sha256, digest);
        assert_eq!(payload_root.as_deref(), root);
        assert_eq!(selected.launchers[0].executable, executable);
        assert_eq!(selected.public_commands[0].name, name);
        let linux = InstallChoice {
            platform: Some("linux".into()),
            arch: Some("x86_64".into()),
            installer: None,
        };
        assert_eq!(
            select_option(&manifest.installer_options, &linux, None)
                .unwrap()
                .id,
            linux_id
        );
        for rejected in [
            InstallChoice {
                platform: Some("windows".into()),
                arch: Some("aarch64".into()),
                installer: None,
            },
            InstallChoice {
                platform: Some("macos".into()),
                arch: Some("x86_64".into()),
                installer: None,
            },
            InstallChoice {
                installer: Some(linux_id.into()),
                ..choice
            },
            InstallChoice {
                installer: Some("macos-arm64-standalone".into()),
                ..linux
            },
        ] {
            assert!(select_option(&manifest.installer_options, &rejected, None).is_err());
        }
    }
}

#[test]
fn windows_pi_codex_options_use_actual_pe_layout() {
    for (text, name, kind, digest, root, executable, args) in [
        (
            include_str!("../../../vibevm/vibepacks/org.vibevm.agents/pi/v1.5.2/vibe.toml"),
            "pi",
            "zip",
            "e6adb8cb8c84eca14391ad24b363de9d2be98a89ca3ba3fdee1fa4de32b7f2e8",
            None,
            "$app_root/pi.exe",
            Vec::<String>::new(),
        ),
        (
            include_str!("../../../vibevm/vibepacks/org.vibevm.agents/codex/v0.170.0/vibe.toml"),
            "codex",
            "tar_gz",
            "f32cad4e92e17f5ba1a4836f0bacca3950457cb204a4274dcb75fec99fbd82cd",
            None,
            "$app_root/codex-x86_64-pc-windows-msvc.exe",
            Vec::new(),
        ),
    ] {
        let manifest = Manifest::parse_str(text).unwrap();
        let choice = InstallChoice {
            platform: Some("windows".into()),
            arch: Some("x86_64".into()),
            installer: None,
        };
        let selected = select_option(&manifest.installer_options, &choice, None).unwrap();
        assert_eq!(selected.id, "windows-x64-standalone");
        let method = selected.installation.unwrap();
        assert_eq!(method.kind(), kind);
        let (actual_digest, actual_root) = match method {
            InstallerMethod::Zip {
                sha256,
                payload_root,
                ..
            }
            | InstallerMethod::TarGz {
                sha256,
                payload_root,
                ..
            } => (sha256, payload_root),
            _ => panic!("expected verified archive, never execute an installer"),
        };
        assert_eq!(actual_digest, digest);
        assert_eq!(actual_root.as_deref(), root);
        assert_eq!(selected.launchers[0].executable, executable);
        assert_eq!(selected.launchers[0].args, args);
        assert_eq!(selected.public_commands[0].name, name);
        for invalid in [
            InstallChoice {
                arch: Some("aarch64".into()),
                ..choice.clone()
            },
            InstallChoice {
                installer: Some("macos-arm64-standalone".into()),
                ..choice
            },
        ] {
            assert!(select_option(&manifest.installer_options, &invalid, None).is_err());
        }
    }
}

#[test]
fn qwen_windows_uses_existing_archive_copy_and_owned_bootstrap_not_shell() {
    use vibe_core::manifest::StepKind;
    let manifest = Manifest::parse_str(include_str!(
        "../../../vibevm/vibepacks/org.vibevm.agents/qwen/v0.28.0/vibe.toml"
    ))
    .unwrap();
    let selected = select_option(
        &manifest.installer_options,
        &InstallChoice {
            platform: Some("windows".into()),
            arch: Some("x86_64".into()),
            installer: None,
        },
        None,
    )
    .unwrap();
    assert_eq!(selected.id, "windows-x64-standalone");
    assert!(selected.installation.is_none());
    assert!(selected.result.as_ref().unwrap().is_owned());
    assert_eq!(selected.steps.len(), 2);
    assert!(
        matches!(&selected.steps[0].kind, StepKind::Archive { sha256, payload_root: Some(root), size: Some(74339211), .. } if sha256 == "9a758331884870e1d6073cc24f5a2545543127df0b6fb91db6a2e8a97bfd25ed" && root == std::path::Path::new("qwen-code"))
    );
    assert!(
        matches!(&selected.steps[1].kind, StepKind::Copy { from, to } if from == std::path::Path::new("qwen-standalone-bootstrap.cjs") && to == "vibe-qwen-standalone.cjs")
    );
    assert_eq!(selected.launchers[0].executable, "$app_root/node/node.exe");
    assert_eq!(
        selected.launchers[0].args,
        ["$app_root/vibe-qwen-standalone.cjs"]
    );
    assert_eq!(selected.public_commands[0].name, "qwen");
}

#[test]
fn opencode_and_claude_native_options_bind_actual_four_archive_layouts() {
    for (text, platform, arch, id, digest, root, executable, command) in [
        (
            include_str!("../../../vibevm/vibepacks/org.vibevm.agents/opencode/v1.19.6/vibe.toml"),
            "macos",
            "aarch64",
            "macos-arm64-standalone",
            "c0be3f34dde2fdcb31b25120f303c6ed5b69b828f16e5e0fbecff21a882adda1",
            None,
            "$app_root/opencode",
            "opencode",
        ),
        (
            include_str!("../../../vibevm/vibepacks/org.vibevm.agents/opencode/v1.19.6/vibe.toml"),
            "windows",
            "x86_64",
            "windows-x64-standalone",
            "fa31c9ec0bac7611ab12699ef8e2fc4c173fb95ac977b09b189b255b8f559bcc",
            None,
            "$app_root/opencode.exe",
            "opencode",
        ),
        (
            include_str!(
                "../../../vibevm/vibepacks/org.vibevm.agents/claude-code/v2.1.328/vibe.toml"
            ),
            "macos",
            "aarch64",
            "macos-arm64-native",
            "df0047f7602ca25c4ad59e9ad885e818e39fa691daa8852229f8f79a7cc3afcf",
            Some(std::path::Path::new("package")),
            "$app_root/claude",
            "claude",
        ),
        (
            include_str!(
                "../../../vibevm/vibepacks/org.vibevm.agents/claude-code/v2.1.328/vibe.toml"
            ),
            "windows",
            "x86_64",
            "windows-x64-native",
            "d4ea3de7e128b751a3ecc0198e699df31d1269ffd1064f89703145b90d74a8c7",
            Some(std::path::Path::new("package")),
            "$app_root/claude.exe",
            "claude",
        ),
    ] {
        let manifest = Manifest::parse_str(text).unwrap();
        assert_eq!(manifest.installer_options.len(), 3);
        let choice = InstallChoice {
            platform: Some(platform.into()),
            arch: Some(arch.into()),
            installer: None,
        };
        let selected = select_option(&manifest.installer_options, &choice, None).unwrap();
        assert_eq!(selected.id, id);
        let (source, sha256, payload_root) = match selected.installation.unwrap() {
            InstallerMethod::Zip {
                source,
                sha256,
                payload_root,
            }
            | InstallerMethod::TarGz {
                source,
                sha256,
                payload_root,
            } => (source, sha256, payload_root),
            _ => panic!("expected exact supplier archive"),
        };
        assert!(source.starts_with(if command == "claude" {
            "https://registry.npmjs.org/@anthropic-ai/claude-code-"
        } else {
            "https://github.com/anomalyco/opencode/releases/download/v1.19.6/"
        }));
        assert_eq!(sha256, digest);
        assert_eq!(payload_root.as_deref(), root);
        assert_eq!(selected.launchers[0].executable, executable);
        assert!(selected.launchers[0].args.is_empty());
        assert_eq!(selected.public_commands[0].name, command);
        assert!(
            select_option(
                &manifest.installer_options,
                &InstallChoice {
                    arch: Some("riscv64".into()),
                    ..choice.clone()
                },
                None
            )
            .is_err()
        );
        assert!(
            select_option(
                &manifest.installer_options,
                &InstallChoice {
                    installer: Some("linux-x64-native".into()),
                    ..choice
                },
                None
            )
            .is_err()
        );
    }
}
