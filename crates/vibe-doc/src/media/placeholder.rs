//! Placeholders derived from the coordinate (PROP-057
//! `##CARD-PLACEHOLDERS-GENERATED`, `##CARD-PREVIEW-COMPOSED`, D-20).
//!
//! A package that ships no image still has one. The banner's colours and
//! pattern and the icon's ground are computed from the hash of its
//! coordinate, so two packages are told apart at a glance and one
//! package looks the same on the site and in the local reader; the glyph
//! on top is its KIND — a book for `doc`, its own sign for each of the
//! others — so what a card is is legible before its title is read.
//!
//! Nothing is stored. The banner and the icon are inline SVG composed
//! here at render time, which is what «generated, never kept» means for
//! two pictures whose every pixel is a function of eleven bytes of hash.
//!
//! The preview cannot be SVG and the reason is not aesthetic: a link
//! card is fetched by a crawler that will not run a renderer, so it is a
//! raster ([`preview_png`]) composed from the same ground, the same
//! glyph and the package's title. It is composed and never cropped from
//! the banner — a 3:1 strip and a 1.91:1 card do not contain each other.
//!
//! ## Why the same numbers twice
//!
//! One [`Card`] value is the ground for both paths: [`Card::to_svg`]
//! writes it as markup and [`Card::raster`] draws it into pixels. A
//! second description of the same picture would drift the day one of the
//! two was edited, and the drift would be invisible — the reader who
//! sees the SVG never sees the raster.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#CARD-PLACEHOLDERS-GENERATED");

mod font;
mod raster;

#[cfg(test)]
mod tests;

use sha2::{Digest, Sha256};
use vibe_core::PackageKind;

/// The banner's canvas, in the proportion D-20 gives it (3:1).
pub const BANNER: (u32, u32) = (1500, 500);
/// The icon's canvas: square, at the top of D-20's side range.
pub const ICON: (u32, u32) = (1024, 1024);
/// The link card's canvas — the size D-20 recommends and every crawler
/// expects (1.91:1).
pub const PREVIEW: (u32, u32) = (1200, 630);

/// One colour, as both backends spell it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// The `#rrggbb` an SVG attribute takes.
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

/// One drawn thing. Three primitives cover every pattern this module
/// generates, and a fourth would be a fourth thing to keep in step
/// between the two backends.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fill: Rgb,
        opacity: f64,
    },
    Circle {
        cx: f64,
        cy: f64,
        r: f64,
        fill: Rgb,
        opacity: f64,
    },
    Polygon {
        points: Vec<(f64, f64)>,
        fill: Rgb,
        opacity: f64,
    },
}

/// A generated picture: a vertical gradient, the shapes over it, and the
/// ink the glyph and any text are drawn in.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    pub width: u32,
    pub height: u32,
    /// The gradient's colour at the top edge.
    pub from: Rgb,
    /// Its colour at the bottom edge.
    pub to: Rgb,
    /// The colour the kind's glyph and the title are drawn in.
    pub ink: Rgb,
    pub shapes: Vec<Shape>,
}

impl Card {
    /// The banner of `coordinate`: the ground, its pattern, and the
    /// kind's glyph standing at the left where a title would begin.
    pub fn banner(coordinate: &str, kind: PackageKind) -> Card {
        let seed = seed(coordinate);
        let mut card = ground(&seed, BANNER.0, BANNER.1);
        card.shapes.extend(pattern(&seed, BANNER.0, BANNER.1));
        card.shapes
            .extend(glyph_at(kind, card.ink, 110.0, 110.0, 280.0));
        card
    }

    /// The icon of `coordinate`: the same ground, no pattern, and the
    /// glyph filling it. An icon is read at 32 px on a shelf, and a
    /// pattern at that size is noise.
    pub fn icon(coordinate: &str, kind: PackageKind) -> Card {
        let seed = seed(coordinate);
        let mut card = ground(&seed, ICON.0, ICON.1);
        let side = f64::from(ICON.0) * 0.52;
        let offset = (f64::from(ICON.0) - side) / 2.0;
        card.shapes
            .extend(glyph_at(kind, card.ink, offset, offset, side));
        card
    }

