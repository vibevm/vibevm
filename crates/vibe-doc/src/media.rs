//! The card's images: what an author supplies, what the build generates
//! when they supply nothing, and what the site publishes (PROP-057 §7,
//! design decision D-20, campaign rule R-19).
//!
//! Three roles and no more. `icon` heads the package page and sits on
//! shelf cards; `banner` heads the package page; `preview` is the link
//! card — `og:image`, `twitter:image`, JSON-LD's `image`
//! (`##CARD-MEDIA-ROLES`). Their proportions are incompatible on
//! purpose, which is why a preview is its own field and never a crop of
//! the banner.
//!
//! ## One home for what D-20 allows
//!
//! Every judgment about a declared image — is it there, is it what it
//! claims to be, does it fit the shape it will be shown in, is it inside
//! the byte ceiling — is made HERE and nowhere else. `vibe check`'s
//! `doc-media` cell renders these findings into a linter report and
//! `vibe doc check --media` prints them; neither restates a limit. Two
//! copies of a byte ceiling is the rot this tree has already paid for
//! once, when a scanner hand-duplicated a manifest parser and the copy
//! went quietly stale.
//!
//! Every judgment is made from the file's FIRST BYTES, never from its
//! name. An extension is an author's assertion; a magic number is the
//! file. The local reader serves a proprietary package's images exactly
//! as they are, so `icon.png` holding an SVG would be a script served
//! under a picture's name — and SVG is refused outright in this wave for
//! that reason (`##CARD-MEDIA-SOURCE`, R-19).
//!
//! ## Nothing declared is not nothing shown
//!
//! A package that declares no image is not a package without one: the
//! banner and the icon are GENERATED from the hash of its coordinate and
//! the glyph of its kind, so two packages look different, one package
//! looks the same on the site and in the local reader, and no file is
//! stored for either (`##CARD-PLACEHOLDERS-GENERATED`). Those are inline
//! SVG at render time — which is not a contradiction of the rule above:
//! the ban is on SVG a package SHIPS and a reader is served verbatim,
//! and this markup is composed here from numbers, never carried.
//!
//! The preview is different, and it has to be: a link card is fetched by
//! a crawler that will not run a renderer, so it is a raster composed at
//! build time from the placeholder, the icon and the title
//! (`##CARD-PREVIEW-COMPOSED`). It is written by [`png`], whose whole
//! design is that the same inputs are the same bytes.
//!
//! ## What publishing does
//!
//! [`publish`] copies each declared image under a name taken from its
//! content, so a cache may keep it forever and a changed picture is a
//! changed address (`##CARD-SITE-COPIES`). It re-encodes nothing: the
//! bytes the author committed are the bytes the reader receives, which
//! is why checking the source package is checking the published one.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE");

pub(crate) mod image;
pub mod placeholder;
pub mod png;

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{DocError, Result};
use image::{Format, Probe};

pub use placeholder::{Card, banner_svg, icon_svg, preview_png};

/// The three roles a card knows, in the order D-20 lists them.
pub const ROLES: [&str; 3] = ["icon", "banner", "preview"];

/// How far a ratio may drift from its target before it stops being that
/// shape. One percent — 1200×630 is 1.9048, not 1.91, and rejecting the
/// recommendation D-20 itself prints would be absurd.
const RATIO_TOLERANCE: f64 = 0.01;

/// What one role is allowed to be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// Target aspect ratio (width / height), or `None` when the role is
    /// square.
    pub ratio: Option<f64>,
    /// Byte ceiling. Small on purpose: packages of ordinary kinds are
    /// materialised and committed in consumers' trees, so an image
    /// nobody asked for is paid for by everyone downstream.
    pub max_bytes: u64,
    /// Inclusive side range, for the one role D-20 bounds in pixels.
    pub side_range: Option<(u32, u32)>,
}

