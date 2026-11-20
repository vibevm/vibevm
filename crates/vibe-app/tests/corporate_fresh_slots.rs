//! Fresh corporate supplier slots: manifest admission, not native execution.
use vibe_app::{InstallChoice, select_option};
use vibe_core::manifest::Manifest;

#[test]
fn fresh_delivery_slots_select_exact_x64_platform_and_refuse_cross_target_option() {
    let subjects = [
        (
            "pi",
            include_str!("../../../vibevm/vibepacks/org.vibevm.agents/pi/v1.6.0/vibe.toml"),
            "1.6.0",
        ),
        (
            "codex",
            include_str!("../../../vibevm/vibepacks/org.vibevm.agents/codex/v0.171.0/vibe.toml"),
            "0.171.0",
        ),
        (
            "opencode",
            include_str!("../../../vibevm/vibepacks/org.vibevm.agents/opencode/v2.3.2/vibe.toml"),
            "2.3.2",
        ),
        (
            "claude-code",
            include_str!(
                "../../../vibevm/vibepacks/org.vibevm.agents/claude-code/v2.1.329/vibe.toml"
            ),
            "2.1.329",
        ),
        (
            "qwen",
            include_str!("../../../vibevm/vibepacks/org.vibevm.agents/qwen/v0.28.0/vibe.toml"),
            "0.28.0",
        ),
    ];
    for (name, text, version) in subjects {
        let manifest = Manifest::parse_str(text).unwrap();
        let package = manifest.package.as_ref().unwrap();
        assert_eq!(package.name, name);
        assert_eq!(package.version.to_string(), version);
        for platform in ["linux", "macos", "windows"] {
            let selected = select_option(
                &manifest.installer_options,
                &InstallChoice {
                    platform: Some(platform.into()),
                    arch: Some("x86_64".into()),
                    installer: None,
                },
                None,
            )
            .unwrap();
            assert_eq!(selected.launchers.len(), 1);
            assert_eq!(selected.public_commands.len(), 1);
            assert_eq!(
                selected.public_commands[0].name,
                if name == "claude-code" {
                    "claude"
                } else {
                    name
                }
            );
            let other = if platform == "linux" {
                "windows"
            } else {
                "linux"
            };
            assert!(
                select_option(
                    &manifest.installer_options,
                    &InstallChoice {
                        platform: Some(other.into()),
                        arch: Some("x86_64".into()),
                        installer: Some(selected.id),
                    },
                    None
                )
                .is_err()
            );
        }
        assert!(
            select_option(
                &manifest.installer_options,
                &InstallChoice {
                    platform: Some("windows".into()),
                    arch: Some("aarch64".into()),
                    installer: None,
                },
                None
            )
            .is_err()
        );
    }
}
