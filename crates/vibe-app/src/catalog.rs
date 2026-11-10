//! The launcher catalog (`PROP-059#launcher-catalog`): the stable launcher
//! key, the revision counter a client polls instead of refetching the whole
//! document, the user layer of collections, pins and overrides, the usage
//! facts, and the detached start.
//!
//! The catalog itself is the listing of `launch/listing.rs`; this module owns
//! everything the listing reads beside the index — the three files
//! `applications/launchers.toml`, `applications/revision` and
//! `applications/usage.json` — so each of them is written in exactly one
//! place, atomically, under the home's mutation lock.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog");

use std::fs;

use specmark::spec;
use vibe_wire::generated::application_revision as wire;

use crate::error::StateError;
use crate::index::Index;
use crate::settings::{SettingsRoot, io, write_atomic};

#[path = "catalog/carry.rs"]
mod carry;
#[path = "catalog/edit.rs"]
mod edit;
#[path = "catalog/events.rs"]
mod events;
#[path = "catalog/running.rs"]
mod running;
#[path = "catalog/seed.rs"]
mod seed;
#[path = "catalog/start.rs"]
mod start;
#[path = "catalog/usage.rs"]
mod usage;
#[path = "catalog/user.rs"]
mod user;
#[path = "catalog/view.rs"]
mod view;
#[path = "catalog/watch.rs"]
mod watch;

pub(crate) use carry::{carry_over, carry_over_ids};
pub use edit::{
    LauncherChange, PinTarget, collection_add, collection_create, collection_delete,
    collection_order, collection_remove, collection_rename, collections, launcher_override, pin,
    unpin,
};
pub use events::{
    ApplicationEvent, EVENTS_PROTOCOL, EventKind, EventTail, HomeEvent, record_event,
};
pub use running::{ProcessState, ProcessTable, RUNNING_PROTOCOL, running, stop as stop_launcher};
pub use seed::{drop_package, seed_package};
pub use start::{StartRequest, start};
pub use usage::{LauncherUsage, Usage};
pub use user::{USER_LAUNCHERS_PROTOCOL, UserCollection, UserLauncher, UserLaunchers, UserPins};
pub use view::LauncherCatalog;
pub use watch::{WatchRequest, watch};

/// The separator between a package key and a launcher id in a launcher key.
pub const LAUNCHER_KEY_SEPARATOR: char = '#';

/// The stable key of one launch point: `<package key>#<launcher id>`, the
/// package key exactly as the index spells it.
///
/// ```
/// use vibe_app::{launcher_key, split_launcher_key};
///
/// let key = launcher_key("org.vibevm.apps/vim@1.1.2", "gvim");
/// assert_eq!(key, "org.vibevm.apps/vim@1.1.2#gvim");
/// assert_eq!(split_launcher_key(&key), Some(("org.vibevm.apps/vim@1.1.2", "gvim")));
/// assert_eq!(split_launcher_key("org.vibevm.apps/vim@1.1.2"), None);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn launcher_key(package_key: &str, launcher_id: &str) -> String {
    format!("{package_key}{LAUNCHER_KEY_SEPARATOR}{launcher_id}")
}

/// The package key and launcher id of a launcher key, or `None` when the
/// spelling carries no launcher part. A launcher id never holds a `#`, so the
/// first separator is the split.
pub fn split_launcher_key(key: &str) -> Option<(&str, &str)> {
    key.split_once(LAUNCHER_KEY_SEPARATOR)
        .filter(|(package, launcher)| !package.is_empty() && !launcher.is_empty())
}

/// Whether `key` names a launcher of an installation this home records.
/// Exposure and availability do not enter it: the user layer is an
/// arrangement, and a hidden or detached installation still has one.
pub(crate) fn names_a_launcher(index: &Index, key: &str) -> bool {
    let Some((package, launcher)) = split_launcher_key(key) else {
        return false;
    };
    index
        .installations
        .get(package)
        .is_some_and(|record| record.launchers.iter().any(|row| row.id == launcher))
}

/// The current catalog revision of a home; a home that never changed is `0`.
/// An unreadable or malformed counter reads as `0` rather than refusing a
/// listing: the number is a change hint, and a client that sees it go
/// backwards refetches, which is what it would do anyway.
pub fn revision(settings: &SettingsRoot) -> u64 {
    fs::read_to_string(settings.revision_path())
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
}

