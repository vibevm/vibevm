//! `vibe app outdated` (`PROP-065#outdated`): every installed application
//! beside the newest version the configured registries offer for the same
//! package, without changing anything; held installations are reported as
//! held, kept previous installations are left out.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#outdated");

use std::collections::BTreeMap;

use vibe_wire::generated::application_outdated as wire;

use crate::error::AppError;
use crate::index::{Index, PackageCoordinate};
use crate::settings::SettingsRoot;

/// Report every current installation of a home with the newest version
/// `latest` finds for its package (`group/name`, looked up once each).
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let report = vibe_app::outdated(&settings, &|_| Ok("1.0.0".into())).unwrap();
/// assert_eq!(report.protocol, "vibe-app-outdated/1");
/// assert!(report.installations.is_empty());
/// ```
pub fn outdated(
    settings: &SettingsRoot,
    latest: &dyn Fn(&PackageCoordinate) -> Result<String, String>,
) -> Result<wire::ApplicationOutdated, AppError> {
    let index = Index::load(settings)?;
    let mut found: BTreeMap<(String, String), Result<String, String>> = BTreeMap::new();
    let mut installations = Vec::new();
    for (key, record) in &index.installations {
        if record.lifecycle.superseded_by.is_some() {
            continue;
        }
        let package = &record.package;
        let newest = found
            .entry((package.group.clone(), package.name.clone()))
            .or_insert_with(|| latest(package))
            .clone();
        let held = record.lifecycle.held;
        let (latest, status, reason) = match newest {
            Err(reason) => (None, wire::Status::Unknown, Some(reason)),
            Ok(newest) => {
                let newer = match (
                    semver::Version::parse(&newest),
                    semver::Version::parse(&package.version),
                ) {
                    (Ok(newest), Ok(installed)) => newest > installed,
                    _ => newest != package.version,
                };
                let status = match (held, newer) {
                    (true, _) => wire::Status::Held,
                    (false, true) => wire::Status::Outdated,
                    (false, false) => wire::Status::Current,
                };
                (Some(newest), status, None)
            }
        };
        installations.push(wire::Installation {
            key: key.clone(),
            slot: record.slot.clone(),
            package: wire::Package {
                group: package.group.clone(),
                name: package.name.clone(),
                version: package.version.clone(),
            },
            installed: package.version.clone(),
            latest,
            status,
            held,
            hidden: record.lifecycle.hidden,
            reason,
        });
    }
    Ok(wire::ApplicationOutdated {
        protocol: "vibe-app-outdated/1".into(),
        installations,
    })
}
