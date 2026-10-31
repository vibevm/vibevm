//! Icon sets resolved to absolute paths, the `vibe app icon` choice and
//! the home's copies of package-relative icon files (`PROP-059#menu`).
//! Nothing here rasterises: a client gets the best file that exists.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#menu");

use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use specmark::spec;
use vibe_core::manifest::{IconDecl, IconSlot, png_dimensions};

/// One file of an icon set after resolution.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#menu")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedIconFile {
    /// `svg`, `png`, `ico`, `icns` or `symbolic`.
    pub slot: &'static str,
    /// A raster's pixel size: the key it is listed under, else its header.
    pub size: Option<u32>,
    pub template: String,
    /// The absolute path, when the file exists.
    pub path: Option<String>,
    /// Why there is no path.
    pub diagnostic: Option<String>,
}

/// An icon set with every file resolved.
///
/// ```
/// use vibe_app::ResolvedIcon;
/// use vibe_core::manifest::IconDecl;
///
/// let dir = tempfile::tempdir().unwrap();
/// std::fs::write(dir.path().join("app.svg"), "<svg/>").unwrap();
/// let icon = IconDecl::Path("app.svg".into());
/// let resolved = ResolvedIcon::resolve(&icon, |template| Ok(dir.path().join(template)));
/// assert_eq!(resolved.files[0].slot, "svg");
/// assert!(resolved.files[0].path.is_some());
/// assert_eq!(resolved.choose(48, None).unwrap().slot, "svg");
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#menu")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedIcon {
    pub maskable: bool,
    pub files: Vec<ResolvedIconFile>,
}

impl ResolvedIcon {
    /// Resolve every file of `icon` with `locate`, which turns a template
    /// into an existing absolute path or says why it cannot.
    pub fn resolve(icon: &IconDecl, locate: impl Fn(&str) -> Result<PathBuf, String>) -> Self {
        let files = icon
            .files()
            .into_iter()
            .map(|(slot, template)| {
                let located = locate(template).and_then(|path| {
                    if path.is_file() {
                        Ok(path)
                    } else {
                        Err(format!("icon `{}` does not exist", path.display()))
                    }
                });
                let size = match slot {
                    IconSlot::Png(0) => located
                        .as_ref()
                        .ok()
                        .and_then(|path| fs::read(path).ok())
                        .and_then(|bytes| png_dimensions(&bytes))
                        .map(|(width, _)| width),
                    IconSlot::Png(size) => Some(size),
                    _ => None,
                };
                let (path, diagnostic) = match located {
                    Ok(path) => (Some(path.display().to_string()), None),
                    Err(reason) => (None, Some(reason)),
                };
                ResolvedIconFile {
                    slot: slot.name(),
                    size,
                    template: template.to_owned(),
                    path,
                    diagnostic,
                }
            })
            .collect();
        Self {
            maskable: icon.maskable(),
            files,
        }
    }

    /// The best existing file for a need of `size` pixels: the preferred
    /// slot when it has a file, otherwise the smallest raster not smaller
    /// than `size`, else the largest raster, else the SVG, else the `ico`,
    /// `icns` or symbolic file. A preferred `png` follows the raster rule.
    pub fn choose(&self, size: u32, prefer: Option<&str>) -> Option<&ResolvedIconFile> {
        let existing = || self.files.iter().filter(|file| file.path.is_some());
        let of = |slot: &str| existing().find(|file| file.slot == slot);
        let raster = || {
            let mut rasters: Vec<&ResolvedIconFile> = existing()
                .filter(|file| file.slot == "png" && file.size.is_some())
                .collect();
            rasters.sort_by_key(|file| file.size);
            rasters
                .iter()
                .find(|file| file.size >= Some(size))
                .or(rasters.last())
                .copied()
        };
        let preferred = match prefer {
            Some("png") => raster(),
            Some(slot) => of(slot),
            None => None,
        };
        preferred
            .or_else(raster)
            .or_else(|| of("svg"))
            .or_else(|| of("ico"))
            .or_else(|| of("icns"))
            .or_else(|| of("symbolic"))
    }
}

