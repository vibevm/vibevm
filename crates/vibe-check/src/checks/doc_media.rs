//! The card's images — that they are there, that they are what they
//! claim to be, and that they fit the shape they will be shown in
//! ([PROP-057](../../../../vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml)
//! §7, design decision D-20, campaign rule R-19).
//!
//! Every judgment here is made from the FILE's first bytes, never from
//! its name. An extension is an author's assertion; a magic number is
//! the file. That distinction is the point of the rule: the local
//! reader serves a proprietary package's images exactly as they are, so
//! `icon.png` holding an SVG would be a script served under a picture's
//! name. SVG is refused outright in this wave — it can carry script —
//! and so is any format outside PNG, JPEG and WebP.
//!
//! The limits are small on purpose (D-20): packages of ordinary kinds
//! are materialised and committed in consumers' trees, so an image
//! nobody asked for is paid for by everyone downstream.
//!
//! * `icon` — square, 256…1024 px on a side, at most 256 KiB.
//! * `banner` — 3:1, at most 1 MiB (1500×500 is the recommendation, not
//!   the rule).
//! * `preview` — 1.91:1, at most 1 MiB (1200×630 recommended), because a
//!   link card's proportions are not a banner's.
//!
//! A format this cell recognises but whose dimensions it cannot read is
//! a WARNING, not an error: the bytes are a legal picture, the shape is
//! simply unknown to a header reader, and refusing there would be the
//! linter overstating what it measured.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#card");

use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::Manifest;

use crate::{Check, CheckId, CheckOptions, CheckReport};

mod image;

use image::{Format, Probe};

/// The [`CheckId::DocMedia`] cell.
#[cell(seam = "Check", variant = "doc-media")]
pub struct DocMediaCheck;

/// What one media role is allowed to be: its aspect ratio (width /
/// height) when the role has one, its byte ceiling, and the side range
/// a square icon must land in.
struct Role {
    /// Target aspect ratio, or `None` when the role is square.
    ratio: Option<f64>,
    /// Byte ceiling, from D-20.
    max_bytes: u64,
    /// Inclusive side range, for the one role D-20 bounds in pixels.
    side_range: Option<(u32, u32)>,
}

/// The limits D-20 sets, by field name. An unknown field cannot reach
/// here — `MediaDecl` has exactly these three.
fn role(field: &str) -> Role {
    match field {
        "icon" => Role {
            ratio: None,
            max_bytes: 256 * 1024,
            side_range: Some((256, 1024)),
        },
        "banner" => Role {
            ratio: Some(3.0),
            max_bytes: 1024 * 1024,
            side_range: None,
        },
        // `preview`, and the only remaining variant.
        _ => Role {
            ratio: Some(1.91),
            max_bytes: 1024 * 1024,
            side_range: None,
        },
    }
}

/// How far a ratio may drift from its target before it stops being that
/// shape. One percent — 1200×630 is 1.9048, not 1.91, and rejecting the
/// recommendation D-20 itself prints would be absurd.
const RATIO_TOLERANCE: f64 = 0.01;

