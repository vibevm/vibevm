//! Slot alternatives: every installation of a slot is a provider, each slot
//! has one global default, and the public commands of all providers in a slot
//! are reserved for that slot alone.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#alternatives");

use std::collections::{BTreeMap, BTreeSet};

use specmark::spec;
use vibe_core::manifest::PublicCommandDecl;
use vibe_wire::generated::application_alternatives as wire;

use crate::error::AppError;
use crate::index::{Index, Installation};
use crate::launch;
use crate::proxies;
use crate::settings::SettingsRoot;
use crate::storage::Storage;

/// Every exposed command name with the slot that reserves it; a hidden
/// installation (`PROP-059#hidden-installations`) has no proxy.
///
/// ```
/// let index = vibe_app::Index::default();
/// assert!(vibe_app::reserved_commands(&index).is_empty());
/// ```
pub fn reserved_commands(index: &Index) -> BTreeMap<String, String> {
    let mut reserved = BTreeMap::new();
    for record in index
        .installations
        .values()
        .filter(|record| !record.lifecycle.hidden)
        // A slot switched off publishes nothing: its commands lose their
        // proxies, so a lookup reaches the system program again
        // (`PROP-059#slot-off`). The reservation itself is kept in the
        // index, so turning the slot back on restores exactly it.
        .filter(|record| !index.is_off(&record.slot))
    {
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
/// records named in `replacing` are about to be replaced and do not count;
/// hidden installations count, so exposing one later never collides.
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
    /// Whether the slot is switched off after the change
    /// (`PROP-059#slot-off`).
    pub off: bool,
}

/// Make `key` the global default of `slot`, then synchronise the proxies.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let image = dir.path().join("vibe-app-proxy");
/// let exposure = vibe_app::SlotExposure::image_only(&image);
/// let error = vibe_app::set_default(&settings, "tool", "org.example/a@1.0.0", exposure);
/// assert!(error.unwrap_err().to_string().contains("no installed provider"));
/// ```
pub fn set_default(
    settings: &SettingsRoot,
    slot: &str,
    key: &str,
    exposure: crate::SlotExposure<'_>,
) -> Result<DefaultChange, AppError> {
    let held = settings.lock()?;
    let lock = &held;
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
    index.set_default(slot, Some(key.to_owned()));
    // Naming a provider is also the "on" switch: the user asked for this
    // provider to be what the slot resolves to (`PROP-059#slot-off`).
    index.slot_mut(slot).off = false;
    index.save_locked(
        settings,
        &crate::catalog::HomeEvent::about(crate::catalog::EventKind::Updated, key),
        lock,
    )?;
    proxies::sync_locked(
        settings,
        &reserved,
        &reserved_names(&index),
        exposure.proxy_image,
        lock,
    )?;
    crate::paths::apply_locked(
        settings,
        &mut index,
        &Storage::load(settings)?,
        exposure.environment,
        lock,
    )?;
    Ok(DefaultChange {
        slot: slot.to_owned(),
        previous,
        default: key.to_owned(),
        off: false,
    })
}

/// Switch `slot` off or back on (`PROP-059#slot-off`): off removes the
/// slot's links and the proxies of its public commands, so every lookup
/// falls through to the system program, and on restores both from the
/// remembered default. Installations, index records and that default are
/// kept either way.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let image = dir.path().join("vibe-app-proxy");
/// let exposure = vibe_app::SlotExposure::image_only(&image);
/// let error = vibe_app::set_slot_off(&settings, "tool", true, exposure);
/// assert_eq!(error.unwrap_err().code(), "unknown_slot");
/// ```
pub fn set_slot_off(
    settings: &SettingsRoot,
    slot: &str,
    off: bool,
    exposure: crate::SlotExposure<'_>,
) -> Result<DefaultChange, AppError> {
    let held = settings.lock()?;
    let lock = &held;
    let mut index = Index::load(settings)?;
    // An unknown slot is named, never invented.
    providers(&index, slot)?;
    let previous = index.default_of(slot).map(str::to_owned);
    let reserved = reserved_names(&index);
    index.slot_mut(slot).off = off;
    index.save_locked(
        settings,
        &crate::catalog::HomeEvent::about(crate::catalog::EventKind::Updated, slot),
        lock,
    )?;
    proxies::sync_locked(
        settings,
        &reserved,
        &reserved_names(&index),
        exposure.proxy_image,
        lock,
    )?;
    crate::paths::apply_locked(
        settings,
        &mut index,
        &Storage::load(settings)?,
        exposure.environment,
        lock,
    )?;
    Ok(DefaultChange {
        slot: slot.to_owned(),
        default: index.default_of(slot).unwrap_or_default().to_owned(),
        previous,
        off,
    })
}

/// The `vibe-app-alternatives/1` listing of one slot: providers with their
/// status, the default, and which providers offer each command. Hidden
/// installations have no place among the alternatives.
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
                path_dirs: record.path_dirs.clone(),
            }
        })
        .collect();
    Ok(wire::ApplicationAlternatives {
        protocol: "vibe-app-alternatives/1".into(),
        slot: slot.to_owned(),
        // Filled by the caller that wrote one (PROP-075
        // `#VERSION-FILE-WRITE`); a listing writes nothing.
        version_file: None,
        off: index.is_off(slot),
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
        .filter(|(_, record)| record.slot == slot && !record.lifecycle.hidden)
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
            source_evidence: None,
            secret_origins: Default::default(),
            configuration: None,
            pending: None,
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
            path_dirs: Vec::new(),
            version_file: Vec::new(),
            template: None,
            migrations: Vec::new(),
            migrated: Vec::new(),
            commands: AuthoredCommands::default(),
            features: Vec::new(),
            parameters: Default::default(),
            license: None,
            icon: None,
            menu: Default::default(),
            steps: Vec::new(),
            lifecycle: crate::index::Lifecycle::default(),
        }
    }

    fn index(records: &[Installation]) -> Index {
        let mut index = Index::default();
        for record in records {
            index
                .installations
                .insert(record.package.key(), record.clone());
            index.slots.entry(record.slot.clone()).or_default();
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
        start.set_default("tool", Some("org.example/alpha@1.0.0".into()));
        start
            .save(
                &settings,
                &crate::catalog::HomeEvent::new(crate::catalog::EventKind::Updated),
            )
            .unwrap();

        let exposure = crate::SlotExposure::image_only(&image);
        let change = set_default(&settings, "tool", "org.example/beta@1.0.0", exposure).unwrap();
        assert_eq!(change.previous.as_deref(), Some("org.example/alpha@1.0.0"));
        let saved = Index::load(&settings).unwrap();
        assert_eq!(saved.default_of("tool"), Some("org.example/beta@1.0.0"));
        assert!(
            settings
                .proxy_dir()
                .join(proxies::proxy_file_name("tool"))
                .is_file()
        );
        // A second mutation right after the first: a provider of another
        // slot is refused and the default stays where the switch put it.
        assert!(matches!(
            set_default(&settings, "tool", "org.example/gamma@1.0.0", exposure),
            Err(AppError::NotProvider { .. })
        ));
        assert_eq!(
            Index::load(&settings).unwrap().default_of("tool"),
            Some("org.example/beta@1.0.0")
        );

        let listed = listing(&settings, "tool").unwrap();
        assert_eq!(listed.default.as_deref(), Some("org.example/beta@1.0.0"));
        assert_eq!(listed.providers.len(), 2);
        assert!(listed.providers[1].default);
        assert_eq!(listed.commands.len(), 1);
        assert_eq!(listed.commands[0].providers.len(), 2);
        assert!(listing(&settings, "missing").is_err());
    }
}
