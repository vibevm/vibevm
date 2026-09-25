//! The `[navigation]` half of the card reader — what a documentation
//! package says about the order its own pages are shown and read in
//! (PROP-057 `##NAV-PINNED`, `##NAV-CHAPTERS`).
//!
//! Out of line from the manifest build per the file-length budget, and
//! along a real seam: everything here reads ONE table, as data, and
//! applies it to nothing. Where a pinned page stands and which chapter a
//! reader meets it in are the reader's business — this only carries the
//! declaration onto the wire, whole and in the order it was written.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED");

use std::path::Path;

use vibe_wire::generated::doc_manifest::{Navigation, NavigationChapter, NavigationSection};

use crate::error::{DocError, Result};

/// Read `[navigation]` as the wire carries it: the pinned paths in the
/// order they were written, one row per named section, and the learning
/// path chapter by chapter.
///
/// Read as data like the rest of the card, and `None` when the package
/// says nothing — which is the state of every documentation written
/// before the table existed, and the state the site renders as «the
/// pages in the order the manifest gives them».
///
/// The chapters are carried ONLY when the package declared them, and the
/// absence is a different answer from an empty list: absent is «this
/// documentation declared no path», which the reader shows exactly as it
/// showed everything before the rows existed; empty is a package that
/// opened the table and named no chapter (`##NAV-CHAPTERS`).
pub(super) fn read(parsed: &toml::Value) -> Option<Navigation> {
    let table = parsed.get("navigation")?;
    let strings = |value: Option<&toml::Value>| -> Vec<String> {
        value
            .and_then(toml::Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let sections = table
        .get("section")
        .and_then(toml::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    Some(NavigationSection {
                        id: row.get("id").and_then(toml::Value::as_str)?.to_owned(),
                        title: row.get("title").and_then(toml::Value::as_str)?.to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let chapters = table
        .get("chapter")
        .and_then(toml::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    Some(NavigationChapter {
                        id: row.get("id").and_then(toml::Value::as_str)?.to_owned(),
                        title: row.get("title").and_then(toml::Value::as_str)?.to_owned(),
                        // A translation's row names the chapter and lists
                        // nothing, and that arrives here as the empty
                        // list the wire writes for it.
                        pages: strings(row.get("pages")),
                        appendix: row
                            .get("appendix")
                            .and_then(toml::Value::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect()
        });
    Some(Navigation {
        pinned: strings(table.get("pinned")),
        sections,
        chapters,
    })
}

/// The learning path a documentation package declares, straight off its
/// manifest, or an empty list when it declares none.
///
/// The narrow door beside [`relations`], and for the same reason: the
/// translation check asks what chapters two packages name, and making it
/// build a whole card first would turn a missing `title` in the source
/// into a defect of the adaptation.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// std::fs::write(
///     dir.path().join("vibe.toml"),
///     "[navigation]\n\n[[navigation.chapter]]\nid = \"start\"\n\
///      title = \"Getting started\"\npages = [\"start/index\"]\n",
/// )
/// .unwrap();
///
/// let path = vibe_doc::manifest::chapters(dir.path()).unwrap();
/// assert_eq!(path.len(), 1);
/// assert_eq!(path[0].id, "start");
/// assert_eq!(path[0].pages, vec!["start/index".to_owned()]);
/// ```
pub fn chapters(package_dir: &Path) -> Result<Vec<NavigationChapter>> {
    let path = package_dir.join(crate::derived::manifest::MANIFEST);
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::io("reading", &path, e))?;
    let parsed: toml::Value = toml::from_str(&text)
        .map_err(|e| DocError::manifest(&path, format!("does not parse: {e}")))?;
    Ok(read(&parsed)
        .and_then(|navigation| navigation.chapters)
        .unwrap_or_default())
}