/// The project-relative path, with `/` separators whatever the host
/// spells, for a finding a human will read.
fn label(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

impl Check for DocMediaCheck {
    fn id(&self) -> CheckId {
        CheckId::DocMedia
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        let Ok(manifest) = Manifest::read(project_root.join(Manifest::FILENAME)) else {
            return;
        };
        let Some(media) = manifest.media.as_ref() else {
            return;
        };
        for (field, declared) in media.declared() {
            check_one(project_root, field, declared, report);
        }
    }
}

/// One declared image, from «is it there» to «does it fit». Each
/// question that fails ends this image's checking: a file that is not
/// there has no signature, and a file that is not a picture has no
/// shape.
fn check_one(project_root: &Path, field: &str, declared: &Path, report: &mut CheckReport) {
    let rel: PathBuf = declared.to_path_buf();
    let absolute = project_root.join(declared);
    let bytes = match std::fs::read(&absolute) {
        Ok(bytes) => bytes,
        Err(e) => {
            report.err(
                CheckId::DocMedia,
                Some(rel.clone()),
                None,
                format!(
                    "[media].{field} names `{path}`, which cannot be read: {e} — the card's \
                     images are source files in the package tree, and a missing one leaves the \
                     site with a declared image it cannot serve \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                     fix: add the file, or drop the `{field}` key — a placeholder is generated \
                     when a role is absent)",
                    path = label(&rel),
                ),
            );
            return;
        }
    };

    let probe = Probe::of(&bytes);
    match probe.format {
        Format::Svg => {
            report.err(
                CheckId::DocMedia,
                Some(rel.clone()),
                None,
                format!(
                    "[media].{field} `{path}` is an SVG — refused in this wave because SVG can \
                     carry script and the local reader serves a package's images exactly as they \
                     are \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                     fix: ship the image as PNG, JPEG or WebP)",
                    path = label(&rel),
                ),
            );
            return;
        }
        Format::Unknown => {
            report.err(
                CheckId::DocMedia,
                Some(rel.clone()),
                None,
                format!(
                    "[media].{field} `{path}` carries no PNG, JPEG or WebP signature — the format \
                     is read from the file's first bytes, never from its name, so a picture named \
                     `.png` that is something else is caught here rather than served \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                     fix: ship the image as PNG, JPEG or WebP)",
                    path = label(&rel),
                ),
            );
            return;
        }
        Format::Png | Format::Jpeg | Format::WebP => {}
    }

    let limits = role(field);
    let size = bytes.len() as u64;
    if size > limits.max_bytes {
        report.err(
            CheckId::DocMedia,
            Some(rel.clone()),
            None,
            format!(
                "[media].{field} `{path}` is {size} bytes, over the {max} the card allows — the \
                 limits are small on purpose: packages of ordinary kinds are materialised and \
                 committed in consumers' trees \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                 fix: re-encode the image smaller)",
                path = label(&rel),
                max = limits.max_bytes,
            ),
        );
    }

    let Some((width, height)) = probe.dimensions else {
        report.warn(
            CheckId::DocMedia,
            Some(rel.clone()),
            None,
            format!(
                "[media].{field} `{path}` is a valid {format} file whose dimensions this check \
                 could not read, so its proportions are unverified — the bytes are a picture, the \
                 shape is simply unknown here \
                 (spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE)",
                path = label(&rel),
                format = probe.format.as_str(),
            ),
        );
        return;
    };
    if width == 0 || height == 0 {
        report.err(
            CheckId::DocMedia,
            Some(rel.clone()),
            None,
            format!(
                "[media].{field} `{path}` reports a {width}×{height} canvas — an image with no \
                 area is not one \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                 fix: re-export the image)",
                path = label(&rel),
            ),
        );
        return;
    }

    match limits.ratio {
        None => {
            if width != height {
                report.err(
                    CheckId::DocMedia,
                    Some(rel.clone()),
                    None,
                    format!(
                        "[media].{field} `{path}` is {width}×{height} — the icon is shown in the \
                         page header and on shelf cards, where it is square \
                         (violates \
                         spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                         fix: export a square image)",
                        path = label(&rel),
                    ),
                );
            } else if let Some((low, high)) = limits.side_range
                && (width < low || width > high)
            {
                report.err(
                    CheckId::DocMedia,
                    Some(rel.clone()),
                    None,
                    format!(
                        "[media].{field} `{path}` is {width} px on a side, outside the \
                         {low}…{high} px the card allows — below the floor it blurs on a shelf, \
                         above the ceiling it is weight nobody asked for \
                         (violates \
                         spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                         fix: export the icon between {low} and {high} px)",
                        path = label(&rel),
                    ),
                );
            }
        }
        Some(target) => {
            let actual = f64::from(width) / f64::from(height);
            if (actual - target).abs() / target > RATIO_TOLERANCE {
                report.err(
                    CheckId::DocMedia,
                    Some(rel.clone()),
                    None,
                    format!(
                        "[media].{field} `{path}` is {width}×{height} — a ratio of {actual:.3}:1 \
                         where this role is shown at {target}:1, so the site would crop or letterbox \
                         it \
                         (violates \
                         spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                         fix: export at {target}:1)",
                        path = label(&rel),
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
#[path = "doc_media/tests.rs"]
mod tests;
