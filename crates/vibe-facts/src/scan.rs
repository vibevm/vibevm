//! Dependency-clean authored-fact scanning and the pure adoption join.
//!
//! The third caller of the authored-fact traversal (the CLI package
//! `adopt`/`report` pair) lived as private walk-and-parse code in
//! `vibe-cli`; this module is its shared home. The scanner answers one
//! frozen question — *which full `spec://…#anchor` addresses does this
//! source root author, and with what status?* — from public `vibe-core`
//! layout paths alone, so `vibe-facts` keeps no `vibe-workspace` edge and
//! in-place slots (which carry no slot record) scan exactly like
//! materialised ones: materialisation only ever rewrites a document's
//! extension, and [`vibe_spec::canonical_doc_path`] strips it, so source
//! and output are one address.
//!
//! Reading goes through the PROP-045 pivot —
//! [`vibe_specdoc::load_spec_text`] projects an XML source to its
//! canonical Markdown before [`vibe_specdoc::from_markdown`] sees it —
//! so no caller re-decides the extension test. The join side keeps the
//! four adoption observations four (R7.5 PROP-054
//! `##REQUIREMENT-OBSERVATION-AXES`): it reads the registry and nothing
//! else, writes nothing, and never collapses presence into a boolean.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#laws");

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

use vibe_core::layout;
use vibe_core::{Group, PackageName};

use crate::{AuthoredFact, FactStatus, Registry, RegistryError, authored_facts};

/// Which kind of source root a scan walks.
///
/// ```text
/// Host    the current project: the specs root only
/// Package an installed slot: the specs root plus a root README
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Host,
    Package,
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Host => "host",
            Self::Package => "package",
        })
    }
}

/// One joined observation of consumer adoption for a single authored
/// address — exactly the four words, never a rollup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdoptionObservation {
    /// The address belongs to the host source: the consumer registry is
    /// not an adoption overlay for it, whatever it contains.
    NotApplicable,
    /// No registry entry exists at the address.
    Absent,
    /// An entry exists but records no status.
    Indeterminate,
    /// An entry exists and carries exactly this status.
    Recorded(FactStatus),
}

/// One per-address row of the adoption join: the authored address, the
/// status its source document claims, and the adoption observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptionRow {
    pub address: String,
    pub authored_status: Option<FactStatus>,
    pub adoption: AdoptionObservation,
}

/// The `spec://<group>/<name>/<doc>#` prefix every full fact address of
/// one authored document begins with — the router's canonical form
/// (extension stripped, `PROP-NNN`/`FEAT-NNN` slug truncated), with the
/// coordinate validated through `vibe-core` so a merely slash-shaped id
/// can never be minted.
///
/// ```
/// use vibe_facts::address_prefix;
///
/// assert_eq!(
///     address_prefix("org.example/demo", "spec/common/PROP-099-slug.md").unwrap(),
///     "spec://org.example/demo/common/PROP-099#"
/// );
/// assert_eq!(
///     address_prefix("org.example/demo", "vibevm/vibespecs/common/RULE.xml").unwrap(),
///     "spec://org.example/demo/common/RULE#"
/// );
/// assert!(address_prefix("org.example", "RULE.md").is_err());
/// assert!(address_prefix("org.example/Demo", "RULE.md").is_err());
/// ```
pub fn address_prefix(
    package_coordinate: &str,
    document_rel: &str,
) -> Result<String, RegistryError> {
    let (group, name) = split_coordinate(package_coordinate)?;
    Ok(format!(
        "spec://{group}/{name}/{}#",
        vibe_spec::canonical_doc_path(document_rel)
    ))
}

/// Validate a `<group>/<name>` source coordinate through the closed
/// `vibe-core` grammars, returning its two halves.
fn split_coordinate(coordinate: &str) -> Result<(&str, &str), RegistryError> {
    let invalid = |reason: String| RegistryError::InvalidCoordinate {
        coordinate: coordinate.to_string(),
        reason,
    };
    let Some((group, name)) = coordinate.split_once('/') else {
        return Err(invalid(
            "coordinate must use the `<group>/<name>` form".to_string(),
        ));
    };
    if name.is_empty() || name.contains('/') {
        return Err(invalid(
            "coordinate must use the `<group>/<name>` form".to_string(),
        ));
    }
    if let Err(source) = Group::parse(group) {
        return Err(invalid(source.to_string()));
    }
    if let Err(source) = PackageName::parse(name) {
        return Err(invalid(source.to_string()));
    }
    Ok((group, name))
}

