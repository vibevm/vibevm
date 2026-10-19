//! Application installer selection by platform, architecture and named route.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use dialoguer::Select;
use serde_json::{Value, json};
use vibe_core::manifest::InstallerOptionDecl;

#[derive(Clone, Debug, Default)]
pub struct InstallChoice {
    pub platform: Option<String>,
    pub arch: Option<String>,
    pub installer_id: Option<String>,
    pub interactive: bool,
}

pub fn detected_platform() -> Option<&'static str> {
    match std::env::consts::OS {
        "linux" => Some("linux"),
        "macos" => Some("macos"),
        "windows" => Some("windows"),
        _ => None,
    }
}

pub fn detected_arch() -> &'static str {
    std::env::consts::ARCH
}

fn platform_ids(options: &[InstallerOptionDecl]) -> Vec<String> {
    options
        .iter()
        .map(|option| option.platform.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn arch_ids(options: &[InstallerOptionDecl], platform: &str) -> Vec<String> {
    options
        .iter()
        .filter(|option| option.platform == platform)
        .map(|option| option.arch.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn platform_label(options: &[InstallerOptionDecl], platform: &str) -> String {
    options
        .iter()
        .find(|option| option.platform == platform)
        .map(|option| option.platform_name.clone())
        .unwrap_or_else(|| platform.into())
}

fn arch_label(options: &[InstallerOptionDecl], platform: &str, arch: &str) -> String {
    options
        .iter()
        .find(|option| option.platform == platform && option.arch == arch)
        .and_then(|option| option.arch_name.clone())
        .unwrap_or_else(|| arch.into())
}

pub fn select_option(
    options: &[InstallerOptionDecl],
    choice: &InstallChoice,
) -> Result<InstallerOptionDecl> {
    if options.is_empty() {
        bail!("application declares no installer options");
    }
    let platforms = platform_ids(options);
    let detected_platform =
        detected_platform().filter(|id| platforms.iter().any(|candidate| candidate == id));
    let platform = if let Some(platform) = &choice.platform {
        platform.clone()
    } else if choice.interactive {
        let mut ordered = platforms.clone();
        if let Some(detected) = detected_platform {
            ordered.retain(|id| id != detected);
            ordered.insert(0, detected.into());
        }
        let labels: Vec<_> = ordered
            .iter()
            .map(|id| {
                let label = platform_label(options, id);
                if Some(id.as_str()) == detected_platform {
                    format!("Detected automatically: {label} ({id})")
                } else {
                    format!("{label} ({id})")
                }
            })
            .collect();
        let index = Select::new()
            .with_prompt("Select application platform")
            .items(&labels)
            .default(0)
            .interact_opt()?
            .ok_or_else(|| anyhow::anyhow!("platform selection cancelled"))?;
        ordered[index].clone()
    } else {
        detected_platform
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "platform selection required; use --platform with one of: {}",
                    platforms.join(", ")
                )
            })?
            .into()
    };
    let arches = arch_ids(options, &platform);
    if arches.is_empty() {
        bail!(
            "no installer option for platform `{platform}`; available platforms: {}",
            platforms.join(", ")
        );
    }
    let detected_arch = detected_arch();
    let auto_arch = arches.iter().any(|id| id == detected_arch);
    let arch = if let Some(arch) = &choice.arch {
        arch.clone()
    } else if choice.interactive {
        let mut ordered = arches.clone();
        if auto_arch {
            ordered.retain(|id| id != detected_arch);
            ordered.insert(0, detected_arch.into());
        }
        let labels: Vec<_> = ordered
            .iter()
            .map(|id| {
                let label = arch_label(options, &platform, id);
                if auto_arch && id == detected_arch {
                    format!("Detected automatically: {label} ({id})")
                } else {
                    format!("{label} ({id})")
                }
            })
            .collect();
        let index = Select::new()
            .with_prompt(format!("Select architecture for {platform}"))
            .items(&labels)
            .default(0)
            .interact_opt()?
            .ok_or_else(|| anyhow::anyhow!("architecture selection cancelled"))?;
        ordered[index].clone()
    } else if auto_arch {
        detected_arch.into()
    } else {
        bail!(
            "architecture selection required for `{platform}`; use --arch with one of: {}",
            arches.join(", ")
        );
    };

    let candidates: Vec<_> = options
        .iter()
        .filter(|option| option.platform == platform && option.arch == arch)
        .filter(|option| {
            choice
                .installer_id
                .as_ref()
                .is_none_or(|id| option.id == *id)
        })
        .collect();
    if candidates.is_empty() {
        bail!(
            "no installer option for `{platform}/{arch}`{}",
            choice
                .installer_id
                .as_ref()
                .map(|id| format!(" named `{id}`"))
                .unwrap_or_default()
        );
    }
    if candidates.len() == 1 {
        return Ok(candidates[0].clone());
    }
    if !choice.interactive {
        bail!(
            "installer selection required for `{platform}/{arch}`; use --installer with one of: {}",
            candidates
                .iter()
                .map(|option| option.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let labels: Vec<_> = candidates
        .iter()
        .map(|option| format!("{} ({})", option.name, option.id))
        .collect();
    let index = Select::new()
        .with_prompt(format!("Select installer for {platform}/{arch}"))
        .items(&labels)
        .default(0)
        .interact_opt()?
        .ok_or_else(|| anyhow::anyhow!("installer selection cancelled"))?;
    Ok(candidates[index].clone())
}

pub fn options_json(package: &str, options: &[InstallerOptionDecl]) -> Value {
    let detected_platform = detected_platform();
    let detected_arch = detected_arch();
    let mut grouped: BTreeMap<String, BTreeMap<String, Vec<Value>>> = BTreeMap::new();
    for option in options {
        grouped.entry(option.platform.clone()).or_default()
            .entry(option.arch.clone()).or_default().push(json!({
                "id": option.id,
                "name": option.name,
                "installation": option.installation,
                "pre_install": option.pre_install,
                "post_install": option.post_install,
                "launchers": option.launchers,
                "public_commands": option.public_commands,
                "detected": Some(option.platform.as_str()) == detected_platform && option.arch == detected_arch,
            }));
    }
    let mut platforms: Vec<Value> = grouped
        .into_iter()
        .map(|(platform, arches)| {
            let mut architectures: Vec<Value> = arches
                .into_iter()
                .map(|(arch, installers)| {
                    json!({
                        "id": arch,
                        "name": arch_label(options, &platform, &arch),
                        "detected": arch == detected_arch,
                        "installers": installers,
                    })
                })
                .collect();
            architectures.sort_by_key(|row| !row["detected"].as_bool().unwrap_or(false));
            json!({
                "id": platform,
                "name": platform_label(options, &platform),
                "detected": Some(platform.as_str()) == detected_platform,
                "architectures": architectures,
            })
        })
        .collect();
    platforms.sort_by_key(|row| !row["detected"].as_bool().unwrap_or(false));
    json!({
        "schema": "vibe-application-install-options/1",
        "package": package,
        "detected": { "platform": detected_platform, "arch": detected_arch },
        "platforms": platforms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_core::manifest::InstallerMethod;

    fn option(id: &str, platform: &str, arch: &str) -> InstallerOptionDecl {
        InstallerOptionDecl {
            id: id.into(),
            name: format!("Installer {id}"),
            platform: platform.into(),
            platform_name: format!("Platform {platform}"),
            arch: arch.into(),
            arch_name: None,
            pre_install: None,
            post_install: None,
            installation: InstallerMethod::Shell {
                app_root: "/tmp/demo".into(),
                install: "echo install".into(),
                force_install: "echo force install".into(),
                update: None,
                uninstall: "echo uninstall".into(),
                force_uninstall: "echo force uninstall".into(),
            },
            launchers: Vec::new(),
            public_commands: Vec::new(),
        }
    }

    #[test]
    fn package_authored_platform_requires_manual_selection() {
        let options = vec![option("apt", "ubuntu", detected_arch())];
        assert!(select_option(&options, &InstallChoice::default()).is_err());
        let selected = select_option(
            &options,
            &InstallChoice {
                platform: Some("ubuntu".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.id, "apt");
        let listing = options_json("org.example/demo", &options);
        assert_eq!(listing["platforms"][0]["detected"], false);
    }

    #[test]
    fn competing_installers_require_explicit_id_unattended() {
        let platform = detected_platform().unwrap_or("linux");
        let options = vec![
            option("apt", platform, detected_arch()),
            option("script", platform, detected_arch()),
        ];
        let error = select_option(&options, &InstallChoice::default()).unwrap_err();
        assert!(error.to_string().contains("installer selection required"));
        let selected = select_option(
            &options,
            &InstallChoice {
                installer_id: Some("script".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.id, "script");
    }
}