    /// The link card of `coordinate`: the ground, its pattern, the glyph
    /// in the upper left and the title set below it.
    ///
    /// The title is drawn by [`raster`], not carried here, because the
    /// SVG backend has no font to set it in and inventing one would put
    /// a picture on the site that the crawler's copy does not match.
    pub fn preview(coordinate: &str, kind: PackageKind) -> Card {
        let seed = seed(coordinate);
        let mut card = ground(&seed, PREVIEW.0, PREVIEW.1);
        card.shapes.extend(pattern(&seed, PREVIEW.0, PREVIEW.1));
        card.shapes
            .extend(glyph_at(kind, card.ink, 90.0, 80.0, 150.0));
        card
    }

    /// The picture as inline SVG: no script, no external reference, no
    /// file — markup a page embeds and a content policy needs no
    /// exception for.
    pub fn to_svg(&self) -> String {
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" \
             width=\"{w}\" height=\"{h}\" role=\"img\">",
            w = self.width,
            h = self.height
        );
        out.push_str(&format!(
            "<defs><linearGradient id=\"g\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\">\
             <stop offset=\"0\" stop-color=\"{from}\"/>\
             <stop offset=\"1\" stop-color=\"{to}\"/></linearGradient></defs>",
            from = self.from.hex(),
            to = self.to.hex()
        ));
        out.push_str(&format!(
            "<rect width=\"{}\" height=\"{}\" fill=\"url(#g)\"/>",
            self.width, self.height
        ));
        for shape in &self.shapes {
            out.push_str(&svg_shape(shape));
        }
        out.push_str("</svg>");
        out
    }

    /// The picture as `width * height * 3` bytes, row-major.
    pub fn raster(&self, title: Option<&str>) -> Vec<u8> {
        raster::draw(self, title)
    }
}

/// The banner of `coordinate`, as inline SVG.
///
/// ```
/// use vibe_core::PackageKind;
/// let svg = vibe_doc::media::banner_svg("org.vibevm.core/vibevm-docs", PackageKind::Doc);
/// assert!(svg.starts_with("<svg"));
/// assert!(!svg.contains("<script"));
/// // Derived, so two builds of one coordinate are one picture.
/// assert_eq!(svg, vibe_doc::media::banner_svg("org.vibevm.core/vibevm-docs", PackageKind::Doc));
/// ```
pub fn banner_svg(coordinate: &str, kind: PackageKind) -> String {
    Card::banner(coordinate, kind).to_svg()
}

/// The icon of `coordinate`, as inline SVG.
pub fn icon_svg(coordinate: &str, kind: PackageKind) -> String {
    Card::icon(coordinate, kind).to_svg()
}

/// The link card of `coordinate`, as PNG bytes.
///
/// ```
/// use vibe_core::PackageKind;
/// let png = vibe_doc::media::preview_png(
///     "org.vibevm.core/vibevm-docs",
///     PackageKind::Doc,
///     "VibeVM Manual",
/// );
/// assert_eq!(&png[1..4], b"PNG");
/// ```
pub fn preview_png(coordinate: &str, kind: PackageKind, title: &str) -> Vec<u8> {
    let card = Card::preview(coordinate, kind);
    let pixels = card.raster(Some(title));
    super::png::encode_rgb(card.width, card.height, &pixels)
}

/// The eleven bytes every decision below is made from.
fn seed(coordinate: &str) -> [u8; 32] {
    Sha256::digest(coordinate.as_bytes()).into()
}

/// The ground: two hues a fixed distance apart, dark enough that white
/// ink reads on both. The distance is drawn from the hash too, so two
/// packages differ in more than their starting hue.
fn ground(seed: &[u8; 32], width: u32, height: u32) -> Card {
    let hue = f64::from(seed[0]) * 360.0 / 256.0;
    let spread = 20.0 + f64::from(seed[1]) * 70.0 / 256.0;
    let saturation = 0.42 + f64::from(seed[2]) * 0.24 / 256.0;
    Card {
        width,
        height,
        from: hsl(hue, saturation, 0.32),
        to: hsl((hue + spread) % 360.0, saturation, 0.18),
        // One ink for every ground: the two lightnesses above are chosen
        // so it carries on both, which is what makes the glyph legible
        // whatever hue the hash picked.
        ink: Rgb(0xF4, 0xF2, 0xEE),
        shapes: Vec::new(),
    }
}

