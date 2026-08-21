//! `load_spec_text` — the one dispatch every host consumer of spec sources
//! reads through (PROP-045 ##PROJECTION-READ, ##LOADER-LAW).
//!
//! The law: XML never reaches a scanner raw. A `.xml` spec entering any
//! consumer is first projected `from_xml → to_markdown` — deterministic and
//! canonical by S1's emitter — and the projection feeds the existing MD
//! machinery; units, facts, anchors, hashes and verdict staleness all work
//! unchanged. `load_spec_text` is that dispatch as one call, so no consumer
//! re-decides the extension test — and every diagnostic an XML source
//! produces is a *projection-relative* one, which is why the kind rides
//! back out with the text ([`SourceKind`]).
//!
//! The second law this module owns: one logical document, one form. `X.md`
//! and `X.xml` beside each other are the same document in two
//! serialisations — a mixed tree may hold a document in EITHER form, never
//! both — and discovering such a pair is a loud error naming both paths
//! ([`pair_collisions_in`], [`discover_pair_collision`]).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#loaders");

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::{from_xml, to_markdown};

/// How a loaded spec source reached the caller — the projection notice's
/// datum (PROP-045 ##PROJECTION-READ: a diagnostic for an XML source cites
/// projection-relative line numbers and must be marked as such, never left
/// pretending).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// Read verbatim — a `.md` source, or a non-spec text file a caller
    /// enumerated for its own reasons (a licence beside the specs). Line
    /// numbers are source-relative.
    Markdown,
    /// A `.xml` dialect source, delivered as its canonical Markdown
    /// projection. Line numbers are projection-relative.
    XmlProjected,
}

impl SourceKind {
    /// The projection notice, or `None` for a verbatim read. One string,
    /// shared by every consumer, so the mark never drifts between surfaces.
    pub fn projection_notice(self) -> Option<&'static str> {
        match self {
            SourceKind::Markdown => None,
            SourceKind::XmlProjected => Some(PROJECTION_NOTICE),
        }
    }
}

/// The suffix an [`SourceKind::XmlProjected`] read appends where it names
/// the path (PROP-045 ##PROJECTION-READ's recorded degradation).
pub const PROJECTION_NOTICE: &str = "(XML source; line numbers are projection-relative)";

/// A `load_spec_text` failure. The path is carried on every error — a
/// dialect violation names the file it sits in, not just the line.
#[derive(Debug)]
pub struct LoadError {
    pub path: PathBuf,
    pub message: String,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "reading `{}`: {}", self.path.display(), self.message)
    }
}

impl std::error::Error for LoadError {}

/// Whether `path` names a spec source by its extension — a Markdown or
/// dialect-XML document (the two serialisations PROP-045 ships).
pub fn is_spec_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md" | "xml")
    )
}

/// Read one spec source as the text its scanners consume.
///
/// * `.md` — verbatim, [`SourceKind::Markdown`].
/// * `.xml` — the canonical projection `from_xml → to_markdown`,
///   [`SourceKind::XmlProjected`]; a dialect violation is an error carrying
///   the path (the closed vocabulary is a contract, not a hint).
/// * any other extension — verbatim as [`SourceKind::Markdown`]: the
///   passthrough for non-spec text a caller enumerates beside the specs
///   (a licence, a README). Such a file is the caller's business; only the
///   two spec forms get semantics here.
pub fn load_spec_text(path: &Path) -> Result<(String, SourceKind), LoadError> {
    let raw = std::fs::read_to_string(path).map_err(|e| LoadError {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    if path.extension().and_then(|e| e.to_str()) == Some("xml") {
        let doc = from_xml(&raw).map_err(|e| LoadError {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        Ok((to_markdown(&doc), SourceKind::XmlProjected))
    } else {
        Ok((raw, SourceKind::Markdown))
    }
}

/// One logical document found in both serialisations — `X.md` and `X.xml`
/// beside each other. The mixed target holds each document in ONE form
/// (PROP-045 ##TARGET-MIXED); a pair is a split brain, and split brains are
/// reported, never resolved by guessing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairCollision {
    pub markdown: PathBuf,
    pub xml: PathBuf,
}

impl PairCollision {
    /// The loud message every consumer prints — one wording, so the law
    /// reads the same on every surface. Names both paths.
    pub fn message(&self) -> String {
        format!(
            "`{}` and `{}` are one logical document in two forms — one \
             document, one form (PROP-045 ##TARGET-MIXED); delete one of \
             the pair or rename one",
            self.markdown.display(),
            self.xml.display()
        )
    }
}

/// Find every `X.md` + `X.xml` pair among `paths` — the pure half of the
/// collision law, over an already-enumerated list (a scope's observed
/// files). Deterministic: sorted by the Markdown path.
///
/// Keyed on the full path minus extension, so `a/X.md` + `a/X.xml`
/// collides and `a/X.md` + `b/X.xml` does not. Files with other
/// extensions never collide. Carried paths are `/`-normalised — the
/// corpus's own report form — so the message reads the same on every
/// platform.
pub fn pair_collisions_in<I, P>(paths: I) -> Vec<PairCollision>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let norm = |p: &Path| PathBuf::from(p.to_string_lossy().replace('\\', "/"));
    let mut by_stem: BTreeMap<PathBuf, (Option<PathBuf>, Option<PathBuf>)> = BTreeMap::new();
    for p in paths {
        let p = p.as_ref();
        if !is_spec_source(p) {
            continue;
        }
        let slot = by_stem.entry(stem_of(p)).or_default();
        if p.extension().and_then(|e| e.to_str()) == Some("md") {
            slot.0 = Some(norm(p));
        } else {
            slot.1 = Some(norm(p));
        }
    }
    let mut out: Vec<PairCollision> = by_stem
        .into_iter()
        .filter_map(|(_, (md, xml))| {
            Some(PairCollision {
                markdown: md?,
                xml: xml?,
            })
        })
        .collect();
    out.sort_by(|a, b| a.markdown.cmp(&b.markdown));
    out
}

/// [`pair_collisions_in`] over a directory tree on disk — the discovery
/// half, for callers that walk (a boot directory). Only files are
/// considered; the walk is depth-first in directory order, and the result
/// is sorted exactly like the pure half's.
pub fn discover_pair_collision(dir: &Path) -> std::io::Result<Vec<PairCollision>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else if is_spec_source(&path) {
                files.push(path);
            }
        }
    }
    Ok(pair_collisions_in(files))
}