/// Increment the revision of a home, append `event` to its journal at the
/// new revision and return the new value. Every change to `index.json`,
/// `launchers.toml` or `usage.json` calls this, so a client comparing one
/// number never misses one of the three (`PROP-059#launcher-catalog`) and
/// `vibe app watch` sees each of them as one line
/// (`PROP-059#launcher-runtime`). The caller holds the home's mutation lock.
///
/// ```
/// use vibe_app::{EventKind, HomeEvent};
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let changed = HomeEvent::new(EventKind::CollectionsChanged);
/// assert_eq!(vibe_app::catalog_revision(&settings), 0);
/// assert_eq!(vibe_app::bump_revision(&settings, &changed).unwrap(), 1);
/// assert_eq!(vibe_app::bump_revision(&settings, &changed).unwrap(), 2);
/// assert_eq!(vibe_app::catalog_revision(&settings), 2);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub fn bump_revision(settings: &SettingsRoot, event: &HomeEvent) -> Result<u64, StateError> {
    let next = revision(settings).saturating_add(1);
    let path = settings.revision_path();
    let dir = settings.applications_dir();
    fs::create_dir_all(&dir).map_err(|source| io("creating", &dir, source))?;
    write_atomic(&path, format!("{next}\n").as_bytes())?;
    events::append(settings, event, next)?;
    Ok(next)
}

/// `vibe app launchers --revision`: the counter alone.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let answer = vibe_app::revision_document(&settings);
/// assert_eq!(answer.protocol, "vibe-app-revision/1");
/// assert_eq!(answer.revision, 0);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn revision_document(settings: &SettingsRoot) -> wire::ApplicationRevision {
    wire::ApplicationRevision {
        protocol: "vibe-app-revision/1".into(),
        revision: revision(settings),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{AuthoredCommands, Installation, Location, Method, PackageCoordinate};
    use vibe_core::manifest::{LauncherDecl, LauncherKind};

    /// An inventory of one installation with the launchers `edit` and
    /// `tool`, in memory: nothing here reaches a disk.
    fn index() -> Index {
        let package = PackageCoordinate {
            group: "org.example".into(),
            name: "editor".into(),
            version: "1.0.0".into(),
        };
        let launcher = |id: &str, kind| LauncherDecl {
            id: id.into(),
            name: id.into(),
            executable: format!("$app_bin/{id}"),
            kind: Some(kind),
            ..LauncherDecl::default()
        };
        let mut index = Index::default();
        index.installations.insert(
            package.key(),
            Installation {
                source_evidence: None,
                secret_origins: Default::default(),
                configuration: None,
                pending: None,
                slot: "editor".into(),
                package: package.clone(),
                option: "main".into(),
                platform: "linux".into(),
                arch: "x86_64".into(),
                method: Method::Zip,
                location: Location::Owned {
                    library: "default".into(),
                    directory: "apps/editor/org.example.editor@1.0.0".into(),
                },
                portable: true,
                launchers: vec![
                    launcher("edit", LauncherKind::Gui),
                    launcher("tool", LauncherKind::Command),
                ],
                public_commands: Vec::new(),
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
                lifecycle: Default::default(),
            },
        );
        index.set_default("editor", Some(package.key()));
        index
    }

    #[test]
    fn the_index_answers_which_keys_name_a_launcher() {
        let index = index();
        assert!(names_a_launcher(&index, "org.example/editor@1.0.0#edit"));
        assert!(names_a_launcher(&index, "org.example/editor@1.0.0#tool"));
        assert!(!names_a_launcher(&index, "org.example/editor@1.0.0#ghost"));
        assert!(!names_a_launcher(&index, "org.example/editor@1.0.0"));
        assert!(!names_a_launcher(&index, "org.example/absent@1.0.0#main"));
    }

    #[test]
    fn a_launcher_key_round_trips_and_an_unreadable_revision_reads_as_zero() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        assert_eq!(revision(&settings), 0);
        fs::create_dir_all(settings.applications_dir()).unwrap();
        fs::write(settings.revision_path(), "not a number").unwrap();
        assert_eq!(revision(&settings), 0, "a change hint never refuses");
        let changed = HomeEvent::new(EventKind::CollectionsChanged);
        assert_eq!(bump_revision(&settings, &changed).unwrap(), 1);
        assert_eq!(revision_document(&settings).revision, 1);

        let key = launcher_key("org.example/editor@1.0.0", "edit");
        assert_eq!(
            split_launcher_key(&key).unwrap(),
            ("org.example/editor@1.0.0", "edit")
        );
        assert_eq!(split_launcher_key("#edit"), None);
        assert_eq!(split_launcher_key("org.example/editor@1.0.0#"), None);
    }
}
