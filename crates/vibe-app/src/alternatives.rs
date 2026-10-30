//! Slot alternatives: every installation of a slot is a provider, each slot
//! has one global default, and the public commands of all providers in a slot
//! are reserved for that slot alone.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#alternatives");

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use specmark::spec;
use vibe_core::manifest::PublicCommandDecl;
use vibe_wire::generated::application_alternatives as wire;

use crate::error::AppError;
use crate::index::{Index, Installation};
use crate::launch;
use crate::proxies;
use crate::settings::SettingsRoot;
use crate::storage::Storage;

/// Every reserved command name with the slot that reserves it.
///
/// ```
/// let index = vibe_app::Index::default();
/// assert!(vibe_app::reserved_commands(&index).is_empty());
/// ```
pub fn reserved_commands(index: &Index) -> BTreeMap<String, String> {
    let mut reserved = BTreeMap::new();
    for record in index.installations.values() {
        for command in &record.public_commands {
            reserved
                .entry(command.name.clone())
                .or_insert_with(|| record.slot.clone());
        }
    }
    reserved
}

/// The reserved command names alone, the input of a proxy synchronisation.
pub(crate) fn reserved_names(index: &Index) -> BTreeSet<String> {
    reserved_commands(index).into_keys().collect()
}

/// Refuse an installation whose public commands another slot reserves. The
/// records named in `replacing` are about to be replaced and do not count.
pub(crate) fn check_reservation(
    index: &Index,
    replacing: &[&str],
    slot: &str,
    commands: &[PublicCommandDecl],
) -> Result<(), AppError> {
    for (key, record) in &index.installations {
        if record.slot == slot || replacing.contains(&key.as_str()) {
            continue;
        }
        if let Some(command) = commands.iter().find(|command| {
            record
                .public_commands
                .iter()
                .any(|own| own.name == command.name)
        }) {
            return Err(AppError::CommandReserved {
                command: command.name.clone(),
                slot: slot.to_owned(),
                owner: record.slot.clone(),
            });
        }
    }
    Ok(())
}

/// The result of `vibe app use`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#alternatives")]
pub struct DefaultChange {
    pub slot: String,
    pub previous: Option<String>,
    pub default: String,
}

/// Make `key` the global default of `slot`, then synchronise the proxies.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let image = dir.path().join("vibe-app-proxy");
/// let error = vibe_app::set_default(&settings, "tool", "org.example/a@1.0.0", &image);
/// assert!(error.unwrap_err().to_string().contains("no installed provider"));
/// ```
pub fn set_default(
    settings: &SettingsRoot,
    slot: &str,
    key: &str,
    proxy_image: &Path,
) -> Result<DefaultChange, AppError> {
    let _lock = settings.lock()?;
    let mut index = Index::load(settings)?;
    let providers = providers(&index, slot)?;
    if !providers.iter().any(|(provider, _)| *provider == key) {
        return Err(AppError::NotProvider {
            slot: slot.to_owned(),
            key: key.to_owned(),
            providers: providers
                .iter()
                .map(|(provider, _)| *provider)
                .collect::<Vec<_>>()
                .join(", "),
        });
    }
    let previous = index.default_of(slot).map(str::to_owned);
    let reserved = reserved_names(&index);
    index.slots.insert(slot.to_owned(), Some(key.to_owned()));
    index.save(settings)?;
    proxies::sync(settings, &reserved, &reserved_names(&index), proxy_image)?;
    Ok(DefaultChange {
        slot: slot.to_owned(),
        previous,
        default: key.to_owned(),
    })
}

/// The `vibe-app-alternatives/1` listing of one slot: providers with their
/// status, the default, and which providers offer each command.
pub fn listing(
    settings: &SettingsRoot,
    slot: &str,
) -> Result<wire::ApplicationAlternatives, AppError> {
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let providers = providers(&index, slot)?;
    let default = index.default_of(slot);
    let mut commands: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let rows = providers
        .iter()
        .map(|(key, record)| {
            for command in &record.public_commands {
                commands
                    .entry(command.name.as_str())
                    .or_default()
                    .push((*key).to_owned());
            }
            let reason = launch::unavailable_reason(&launch::app_root(record, &storage));
            wire::Provider {
                key: (*key).to_owned(),
                package: wire::Package {
                    group: record.package.group.clone(),
                    name: record.package.name.clone(),
                    version: record.package.version.clone(),
                },
                method: record.method.as_str().into(),
                status: if reason.is_none() {
                    wire::Status::Ready
                } else {
                    wire::Status::Unavailable
                },
                reason,
                default: default == Some(*key),
                commands: record
                    .public_commands
                    .iter()
                    .map(|command| command.name.clone())
                    .collect(),
            }
        })
        .collect();
    Ok(wire::ApplicationAlternatives {
        protocol: "vibe-app-alternatives/1".into(),
        slot: slot.to_owned(),
        default: default.map(str::to_owned),
        providers: rows,
        commands: commands
            .into_iter()
            .map(|(name, providers)| wire::Command {
                name: name.to_owned(),
                providers,
            })
            .collect(),
    })
}

