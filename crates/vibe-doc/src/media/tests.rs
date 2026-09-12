//! The card's rules, on bytes built here. Every image in these tests is
//! a header assembled in the test itself, so each case is exactly the
//! question under test and nothing else.

use std::fs;
use std::path::Path;

use super::image::tests::{jpeg, png, webp_vp8x};
use super::*;

/// A package whose `[media]` is `declarations` and whose files are
/// `files`.
fn package(tmp: &Path, declarations: &str, files: &[(&str, Vec<u8>)]) -> std::path::PathBuf {
    let dir = tmp.join("pkg");
    fs::create_dir_all(dir.join("media")).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\nname = \"a-docs\"\ngroup = \"com.example\"\nkind = \"doc\"\n{declarations}"
        ),
    )
    .unwrap();
    for (name, bytes) in files {
        fs::write(dir.join(name), bytes).unwrap();
    }
    dir
}

fn messages(report: &Report) -> String {
    report
        .findings
        .iter()
        .map(|f| f.message.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The whole check in one case: three images that fit their roles say
/// nothing.
#[test]
fn a_card_whose_images_fit_their_roles_passes_clean() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = package(
        tmp.path(),
        "[media]\nicon = \"media/icon.png\"\nbanner = \"media/banner.jpg\"\n\
         preview = \"media/preview.webp\"\n",
        &[
            ("media/icon.png", png(512, 512)),
            ("media/banner.jpg", jpeg(1500, 500)),
            ("media/preview.webp", webp_vp8x(1200, 630)),
        ],
    );
    let report = check(&dir).unwrap();
    assert!(report.ok(), "{}", messages(&report));
    assert_eq!(report.declared, vec!["icon", "banner", "preview"]);
    assert!(report.render().contains("0 role(s) generated"));
}

/// A package that declares nothing is not a package without a card: the
/// placeholders are generated, and the report says how many roles took
/// that path.
#[test]
fn a_package_declaring_nothing_is_clean_and_says_what_is_generated() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = package(tmp.path(), "", &[]);
    let report = check(&dir).unwrap();
    assert!(report.ok());
    assert!(report.declared.is_empty());
    assert!(
        report.render().contains("3 role(s) generated"),
        "{}",
        report.render()
    );
}

/// The rule that gives the whole module its shape: the format is read
/// from the bytes, never from the name.
#[test]
fn an_svg_named_png_is_refused_by_its_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = package(
        tmp.path(),
        "[media]\nicon = \"media/icon.png\"\n",
        &[("media/icon.png", b"<svg xmlns=\"...\"></svg>".to_vec())],
    );
    let report = check(&dir).unwrap();
    assert!(!report.ok());
    assert!(
        messages(&report).contains("is an SVG"),
        "{}",
        messages(&report)
    );
    assert!(messages(&report).contains("CARD-MEDIA-SOURCE"));
}

/// Proportions are per role, and the roles do not contain one another —
/// which is exactly why `preview` is a field of its own.
#[test]
fn a_banner_shaped_preview_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = package(
        tmp.path(),
        "[media]\npreview = \"media/p.png\"\n",
        &[("media/p.png", png(1500, 500))],
    );
    let report = check(&dir).unwrap();
    assert!(!report.ok());
    assert!(
        messages(&report).contains("1.91:1"),
        "{}",
        messages(&report)
    );
}

/// A recognised format whose shape could not be measured is a warning:
/// the bytes are a picture and only the measurement failed.
#[test]
fn an_unmeasurable_picture_is_a_warning_and_not_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    // A WebP container with a chunk this reader does not follow.
    let mut odd = b"RIFF".to_vec();
    odd.extend_from_slice(&0u32.to_le_bytes());
    odd.extend_from_slice(b"WEBPXXXX");
    let dir = package(
        tmp.path(),
        "[media]\nicon = \"media/i.webp\"\n",
        &[("media/i.webp", odd)],
    );
    let report = check(&dir).unwrap();
    assert!(report.ok(), "a warning is not a failure");
    assert_eq!(report.warnings(), 1);
    assert!(messages(&report).contains("unverified"));
}

/// Publishing is content-addressed: the name changes when the picture
/// does and never otherwise, and the bytes are copied rather than
/// re-encoded.
#[test]
fn publishing_names_an_image_by_its_content() {
    let tmp = tempfile::tempdir().unwrap();
    let icon = png(512, 512);
    let dir = package(
        tmp.path(),
        "[media]\nicon = \"media/icon.png\"\n",
        &[("media/icon.png", icon.clone())],
    );
    let out = tmp.path().join("site");
    fs::create_dir_all(&out).unwrap();
    let published = publish(&dir, &out).unwrap();

    let name = published.of("icon").expect("the icon was published");
    assert!(name.ends_with(".png"), "{name}");
    assert_eq!(
        fs::read(out.join(name)).unwrap(),
        icon,
        "copied, not re-encoded"
    );
    // The same picture is the same address; a different one is not.
    assert_eq!(hashed_name(&icon, "media/icon.png"), name);
    assert_ne!(hashed_name(&png(256, 256), "media/icon.png"), name);
}

