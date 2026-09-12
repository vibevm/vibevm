//! The surface snapshot and the version diff — the pseudo-history of
//! versions (PROP-057 `##OBS-SURFACE-SNAPSHOTS`, `##OBS-VERSION-CONTRACT`,
//! `MAINTENANCE.md` §2.5).
//!
//! A version is a behavioural contract, not a frozen set of files. Inside
//! one, the product changes as often as it likes — amend, rewritten
//! history, ten releases in a day — and that is invisible by design. So
//! this module refuses every question that would need a history: there is
//! no hash anywhere in a snapshot, no build date, no state identifier, no
//! «how far behind». The one thing it keys on is the version NUMBER the
//! owner declared, because that number is the only event by which this
//! project computes a difference between versions.
//!
//! What a snapshot holds is the SURFACE: the command tree with its flags,
//! the keys the manifest and the lock file accept, the members of every
//! published schema, the text of every documentation obligation, and the
//! format registry. All of it read from the product itself — the help a
//! reader would read, the parser a manifest actually meets — so the
//! snapshot states what the product does rather than what its source is
//! called.
//!
//! ## What it is FOR
//!
//! Not archaeology. The point is to name the pages that must be updated
//! when the contract changes, so a writer edits a list instead of
//! re-reading the manual ([`diff`]). Every reason is one of three
//! relations the package already has with the product: a page cites a
//! rule, a page derives its text from a command or a schema, or a page
//! owes an audience a promise. A change that reaches none of them is not
//! dropped — it is reported as a page that does not exist yet.
//!
//! ## What never leaves the kitchen
//!
//! Snapshots live in `maintenance/surface/` of a documentation package,
//! a directory the site and the `llms` files do not render. A reader sees
//! a version number and a contract; the human changelog between versions
//! is written by hand from the diff's output (`##OBS-NOTHING-LEAKS`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-SURFACE-SNAPSHOTS");

pub mod commands;
pub mod diff;
pub mod fields;
pub mod schemas;

use std::path::{Path, PathBuf};

use vibe_wire::generated::doc_surface::{DocSurface, SurfaceFact};

use crate::coverage::Obligation;
use crate::error::{DocError, Result};

/// The schema version a snapshot written today carries.
pub const SCHEMA_VERSION: u32 = 1;

/// Where a documentation package keeps its snapshots, relative to its
/// root. The site does not render this directory and neither do the
/// `llms` files: it is the documentation developers' own workbench.
pub const DIRECTORY: &str = "maintenance/surface";

/// What recording a surface needs from its caller.
///
/// Every one of these is named rather than discovered, for the reason the
/// rest of this crate names its inputs: a snapshot whose content depended
/// on an unstated ambient value would be a snapshot nobody could
/// reproduce.
#[derive(Debug, Clone)]
pub struct SurfaceEnv {
    /// The built binary whose `--help` tree is the command surface.
    pub binary: PathBuf,
    /// The tree that holds the schemas and the format registry.
    pub repo_root: PathBuf,
    /// Seconds one `--help` may take before it is killed.
    pub timeout_secs: u64,
}

/// Record the surface of the product `env` names, under the version
/// number the caller declares.
///
/// `obligations` arrive from the caller for the same reason the coverage
/// gate takes them that way: which files the project observes is decided
/// once, in the grounding cell every `vibe facts` verb enters through, and
/// a second enumeration here would give the snapshot a corpus nobody else
/// can see.
///
/// The version is written down exactly as it was handed in. Nothing in
/// this function reads a version from anywhere: the product cannot tell
/// one amend of a version from another, and a snapshot that guessed its
/// own number would be claiming a history the project does not keep.
pub fn record(version: &str, obligations: &[Obligation], env: &SurfaceEnv) -> Result<DocSurface> {
    let commands = commands::tree(env)?;
    let (schemas, formats) = schemas::read(&env.repo_root)?;
    Ok(DocSurface {
        schema_version: SCHEMA_VERSION,
        version: version.to_owned(),
        commands,
        manifest_fields: fields::manifest_fields(),
        lock_fields: fields::lock_fields(),
        schemas,
        facts: facts(obligations),
        formats,
    })
}

/// The obligations as the snapshot carries them: the address, the
/// audiences in vocabulary order, and the TEXT.
///
/// The text and not a hash of it. A diff whose only possible sentence was
/// «this rule changed» would send a writer back to read the corpus; with
/// the text on both sides, the diff can say what it now says.
fn facts(obligations: &[Obligation]) -> Vec<SurfaceFact> {
    let mut out: Vec<SurfaceFact> = obligations
        .iter()
        .map(|o| SurfaceFact {
            address: o.address.clone(),
            audiences: o.audiences.iter().map(|a| a.as_str().to_owned()).collect(),
            text: collapse(&o.text),
        })
        .collect();
    out.sort_by(|a, b| a.address.cmp(&b.address));
    out.dedup_by(|a, b| a.address == b.address);
    out
}

/// One line of whitespace-collapsed text.
///
/// A fact's body is authored prose, and prose is re-wrapped by whoever
/// edits it. Comparing the wrapping would report a change every time
/// somebody reflowed a paragraph, which is the class of false alarm that
/// teaches a team to ignore a report.
pub(crate) fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Where the snapshot of one version sits inside a package.
pub fn path_for(package_dir: &Path, version: &str) -> PathBuf {
    package_dir.join(DIRECTORY).join(format!("{version}.json"))
}

/// The versions a package holds snapshots for, sorted by their file name.
///
/// A missing directory is «no snapshot yet», which is the state of every
/// package before its first version bump — not a defect.
pub fn recorded(package_dir: &Path) -> Result<Vec<String>> {
    let dir = package_dir.join(DIRECTORY);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| DocError::io("reading", &dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| DocError::io("reading", &dir, e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            out.push(stem.to_owned());
        }
    }
    out.sort();
    Ok(out)
}

/// Read one snapshot.
pub fn read(path: &Path) -> Result<DocSurface> {
    let text = std::fs::read_to_string(path).map_err(|e| DocError::io("reading", path, e))?;
    serde_json::from_str(&text).map_err(|e| DocError::Surface {
        message: format!("`{}` is not a surface snapshot: {e}", path.display()),
    })
}

/// Write one snapshot, pretty and newline-terminated so a review of it
/// reads as a diff of lines rather than of one enormous line.
pub fn write(surface: &DocSurface, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DocError::io("creating", parent, e))?;
    }
    std::fs::write(path, to_json(surface)).map_err(|e| DocError::io("writing", path, e))
}

/// A snapshot as its file holds it.
///
/// ```
/// use vibe_wire::generated::doc_surface::DocSurface;
///
/// let surface = DocSurface {
///     schema_version: vibe_doc::surface::SCHEMA_VERSION,
///     version: "1.0.0".into(),
///     commands: Vec::new(),
///     manifest_fields: vec!["package".into()],
///     lock_fields: Vec::new(),
///     schemas: Vec::new(),
///     facts: Vec::new(),
///     formats: Vec::new(),
/// };
/// let text = vibe_doc::surface::to_json(&surface);
/// assert!(text.ends_with('\n'));
/// assert!(text.contains("\"version\": \"1.0.0\""));
/// ```
pub fn to_json(surface: &DocSurface) -> String {
    // The type is generated from the schema and holds only JSON scalars
    // and sequences, so there is no serialisable state that can fail
    // here; a fallible signature would push an impossible arm onto every
    // caller.
    let mut text = serde_json::to_string_pretty(surface).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests;
