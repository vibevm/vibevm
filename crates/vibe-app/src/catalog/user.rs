//! `<home>/applications/launchers.toml`, the user's own arrangement over the
//! launcher catalog (`PROP-059#launcher-catalog`): named ordered collections
//! of launcher keys, the ordered pinned lists, and the per-launcher overrides
//! of name, icon and visibility. Commands edit it so no client writes TOML;
//! this module is the only reader and the only writer.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog");

use std::fs;

use serde::{Deserialize, Serialize};
use specmark::spec;
use vibe_wire::generated::application_collections as wire;

use crate::error::StateError;
use crate::settings::{SettingsRoot, io, write_atomic};

/// The protocol tag of the user layer on disk, named in the file's header
/// comment so a human reading it knows which shape it is.
pub const USER_LAUNCHERS_PROTOCOL: &str = "vibe-app-user-launchers/1";

const HEADER: &str = "\
# vibe-app-user-launchers/1 — the user layer over the launcher catalog
# (spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog).
#
# vibe writes this file; edit it through `vibe app collection`, `vibe app pin`,
# `vibe app unpin` and `vibe app launcher` rather than by hand, because those
# commands refuse a key that names no launcher of this home and keep the
# revision counter beside it current.
#
# `[[collection]]` is a named, ordered list of launcher keys
# (`<group>/<name>@<version>#<launcher id>`). `seeded = true` marks a
# collection a package's `menu_group` created rather than the user; `removed`
# keeps the keys the user took out of it, so seeding never puts them back.
# `[pins]` holds the ordered taskbar and favourites lists. `[[launcher]]` is
# one override: `hidden` keeps a launcher out of a client's menus while it
# stays in the catalog, and `name` / `icon` survive update and reinstall.

";

/// Where a launcher key may be pinned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub enum PinTarget {
    Taskbar,
    Favorites,
}

impl PinTarget {
    /// The stable spelling (`taskbar`, `favorites`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Taskbar => "taskbar",
            Self::Favorites => "favorites",
        }
    }
}

/// One collection: a named, ordered list of launcher keys with an optional
/// icon.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub struct UserCollection {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Created by a package's `menu_group` rather than by the user; an empty
    /// seeded collection the user never touched is dropped on uninstall.
    #[serde(default, skip_serializing_if = "is_false")]
    pub seeded: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launchers: Vec<String>,
    /// Keys the user took out: seeding never re-adds one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
}

/// The ordered pinned lists.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub struct UserPins {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub taskbar: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub favorites: Vec<String>,
}

