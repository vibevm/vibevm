//! Reading a documentation package's pages — the library's entry to
//! content (PROP-057 `##PIPE-LIBRARY`).
//!
//! A documentation package keeps its pages under `vibevm/vibespecs/`, the
//! same place every other package keeps its specs, and they are read
//! through the pivot with the documentation vocabulary open
//! (PROP-045 `##DOC-VOCAB-BY-KIND`). This crate never parses XML itself:
//! the mapping «package kind `doc` → the genre» is exactly the caller's
//! job the pivot refuses to do, and doing it here is the whole reason
//! this module exists.
//!
//! A page that will not parse is NOT a reason to stop reading the rest.
//! The corpus was authored before a reader existed for it, and a check
//! that aborts on the first defect reports one defect per run; one that
//! collects them reports all of them once. So the walk returns both
//! halves and the surfaces above decide what a refusal costs.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY");

use std::path::{Path, PathBuf};

use vibe_specdoc::doc::{SpecDoc, Vocabulary};

use crate::error::{DocError, Result};

/// Where a package keeps the text it publishes, relative to its root.
pub const SPEC_ROOT: &str = "vibevm/vibespecs";

/// One page that parsed: its address inside the package, its path on disk
/// and the document itself.
#[derive(Debug, Clone)]
pub struct Page {
    /// The page's address inside the package — a `/`-separated path under
    /// [`SPEC_ROOT`], e.g. `start/first-project.xml`. Platform separators
    /// never reach it, so a report reads the same on every OS.
    pub rel: String,
    /// The absolute path the page was read from.
    pub path: PathBuf,
    /// The parsed document.
    pub doc: SpecDoc,
}

/// One page that did NOT parse, with the refusal that stopped it. The
/// message is the pivot's own, so the author sees the line and the
/// construct, not a summary of them.
#[derive(Debug, Clone)]
pub struct UnreadablePage {
    pub rel: String,
    pub path: PathBuf,
    pub message: String,
}

/// Everything under a package's spec root, split by whether it parsed.
#[derive(Debug, Clone, Default)]
pub struct PageSet {
    pub pages: Vec<Page>,
    pub unreadable: Vec<UnreadablePage>,
}

impl PageSet {
    /// How many pages the walk found, readable or not.
    pub fn total(&self) -> usize {
        self.pages.len() + self.unreadable.len()
    }
}

/// Walk `<package>/vibevm/vibespecs/**/*.xml` and read every page through
/// the documentation vocabulary, in a stable order (the address, sorted),
/// so two runs of any check over an unchanged package report in the same
/// sequence.
///
/// A missing spec root is not an error: a package may legitimately carry
/// no pages yet, and an empty walk says so more usefully than a refusal.
///
/// ```
/// use std::fs;
/// let tmp = tempfile::tempdir().unwrap();
/// let pkg = tmp.path();
/// let dir = pkg.join("vibevm/vibespecs/start");
/// fs::create_dir_all(&dir).unwrap();
/// fs::write(
///     dir.join("hello.xml"),
///     "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
///      <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
///        <title id=\"root\">Hello</title>\n  \
///        <example id=\"v\" fixture=\"none\">\n    \
///          <run>vibe --version</run>\n    \
///          <expect>vibe 1.0.0</expect>\n  \
///        </example>\n\
///      </spec>\n",
/// )
/// .unwrap();
///
/// let set = vibe_doc::pages::read_package(pkg).unwrap();
/// assert_eq!(set.pages.len(), 1);
/// assert_eq!(set.pages[0].rel, "start/hello.xml");
/// assert!(set.unreadable.is_empty());
/// ```
pub fn read_package(package_dir: &Path) -> Result<PageSet> {
    let root = package_dir.join(SPEC_ROOT);
    if !root.is_dir() {
        return Ok(PageSet::default());
    }
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    for entry in walkdir::WalkDir::new(&root).sort_by_file_name() {
        let entry = entry.map_err(|e| {
            DocError::io(
                "walking",
                root.clone(),
                e.into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("walk failed")),
            )
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if path.extension().and_then(|e| e.to_str()) != Some("xml") {
            continue;
        }
        let Ok(rel) = path.strip_prefix(&root) else {
            continue;
        };
        let rel = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        found.push((rel, path));
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));

    let mut set = PageSet::default();
    for (rel, path) in found {
        let text = std::fs::read_to_string(&path).map_err(|e| DocError::io("reading", &path, e))?;
        match vibe_specdoc::from_xml_with(&text, Vocabulary::Doc) {
            Ok(doc) => set.pages.push(Page { rel, path, doc }),
            Err(e) => set.unreadable.push(UnreadablePage {
                rel,
                path,
                message: e.to_string(),
            }),
        }
    }
    Ok(set)
}

#[cfg(test)]
mod tests;