/// One of five patterns, chosen by the hash and drawn faintly: the
/// ground must stay a ground, and a busy one fights the glyph.
fn pattern(seed: &[u8; 32], width: u32, height: u32) -> Vec<Shape> {
    let w = f64::from(width);
    let h = f64::from(height);
    let ink = Rgb(0xFF, 0xFF, 0xFF);
    let opacity = 0.055 + f64::from(seed[4]) * 0.05 / 256.0;
    let count = 7 + (seed[5] % 6) as usize;
    let mut out = Vec::new();
    match seed[3] % 5 {
        // Diagonal bands.
        0 => {
            let step = (w + h) / count as f64;
            let band = step / 3.0;
            for i in 0..count * 2 {
                let x = i as f64 * step - h;
                out.push(Shape::Polygon {
                    points: vec![(x, h), (x + band, h), (x + band + h, 0.0), (x + h, 0.0)],
                    fill: ink,
                    opacity,
                });
            }
        }
        // Dots on a lattice.
        1 => {
            let step = w / count as f64;
            let radius = step / 7.0;
            let mut y = step / 2.0;
            let mut row = 0usize;
            while y < h + step {
                let offset = if row.is_multiple_of(2) {
                    0.0
                } else {
                    step / 2.0
                };
                let mut x = offset + step / 2.0;
                while x < w + step {
                    out.push(Shape::Circle {
                        cx: x,
                        cy: y,
                        r: radius,
                        fill: ink,
                        opacity,
                    });
                    x += step;
                }
                y += step;
                row += 1;
            }
        }
        // A ruled grid.
        2 => {
            let step = w / count as f64;
            let line = (step / 24.0).max(1.0);
            let mut x = step;
            while x < w {
                out.push(Shape::Rect {
                    x,
                    y: 0.0,
                    w: line,
                    h,
                    fill: ink,
                    opacity,
                });
                x += step;
            }
            let mut y = step;
            while y < h {
                out.push(Shape::Rect {
                    x: 0.0,
                    y,
                    w,
                    h: line,
                    fill: ink,
                    opacity,
                });
                y += step;
            }
        }
        // Rays from the upper right corner.
        3 => {
            let origin = (w, 0.0);
            for i in 0..count * 2 {
                let span = (w + h) / (count * 2) as f64;
                let a = i as f64 * span;
                out.push(Shape::Polygon {
                    points: vec![origin, (w - a, h), (w - a - span / 2.0, h)],
                    fill: ink,
                    opacity,
                });
            }
        }
        // Rows of chevrons.
        _ => {
            let step = w / count as f64;
            let mut y = 0.0;
            while y < h + step {
                let mut x = -step;
                while x < w + step {
                    out.push(Shape::Polygon {
                        points: vec![
                            (x, y + step * 0.5),
                            (x + step * 0.5, y),
                            (x + step, y + step * 0.5),
                            (x + step * 0.5, y + step * 0.25),
                        ],
                        fill: ink,
                        opacity,
                    });
                    x += step;
                }
                y += step;
            }
        }
    }
    out
}