#[path = "icons/copies.rs"]
mod copies;

pub(crate) use copies::{
    copies_dir, copy_package_icons, is_package_relative, join_relative, remove_package_icons,
};

/// Convert a serializable domain value into a generated wire type of the
/// same JSON shape; `None` only when the shapes disagree, which the unit
/// tests of every caller rule out.
pub(crate) fn wire_shape<T: serde::de::DeserializeOwned>(value: &impl Serialize) -> Option<T> {
    serde_json::to_value(value)
        .ok()
        .and_then(|json| serde_json::from_value(json).ok())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use vibe_core::manifest::IconDecl;
    use vibe_wire::generated::{application_describe, application_launchers, application_list};

    use super::*;

    fn png(size: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        bytes.extend(size.to_be_bytes());
        bytes.extend(size.to_be_bytes());
        bytes
    }

    /// A hicolor tree with 48 and 256 pixel rasters, a scalable and a
    /// symbolic SVG, collected and resolved under the payload root.
    #[test]
    fn a_hicolor_tree_resolves_and_chooses_by_size() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let theme = root.join("share/icons/hicolor");
        for size in [48, 256] {
            let apps = theme.join(format!("{size}x{size}/apps"));
            fs::create_dir_all(&apps).unwrap();
            fs::write(apps.join("edit.png"), png(size)).unwrap();
        }
        fs::create_dir_all(theme.join("scalable/apps")).unwrap();
        fs::write(theme.join("scalable/apps/edit.svg"), "<svg/>").unwrap();
        fs::create_dir_all(theme.join("symbolic/apps")).unwrap();
        fs::write(theme.join("symbolic/apps/edit-symbolic.svg"), "<svg/>").unwrap();
        let set = crate::derive::hicolor("edit", root).unwrap();
        let icon = IconDecl::Set(set);
        let resolved = ResolvedIcon::resolve(&icon, |template| {
            Ok(root.join(template.trim_start_matches("$app_root/")))
        });
        assert!(resolved.files.iter().all(|file| file.path.is_some()));
        let choose = |size: u32, prefer: Option<&str>| {
            let file = resolved.choose(size, prefer).unwrap();
            (file.slot, file.size)
        };
        assert_eq!(choose(32, None), ("png", Some(48)));
        assert_eq!(choose(48, None), ("png", Some(48)));
        assert_eq!(choose(100, None), ("png", Some(256)));
        assert_eq!(choose(1024, None), ("png", Some(256)));
        assert_eq!(choose(48, Some("svg")), ("svg", None));
        assert_eq!(choose(48, Some("symbolic")), ("symbolic", None));
        assert_eq!(choose(48, Some("ico")), ("png", Some(48)));
        let only_svg = IconDecl::Path("share/icons/hicolor/scalable/apps/edit.svg".into());
        let resolved = ResolvedIcon::resolve(&only_svg, |template| Ok(root.join(template)));
        assert_eq!(resolved.choose(16, None).unwrap().slot, "svg");
        let missing = IconDecl::Path("absent.png".into());
        let resolved = ResolvedIcon::resolve(&missing, |template| Ok(root.join(template)));
        assert!(resolved.files[0].diagnostic.is_some());
        assert!(resolved.choose(16, None).is_none());
    }

    /// The one domain shape converts into every generated icon set.
    #[test]
    fn the_resolved_shape_matches_every_document() {
        let icon = ResolvedIcon {
            maskable: true,
            files: vec![ResolvedIconFile {
                slot: "png",
                size: Some(48),
                template: "a.png".into(),
                path: Some("/x/a.png".into()),
                diagnostic: None,
            }],
        };
        let launchers: Option<application_launchers::IconSet> = wire_shape(&icon);
        let describe: Option<application_describe::IconSet> = wire_shape(&icon);
        let list: Option<application_list::IconSet> = wire_shape(&icon);
        assert_eq!(launchers.unwrap().files[0].size, Some(48));
        assert!(describe.unwrap().maskable);
        assert_eq!(list.unwrap().files[0].path.as_deref(), Some("/x/a.png"));
    }
}