fn providers<'a>(
    index: &'a Index,
    slot: &str,
) -> Result<Vec<(&'a str, &'a Installation)>, AppError> {
    let providers: Vec<_> = index
        .installations
        .iter()
        .filter(|(_, record)| record.slot == slot)
        .map(|(key, record)| (key.as_str(), record))
        .collect();
    if providers.is_empty() {
        return Err(AppError::UnknownSlot {
            slot: slot.to_owned(),
        });
    }
    Ok(providers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{AuthoredCommands, Location, Method, PackageCoordinate};

    fn record(name: &str, slot: &str, commands: &[&str]) -> Installation {
        Installation {
            slot: slot.into(),
            package: PackageCoordinate {
                group: "org.example".into(),
                name: name.into(),
                version: "1.0.0".into(),
            },
            option: "linux".into(),
            platform: "linux".into(),
            arch: "x86_64".into(),
            method: Method::Shell,
            location: Location::External {
                app_root: std::env::temp_dir(),
            },
            portable: false,
            launchers: Vec::new(),
            public_commands: commands
                .iter()
                .map(|name| PublicCommandDecl {
                    name: (*name).into(),
                    launcher: "main".into(),
                })
                .collect(),
            commands: AuthoredCommands::default(),
        }
    }

    fn index(records: &[Installation]) -> Index {
        let mut index = Index::default();
        for record in records {
            index
                .installations
                .insert(record.package.key(), record.clone());
            index.slots.entry(record.slot.clone()).or_insert(None);
        }
        index
    }

    fn decls(names: &[&str]) -> Vec<PublicCommandDecl> {
        record("x", "x", names).public_commands
    }

    #[test]
    fn two_slots_may_not_claim_one_command() {
        let index = index(&[
            record("alpha", "tool", &["tool", "tool-help"]),
            record("beta", "tool", &["tool"]),
        ]);
        let reserved = reserved_commands(&index);
        assert_eq!(reserved.get("tool").map(String::as_str), Some("tool"));
        assert_eq!(reserved.len(), 2);
        assert!(check_reservation(&index, &[], "tool", &decls(&["tool", "new"])).is_ok());
        let error = check_reservation(&index, &[], "other", &decls(&["free", "tool-help"]))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("`tool-help` is reserved by slot `tool`"),
            "{error}"
        );
        assert!(check_reservation(&index, &[], "other", &decls(&["free"])).is_ok());
        let replacing = ["org.example/alpha@1.0.0", "org.example/beta@1.0.0"];
        assert!(check_reservation(&index, &replacing, "other", &decls(&["tool"])).is_ok());
    }

    #[test]
    fn use_switches_the_default_and_listing_reports_it() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let image = dir.path().join("vibe-app-proxy");
        std::fs::write(&image, b"proxy").unwrap();
        let mut start = index(&[
            record("alpha", "tool", &["tool"]),
            record("beta", "tool", &["tool"]),
            record("gamma", "other", &[]),
        ]);
        start
            .slots
            .insert("tool".into(), Some("org.example/alpha@1.0.0".into()));
        start.save(&settings).unwrap();

        let change = set_default(&settings, "tool", "org.example/beta@1.0.0", &image).unwrap();
        assert_eq!(change.previous.as_deref(), Some("org.example/alpha@1.0.0"));
        let saved = Index::load(&settings).unwrap();
        assert_eq!(saved.default_of("tool"), Some("org.example/beta@1.0.0"));
        assert!(
            settings
                .proxy_dir()
                .join(proxies::proxy_file_name("tool"))
                .is_file()
        );
        // One mutation per test: a second lock right after the first can race
        // a child another test thread forks while the first lock is held. The
        // refusal of a foreign provider is proven by the public CLI fixture.

        let listed = listing(&settings, "tool").unwrap();
        assert_eq!(listed.default.as_deref(), Some("org.example/beta@1.0.0"));
        assert_eq!(listed.providers.len(), 2);
        assert!(listed.providers[1].default);
        assert_eq!(listed.commands.len(), 1);
        assert_eq!(listed.commands[0].providers.len(), 2);
        assert!(listing(&settings, "missing").is_err());
    }
}
