//! Where the card's three images are SERVED from (PROP-057
//! `##CARD-SITE-COPIES`, `##CARD-PLACEHOLDERS-GENERATED`).
//!
//! One rule with three readers: [`slots`] decides the address,
//! `vibe doc build` writes the file there, and the page manifest carries
//! it so the shell can show a picture without computing a name
//! (`##PIPE-SHELL-PARSES-NOTHING`). Three copies of the rule would drift
//! the day a hash changed length, and the symptom would be a picture that
//! silently stops loading — which is exactly the defect the deferral
//! X-042 was filed about.
//!
//! The names themselves are content names, and that is what makes them
//! cacheable forever: a declared image is named by its BYTES, a generated
//! placeholder by the COORDINATE it is derived from. Either way a changed
//! picture is a changed address.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#CARD-SITE-COPIES");

use std::path::Path;

use super::{ROLES, declared_media, generated_name, hashed_name};
use crate::error::{DocError, Result};

/// The directory the card's images are published into, relative to the
/// base the documentation is served under.
pub const PUBLISHED_DIR: &str = "media";

/// One role's published image: where it is served from, and the bytes
/// behind it when they came from a file the package ships.
///
/// This is the ONE place an image's published address is decided, and the
/// reason is the reason for the type: the build writes the file, the
/// manifest names it, and the shell shows what the manifest named
/// (`##PIPE-SHELL-PARSES-NOTHING`). Three readers of one rule is fine;
/// three copies of it would drift the day a hash changed length, and the
/// symptom would be a picture that silently stops loading.
#[derive(Debug, Clone)]
pub struct Slot {
    /// `icon`, `banner` or `preview`.
    pub role: &'static str,
    /// `media/<content name>`, relative to the base.
    pub address: String,
    /// The declared file's bytes, read once to take its content name.
    /// `None` when the role declared nothing and the picture is
    /// generated.
    pub bytes: Option<Vec<u8>>,
}

impl Slot {
    /// Was this role's picture generated rather than shipped?
    pub fn generated(&self) -> bool {
        self.bytes.is_none()
    }

    /// The bytes served at this slot's address: the file the package
    /// ships, or the placeholder drawn for a role that ships none
    /// (`##CARD-PLACEHOLDERS-GENERATED`).
    ///
    /// One rule with two readers, which is the reason it is a function
    /// and not a `match` written twice: a build writes these bytes into
    /// a tree and the local reader answers a request with them, and a
    /// picture that differed between the two would be a card that
    /// changes when you read it locally.
    ///
    /// ```
    /// use vibe_core::PackageKind;
    /// use vibe_doc::media;
    ///
    /// let tmp = tempfile::tempdir().unwrap();
    /// std::fs::write(tmp.path().join("vibe.toml"), "[package]\nname = \"a\"\n").unwrap();
    /// let slots = media::slots(tmp.path(), "com.example/a").unwrap();
    /// let icon = slots.iter().find(|slot| slot.role == "icon").unwrap();
    /// // Nothing was declared, so the picture is drawn and its address
    /// // is the coordinate's rather than any file's content name.
    /// assert!(icon.generated());
    /// let bytes = icon.render("com.example/a", PackageKind::Doc, "A");
    /// assert!(String::from_utf8_lossy(&bytes).starts_with("<svg"));
    /// ```
    pub fn render(&self, coordinate: &str, kind: vibe_core::PackageKind, title: &str) -> Vec<u8> {
        match &self.bytes {
            Some(bytes) => bytes.clone(),
            None => match self.role {
                "icon" => super::icon_svg(coordinate, kind).into_bytes(),
                "banner" => super::banner_svg(coordinate, kind).into_bytes(),
                _ => super::preview_png(coordinate, kind, title),
            },
        }
    }

    /// The extension a generated picture of this role carries.
    pub fn generated_extension(&self) -> &'static str {
        if self.role == "preview" { "png" } else { "svg" }
    }
}

/// Where each of the card's three images is published, in role order.
///
/// A declared image is named by its BYTES and a generated one by the
/// COORDINATE, which is why this reads the declared files: an address
/// that did not depend on the content could not be cached forever, and
/// caching forever is the whole point of a content name
/// (`##CARD-SITE-COPIES`).
pub fn slots(package_dir: &Path, coordinate: &str) -> Result<Vec<Slot>> {
    let declared = declared_media(package_dir)?;
    let mut out = Vec::with_capacity(ROLES.len());
    for role in ROLES {
        let found = declared.iter().find(|(name, _)| name == role);
        let slot = match found {
            Some((_, path)) => {
                let from = package_dir.join(path);
                let bytes = std::fs::read(&from).map_err(|e| DocError::io("reading", &from, e))?;
                let name = hashed_name(&bytes, path);
                Slot {
                    role,
                    address: format!("{PUBLISHED_DIR}/{name}"),
                    bytes: Some(bytes),
                }
            }
            None => {
                let extension = if role == "preview" { "png" } else { "svg" };
                Slot {
                    role,
                    address: format!(
                        "{PUBLISHED_DIR}/{}",
                        generated_name(coordinate, role, extension)
                    ),
                    bytes: None,
                }
            }
        };
        out.push(slot);
    }
    Ok(out)
}

/// The address of one role among `slots`, or the empty string when the
/// role is not there — which cannot happen for a set built by [`slots`]
/// and is not worth an option at every call site.
pub fn address_of(slots: &[Slot], role: &str) -> String {
    slots
        .iter()
        .find(|slot| slot.role == role)
        .map(|slot| slot.address.clone())
        .unwrap_or_default()
}
