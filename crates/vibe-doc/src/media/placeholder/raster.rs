//! Drawing a [`Card`] into pixels — the half of the placeholder that a
//! crawler sees (PROP-057 `##CARD-PREVIEW-COMPOSED`).
//!
//! Every shape is drawn inside its own bounding box, so the cost of a
//! card is the area it covers rather than the canvas times the number of
//! shapes. Edges are hard: the picture is flat colour over a gradient,
//! and an anti-aliaser would be a second thing to keep identical between
//! two builds for a difference nobody sees at a link card's size.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#CARD-PREVIEW-COMPOSED");

use super::font;
use super::{Card, Rgb, Shape};

/// Bytes per pixel: the one layout [`super::super::png`] writes.
const CHANNELS: usize = 3;

/// Draw `card`, optionally setting `title` under its glyph.
pub fn draw(card: &Card, title: Option<&str>) -> Vec<u8> {
    let mut canvas = Canvas::new(card.width, card.height);
    canvas.gradient(card.from, card.to);
    for shape in &card.shapes {
        canvas.shape(shape);
    }
    if let Some(title) = title {
        canvas.title(title, card);
    }
    canvas.pixels
}

struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Canvas {
    fn new(width: u32, height: u32) -> Canvas {
        Canvas {
            width,
            height,
            pixels: vec![0u8; width as usize * height as usize * CHANNELS],
        }
    }

    /// The ground: a vertical lerp, which is also what keeps the encoded
    /// file small — a row of one colour filters to a row of zeros.
    fn gradient(&mut self, from: Rgb, to: Rgb) {
        let last = f64::from(self.height.saturating_sub(1)).max(1.0);
        for y in 0..self.height {
            let t = f64::from(y) / last;
            let colour = Rgb(
                lerp(from.0, to.0, t),
                lerp(from.1, to.1, t),
                lerp(from.2, to.2, t),
            );
            let row = y as usize * self.width as usize * CHANNELS;
            for x in 0..self.width as usize {
                let at = row + x * CHANNELS;
                self.pixels[at] = colour.0;
                self.pixels[at + 1] = colour.1;
                self.pixels[at + 2] = colour.2;
            }
        }
    }

    fn shape(&mut self, shape: &Shape) {
        match shape {
            Shape::Rect {
                x,
                y,
                w,
                h,
                fill,
                opacity,
            } => {
                let (x0, y0, x1, y1) = self.clip(*x, *y, x + w, y + h);
                for py in y0..y1 {
                    for px in x0..x1 {
                        self.blend(px, py, *fill, *opacity);
                    }
                }
            }
            Shape::Circle {
                cx,
                cy,
                r,
                fill,
                opacity,
            } => {
                let (x0, y0, x1, y1) = self.clip(cx - r, cy - r, cx + r, cy + r);
                let rr = r * r;
                for py in y0..y1 {
                    for px in x0..x1 {
                        let dx = f64::from(px) + 0.5 - cx;
                        let dy = f64::from(py) + 0.5 - cy;
                        if dx * dx + dy * dy <= rr {
                            self.blend(px, py, *fill, *opacity);
                        }
                    }
                }
            }
            Shape::Polygon {
                points,
                fill,
                opacity,
            } => self.polygon(points, *fill, *opacity),
        }
    }

    /// Scanline fill, even-odd. The crossings of one row are collected,
    /// sorted and filled in pairs — the textbook algorithm, and the
    /// reason the shapes above are all simple polygons.
    fn polygon(&mut self, points: &[(f64, f64)], fill: Rgb, opacity: f64) {
        if points.len() < 3 {
            return;
        }
        let min_x = points.iter().map(|p| p.0).fold(f64::MAX, f64::min);
        let max_x = points.iter().map(|p| p.0).fold(f64::MIN, f64::max);
        let min_y = points.iter().map(|p| p.1).fold(f64::MAX, f64::min);
        let max_y = points.iter().map(|p| p.1).fold(f64::MIN, f64::max);
        let (_, y0, _, y1) = self.clip(min_x, min_y, max_x, max_y);
        let mut crossings: Vec<f64> = Vec::with_capacity(points.len());
        for py in y0..y1 {
            let y = f64::from(py) + 0.5;
            crossings.clear();
            for i in 0..points.len() {
                let (ax, ay) = points[i];
                let (bx, by) = points[(i + 1) % points.len()];
                if (ay <= y) == (by <= y) {
                    continue;
                }
                crossings.push(ax + (y - ay) / (by - ay) * (bx - ax));
            }
            crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            for pair in crossings.chunks(2) {
                let [start, end] = pair else { continue };
                let (sx, _, ex, _) = self.clip(*start, 0.0, *end, 1.0);
                for px in sx..ex {
                    self.blend(px, py, fill, opacity);
                }
            }
        }
    }

    /// Set the title under the glyph, in the built-in face, at the
    /// largest scale that fits the card's text column.
    fn title(&mut self, title: &str, card: &Card) {
        let text = font::renderable(title);
        if text.is_empty() {
            return;
        }
        let margin = f64::from(card.width) * 0.075;
        let column = f64::from(card.width) - margin * 2.0;
        // The band the title has to itself: from under the glyph to the
        // bottom margin. Sizing by the column alone is how a third line
        // ends up off the card.
        let top = f64::from(card.height) * 0.40;
        let band = f64::from(card.height) - margin - top;
        let lines = font::wrap(&text, column, band);
        let scale = lines.scale;
        let mut y = top;
        for line in &lines.lines {
            self.line(line, margin, y, scale, card.ink);
            y += f64::from(font::LINE_ADVANCE) * scale;
        }
    }

    fn line(&mut self, text: &str, x: f64, y: f64, scale: f64, ink: Rgb) {
        let mut pen = x;
        for ch in text.chars() {
            if let Some(rows) = font::glyph(ch) {
                for (row, bits) in rows.iter().enumerate() {
                    for column in 0..font::WIDTH {
                        if bits & (1 << (font::WIDTH - 1 - column)) == 0 {
                            continue;
                        }
                        let px = pen + f64::from(column) * scale;
                        let py = y + row as f64 * scale;
                        self.shape(&Shape::Rect {
                            x: px,
                            y: py,
                            w: scale,
                            h: scale,
                            fill: ink,
                            opacity: 1.0,
                        });
                    }
                }
            }
            pen += f64::from(font::ADVANCE) * scale;
        }
    }

    /// A rectangle of the canvas, clamped to it: `(x0, y0, x1, y1)` with
    /// the ends exclusive.
    fn clip(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> (u32, u32, u32, u32) {
        let lo = |v: f64, max: u32| v.floor().clamp(0.0, f64::from(max)) as u32;
        let hi = |v: f64, max: u32| v.ceil().clamp(0.0, f64::from(max)) as u32;
        (
            lo(x0, self.width),
            lo(y0, self.height),
            hi(x1, self.width),
            hi(y1, self.height),
        )
    }

    fn blend(&mut self, x: u32, y: u32, fill: Rgb, opacity: f64) {
        if x >= self.width || y >= self.height {
            return;
        }
        let at = (y as usize * self.width as usize + x as usize) * CHANNELS;
        let a = opacity.clamp(0.0, 1.0);
        for (channel, value) in [fill.0, fill.1, fill.2].into_iter().enumerate() {
            let old = f64::from(self.pixels[at + channel]);
            let new = old * (1.0 - a) + f64::from(value) * a;
            self.pixels[at + channel] = new.round().clamp(0.0, 255.0) as u8;
        }
    }
}

fn lerp(from: u8, to: u8, t: f64) -> u8 {
    (f64::from(from) + (f64::from(to) - f64::from(from)) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}
