//! The menu and desktop-entry vocabulary of a launcher (`PROP-059#menu`,
//! `#desktop-entries`): the explicit kind, icon sets, jump-list actions and
//! the closed category list.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#menu");

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use specmark::spec;

/// The freedesktop main categories, the closed vocabulary of `categories`.
pub const MENU_CATEGORIES: [&str; 13] = [
    "AudioVideo",
    "Audio",
    "Video",
    "Development",
    "Education",
    "Game",
    "Graphics",
    "Network",
    "Office",
    "Science",
    "Settings",
    "System",
    "Utility",
];

/// The placeholders a launcher's `args` may carry for what a client opens
/// with it (`%f`, `%F`, `%u`, `%U` of a desktop entry).
pub const ARG_PLACEHOLDERS: [&str; 4] = ["{file}", "{files}", "{url}", "{urls}"];

/// What a launcher opens: a window of its own, a full-screen terminal
/// program, or a console program that reads arguments and prints.
///
/// ```
/// use vibe_core::manifest::LauncherKind;
///
/// assert!(!LauncherKind::Gui.terminal());
/// assert!(LauncherKind::Tui.terminal() && LauncherKind::Command.terminal());
/// assert_eq!(LauncherKind::from_terminal(true), LauncherKind::Tui);
/// assert_eq!(LauncherKind::Command.as_str(), "command");
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launch-points")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LauncherKind {
    Gui,
    Tui,
    Command,
}

impl LauncherKind {
    /// The `terminal` value the kind implies.
    pub fn terminal(self) -> bool {
        !matches!(self, Self::Gui)
    }

    /// The kind an index record written before kinds existed is read as:
    /// `tui` for a terminal launcher, otherwise `gui`.
    pub fn from_terminal(terminal: bool) -> Self {
        if terminal { Self::Tui } else { Self::Gui }
    }

    /// The stable spelling (`gui`, `tui`, `command`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gui => "gui",
            Self::Tui => "tui",
            Self::Command => "command",
        }
    }
}

/// Where an installed launcher row came from: the package author's
/// `[[installer_options.launchers]]`, or a desktop entry read through
/// `launchers_from` (`PROP-059#desktop-entries`). Never parsed from a
/// manifest.
///
/// ```
/// use vibe_core::manifest::LauncherSource;
///
/// assert_eq!(LauncherSource::default(), LauncherSource::Authored);
/// assert_eq!(LauncherSource::DesktopEntry.as_str(), "desktop-entry");
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#desktop-entries")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LauncherSource {
    #[default]
    Authored,
    DesktopEntry,
}

impl LauncherSource {
    /// The stable spelling (`authored`, `desktop-entry`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Authored => "authored",
            Self::DesktopEntry => "desktop-entry",
        }
    }
}

/// The icon set of `#menu`: the scalable source, rasters keyed by their
/// decimal pixel size, ready containers, a monochrome variant and the
/// maskable flag. Every value is a path template like a launcher's
/// `executable`.
///
/// ```
/// use vibe_core::manifest::{IconSet, IconSlot};
///
/// let mut set = IconSet::default();
/// set.svg = Some("share/app.svg".into());
/// set.png.insert("48".into(), "share/48.png".into());
/// let slots: Vec<IconSlot> = set.files().into_iter().map(|(slot, _)| slot).collect();
/// assert_eq!(slots, [IconSlot::Svg, IconSlot::Png(48)]);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#menu")]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IconSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub svg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ico: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icns: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbolic: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub maskable: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub png: BTreeMap<String, String>,
}

/// One file of an icon set: which slot it fills.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#menu")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IconSlot {
    Svg,
    /// A raster of this many pixels square; `0` for a short-form PNG whose
    /// size is known only from its header.
    Png(u32),
    Ico,
    Icns,
    Symbolic,
}

impl IconSlot {
    /// The slot's name as `--prefer` and the JSON documents spell it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png(_) => "png",
            Self::Ico => "ico",
            Self::Icns => "icns",
            Self::Symbolic => "symbolic",
        }
    }
}

