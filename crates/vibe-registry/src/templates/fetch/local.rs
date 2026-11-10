//! Reading a template document the Locator put on this filesystem
//! (PROP-066 `#locator`, `#drops`).
//!
//! A template declares its two documents as credential-free HTTPS URLs —
//! `#TEMPLATE-DIGEST-SOURCE` refuses anything else, and `vibe check`
//! enforces it. **A closed network does not change the declaration; it
//! changes where the bytes are found**, with a `[[remap]]` row it already
//! uses for every other artifact. A `path` target, and a `url` target
//! spelled `file://`, therefore both reach the template layer, and both
//! have to be read off disk rather than handed to an HTTP client — which
//! is also what makes `#MIRROR-TEMPLATE-SNAPSHOT`'s "a client inside the
//! closed network renders a selected version locally from that snapshot
//! with the same code as outside" literally true.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#locator");

use std::path::{Path, PathBuf};

/// The local file a chosen location names, or `None` when the location is
/// a network URL the HTTP client must fetch.
///
/// `declared` is the template's own URL, whose **last path segment** names
/// the file inside a directory target — the same convention
/// `PROP-066#drops` reads a drop by, and the only one under which a
/// mirrored directory of `SHASUMS256-<version>.txt` files is findable.
///
/// ```
/// use std::path::Path;
/// use vibe_registry::templates::local_file;
///
/// // A network URL is not ours.
/// assert!(local_file("https://nodejs.org/dist/index.json", "https://nodejs.org/dist/index.json").is_none());
/// // A `file://` url target, with and without a host part.
/// assert_eq!(
///     local_file("file:///srv/mirror/index.json", "https://n.invalid/index.json"),
///     Some(Path::new("/srv/mirror/index.json").to_path_buf())
/// );
/// // A bare path target, which is what a `path = "…"` row produces.
/// assert_eq!(
///     local_file("/srv/mirror/index.json", "https://n.invalid/index.json"),
///     Some(Path::new("/srv/mirror/index.json").to_path_buf())
/// );
/// ```
pub fn local_file(chosen: &str, declared: &str) -> Option<PathBuf> {
    let path = if let Some(rest) = chosen.strip_prefix("file://") {
        // `file:///abs` and the Windows `file://C:/…` spelling both land
        // here; the first keeps its leading `/`, the second has none.
        PathBuf::from(if rest.starts_with('/') || rest.contains(':') {
            rest
        } else {
            return None;
        })
    } else if chosen.contains("://") {
        // Any other scheme is the HTTP client's business.
        return None;
    } else {
        PathBuf::from(chosen)
    };
    Some(if path.is_dir() {
        path.join(file_name_of(declared))
    } else {
        path
    })
}

/// The last path segment of a declared URL — the file name a directory
/// target is searched by.
fn file_name_of(declared: &str) -> &str {
    let without_query = declared
        .split_once('?')
        .map_or(declared, |(before, _)| before);
    without_query
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or(without_query)
}

/// Read one local template document.
pub fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}
