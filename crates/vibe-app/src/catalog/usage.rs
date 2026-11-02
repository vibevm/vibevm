//! `<home>/applications/usage.json` (`PROP-059#launcher-catalog`): `starts`
//! and `last_started` per launcher key. It is its own file and not part of
//! the index, so starting a program never rewrites the inventory — the one
//! durable record an ordinary launch touches is the smallest one.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog");

use std::collections::BTreeMap;
use std::fs;

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::events::{EventKind, HomeEvent};
use crate::error::StateError;
use crate::settings::{SettingsRoot, io, write_atomic};

/// The protocol tag of the usage file.
pub(crate) const USAGE_PROTOCOL: &str = "vibe-app-usage/1";

/// What the home recorded about one launcher's starts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub struct LauncherUsage {
    #[serde(default)]
    pub starts: u64,
    /// RFC 3339 time of the newest start, or `None` while there is none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_started: Option<String>,
}

/// The usage facts of one home, by launcher key.
///
/// ```
/// use vibe_app::{SettingsRoot, Usage};
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = SettingsRoot::new(dir.path());
/// assert_eq!(Usage::load(&settings).unwrap().of("org.example/x@1.0.0#main").starts, 0);
/// Usage::record_start(&settings, "org.example/x@1.0.0#main").unwrap();
/// let usage = Usage::load(&settings).unwrap();
/// assert_eq!(usage.of("org.example/x@1.0.0#main").starts, 1);
/// assert!(usage.of("org.example/x@1.0.0#main").last_started.is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub struct Usage {
    pub protocol: String,
    /// Launcher key → its facts.
    #[serde(default)]
    pub launchers: BTreeMap<String, LauncherUsage>,
}

impl Default for Usage {
    fn default() -> Self {
        Self {
            protocol: USAGE_PROTOCOL.into(),
            launchers: BTreeMap::new(),
        }
    }
}

impl Usage {
    /// Read the usage facts; an absent or unreadable file is no facts at
    /// all. A count is not truth anything depends on, so a damaged file
    /// starts over instead of refusing every listing and every start.
    pub fn load(settings: &SettingsRoot) -> Result<Self, StateError> {
        let path = settings.usage_path();
        match fs::read(&path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or_default()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(io("reading", &path, source)),
        }
    }

    /// Replace the usage facts atomically and increment the catalog
    /// revision with `event`; the caller holds the home's mutation lock.
    pub fn save(&self, settings: &SettingsRoot, event: &HomeEvent) -> Result<u64, StateError> {
        self.write(settings)?;
        super::bump_revision(settings, event)
    }

    /// Replace the facts without a revision of their own, for a change that
    /// bumps once for several files.
    pub(crate) fn write(&self, settings: &SettingsRoot) -> Result<(), StateError> {
        let path = settings.usage_path();
        let mut bytes = serde_json::to_vec_pretty(self).map_err(|error| StateError::Index {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
        bytes.push(b'\n');
        write_atomic(&path, &bytes)
    }

    /// The facts of one launcher key; a key never started has none.
    pub fn of(&self, key: &str) -> LauncherUsage {
        self.launchers.get(key).cloned().unwrap_or_default()
    }

    /// Count one start of `key` now, under the home's mutation lock. The
    /// lock is taken and released here rather than held across the launch,
    /// so the started child never inherits it.
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
    pub fn record_start(settings: &SettingsRoot, key: &str) -> Result<LauncherUsage, StateError> {
        let _lock = settings.lock()?;
        let mut usage = Self::load(settings)?;
        let entry = usage.launchers.entry(key.to_owned()).or_default();
        entry.starts = entry.starts.saturating_add(1);
        entry.last_started =
            Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        let recorded = entry.clone();
        usage.save(settings, &HomeEvent::about(EventKind::Started, key))?;
        Ok(recorded)
    }

    /// Forget every named launcher key, so an uninstalled installation
    /// leaves no counts behind.
    pub(crate) fn forget(&mut self, keys: &[String]) {
        self.launchers.retain(|key, _| !keys.contains(key));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "org.example/editor@1.0.0#edit";

    /// The stored file only; what `record_start` counts is proved through
    /// the public API in `tests/user_layer.rs`, so it is not repeated here.
    #[test]
    fn a_saved_count_round_trips_a_damaged_file_starts_over_and_keys_are_forgotten() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let mut usage = Usage::default();
        usage.launchers.insert(
            KEY.into(),
            LauncherUsage {
                starts: 2,
                last_started: Some("2026-10-31T00:00:00Z".into()),
            },
        );
        assert_eq!(
            usage
                .save(&settings, &HomeEvent::new(EventKind::LaunchersChanged))
                .unwrap(),
            1,
            "a save bumps the revision"
        );

        let loaded = Usage::load(&settings).unwrap();
        assert_eq!(loaded.protocol, USAGE_PROTOCOL);
        assert_eq!(loaded.of(KEY).starts, 2);
        assert_eq!(
            loaded.of("org.example/other@1.0.0#main"),
            LauncherUsage::default()
        );

        fs::write(settings.usage_path(), b"{not json").unwrap();
        assert!(Usage::load(&settings).unwrap().launchers.is_empty());

        let mut usage = loaded;
        usage.forget(&[KEY.into()]);
        assert!(usage.launchers.is_empty());
    }
}