/// The extension a published file takes is the one its FORMAT earns, so
/// a correctly-formatted picture under a wrong name is served correctly.
#[test]
fn the_published_extension_comes_from_the_bytes() {
    assert!(hashed_name(&png(8, 8), "media/icon.jpg").ends_with(".png"));
    assert!(hashed_name(&jpeg(8, 8), "media/banner.png").ends_with(".jpg"));
    assert!(hashed_name(&webp_vp8x(8, 8), "media/p.png").ends_with(".webp"));
}

/// A placeholder is a function of the coordinate and of nothing else —
/// that is what lets the site and the local reader show one package the
/// same way without either storing a file.
#[test]
fn a_placeholder_is_the_same_picture_for_the_same_coordinate() {
    use vibe_core::PackageKind;
    let a = banner_svg("com.example/thing", PackageKind::Tool);
    let b = banner_svg("com.example/thing", PackageKind::Tool);
    let other = banner_svg("com.example/other", PackageKind::Tool);
    assert_eq!(a, b);
    assert_ne!(a, other, "two packages are told apart");
    // And the glyph is the kind's, so one coordinate under two kinds is
    // two pictures.
    assert_ne!(a, banner_svg("com.example/thing", PackageKind::Doc));
}

/// Generated markup carries no script and no outside address: the local
/// reader embeds it into a page whose policy names no external source.
#[test]
fn generated_markup_is_inert() {
    use vibe_core::PackageKind;
    for kind in vibe_core::PackageKind::ALL {
        for svg in [
            banner_svg("com.example/thing", kind),
            icon_svg("com.example/thing", kind),
        ] {
            assert!(svg.starts_with("<svg"), "{kind:?}");
            assert!(!svg.contains("<script"), "{kind:?}");
            assert!(!svg.contains("<image"), "{kind:?}");
            assert!(!svg.contains("<foreignObject"), "{kind:?}");
            assert!(!svg.contains("href"), "{kind:?}");
            // The one absolute address in the markup is the SVG
            // namespace, which fetches nothing and is what makes the
            // element an SVG at all.
            assert_eq!(svg.matches("http").count(), 1, "{kind:?}");
        }
    }
    // And every kind draws something: an empty glyph would be a card
    // that says less than the ground under it.
    let doc = Card::icon("com.example/thing", PackageKind::Doc);
    assert!(!doc.shapes.is_empty());
}

/// The composed preview is a real PNG of the size a link card is fetched
/// at, it fits the ceiling its own role sets, and it is the same file
/// twice.
#[test]
fn the_preview_is_a_link_card_within_its_own_limits() {
    use vibe_core::PackageKind;
    let png_bytes = preview_png(
        "org.vibevm.core/vibevm-docs",
        PackageKind::Doc,
        "VibeVM Manual",
    );
    let probe = image::Probe::of(&png_bytes);
    assert_eq!(probe.format, image::Format::Png);
    assert_eq!(probe.dimensions, Some(placeholder::PREVIEW));
    assert!(
        (png_bytes.len() as u64) <= limits("preview").max_bytes,
        "the generated preview must pass the check it will be judged by, got {} bytes",
        png_bytes.len()
    );
    assert_eq!(
        png_bytes,
        preview_png(
            "org.vibevm.core/vibevm-docs",
            PackageKind::Doc,
            "VibeVM Manual"
        )
    );
}

/// The title is part of the picture, so two titles are two cards.
#[test]
fn the_title_reaches_the_card() {
    use vibe_core::PackageKind;
    let a = preview_png("com.example/thing", PackageKind::Doc, "One Title");
    let b = preview_png("com.example/thing", PackageKind::Doc, "Another Title");
    assert_ne!(a, b);
}

/// A title the built-in face cannot set does not become debris: the card
/// keeps its ground and its glyph and sets no text at all.
#[test]
fn a_title_in_an_unsupported_script_sets_no_text() {
    use vibe_core::PackageKind;
    let cyrillic = preview_png("com.example/thing", PackageKind::Doc, "Руководство");
    let empty = preview_png("com.example/thing", PackageKind::Doc, "");
    assert_eq!(cyrillic, empty);
}

/// The generated name is derived too, and role by role: one package's
/// three placeholders are three addresses.
#[test]
fn every_generated_role_has_its_own_derived_name() {
    let names: Vec<String> = ROLES
        .iter()
        .map(|role| generated_name("com.example/thing", role, "svg"))
        .collect();
    assert_eq!(names.len(), 3);
    assert_ne!(names[0], names[1]);
    assert_ne!(names[1], names[2]);
    assert_eq!(names[0], generated_name("com.example/thing", "icon", "svg"));
}