impl IconSet {
    /// Every named file with its slot: the SVG, rasters by ascending size,
    /// then `ico`, `icns` and `symbolic`. A raster key that is not a
    /// decimal size is listed as size `0`; validation refuses it.
    pub fn files(&self) -> Vec<(IconSlot, &str)> {
        let mut out = Vec::new();
        if let Some(svg) = &self.svg {
            out.push((IconSlot::Svg, svg.as_str()));
        }
        let mut rasters: Vec<(u32, &str)> = self
            .png
            .iter()
            .map(|(size, path)| (size.parse().unwrap_or(0), path.as_str()))
            .collect();
        rasters.sort();
        out.extend(
            rasters
                .into_iter()
                .map(|(size, path)| (IconSlot::Png(size), path)),
        );
        for (slot, value) in [
            (IconSlot::Ico, &self.ico),
            (IconSlot::Icns, &self.icns),
            (IconSlot::Symbolic, &self.symbolic),
        ] {
            if let Some(path) = value {
                out.push((slot, path.as_str()));
            }
        }
        out
    }

    /// Whether nothing is named.
    pub fn is_empty(&self) -> bool {
        self.files().is_empty()
    }
}

/// An icon: one path (a PNG or SVG, the short form) or an icon set.
///
/// ```
/// use vibe_core::manifest::{IconDecl, IconSlot};
///
/// let short: IconDecl = toml::from_str::<toml::Table>("icon = \"share/app.svg\"")
///     .unwrap()["icon"].clone().try_into().unwrap();
/// assert_eq!(short.files(), [(IconSlot::Svg, "share/app.svg")]);
/// let set: IconDecl = toml::from_str::<toml::Table>("icon = { ico = \"app.ico\" }")
///     .unwrap()["icon"].clone().try_into().unwrap();
/// assert_eq!(set.files(), [(IconSlot::Ico, "app.ico")]);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#menu")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconDecl {
    Path(String),
    Set(IconSet),
}

impl IconDecl {
    /// The named files with their slots; a short-form path is an SVG when
    /// it ends in `.svg`, otherwise a PNG of unknown size.
    pub fn files(&self) -> Vec<(IconSlot, &str)> {
        match self {
            Self::Path(path) if path.is_empty() => Vec::new(),
            Self::Path(path) if path.to_ascii_lowercase().ends_with(".svg") => {
                vec![(IconSlot::Svg, path.as_str())]
            }
            Self::Path(path) => vec![(IconSlot::Png(0), path.as_str())],
            Self::Set(set) => set.files(),
        }
    }

    /// The `maskable` flag of a set; a short form is never maskable.
    pub fn maskable(&self) -> bool {
        matches!(self, Self::Set(set) if set.maskable)
    }
}

impl Serialize for IconDecl {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Path(path) => serializer.serialize_str(path),
            Self::Set(set) => set.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for IconDecl {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct IconVisitor;
        impl<'de> Visitor<'de> for IconVisitor {
            type Value = IconDecl;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an icon path or an icon set table")
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<IconDecl, E> {
                Ok(IconDecl::Path(value.to_owned()))
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<IconDecl, A::Error> {
                IconSet::deserialize(de::value::MapAccessDeserializer::new(map)).map(IconDecl::Set)
            }
        }
        deserializer.deserialize_any(IconVisitor)
    }
}

/// One `[[installer_options.launchers.actions]]` row: a jump-list entry
/// (`[Desktop Action …]`) running the launcher's executable with its own
/// arguments.
///
/// ```
/// use vibe_core::manifest::LauncherActionDecl;
///
/// let action: LauncherActionDecl =
///     toml::from_str("id = \"new-window\"\nname = \"New Window\"\nargs = [\"--new\"]").unwrap();
/// assert_eq!(action.args, ["--new"]);
/// assert!(action.icon.is_none());
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#desktop-entries")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LauncherActionDecl {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconDecl>,
}

/// The width and height a PNG header declares, or `None` when the bytes do
/// not start with a PNG signature and an `IHDR` chunk.
///
/// ```
/// use vibe_core::manifest::png_dimensions;
///
/// let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
/// png.extend(48u32.to_be_bytes());
/// png.extend(48u32.to_be_bytes());
/// assert_eq!(png_dimensions(&png), Some((48, 48)));
/// assert_eq!(png_dimensions(b"GIF89a"), None);
/// ```
pub fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 24 || &bytes[..8] != SIGNATURE || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let word =
        |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    Some((word(16), word(20)))
}
