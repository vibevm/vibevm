//! Installer-option selection by platform, architecture and stable id, and
//! the `vibe app installers` listing.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-options");

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use specmark::spec;
use vibe_core::manifest::{InstallerOptionDecl, steps_of};

use crate::error::ResolveError;

/// One row of an attended question: the parts a surface may align into
/// columns, and the flat one-line spelling every other surface reads.
///
/// The library builds the parts and never the presentation: a terminal
/// adapter aligns them, a test or a plain terminal takes [`Choice::label`].
///
/// ```
/// use vibe_app::Choice;
///
/// let host = Choice {
///     name: "Windows".into(),
///     id: Some("windows".into()),
///     detected: true,
///     ..Choice::default()
/// };
/// assert_eq!(host.label(), "Windows (windows)  [detected]");
///
/// let nested = Choice {
///     name: "Plugins".into(),
///     id: Some("plugins".into()),
///     description: Some("the bundled plugin set".into()),
///     depth: 1,
///     ..Choice::default()
/// };
/// assert_eq!(nested.label(), "  Plugins (plugins) — the bundled plugin set");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-options")]
pub struct Choice {
    /// The human name — the leading column of an aligned row.
    pub name: String,
    /// The stable id behind the name; a setup profile and the
    /// choose-the-features-one-by-one row carry none.
    pub id: Option<String>,
    /// One sentence about the row.
    pub description: Option<String>,
    /// Whether this row is what the host was detected to be.
    pub detected: bool,
    /// Nesting depth inside a tree question; indents the name.
    pub depth: usize,
}

impl Choice {
    /// A row that is only a name and an id.
    ///
    /// ```
    /// assert_eq!(vibe_app::Choice::named("MSI", "msi").label(), "MSI (msi)");
    /// ```
    pub fn named(name: &str, id: &str) -> Self {
        Self {
            name: name.into(),
            id: Some(id.into()),
            ..Self::default()
        }
    }

    /// The flat spelling of the row, with every part it carries.
    pub fn label(&self) -> String {
        let mut label = format!("{}{}", "  ".repeat(self.depth), self.name);
        if let Some(id) = &self.id {
            label.push_str(&format!(" ({id})"));
        }
        if self.detected {
            label.push_str("  [detected]");
        }
        if let Some(description) = &self.description {
            label.push_str(&format!(" — {description}"));
        }
        label
    }
}

/// An attended-terminal prompt: show `items` under `prompt`, return the
/// chosen index or `None` when the user cancels.
pub type Chooser<'a> = &'a dyn Fn(&str, &[Choice]) -> Result<Option<usize>, String>;

/// The explicit flags of one selection; absent values are auto-detected or
/// asked through the chooser.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-options")]
pub struct InstallChoice {
    pub platform: Option<String>,
    pub arch: Option<String>,
    pub installer: Option<String>,
}

/// The predefined platform id of this host, if it has one.
pub fn detected_platform() -> Option<&'static str> {
    match std::env::consts::OS {
        "linux" => Some("linux"),
        "macos" => Some("macos"),
        "windows" => Some("windows"),
        _ => None,
    }
}

/// The CPU architecture id of this host.
pub fn detected_arch() -> &'static str {
    std::env::consts::ARCH
}

fn platform_label(options: &[InstallerOptionDecl], platform: &str) -> String {
    options
        .iter()
        .find(|option| option.platform == platform)
        .map_or_else(|| platform.into(), |option| option.platform_name.clone())
}

fn arch_label(options: &[InstallerOptionDecl], platform: &str, arch: &str) -> String {
    options
        .iter()
        .find(|option| option.platform == platform && option.arch == arch)
        .and_then(|option| option.arch_name.clone())
        .unwrap_or_else(|| arch.into())
}

fn ask(
    chooser: Chooser<'_>,
    prompt: &str,
    items: &[Choice],
    what: &'static str,
) -> Result<usize, ResolveError> {
    chooser(prompt, items)
        .map_err(|reason| ResolveError::Prompt { reason })?
        .filter(|index| *index < items.len())
        .ok_or(ResolveError::Cancelled { what })
}

