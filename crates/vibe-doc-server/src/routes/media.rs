//! The card's pictures (PROP-057 `##CARD-SITE-COPIES`,
//! `##CARD-PLACEHOLDERS-GENERATED`, `##LOCAL-STATIC`).
//!
//! An icon, a banner and a link preview: three roles, each either a file
//! the package ships or a placeholder drawn from the coordinate. The
//! addresses are `vibe_doc::media::slots`' and nobody else's — the same
//! function `vibe doc build` writes the files from and the page manifest
//! names them by — so the picture a reader sees is the picture the
//! manifest promised, byte for byte, and the local reader agrees with the
//! site without either of them computing a name.
//!
//! Two spellings answer, because the two worlds put the same picture in
//! two places. The manifest this reader serves lives at `<base>`, so the
//! addresses inside it are `<base>media/<name>`; a PAGE, wherever it is
//! mounted, reaches the pictures of its own edition by climbing to the
//! edition's root, which locally is `<base><coordinate>/<version>/`. One
//! lane, one set of bytes, both addresses.
//!
//! ## The name is matched, never joined
//!
//! `##LOCAL-STATIC` names one form a reader must not repeat: a path built
//! out of a request. This route does not build one at all. The name is
//! first checked to be a single ordinary segment, then looked for among
//! the addresses the manifest itself carries, and the bytes come from the
//! slot that matched. A name nobody published is a `404` before any part
//! of it reaches the filesystem, and there is no directory to list.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#CARD-SITE-COPIES");

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::Reader;
use crate::error::ApiError;

/// The directory the card's pictures are published under, as the
/// pipeline spells it.
const DIRECTORY: &str = vibe_doc::media::PUBLISHED_DIR;

/// One of the card's pictures, or `None` when this address is not one.
pub(crate) fn answer(reader: &Reader, rest: &str) -> Result<Option<Response>, ApiError> {
    let Some(name) = requested(reader, rest) else {
        return Ok(None);
    };
    // First check: a picture's name is ONE ordinary segment. A separator,
    // an escape that decoded to one, a drive prefix — none of them is a
    // name the pipeline ever wrote, and each is refused here rather than
    // further in.
    if !is_plain_name(name) {
        return Err(ApiError::bad_request(format!(
            "`{name}` is not the name of a picture — the card's pictures are named by their \
             content, and the manifest carries the names"
        )));
    }
    let address = format!("{DIRECTORY}/{name}");
    let slots = vibe_doc::media::slots(&reader.package_dir, &reader.coordinate)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    // Second check: the address must be one the MANIFEST carries. The
    // request never names a file — it names a published address, and the
    // bytes come from the slot behind it.
    let Some(slot) = slots.iter().find(|slot| slot.address == address) else {
        return Err(ApiError::not_found(format!(
            "this documentation publishes no picture `{name}`"
        )));
    };
    let (kind, title) = card(reader)?;
    Ok(Some(picture(
        slot.render(&reader.coordinate, kind, &title),
        content_type(name),
    )))
}

/// The picture's name inside this address, under the mount or under the
/// edition — or `None` when the address is about something else
/// entirely.
///
/// The `latest` spelling of the edition is not a third case: the address
/// arrives here already read into the number
/// ([`crate::routes::as_numbered`]).
fn requested<'a>(reader: &Reader, rest: &'a str) -> Option<&'a str> {
    let under_mount = rest.strip_prefix(&format!("{DIRECTORY}/"));
    let under_edition = rest
        .strip_prefix(&reader.prefix)
        .and_then(|tail| tail.strip_prefix(&format!("{DIRECTORY}/")));
    under_mount.or(under_edition)
}

/// The kind whose glyph a generated placeholder wears, and the title a
/// preview is composed around.
///
/// Read through the typed manifest rather than by a second reading of
/// TOML: what a package IS has one home (`vibe_core::manifest`), and this
/// reader already opened the package through it to learn its coordinate.
fn card(reader: &Reader) -> Result<(vibe_core::PackageKind, String), ApiError> {
    let path = reader
        .package_dir
        .join(vibe_core::manifest::Manifest::FILENAME);
    let manifest = vibe_core::manifest::Manifest::read(&path)
        .map_err(|e| ApiError::internal(format!("`{}` is not readable: {e}", path.display())))?;
    let Some(package) = manifest.package else {
        return Err(ApiError::internal(
            "this reader's package carries no `[package]` table",
        ));
    };
    Ok((package.kind, package.title.unwrap_or_default()))
}

/// Is this exactly one ordinary name?
///
/// The colon is refused for the reason the shell refuses it: on Windows a
/// segment like `C:` is a DRIVE PREFIX, and no name the pipeline writes
/// carries one.
fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
        && !name.contains(':')
}

/// What a picture is, by the extension the pipeline gave it.
///
/// A closed table, and a short one: `##CARD-MEDIA-SOURCE` admits PNG,
/// JPEG and WebP for a declared picture, and a generated placeholder is
/// an SVG or the composed PNG. An extension outside that set is served as
/// bytes rather than guessed at, and every response carries `nosniff`.
fn content_type(name: &str) -> &'static str {
    match name.rsplit_once('.').map(|(_, extension)| extension) {
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

/// A picture's response. The name is a content name, so a changed picture
/// is a changed address and this may be cached for as long as anybody
/// likes — the same bargain the shell's hashed directories make.
fn picture(bytes: Vec<u8>, content_type: &'static str) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            ),
        ],
        bytes,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_medium_the_card_admits_has_a_type() {
        assert_eq!(content_type("e65e5833a9f1d438.svg"), "image/svg+xml");
        assert_eq!(content_type("d54e7cc3e2097d60.png"), "image/png");
        assert_eq!(content_type("a.jpeg"), "image/jpeg");
        assert_eq!(content_type("a.webp"), "image/webp");
        assert_eq!(content_type("nameless"), "application/octet-stream");
    }

    #[test]
    fn only_one_ordinary_segment_is_a_picture_name() {
        assert!(is_plain_name("e65e5833a9f1d438.svg"));
        for spelling in ["", ".", "..", "a/b.svg", "a\\b.svg", "C:", "a\0b"] {
            assert!(!is_plain_name(spelling), "`{spelling}` is not a name");
        }
    }
}
