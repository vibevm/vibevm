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
//! so no caller re-decides the extension test. The R7.5 A2a one-read
//! seam sits here: [`observe_authored_source`] reads every document
//! exactly once as raw bytes, witnesses each read as
//! [`SourceFileWitness`] metadata (no prose crosses the boundary), and
//! projects those same in-memory bytes through
//! [`vibe_specdoc::project_spec_text`]; [`scan_authored_facts`] is its
//! thin facts-only wrapper. The join side keeps the
//! four adoption observations four (R7.5 PROP-054
//! `##REQUIREMENT-OBSERVATION-AXES`): it reads the registry and nothing
//! else, writes nothing, and never collapses presence into a boolean.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#laws");

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
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

/// A content witness for one file a scan or registry load actually read —
/// R7.5 A2a's one-read law (PROP-054 `##FACT-QUERY-CONTRACT`, R7
/// architecture §5.2).
///
/// Metadata ONLY: the forward-slashed root-relative path, the exact byte
/// count and `sha256:` + 64 lowercase hex over those same raw bytes. It is
/// deliberately not a serde wire value and carries neither the raw nor the
/// projected text — the digest binds the bytes that were read once, so a
/// consumer can name what it observed without a second read and without
/// exporting prose across the crate boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFileWitness {
    /// Forward-slashed path relative to the root the walk answered for.
    pub path: String,
    /// Exact raw byte count of the one read.
    pub bytes: u64,
    /// `sha256:` + 64 lowercase hex over the exact raw bytes.
    pub digest: String,
}

impl SourceFileWitness {
    /// Witness raw bytes already read — the one-read construction. The
    /// digest spelling matches the crate's other content hashes
    /// ([`crate::overlay_file_hash`]).
    pub fn of(path: impl Into<String>, raw: &[u8]) -> Self {
        let digest = Sha256::digest(raw);
        Self {
            path: path.into(),
            bytes: raw.len() as u64,
            digest: format!("sha256:{digest:x}"),
        }
    }
}

/// The diagnostic metadata of a source defect: which file, which source
/// line (or the pivot's positionless zero), and a machine reason. No raw
/// or projected body rides as a field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceIssue {
    pub path: PathBuf,
    pub line: usize,
    pub message: String,
}

/// One authored source observed in a single read pass — the A2b seam.
///
/// Valid and invalid are mutually exclusive IN THE TYPE: an invalid
/// source always emits zero fact rows, so no `Result` member can hold
/// facts beside an issue. Both variants carry every sorted document
/// witness for the enumerated source — including both raw documents of a
/// same-stem pair — because the read/witness pass completes before any
/// parse is attempted; a collision or parse defect therefore never mints
/// a fake empty-set digest over files that were read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoredSourceObservation {
    /// The source read and parsed; its addressed facts, sorted by full
    /// address, beside the witnesses of the documents they came from.
    Available {
        facts: Vec<AuthoredFact>,
        documents: Vec<SourceFileWitness>,
    },
    /// Present bytes whose authored source cannot be trusted (invalid
    /// UTF-8, a same-stem split brain, a Markdown/XML parse or dialect
    /// failure, or a duplicate full address). Zero facts, all witnesses,
    /// one bounded issue.
    Invalid {
        documents: Vec<SourceFileWitness>,
        issue: SourceIssue,
    },
}