/// The limits D-20 sets, by role name. An unknown name cannot reach a
/// caller that walks [`ROLES`] or the manifest's own three fields; it
/// answers with the preview's limits rather than panicking, because a
/// linter that aborts tells an author nothing.
pub fn limits(role: &str) -> Limits {
    match role {
        "icon" => Limits {
            ratio: None,
            max_bytes: 256 * 1024,
            side_range: Some((256, 1024)),
        },
        "banner" => Limits {
            ratio: Some(3.0),
            max_bytes: 1024 * 1024,
            side_range: None,
        },
        // `preview`, and the only remaining role.
        _ => Limits {
            ratio: Some(1.91),
            max_bytes: 1024 * 1024,
            side_range: None,
        },
    }
}

/// How loudly a finding speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The card cannot be served as declared.
    Error,
    /// The bytes are a picture and something about them is unverified.
    Warning,
}

/// One thing wrong (or unverifiable) about one declared image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// `icon`, `banner` or `preview`.
    pub role: String,
    /// The image's package-relative path, forward-slashed.
    pub path: String,
    pub severity: Severity,
    /// The whole message, ending in the `spec://` it enforces and a
    /// `fix:` — the shape every refusal in this crate takes.
    pub message: String,
}

/// One `--media` run.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// The roles the manifest declared, in [`ROLES`] order.
    pub declared: Vec<String>,
    pub findings: Vec<Finding>,
}

impl Report {
    /// Green when nothing is an error. A warning is a measurement this
    /// check could not make, not a defect it found.
    pub fn ok(&self) -> bool {
        !self.findings.iter().any(|f| f.severity == Severity::Error)
    }

    pub fn errors(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count()
    }

    pub fn warnings(&self) -> usize {
        self.findings.len() - self.errors()
    }

    /// The human form.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for finding in &self.findings {
            let word = match finding.severity {
                Severity::Error => "MEDIA",
                Severity::Warning => "media",
            };
            out.push_str(&format!(
                "  {word} {} `{}`\n    {}\n",
                finding.role, finding.path, finding.message
            ));
        }
        // The generated half is named as a count rather than left
        // silent: «0 declared» and «3 declared, all fine» print
        // differently, and an author who expected an icon to be picked
        // up needs to see which of the two happened.
        out.push_str(&format!(
            "media: {} declared ({}), {} error(s), {} warning(s), {} role(s) generated\n",
            self.declared.len(),
            if self.declared.is_empty() {
                "none".to_string()
            } else {
                self.declared.join(", ")
            },
            self.errors(),
            self.warnings(),
            ROLES.len() - self.declared.len()
        ));
        out
    }
}

/// Check the card images of the package at `package_dir`.
///
/// The manifest is read as TOML data, for the reason the rest of this
/// crate reads one that way: the question is «what does this package say
/// about itself», and a strict parse would fail over a field this
/// library never looks at.
pub fn check(package_dir: &Path) -> Result<Report> {
    let mut report = Report::default();
    for (role, declared) in declared_media(package_dir)? {
        report.declared.push(role.clone());
        report.findings.extend(judge(package_dir, &role, &declared));
    }
    Ok(report)
}