/// Scan one source root for every authored fact it addresses.
///
/// `source_root` is the root the layout resolves against — the project
/// root for [`SourceKind::Host`], the slot root for
/// [`SourceKind::Package`]. The coordinate is validated before anything
/// is read. Documents are the `.md`/`.xml` files under the current specs
/// root (a package adds a root `README.md`/`README.xml`), minus the
/// generated boot lane (`INDEX.md`, `STATIC.md`, `STATIC.xml`); both
/// forms load through [`vibe_specdoc::load_spec_text`], and an explicit
/// symlink is never followed as a document. One logical document lives
/// in one form: a same-stem Markdown/XML pair refuses through
/// [`vibe_specdoc::pair_collisions_in`] before either form is parsed,
/// whatever its anchors. The result is sorted by full address, and a
/// duplicate full address refuses rather than choosing a document.
pub fn scan_authored_facts(
    source_root: &Path,
    package_coordinate: &str,
    source_kind: SourceKind,
) -> Result<Vec<AuthoredFact>, RegistryError> {
    split_coordinate(package_coordinate)?;
    let mut documents: Vec<String> = Vec::new();
    let specs_root = source_root.join(layout::current_specs_root());
    collect_spec_documents(source_root, &specs_root, &mut documents)?;
    if matches!(source_kind, SourceKind::Package) {
        for readme in ["README.md", "README.xml"] {
            if is_plain_file(&source_root.join(readme)) {
                documents.push(readme.to_string());
            }
        }
    }
    documents.sort();

    // PROP-045's one-document/one-form law, through the shared pivot's
    // own collision finder: a split brain refuses even when the two
    // forms carry disjoint anchors. The first sorted collision names
    // both paths; no body text rides out.
    if let Some(collision) = vibe_specdoc::pair_collisions_in(&documents)
        .into_iter()
        .next()
    {
        return Err(RegistryError::SpecParse {
            path: source_root.join(&collision.markdown),
            line: 1,
            message: collision.message(),
        });
    }

    let mut authored = Vec::new();
    let mut minter_of: BTreeMap<String, String> = BTreeMap::new();
    for document in &documents {
        let prefix = address_prefix(package_coordinate, document)?;
        let path = source_root.join(document);
        let (text, _kind) =
            vibe_specdoc::load_spec_text(&path).map_err(|error| RegistryError::SpecParse {
                path: path.clone(),
                line: 1,
                message: error.message,
            })?;
        let doc = vibe_specdoc::from_markdown(&text).map_err(|error| RegistryError::SpecParse {
            path: path.clone(),
            line: error.line,
            message: error.message,
        })?;
        for fact in authored_facts(&doc, &prefix) {
            if let Some(first) = minter_of.insert(fact.address.clone(), document.clone()) {
                return Err(RegistryError::SpecParse {
                    path,
                    line: 1,
                    message: format!(
                        "duplicate full fact address `{}` (also minted by `{first}`)",
                        fact.address
                    ),
                });
            }
            authored.push(fact);
        }
    }
    authored.sort_by(|a, b| a.address.cmp(&b.address));
    Ok(authored)
}

/// Walk the specs root, collecting spec documents relative to the source
/// root as `/`-separated paths, excluding the generated boot lane.
fn collect_spec_documents(
    source_root: &Path,
    dir: &Path,
    out: &mut Vec<String>,
) -> Result<(), RegistryError> {
    if !dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|source| RegistryError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| RegistryError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        // An explicit link is not a document of this root — never follow
        // it, never error on it (junction/reparse policy stays unreviewed
        // out of this atom; skipping a link is the conservative floor).
        let file_type = entry.file_type().map_err(|source| RegistryError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            collect_spec_documents(source_root, &path, out)?;
        } else if vibe_specdoc::is_spec_source(&path) {
            let relative = path.strip_prefix(source_root).map_err(|_| {
                RegistryError::Invariant(format!(
                    "walked path `{}` escaped `{}`",
                    path.display(),
                    source_root.display()
                ))
            })?;
            let document = vibe_core::machine_json_path(relative);
            if !is_generated_boot_artifact(&document) {
                out.push(document);
            }
        }
    }
    Ok(())
}

/// The generated boot lane — every file the compiler owns under
/// `boot/`, re-expressed from the public `vibe-core` layout (the two
/// `STATIC` spellings and the boot INDEX; `INLINE.*` shares the dir but
/// is not a spec document a consumer may author).
fn is_generated_boot_artifact(document: &str) -> bool {
    document == vibe_core::machine_json_path(&layout::current_boot_index())
        || document == vibe_core::machine_json_path(&layout::current_boot_static_md())
        || document == vibe_core::machine_json_path(&layout::current_boot_static_xml())
}

/// A regular file that is not itself a link.
fn is_plain_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| !metadata.file_type().is_symlink() && metadata.is_file())
        .unwrap_or(false)
}

/// Join authored rows with the consumer registry, one address at a time.
///
/// Host rows are [`AdoptionObservation::NotApplicable`] regardless of
/// registry content; package rows observe `Absent | Indeterminate |
/// Recorded(exact status)` by what the registry holds at the address.
/// The join is pure — it reads, never writes — and its rows come back in
/// deterministic address order, whatever order the authored input carried.
///
/// ```
/// use vibe_facts::{AdoptionObservation, AuthoredFact, Registry, SourceKind, join_adoption};
///
/// let authored = [AuthoredFact {
///     address: "spec://org.example/pkg/RULE#A".to_string(),
///     status: None,
/// }];
/// let rows = join_adoption(&Registry::default(), "org.example/pkg", SourceKind::Package, &authored).unwrap();
/// assert_eq!(rows[0].adoption, AdoptionObservation::Absent);
///
/// let rows = join_adoption(&Registry::default(), "org.example/pkg", SourceKind::Host, &authored).unwrap();
/// assert_eq!(rows[0].adoption, AdoptionObservation::NotApplicable);
/// ```
pub fn join_adoption(
    registry: &Registry,
    package_coordinate: &str,
    source_kind: SourceKind,
    authored: &[AuthoredFact],
) -> Result<Vec<AdoptionRow>, RegistryError> {
    split_coordinate(package_coordinate)?;
    let mut rows: Vec<AdoptionRow> = authored
        .iter()
        .map(|fact| AdoptionRow {
            address: fact.address.clone(),
            authored_status: fact.status,
            adoption: match source_kind {
                SourceKind::Host => AdoptionObservation::NotApplicable,
                SourceKind::Package => match registry.get(&fact.address) {
                    None => AdoptionObservation::Absent,
                    Some(entry) => match entry.status {
                        None => AdoptionObservation::Indeterminate,
                        Some(status) => AdoptionObservation::Recorded(status),
                    },
                },
            },
        })
        .collect();
    rows.sort_by(|a, b| a.address.cmp(&b.address));
    Ok(rows)
}
