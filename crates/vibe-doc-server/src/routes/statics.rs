//! The shell's own files (PROP-057 `##LOCAL-STATIC`).
//!
//! Chunks, the stylesheet, the font faces — everything a page needs that
//! is not the page. They come out of [`vibe_doc_shell::Shell`], which
//! means out of the binary or out of the machine store, and never out of
//! a directory a request named: the address is checked to be a sequence
//! of ordinary names by the caller, and checked AGAIN by the shell before
//! it becomes a path. Two checks on purpose — this is the one place where
//! a request's bytes and the filesystem meet, and the cost of the second
//! one is a string walk.
//!
//! There is no directory listing and there is no index-on-directory: a
//! name either is a file the shell carries or is not.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-STATIC");

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::Reader;

/// The directories of the shell whose file names are content hashes —
/// the bundler's own output. A name that is a hash may be cached
/// forever, because a changed file is a changed name.
const IMMUTABLE_DIRS: [&str; 2] = ["assets/", "build/"];

/// One shell file, or `None` when the shell carries no such name.
pub(crate) fn asset(reader: &Reader, relative: &str) -> Option<Response> {
    let bytes = reader.shell.asset(relative)?;
    let cache = if IMMUTABLE_DIRS.iter().any(|dir| relative.starts_with(dir)) {
        "public, max-age=31536000, immutable"
    } else {
        "no-store"
    };
    Some(
        (
            StatusCode::OK,
            [
                (
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(content_type(relative)),
                ),
                (header::CACHE_CONTROL, HeaderValue::from_static(cache)),
            ],
            bytes,
        )
            .into_response(),
    )
}

/// What a file of the shell is, by the extension the build gave it.
///
/// A closed table rather than a sniffer: the shell is a build product of
/// one known package, so the set of things in it is known, and an
/// unfamiliar extension is served as bytes rather than guessed at. Every
/// response also carries `nosniff`, so a browser will not improve on this
/// answer either.
fn content_type(relative: &str) -> &'static str {
    let extension = relative.rsplit_once('.').map(|(_, ext)| ext).unwrap_or("");
    match extension {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_of_file_the_shell_carries_has_a_type() {
        assert_eq!(
            content_type("build/q-a.js"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(
            content_type("assets/a-style.css"),
            "text/css; charset=utf-8"
        );
        assert_eq!(content_type("assets/a-Inter.woff2"), "font/woff2");
        assert_eq!(content_type("favicon.svg"), "image/svg+xml");
        assert_eq!(
            content_type("assets/graph.json"),
            "application/json; charset=utf-8"
        );
        assert_eq!(content_type("odd"), "application/octet-stream");
    }

    #[test]
    fn only_the_bundlers_own_hashed_directories_are_cached_forever() {
        assert!(IMMUTABLE_DIRS.iter().any(|d| "build/q-a.js".starts_with(d)));
        assert!(!IMMUTABLE_DIRS.iter().any(|d| "fallback.css".starts_with(d)));
    }
}