/// Detected entry first, labelled; the rest in id order.
fn ordered(
    ids: Vec<String>,
    detected: Option<&str>,
    label: impl Fn(&str) -> String,
) -> (Vec<String>, Vec<Choice>) {
    let mut ids = ids;
    if let Some(detected) = detected
        && let Some(position) = ids.iter().position(|id| id == detected)
    {
        let id = ids.remove(position);
        ids.insert(0, id);
    }
    let choices = ids
        .iter()
        .map(|id| Choice {
            name: label(id),
            id: Some(id.clone()),
            detected: Some(id.as_str()) == detected,
            ..Choice::default()
        })
        .collect();
    (ids, choices)
}

/// Select exactly one installer option: platform, then architecture, then
/// installer. Without a chooser every ambiguity is a selection error.
///
/// ```
/// use vibe_app::{InstallChoice, select_option};
/// use vibe_core::manifest::Manifest;
///
/// let manifest = Manifest::parse_str(r#"
/// [package]
/// name = "demo"
/// group = "org.example"
/// kind = "app"
/// version = "1.0.0"
/// [application]
/// id = "demo"
/// [[installer_options]]
/// id = "apt"
/// name = "APT"
/// platform = "ubuntu"
/// platform_name = "Ubuntu"
/// arch = "x86_64"
/// [installer_options.installation]
/// type = "shell"
/// app_root = "/usr/lib/demo"
/// install = "apt-get install demo"
/// force_install = "apt-get install --reinstall demo"
/// uninstall = "apt-get remove demo"
/// force_uninstall = "apt-get purge demo"
/// "#).unwrap();
/// let choice = InstallChoice {
///     platform: Some("ubuntu".into()),
///     arch: Some("x86_64".into()),
///     installer: None,
/// };
/// let option = select_option(&manifest.installer_options, &choice, None).unwrap();
/// assert_eq!(option.id, "apt");
/// ```
pub fn select_option(
    options: &[InstallerOptionDecl],
    choice: &InstallChoice,
    chooser: Option<Chooser<'_>>,
) -> Result<InstallerOptionDecl, ResolveError> {
    let platforms: Vec<String> = options
        .iter()
        .map(|option| option.platform.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let detected = detected_platform().filter(|id| platforms.iter().any(|known| known == id));
    let platform = match (&choice.platform, chooser) {
        (Some(platform), _) => platform.clone(),
        (None, Some(chooser)) => {
            let (ids, choices) = ordered(platforms.clone(), detected, |id| {
                platform_label(options, id)
            });
            let index = ask(chooser, "Platform", &choices, "platform")?;
            ids[index].clone()
        }
        (None, None) => detected
            .ok_or_else(|| ResolveError::SelectionRequired {
                what: "platform selection required".into(),
                flag: "--platform",
                choices: platforms.join(", "),
            })?
            .into(),
    };
    let arches: Vec<String> = options
        .iter()
        .filter(|option| option.platform == platform)
        .map(|option| option.arch.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if arches.is_empty() {
        return Err(ResolveError::NoOption {
            detail: format!("platform `{platform}`"),
            choices: platforms.join(", "),
        });
    }
    let host_arch = Some(detected_arch()).filter(|id| arches.iter().any(|known| known == id));
    let arch = match (&choice.arch, chooser) {
        (Some(arch), _) => arch.clone(),
        (None, Some(chooser)) => {
            let (ids, choices) = ordered(arches.clone(), host_arch, |id| {
                arch_label(options, &platform, id)
            });
            let prompt = format!("Architecture ({platform})");
            let index = ask(chooser, &prompt, &choices, "architecture")?;
            ids[index].clone()
        }
        (None, None) => host_arch
            .ok_or_else(|| ResolveError::SelectionRequired {
                what: format!("architecture selection required for `{platform}`"),
                flag: "--arch",
                choices: arches.join(", "),
            })?
            .into(),
    };
    let on_target: Vec<&InstallerOptionDecl> = options
        .iter()
        .filter(|option| option.platform == platform && option.arch == arch)
        .collect();
    let candidates: Vec<&InstallerOptionDecl> = on_target
        .iter()
        .copied()
        .filter(|option| choice.installer.as_ref().is_none_or(|id| &option.id == id))
        .collect();
    match candidates.as_slice() {
        [] => Err(ResolveError::NoOption {
            detail: match &choice.installer {
                Some(id) => format!("`{platform}/{arch}` named `{id}`"),
                None => format!("`{platform}/{arch}`"),
            },
            choices: if on_target.is_empty() {
                arches.join(", ")
            } else {
                ids(&on_target)
            },
        }),
        [only] => Ok((*only).clone()),
        many => match chooser {
            None => Err(ResolveError::SelectionRequired {
                what: format!("installer selection required for `{platform}/{arch}`"),
                flag: "--installer",
                choices: ids(many),
            }),
            Some(chooser) => {
                let choices: Vec<Choice> = many
                    .iter()
                    .map(|option| Choice::named(&option.name, &option.id))
                    .collect();
                let prompt = format!("Installer ({platform}/{arch})");
                let index = ask(chooser, &prompt, &choices, "installer")?;
                Ok(many[index].clone())
            }
        },
    }
}

fn ids(options: &[&InstallerOptionDecl]) -> String {
    options
        .iter()
        .map(|option| option.id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// One launcher in the shape of the installer listing: its short-form icon
/// and the effective `terminal`; `vibe app describe` carries the rest.
fn listed_launcher(launcher: &vibe_core::manifest::LauncherDecl) -> Value {
    let mut row = json!({
        "id": launcher.id,
        "name": launcher.name,
        "executable": launcher.executable,
        "terminal": launcher.is_terminal(),
        "default": launcher.default,
    });
    let icon = match &launcher.icon {
        Some(vibe_core::manifest::IconDecl::Path(path)) => Some(path.clone()),
        _ => None,
    };
    let optional = [
        ("description", launcher.description.clone().map(Value::from)),
        (
            "working_directory",
            launcher.working_directory.clone().map(Value::from),
        ),
        ("icon", icon.map(Value::from)),
        (
            "args",
            (!launcher.args.is_empty()).then(|| json!(launcher.args)),
        ),
    ];
    if let Some(object) = row.as_object_mut() {
        for (key, value) in optional {
            if let Some(value) = value {
                object.insert(key.into(), value);
            }
        }
    }
    row
}

/// The `vibe-application-install-options/1` listing of every option, grouped
/// by platform and architecture with the detected entries first.
pub fn options_listing(package: &str, options: &[InstallerOptionDecl]) -> Value {
    let host_platform = detected_platform();
    let host_arch = detected_arch();
    let mut grouped: BTreeMap<&str, BTreeMap<&str, Vec<Value>>> = BTreeMap::new();
    for option in options {
        grouped
            .entry(&option.platform)
            .or_default()
            .entry(&option.arch)
            .or_default()
            .push(json!({
                "id": option.id,
                "name": option.name,
                "portable": option.portable,
                "installation": option.installation,
                "step_ids": steps_of(option).iter().map(|step| step.id.as_str()).collect::<Vec<_>>(),
                "pre_install": option.pre_install,
                "post_install": option.post_install,
                "launchers": option.launchers.iter().map(listed_launcher).collect::<Vec<_>>(),
                "public_commands": option.public_commands,
                "detected": Some(option.platform.as_str()) == host_platform && option.arch == host_arch,
            }));
    }
    let mut platforms: Vec<(bool, Value)> = grouped
        .into_iter()
        .map(|(platform, arches)| {
            let mut architectures: Vec<(bool, Value)> = arches
                .into_iter()
                .map(|(arch, installers)| {
                    let detected = arch == host_arch;
                    let row = json!({
                        "id": arch,
                        "name": arch_label(options, platform, arch),
                        "detected": detected,
                        "installers": installers,
                    });
                    (detected, row)
                })
                .collect();
            architectures.sort_by_key(|(detected, _)| !detected);
            let detected = Some(platform) == host_platform;
            let row = json!({
                "id": platform,
                "name": platform_label(options, platform),
                "detected": detected,
                "architectures": architectures.into_iter().map(|(_, row)| row).collect::<Vec<_>>(),
            });
            (detected, row)
        })
        .collect();
    platforms.sort_by_key(|(detected, _)| !detected);
    json!({
        "schema": "vibe-application-install-options/1",
        "package": package,
        "detected": { "platform": host_platform, "arch": host_arch },
        "platforms": platforms.into_iter().map(|(_, row)| row).collect::<Vec<_>>(),
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
            portable: None,
            pre_install: None,
            post_install: None,
            installation: Some(InstallerMethod::Shell {
                app_root: "/tmp/demo".into(),
                install: "echo install".into(),
                force_install: "echo force install".into(),
                update: None,
                uninstall: "echo uninstall".into(),
                force_uninstall: "echo force uninstall".into(),
                interpreter: None,
            }),
            result: None,
            features: Vec::new(),
            configuration: None,
            profiles: Vec::new(),
            parameters: Vec::new(),
            license: None,
            steps: Vec::new(),
            requires: Vec::new(),
            launchers_from: Vec::new(),
            launchers: Vec::new(),
            public_commands: Vec::new(),
        }
    }

    #[test]
    fn package_authored_platform_requires_explicit_selection() {
        let options = vec![option("apt", "ubuntu", detected_arch())];
        let error = select_option(&options, &InstallChoice::default(), None).unwrap_err();
        assert!(error.to_string().contains("--platform"), "{error}");
        let selected = select_option(
            &options,
            &InstallChoice {
                platform: Some("ubuntu".into()),
                ..Default::default()
            },
            None,
        )
        .unwrap();
        assert_eq!(selected.id, "apt");
        let listing = options_listing("org.example/demo", &options);
        assert_eq!(listing["platforms"][0]["detected"], false);
    }

    #[test]
    fn competing_installers_require_explicit_id_unattended() {
        let platform = detected_platform().unwrap_or("linux");
        let options = vec![
            option("apt", platform, detected_arch()),
            option("script", platform, detected_arch()),
        ];
        let error = select_option(&options, &InstallChoice::default(), None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("selection_required"), "{error}");
        assert!(error.contains("apt, script"), "{error}");
        let explicit = InstallChoice {
            installer: Some("script".into()),
            ..Default::default()
        };
        assert_eq!(
            select_option(&options, &explicit, None).unwrap().id,
            "script"
        );
        let missing = InstallChoice {
            installer: Some("nope".into()),
            ..Default::default()
        };
        assert!(select_option(&options, &missing, None).is_err());
    }

    #[test]
    fn attended_selection_offers_the_detected_platform_first() {
        let platform = detected_platform().unwrap_or("linux");
        let options = vec![
            option("aaa", "aaa-os", "riscv64"),
            option("host", platform, detected_arch()),
            option("other", platform, "zz-cpu"),
        ];
        let prompts = std::cell::RefCell::new(Vec::new());
        let chooser = |prompt: &str, items: &[Choice]| -> Result<Option<usize>, String> {
            prompts
                .borrow_mut()
                .push((prompt.to_owned(), items.to_vec()));
            Ok(Some(0))
        };
        let selected = select_option(&options, &InstallChoice::default(), Some(&chooser)).unwrap();
        assert_eq!(selected.id, "host");
        let prompts = prompts.borrow();
        assert!(prompts[0].1[0].detected);
        assert!(prompts[0].1[0].label().ends_with("[detected]"));
        assert!(prompts[1].1[0].label().ends_with("[detected]"));
        let cancel = |_: &str, _: &[Choice]| -> Result<Option<usize>, String> { Ok(None) };
        assert!(matches!(
            select_option(&options, &InstallChoice::default(), Some(&cancel)),
            Err(ResolveError::Cancelled { what: "platform" })
        ));
    }
}
