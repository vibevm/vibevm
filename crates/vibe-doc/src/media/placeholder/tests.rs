//! The generator's own laws: the picture is a function of the
//! coordinate and the kind, the markup is inert, and the raster is the
//! same picture drawn the other way.

use super::*;

/// Every variant of the hash-chosen pattern draws something, and none of
/// them draws over the whole card at full strength: the ground has to
/// stay a ground.
#[test]
fn every_pattern_variant_is_drawn_and_stays_faint() {
    let mut seen = std::collections::BTreeSet::new();
    for n in 0u32..64 {
        let coordinate = format!("com.example/p{n}");
        let card = Card::banner(&coordinate, PackageKind::Tool);
        let pattern: Vec<&Shape> = card.shapes.iter().filter(|s| opacity_of(s) < 1.0).collect();
        assert!(!pattern.is_empty(), "{coordinate} drew no pattern");
        for shape in pattern {
            assert!(opacity_of(shape) <= 0.12, "{coordinate}: {shape:?}");
        }
        seen.insert(seed(&coordinate)[3] % 5);
    }
    assert_eq!(
        seen.len(),
        5,
        "sixty-four coordinates reach all five patterns"
    );
}

fn opacity_of(shape: &Shape) -> f64 {
    match shape {
        Shape::Rect { opacity, .. }
        | Shape::Circle { opacity, .. }
        | Shape::Polygon { opacity, .. } => *opacity,
    }
}

/// The two backends draw ONE picture. The rasteriser is checked against
/// the card it was given rather than against the markup — comparing two
/// renderings pixel for pixel would need a parser — but the ground is
/// the one place the two must literally agree, and it is checked here.
#[test]
fn the_raster_ground_is_the_gradient_the_markup_declares() {
    let card = Card::preview("com.example/thing", PackageKind::Doc);
    let pixels = card.raster(None);
    assert_eq!(pixels.len(), (card.width * card.height * 3) as usize);

    // The top-left pixel is the gradient's first stop, within the one
    // step of rounding a lerp may cost.
    let top = Rgb(pixels[0], pixels[1], pixels[2]);
    near(top, card.from);
    // And the bottom-left is its last.
    let last_row = ((card.height - 1) * card.width * 3) as usize;
    let bottom = Rgb(pixels[last_row], pixels[last_row + 1], pixels[last_row + 2]);
    near(bottom, card.to);
}

fn near(got: Rgb, want: Rgb) {
    for (a, b) in [(got.0, want.0), (got.1, want.1), (got.2, want.2)] {
        assert!(a.abs_diff(b) <= 1, "{got:?} is not {want:?}");
    }
}

/// The icon carries no pattern. At a shelf's size a pattern is noise,
/// and this is the rule that says so in code rather than in a comment.
#[test]
fn the_icon_is_ground_and_glyph_and_nothing_else() {
    let card = Card::icon("com.example/thing", PackageKind::Feat);
    assert!(card.shapes.iter().all(|s| opacity_of(s) == 1.0));
}

/// Numbers are written the same way every time, because the markup is a
/// derived artefact and a derived artefact is bytes.
#[test]
fn numbers_are_spelled_one_way() {
    assert_eq!(num(1.0), "1");
    assert_eq!(num(1.5), "1.5");
    assert_eq!(num(0.0), "0");
    assert_eq!(num(-0.0004), "0");
    assert_eq!(num(1.23456), "1.235");
}

/// The face sets what it has and says so honestly when it has too
/// little.
#[test]
fn the_face_folds_case_and_refuses_a_script_it_cannot_set() {
    assert_eq!(font::renderable("VibeVM Manual"), "VIBEVM MANUAL");
    assert_eq!(font::renderable("Руководство"), "");
    assert_eq!(font::renderable(""), "");
    // A stray unsupported character in an otherwise Latin title becomes
    // a space rather than taking the whole line down.
    assert_eq!(font::renderable("Vibe™ Manual"), "VIBE MANUAL");
}

/// A long title is set smaller rather than clipped, and the ladder is
/// finite so the type size is a decision and not a fraction.
#[test]
fn a_longer_title_is_set_at_a_smaller_step() {
    let (column, band) = (1000.0, 340.0);
    let short = font::wrap("VIBEVM MANUAL", column, band);
    let long = font::wrap(
        "A VERY MUCH LONGER TITLE THAT KEEPS GOING WELL PAST ONE LINE OF TYPE",
        column,
        band,
    );
    assert!(
        long.scale < short.scale,
        "{} vs {}",
        long.scale,
        short.scale
    );
    assert!(long.lines.len() <= font::MAX_LINES);
    assert!(short.lines.len() <= font::MAX_LINES);
    // And both blocks stay inside the band they were given: a card whose
    // last line runs off the bottom is the failure this measurement is
    // here to prevent.
    for set in [&short, &long] {
        let height = (set.lines.len().saturating_sub(1) as f64 * f64::from(font::LINE_ADVANCE)
            + f64::from(font::HEIGHT))
            * set.scale;
        assert!(height <= band, "{height} over {band}");
    }
}