/// The `[media]` table as declarations: `(role, package-relative path)`,
/// in [`ROLES`] order. An absent table and an empty one are one case.
pub fn declared_media(package_dir: &Path) -> Result<Vec<(String, String)>> {
    let path = package_dir.join("vibe.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let value: toml::Value = toml::from_str(&text)
        .map_err(|e| DocError::manifest(&path, format!("is not readable as TOML: {e}")))?;
    let Some(media) = value.get("media").and_then(toml::Value::as_table) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for role in ROLES {
        if let Some(declared) = media.get(role).and_then(toml::Value::as_str) {
            out.push((role.to_string(), declared.replace('\\', "/")));
        }
    }
    Ok(out)
}

/// Judge one declared image, from «is it there» to «does it fit».
///
/// Each question that fails ends this image's checking: a file that is
/// not there has no signature, and a file that is not a picture has no
/// shape. Reporting the rest would be inventing measurements.
pub fn judge(package_dir: &Path, role: &str, declared: &str) -> Vec<Finding> {
    let absolute = package_dir.join(declared);
    let bytes = match std::fs::read(&absolute) {
        Ok(bytes) => bytes,
        Err(e) => {
            return vec![error(
                role,
                declared,
                format!(
                    "[media].{role} names `{declared}`, which cannot be read: {e} — the card's \
                     images are source files in the package tree, and a missing one leaves the \
                     site with a declared image it cannot serve \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                     fix: add the file, or drop the `{role}` key — a placeholder is generated \
                     when a role is absent)"
                ),
            )];
        }
    };

    let probe = Probe::of(&bytes);
    match probe.format {
        Format::Svg => {
            return vec![error(
                role,
                declared,
                format!(
                    "[media].{role} `{declared}` is an SVG — refused in this wave because SVG can \
                     carry script and the local reader serves a package's images exactly as they \
                     are \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                     fix: ship the image as PNG, JPEG or WebP)"
                ),
            )];
        }
        Format::Unknown => {
            return vec![error(
                role,
                declared,
                format!(
                    "[media].{role} `{declared}` carries no PNG, JPEG or WebP signature — the \
                     format is read from the file's first bytes, never from its name, so a \
                     picture named `.png` that is something else is caught here rather than \
                     served \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                     fix: ship the image as PNG, JPEG or WebP)"
                ),
            )];
        }
        Format::Png | Format::Jpeg | Format::WebP => {}
    }

    let mut out = Vec::new();
    let allowed = limits(role);
    let size = bytes.len() as u64;
    if size > allowed.max_bytes {
        out.push(error(
            role,
            declared,
            format!(
                "[media].{role} `{declared}` is {size} bytes, over the {max} the card allows — \
                 the limits are small on purpose: packages of ordinary kinds are materialised \
                 and committed in consumers' trees \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                 fix: re-encode the image smaller)",
                max = allowed.max_bytes,
            ),
        ));
    }

    let Some((width, height)) = probe.dimensions else {
        out.push(warning(
            role,
            declared,
            format!(
                "[media].{role} `{declared}` is a valid {format} file whose dimensions this check \
                 could not read, so its proportions are unverified — the bytes are a picture, the \
                 shape is simply unknown here \
                 (spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE)",
                format = probe.format.as_str(),
            ),
        ));
        return out;
    };
    if width == 0 || height == 0 {
        out.push(error(
            role,
            declared,
            format!(
                "[media].{role} `{declared}` reports a {width}×{height} canvas — an image with no \
                 area is not one \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                 fix: re-export the image)"
            ),
        ));
        return out;
    }

    match allowed.ratio {
        None => {
            if width != height {
                out.push(error(
                    role,
                    declared,
                    format!(
                        "[media].{role} `{declared}` is {width}×{height} — the icon is shown in \
                         the page header and on shelf cards, where it is square \
                         (violates \
                         spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                         fix: export a square image)"
                    ),
                ));
            } else if let Some((low, high)) = allowed.side_range
                && (width < low || width > high)
            {
                out.push(error(
                    role,
                    declared,
                    format!(
                        "[media].{role} `{declared}` is {width} px on a side, outside the \
                         {low}…{high} px the card allows — below the floor it blurs on a shelf, \
                         above the ceiling it is weight nobody asked for \
                         (violates \
                         spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                         fix: export the icon between {low} and {high} px)"
                    ),
                ));
            }
        }
        Some(target) => {
            let actual = f64::from(width) / f64::from(height);
            if (actual - target).abs() / target > RATIO_TOLERANCE {
                out.push(error(
                    role,
                    declared,
                    format!(
                        "[media].{role} `{declared}` is {width}×{height} — a ratio of \
                         {actual:.3}:1 where this role is shown at {target}:1, so the site would \
                         crop or letterbox it \
                         (violates \
                         spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                         fix: export at {target}:1)"
                    ),
                ));
            }
        }
    }
    out
}

/// What [`publish`] put where.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Published {
    /// `(role, published file name)`, in [`ROLES`] order. The name is
    /// relative to the directory `publish` was given.
    pub files: Vec<(String, String)>,
}