/// `path` without its final extension — the pair-collision key.
fn stem_of(path: &Path) -> PathBuf {
    let mut s = path.to_path_buf();
    s.set_extension("");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const NS: &str = "xmlns=\"https://vibevm.org/spec/1\"";

    #[test]
    fn markdown_reads_verbatim_and_xml_projects() {
        let dir = tempfile::tempdir().unwrap();
        let md = dir.path().join("a.md");
        std::fs::write(&md, "# T {#t}\n\n@fact:A text @status:impl/done\n").unwrap();
        let (text, kind) = load_spec_text(&md).unwrap();
        assert_eq!(kind, SourceKind::Markdown);
        assert!(text.starts_with("# T {#t}"), "verbatim: {text}");
        assert!(kind.projection_notice().is_none());

        let xml = dir.path().join("b.xml");
        std::fs::write(
            &xml,
            format!(
                "<spec {NS}>\n  <p><fact id=\"ONLY\" status=\"impl/done\">one</fact></p>\n</spec>"
            ),
        )
        .unwrap();
        let (text, kind) = load_spec_text(&xml).unwrap();
        assert_eq!(kind, SourceKind::XmlProjected);
        assert!(text.contains("@fact:ONLY"), "projection: {text}");
        assert_eq!(kind.projection_notice(), Some(PROJECTION_NOTICE));
        // and the projection is deterministic: two loads are byte-equal
        let again = load_spec_text(&xml).unwrap().0;
        assert_eq!(text, again);
    }

    #[test]
    fn a_dialect_error_carries_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let xml = dir.path().join("bad.xml");
        std::fs::write(&xml, "<spec><p>x</p></spec>").unwrap();
        let err = load_spec_text(&xml).unwrap_err();
        assert!(err.to_string().contains("bad.xml"), "path must ride: {err}");
        assert!(
            err.message.contains("xmlns"),
            "dialect message must ride: {err}"
        );
    }

    #[test]
    fn a_non_spec_extension_reads_verbatim() {
        let dir = tempfile::tempdir().unwrap();
        let misc = dir.path().join("NOTES.txt");
        std::fs::write(&misc, "plain\ntext\n").unwrap();
        let (text, kind) = load_spec_text(&misc).unwrap();
        assert_eq!(kind, SourceKind::Markdown);
        assert_eq!(text, "plain\ntext\n");
    }

    #[test]
    fn a_pair_collides_and_the_message_names_both_paths() {
        let cols = pair_collisions_in([
            "spec/one.md",
            "spec/one.xml",
            "spec/two.md",
            "spec/deep/three.xml",
        ]);
        assert_eq!(cols.len(), 1, "{cols:?}");
        assert_eq!(cols[0].markdown, Path::new("spec/one.md"));
        assert_eq!(cols[0].xml, Path::new("spec/one.xml"));
        let msg = cols[0].message();
        assert!(msg.contains("spec/one.md"), "{msg}");
        assert!(msg.contains("spec/one.xml"), "{msg}");
        assert!(msg.contains("one document, one form"), "{msg}");
        // Same stem in different directories is two documents, not a pair.
        assert!(pair_collisions_in(["a/X.md", "b/X.xml"]).is_empty());
    }

    #[test]
    fn discover_walks_the_tree_and_finds_the_pair() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("boot.md"), "# a\n").unwrap();
        std::fs::write(dir.path().join("boot.xml"), "<spec/>").unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/only.md"), "# b\n").unwrap();
        let cols = discover_pair_collision(dir.path()).unwrap();
        assert_eq!(cols.len(), 1, "{cols:?}");
        assert!(cols[0].message().contains("boot.md"));
    }
}