/// Observe one authored source in ONE read pass: enumerate and sort the
/// documents, read every document exactly once as raw bytes and witness
/// it, then UTF-8/project/parse those same in-memory bytes through the
/// [`vibe_specdoc::project_spec_text`] pivot and collect the addressed
/// facts.
///
/// The outer [`Err`] is authority/I/O/unavailability — an invalid
/// coordinate, an enumeration failure or a raw read failure (a file that
/// could not be read at all). Everything present bytes say when they
/// cannot be trusted as an authored source is the
/// [`AuthoredSourceObservation::Invalid`] variant, never an [`Err`]: a
/// malformed present source is named in its own observation, not turned
/// into a lifecycle failure.
pub fn observe_authored_source(
    source_root: &Path,
    package_coordinate: &str,
    source_kind: SourceKind,
) -> Result<AuthoredSourceObservation, RegistryError> {
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

    // The ONE raw read/witness pass — every enumerated document is read
    // exactly once, before any interpretation, so an invalid outcome
    // still carries the witnesses of the bytes that were there.
    let invalid = |documents: Vec<SourceFileWitness>, issue: SourceIssue| {
        AuthoredSourceObservation::Invalid { documents, issue }
    };
    let mut witnesses: Vec<SourceFileWitness> = Vec::with_capacity(documents.len());
    let mut raws: Vec<(String, Vec<u8>)> = Vec::with_capacity(documents.len());
    for document in &documents {
        let path = source_root.join(document);
        let raw = fs::read(&path).map_err(|source| RegistryError::Io {
            path: path.clone(),
            source,
        })?;
        witnesses.push(SourceFileWitness::of(document, &raw));
        raws.push((document.clone(), raw));
    }

    // PROP-045's one-document/one-form law, through the shared pivot's
    // own collision finder — after the read pass, so both halves of the
    // split brain are witnessed. The first sorted collision names both
    // paths; no body text rides out.
    if let Some(collision) = vibe_specdoc::pair_collisions_in(&documents)
        .into_iter()
        .next()
    {
        return Ok(invalid(
            witnesses,
            SourceIssue {
                path: source_root.join(&collision.markdown),
                line: 1,
                message: collision.message(),
            },
        ));
    }

    let mut authored = Vec::new();
    let mut minter_of: BTreeMap<String, String> = BTreeMap::new();
    for (document, raw) in raws {
        let prefix = address_prefix(package_coordinate, &document)?;
        let path = source_root.join(&document);
        // UTF-8 first, then the pivot's one extension dispatch over the
        // already-owned text — no second read, no caller-side md/xml
        // branch. A UTF-8 failure names present-but-untrustworthy bytes.
        let text = match String::from_utf8(raw) {
            Ok(text) => text,
            Err(error) => {
                return Ok(invalid(
                    witnesses,
                    SourceIssue {
                        path: path.clone(),
                        line: 1,
                        message: format!("invalid UTF-8: {error}"),
                    },
                ));
            }
        };
        let (projected, _kind) = match vibe_specdoc::project_spec_text(&path, &text) {
            Ok(projected) => projected,
            Err(error) => {
                return Ok(invalid(
                    witnesses,
                    SourceIssue {
                        path: path.clone(),
                        line: 1,
                        message: error.message,
                    },
                ));
            }
        };
        let doc = match vibe_specdoc::from_markdown(&projected) {
            Ok(doc) => doc,
            Err(error) => {
                return Ok(invalid(
                    witnesses,
                    SourceIssue {
                        path: path.clone(),
                        line: error.line,
                        message: error.message,
                    },
                ));
            }
        };
        for fact in authored_facts(&doc, &prefix) {
            if let Some(first) = minter_of.insert(fact.address.clone(), document.clone()) {
                return Ok(invalid(
                    witnesses,
                    SourceIssue {
                        path,
                        line: 1,
                        message: format!(
                            "duplicate full fact address `{}` (also minted by `{first}`)",
                            fact.address
                        ),
                    },
                ));
            }
            authored.push(fact);
        }
    }
    authored.sort_by(|a, b| a.address.cmp(&b.address));
    Ok(AuthoredSourceObservation::Available {
        facts: authored,
        documents: witnesses,
    })
}

/// Scan one source root for every authored fact it addresses.
///
/// `source_root` is the root the layout resolves against — the project
/// root for [`SourceKind::Host`], the slot root for
/// [`SourceKind::Package`]. The coordinate is validated before anything
/// else is read. Documents are the `.md`/`.xml` files under the current specs
/// root (a package adds a root `README.md`/`README.xml`), minus the
/// generated boot lane (`INDEX.md`, `STATIC.md`, `STATIC.xml`); both
/// forms load through the [`vibe_specdoc::load_spec_text`] pivot, and an
/// explicit symlink is never followed as a document. One logical document
/// lives in one form: a same-stem Markdown/XML pair refuses through
/// [`vibe_specdoc::pair_collisions_in`] before either form is parsed,
/// whatever its anchors. The result is sorted by full address, and a
/// duplicate full address refuses rather than choosing a document.
///
/// A thin wrapper over [`observe_authored_source`]: an invalid source
/// converts back to the typed [`RegistryError::SpecParse`] this API has
/// always returned, so the A1 CLI behavior is unchanged.
pub fn scan_authored_facts(
    source_root: &Path,
    package_coordinate: &str,
    source_kind: SourceKind,
) -> Result<Vec<AuthoredFact>, RegistryError> {
    match observe_authored_source(source_root, package_coordinate, source_kind)? {
        AuthoredSourceObservation::Available { facts, .. } => Ok(facts),
        AuthoredSourceObservation::Invalid { issue, .. } => Err(RegistryError::SpecParse {
            path: issue.path,
            line: issue.line,
            message: issue.message,
        }),
    }
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