impl Published {
    /// The published name of one role, when it was declared.
    pub fn of(&self, role: &str) -> Option<&str> {
        self.files
            .iter()
            .find(|(r, _)| r == role)
            .map(|(_, name)| name.as_str())
    }
}

/// Copy every declared image of `package_dir` into `out_dir` under a
/// name taken from its content (`##CARD-SITE-COPIES`).
///
/// The name is the first sixteen hex digits of the file's SHA-256 plus
/// the extension its format earns — so the address changes exactly when
/// the picture does, two roles holding one picture publish one file, and
/// a cache may hold the result forever. The bytes are copied, never
/// re-encoded: a build that re-encodes an author's image publishes a
/// picture nobody approved.
///
/// An image the checks would refuse is copied anyway and reported by
/// [`check`]: publishing is not the place that decides what is allowed,
/// and a build that silently dropped a declared image would leave the
/// site with an empty frame and no reason for it.
pub fn publish(package_dir: &Path, out_dir: &Path) -> Result<Published> {
    let mut published = Published::default();
    for (role, declared) in declared_media(package_dir)? {
        let from = package_dir.join(&declared);
        let bytes = std::fs::read(&from).map_err(|e| DocError::io("reading", &from, e))?;
        let name = hashed_name(&bytes, &declared);
        let to = out_dir.join(&name);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DocError::io("creating", parent, e))?;
        }
        std::fs::write(&to, &bytes).map_err(|e| DocError::io("writing", &to, e))?;
        published.files.push((role, name));
    }
    Ok(published)
}

/// The name one image is published under: content, then the extension
/// its FORMAT earns rather than the one its name claims — the same
/// posture the checks take, one step later.
pub fn hashed_name(bytes: &[u8], declared: &str) -> String {
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    let extension = match Probe::of(bytes).format {
        Format::Png => "png",
        Format::Jpeg => "jpg",
        Format::WebP => "webp",
        // Neither is publishable and the checks say so; the declared
        // spelling is kept so the file a reader finds still matches the
        // one the report named.
        _ => Path::new(declared)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin"),
    };
    format!("{hex}.{extension}")
}

fn error(role: &str, path: &str, message: String) -> Finding {
    Finding {
        role: role.to_string(),
        path: path.to_string(),
        severity: Severity::Error,
        message,
    }
}

fn warning(role: &str, path: &str, message: String) -> Finding {
    Finding {
        role: role.to_string(),
        path: path.to_string(),
        severity: Severity::Warning,
        message,
    }
}

/// The coordinate a placeholder is derived from, and the file name its
/// generated markup is published under.
///
/// Both are pure functions of the coordinate, so a package's own
/// placeholder is stable across builds and machines — that stability is
/// the whole reason the placeholder is derived rather than stored
/// (`##CARD-PLACEHOLDERS-GENERATED`).
pub fn generated_name(coordinate: &str, role: &str, extension: &str) -> String {
    let digest = Sha256::digest(format!("{coordinate}#{role}").as_bytes());
    let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    format!("{hex}.{extension}")
}

/// Absolute path of the generated placeholder set for one package, as a
/// build writes it beside the copied images.
pub fn generated_paths(out_dir: &Path, coordinate: &str) -> Vec<(String, PathBuf)> {
    ROLES
        .iter()
        .map(|role| {
            let ext = if *role == "preview" { "png" } else { "svg" };
            (
                (*role).to_string(),
                out_dir.join(generated_name(coordinate, role, ext)),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