/// One per-launcher override.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub struct UserLauncher {
    pub key: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl UserLauncher {
    /// Whether the row still says anything; a row that says nothing is
    /// dropped instead of written.
    pub(crate) fn is_empty(&self) -> bool {
        !self.hidden && self.name.is_none() && self.icon.is_none()
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// The whole user layer of one home.
///
/// ```
/// use vibe_app::{SettingsRoot, UserLaunchers};
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = SettingsRoot::new(dir.path());
/// let mut layer = UserLaunchers::load(&settings).unwrap();
/// assert!(layer.collections.is_empty());
/// layer.collections.push(vibe_app::UserCollection {
///     name: "Games".into(),
///     launchers: vec!["org.example/doom@1.0.0#doom".into()],
///     ..Default::default()
/// });
/// layer.save(&settings).unwrap();
/// assert_eq!(UserLaunchers::load(&settings).unwrap(), layer);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub struct UserLaunchers {
    #[serde(default)]
    pub pins: UserPins,
    #[serde(default, rename = "collection", skip_serializing_if = "Vec::is_empty")]
    pub collections: Vec<UserCollection>,
    #[serde(default, rename = "launcher", skip_serializing_if = "Vec::is_empty")]
    pub launchers: Vec<UserLauncher>,
}

impl UserLaunchers {
    /// Read the layer; an absent file is an empty arrangement.
    pub fn load(settings: &SettingsRoot) -> Result<Self, StateError> {
        let path = settings.user_launchers_path();
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).map_err(|error| StateError::UserLaunchers {
                path: path.display().to_string(),
                reason: error.to_string(),
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(io("reading", &path, source)),
        }
    }

    /// Replace the layer atomically and increment the catalog revision; the
    /// caller holds the home's mutation lock.
    pub fn save(&self, settings: &SettingsRoot) -> Result<u64, StateError> {
        let path = settings.user_launchers_path();
        let body = toml::to_string(self).map_err(|error| StateError::UserLaunchers {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
        write_atomic(&path, format!("{HEADER}{body}").as_bytes())?;
        super::bump_revision(settings)
    }

    /// The collection of that name, by exact spelling.
    pub fn collection(&self, name: &str) -> Option<&UserCollection> {
        self.collections
            .iter()
            .find(|collection| collection.name == name)
    }

    pub(crate) fn collection_mut(&mut self, name: &str) -> Option<&mut UserCollection> {
        self.collections
            .iter_mut()
            .find(|collection| collection.name == name)
    }

    /// The names of the collections one launcher key sits in, in the order
    /// the collections are kept.
    pub fn collections_of(&self, key: &str) -> Vec<String> {
        self.collections
            .iter()
            .filter(|collection| collection.launchers.iter().any(|held| held == key))
            .map(|collection| collection.name.clone())
            .collect()
    }

    /// Where one launcher key is pinned.
    pub fn pins_of(&self, key: &str) -> Vec<PinTarget> {
        [
            (PinTarget::Taskbar, &self.pins.taskbar),
            (PinTarget::Favorites, &self.pins.favorites),
        ]
        .into_iter()
        .filter(|(_, list)| list.iter().any(|pinned| pinned == key))
        .map(|(target, _)| target)
        .collect()
    }

    pub(crate) fn pin_list_mut(&mut self, target: PinTarget) -> &mut Vec<String> {
        match target {
            PinTarget::Taskbar => &mut self.pins.taskbar,
            PinTarget::Favorites => &mut self.pins.favorites,
        }
    }

    /// The override of one launcher key, when the user made one.
    pub fn launcher(&self, key: &str) -> Option<&UserLauncher> {
        self.launchers.iter().find(|row| row.key == key)
    }

    /// Forget one launcher key everywhere: every collection, both pinned
    /// lists and its override row. Used when its installation goes away.
    pub(crate) fn forget(&mut self, keys: &[String]) {
        let gone = |key: &String| keys.contains(key);
        for collection in &mut self.collections {
            collection.launchers.retain(|key| !gone(key));
            collection.removed.retain(|key| !gone(key));
        }
        self.pins.taskbar.retain(|key| !gone(key));
        self.pins.favorites.retain(|key| !gone(key));
        self.launchers.retain(|row| !gone(&row.key));
    }

    /// The `vibe-app-collections/1` document of this layer.
    pub fn to_wire(&self, settings: &SettingsRoot) -> wire::ApplicationCollections {
        wire::ApplicationCollections {
            protocol: "vibe-app-collections/1".into(),
            home: settings.path().display().to_string(),
            revision: super::revision(settings),
            collections: self
                .collections
                .iter()
                .map(|collection| wire::Collection {
                    name: collection.name.clone(),
                    icon: collection.icon.clone(),
                    seeded: collection.seeded,
                    launchers: collection.launchers.clone(),
                    removed: collection.removed.clone(),
                })
                .collect(),
            pins: wire::Pins {
                taskbar: self.pins.taskbar.clone(),
                favorites: self.pins.favorites.clone(),
            },
            overrides: self
                .launchers
                .iter()
                .map(|row| wire::Override {
                    key: row.key.clone(),
                    hidden: row.hidden,
                    name: row.name.clone(),
                    icon: row.icon.clone(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOOM: &str = "org.example/doom@1.0.0#doom";
    const QUAKE: &str = "org.example/quake@1.0.0#quake";

    fn layer() -> UserLaunchers {
        UserLaunchers {
            pins: UserPins {
                taskbar: vec![DOOM.into(), QUAKE.into()],
                favorites: vec![QUAKE.into()],
            },
            collections: vec![UserCollection {
                name: "Games".into(),
                icon: Some("/icons/games.svg".into()),
                seeded: true,
                launchers: vec![DOOM.into(), QUAKE.into()],
                removed: vec!["org.example/doom@1.0.0#setup".into()],
            }],
            launchers: vec![UserLauncher {
                key: DOOM.into(),
                hidden: true,
                name: Some("DOOM".into()),
                icon: None,
            }],
        }
    }

    #[test]
    fn the_layer_round_trips_through_toml_with_its_header() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let layer = layer();
        assert_eq!(
            layer.save(&settings).unwrap(),
            1,
            "a save bumps the revision"
        );
        let text = fs::read_to_string(settings.user_launchers_path()).unwrap();
        assert!(
            text.starts_with(&format!("# {USER_LAUNCHERS_PROTOCOL} —")),
            "{text}"
        );
        assert!(
            text.contains("[[collection]]") && text.contains("[pins]"),
            "{text}"
        );
        assert_eq!(UserLaunchers::load(&settings).unwrap(), layer);

        // A row that says nothing is not worth keeping.
        assert!(
            UserLauncher {
                key: DOOM.into(),
                ..Default::default()
            }
            .is_empty()
        );
    }

    #[test]
    fn the_layer_answers_where_a_key_sits_and_forgets_it_whole() {
        let mut layer = layer();
        assert_eq!(layer.collections_of(DOOM), ["Games"]);
        assert_eq!(layer.pins_of(DOOM), [PinTarget::Taskbar]);
        assert_eq!(
            layer.pins_of(QUAKE),
            [PinTarget::Taskbar, PinTarget::Favorites]
        );
        assert!(layer.launcher(DOOM).is_some() && layer.launcher(QUAKE).is_none());

        layer.forget(&[DOOM.into(), "org.example/doom@1.0.0#setup".into()]);
        assert_eq!(layer.collection("Games").unwrap().launchers, [QUAKE]);
        assert!(layer.collection("Games").unwrap().removed.is_empty());
        assert_eq!(layer.pins.taskbar, [QUAKE]);
        assert!(layer.launchers.is_empty());
    }

    #[test]
    fn an_invalid_layer_is_a_refusal_and_an_absent_one_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        assert_eq!(
            UserLaunchers::load(&settings).unwrap(),
            UserLaunchers::default()
        );
        fs::create_dir_all(settings.applications_dir()).unwrap();
        fs::write(settings.user_launchers_path(), "[[collection]]\n").unwrap();
        let error = UserLaunchers::load(&settings).unwrap_err();
        assert_eq!(error.code(), "user_launchers_invalid");
    }
}