/// The kind's glyph, scaled into a `side`-wide box at `(x, y)`.
fn glyph_at(kind: PackageKind, ink: Rgb, x: f64, y: f64, side: f64) -> Vec<Shape> {
    let place = |points: &[(f64, f64)]| Shape::Polygon {
        points: points
            .iter()
            .map(|(px, py)| (x + px * side, y + py * side))
            .collect(),
        fill: ink,
        opacity: 1.0,
    };
    let bar = |bx: f64, by: f64, bw: f64, bh: f64| Shape::Rect {
        x: x + bx * side,
        y: y + by * side,
        w: bw * side,
        h: bh * side,
        fill: ink,
        opacity: 1.0,
    };
    let dot = |cx: f64, cy: f64, r: f64| Shape::Circle {
        cx: x + cx * side,
        cy: y + cy * side,
        r: r * side,
        fill: ink,
        opacity: 1.0,
    };
    match kind {
        // An open book: two leaves falling away from a spine, which is
        // what tells it apart from a box at a shelf's size.
        PackageKind::Doc => vec![
            place(&[(0.02, 0.20), (0.47, 0.30), (0.47, 0.88), (0.02, 0.78)]),
            place(&[(0.53, 0.30), (0.98, 0.20), (0.98, 0.78), (0.53, 0.88)]),
            bar(0.47, 0.30, 0.06, 0.58),
        ],
        // A flow: an arrow through a channel.
        PackageKind::Flow => vec![
            bar(0.06, 0.42, 0.62, 0.16),
            place(&[(0.60, 0.22), (0.96, 0.50), (0.60, 0.78)]),
        ],
        // A feature: a facetted gem.
        PackageKind::Feat => vec![place(&[
            (0.50, 0.08),
            (0.92, 0.38),
            (0.76, 0.90),
            (0.24, 0.90),
            (0.08, 0.38),
        ])],
        // A stack: three layers.
        PackageKind::Stack => vec![
            bar(0.10, 0.18, 0.80, 0.18),
            bar(0.10, 0.41, 0.80, 0.18),
            bar(0.10, 0.64, 0.80, 0.18),
        ],
        // A tool: a head and a handle.
        PackageKind::Tool => vec![
            dot(0.32, 0.32, 0.24),
            bar(0.44, 0.44, 0.46, 0.16),
            place(&[(0.80, 0.40), (0.96, 0.52), (0.80, 0.64)]),
        ],
        // A server: a socket with two pins.
        PackageKind::Mcp => vec![
            bar(0.22, 0.10, 0.14, 0.24),
            bar(0.64, 0.10, 0.14, 0.24),
            bar(0.10, 0.34, 0.80, 0.28),
            bar(0.40, 0.62, 0.20, 0.28),
        ],
        // A language: the two brackets everything is written between.
        PackageKind::Lang => vec![
            place(&[
                (0.36, 0.12),
                (0.46, 0.20),
                (0.20, 0.50),
                (0.46, 0.80),
                (0.36, 0.88),
                (0.04, 0.50),
            ]),
            place(&[
                (0.64, 0.12),
                (0.96, 0.50),
                (0.64, 0.88),
                (0.54, 0.80),
                (0.80, 0.50),
                (0.54, 0.20),
            ]),
        ],
        // An application: a window with its own title bar.
        PackageKind::App => vec![bar(0.08, 0.16, 0.84, 0.16), bar(0.08, 0.36, 0.84, 0.48)],
    }
}

/// One shape as SVG.
fn svg_shape(shape: &Shape) -> String {
    match shape {
        Shape::Rect {
            x,
            y,
            w,
            h,
            fill,
            opacity,
        } => format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" opacity=\"{}\"/>",
            num(*x),
            num(*y),
            num(*w),
            num(*h),
            fill.hex(),
            num(*opacity)
        ),
        Shape::Circle {
            cx,
            cy,
            r,
            fill,
            opacity,
        } => format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" opacity=\"{}\"/>",
            num(*cx),
            num(*cy),
            num(*r),
            fill.hex(),
            num(*opacity)
        ),
        Shape::Polygon {
            points,
            fill,
            opacity,
        } => {
            let spelled: Vec<String> = points
                .iter()
                .map(|(x, y)| format!("{},{}", num(*x), num(*y)))
                .collect();
            format!(
                "<polygon points=\"{}\" fill=\"{}\" opacity=\"{}\"/>",
                spelled.join(" "),
                fill.hex(),
                num(*opacity)
            )
        }
    }
}

/// A number with a fixed number of places, so two runs of one coordinate
/// write the same characters and not merely the same value.
fn num(value: f64) -> String {
    let text = format!("{value:.3}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    // A value that rounds away to nothing is zero, and zero has one
    // spelling: `-0` and `0` are the same coordinate and would be two
    // different files.
    if trimmed.is_empty() || trimmed == "-" || trimmed == "-0" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// HSL to RGB, the one conversion this module needs. Hue in degrees,
/// saturation and lightness in `0.0..=1.0`.
fn hsl(hue: f64, saturation: f64, lightness: f64) -> Rgb {
    let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let h = (hue.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = lightness - c / 2.0;
    let byte = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb(byte(r), byte(g), byte(b))
}
