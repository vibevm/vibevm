//! The `derived` generators — command help, schema field tables and
//! manifest fields, produced at build time and never committed as page
//! text (PROP-057 `##PIPE-DERIVED`, `##INV-DERIVED-NEVER-HAND-KEPT`).
//!
//! A page says WHERE the text comes from and never what it says. The
//! reason is the failure mode a stored copy has: it is correct on the day
//! it is pasted and wrong on every day after, and nothing in the
//! repository can tell the two days apart. So the text is generated on
//! every render, and what the repository keeps instead is a RECORD of
//! what the last build produced — one line per block, a hash and a size,
//! never the text.
//!
//! That record is what makes drift visible: `vibe doc check --derived`
//! rebuilds every block and compares. A divergence is red, exactly as the
//! dialect's own row says, and `--accept` is the one way to move the
//! record forward (PROP-045 `##ROW-DOCVOCAB-DERIVED-CHECK`).
//!
//! Determinism is not assumed either. The check generates EVERY block
//! twice and refuses if the two runs disagree: a generator that is not a
//! function of the tree would make the record meaningless, and would do
//! it quietly.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-DERIVED");

pub mod cli_help;
pub mod manifest;
pub mod schema;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vibe_specdoc::doc::{Block, BlockNode, DerivedKind, Section};

use crate::error::{DocError, Result};
use crate::pages::{self, Page};

/// Where the package keeps the record of its last derived build.
pub const RECORD: &str = "maintenance/derived.json";

/// What the generators need from their caller.
#[derive(Debug, Clone)]
pub struct DerivedEnv {
    /// The built binary whose `--help` a `cli-help` block quotes.
    pub binary: PathBuf,
    /// The source tree that holds the schemas and the format registry.
    pub repo_root: PathBuf,
    /// The documenting package's own `<group>/<name>`.
    pub coordinate: String,
    pub timeout_secs: u64,
}

/// One generated block: where it sits, what it names, and what came back.
#[derive(Debug, Clone)]
pub struct Generated {
    /// `<page>#<kind>:<reference>`, with an ordinal when a page repeats
    /// one reference — the row's identity in the record.
    pub key: String,
    pub kind: DerivedKind,
    pub reference: String,
    pub text: String,
}

/// One row of the committed record: what a block produced last time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordRow {
    pub sha256: String,
    pub bytes: usize,
}

/// The record file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Record {
    pub schema: u32,
    /// Key → what that block last produced, ordered so the file is the
    /// same bytes for the same tree.
    #[serde(default)]
    pub derived: BTreeMap<String, RecordRow>,
}

/// What became of one block between the record and this build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Same,
    /// The text changed. Red unless `--accept` moves the record forward.
    Changed,
    /// The block is new — the record has never seen it.
    New,
    /// The record names a block no page carries any more.
    Vanished,
}

/// One line of the `--derived` report.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub key: String,
    pub reference: String,
    pub verdict: Verdict,
}

/// A whole `--derived` run.
#[derive(Debug, Clone)]
pub struct Report {
    pub outcomes: Vec<Outcome>,
    pub accepted: bool,
}

impl Report {
    /// Green when the tree agrees with the record, or when `--accept`
    /// moved the record to the tree.
    pub fn ok(&self) -> bool {
        self.accepted
            || self
                .outcomes
                .iter()
                .all(|o| matches!(o.verdict, Verdict::Same))
    }

    /// The human form.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let mut same = 0usize;
        for outcome in &self.outcomes {
            match outcome.verdict {
                Verdict::Same => same += 1,
                Verdict::Changed => out.push_str(&format!(
                    "  CHANGED {} — `{}`\n",
                    outcome.key, outcome.reference
                )),
                Verdict::New => out.push_str(&format!(
                    "  new     {} — `{}`\n",
                    outcome.key, outcome.reference
                )),
                Verdict::Vanished => {
                    out.push_str(&format!("  gone    {}\n", outcome.key));
                }
            }
        }
        let moved = self.outcomes.len() - same;
        out.push_str(&format!(
            "derived: {same} unchanged, {moved} moved{}\n",
            if self.accepted {
                " — recorded (--accept)"
            } else {
                ""
            }
        ));
        out
    }
}

/// The documenting package's own `<group>/<name>` — the one coordinate a
/// `manifest-field` reference can name without going through the store.
pub fn coordinate_of(package_dir: &Path) -> Result<String> {
    let path = package_dir.join(manifest::MANIFEST);
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::io("reading", &path, e))?;
    let parsed: toml::Value = toml::from_str(&text).map_err(|e| DocError::Derived {
        kind: "manifest-field",
        reference: manifest::MANIFEST.to_owned(),
        message: format!("`{}` does not parse: {e}", path.display()),
    })?;
    let read = |key: &str| {
        parsed
            .get("package")
            .and_then(|p| p.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    match (read("group"), read("name")) {
        (Some(group), Some(name)) => Ok(format!("{group}/{name}")),
        _ => Err(DocError::Derived {
            kind: "manifest-field",
            reference: manifest::MANIFEST.to_owned(),
            message: format!(
                "`{}` declares no `[package]` group and name, so the package has \
                 no coordinate to resolve its own fields against",
                path.display()
            ),
        }),
    }
}

/// Generate every `derived` block of a package, in page order.
pub fn generate(package_dir: &Path, env: &DerivedEnv) -> Result<Vec<Generated>> {
    let set = pages::read_package(package_dir)?;
    let mut out: Vec<Generated> = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for page in &set.pages {
        for (kind, reference) in collect(page) {
            let stem = format!("{}#{}:{}", page.rel, kind.as_str(), reference);
            let ordinal = seen.entry(stem.clone()).or_insert(0);
            let key = if *ordinal == 0 {
                stem.clone()
            } else {
                format!("{stem}@{ordinal}")
            };
            *ordinal += 1;
            let text = one(kind, &reference, package_dir, env)?;
            out.push(Generated {
                key,
                kind,
                reference,
                text,
            });
        }
    }
    Ok(out)
}

/// Build one block.
fn one(kind: DerivedKind, reference: &str, package_dir: &Path, env: &DerivedEnv) -> Result<String> {
    match kind {
        DerivedKind::CliHelp => cli_help::generate(
            reference,
            &cli_help::HelpEnv {
                binary: env.binary.clone(),
                timeout_secs: env.timeout_secs,
            },
        ),
        DerivedKind::JtdSchema => schema::generate(reference, &env.repo_root),
        DerivedKind::ManifestField => manifest::generate(reference, package_dir, &env.coordinate),
    }
}

/// Rebuild every block, prove the build is a function of the tree, and
/// compare with the record.
pub fn check(package_dir: &Path, env: &DerivedEnv, accept: bool) -> Result<Report> {
    let first = generate(package_dir, env)?;
    let second = generate(package_dir, env)?;
    if let Some(unstable) = disagreement(&first, &second) {
        return Err(DocError::Derived {
            kind: "any",
            reference: unstable,
            message: "two builds of the same tree produced different text — a \
                      generator that is not a function of the tree makes the \
                      record meaningless"
                .into(),
        });
    }

    let record = read_record(package_dir)?;
    let mut outcomes: Vec<Outcome> = Vec::new();
    let mut next: BTreeMap<String, RecordRow> = BTreeMap::new();
    for block in &first {
        let row = RecordRow {
            sha256: sha256_hex(block.text.as_bytes()),
            bytes: block.text.len(),
        };
        let verdict = match record.derived.get(&block.key) {
            Some(old) if *old == row => Verdict::Same,
            Some(_) => Verdict::Changed,
            None => Verdict::New,
        };
        outcomes.push(Outcome {
            key: block.key.clone(),
            reference: block.reference.clone(),
            verdict,
        });
        next.insert(block.key.clone(), row);
    }
    for key in record.derived.keys() {
        if !next.contains_key(key) {
            outcomes.push(Outcome {
                key: key.clone(),
                reference: String::new(),
                verdict: Verdict::Vanished,
            });
        }
    }
    if accept {
        write_record(
            package_dir,
            &Record {
                schema: 1,
                derived: next,
            },
        )?;
    }
    Ok(Report {
        outcomes,
        accepted: accept,
    })
}

/// The first reference whose two builds disagree.
fn disagreement(first: &[Generated], second: &[Generated]) -> Option<String> {
    if first.len() != second.len() {
        return Some("<the set of blocks itself>".into());
    }
    first
        .iter()
        .zip(second)
        .find(|(a, b)| a.key != b.key || a.text != b.text)
        .map(|(a, _)| a.key.clone())
}

/// Read the committed record. Its absence means «never built», which is
/// a state a fresh package is legitimately in.
pub fn read_record(package_dir: &Path) -> Result<Record> {
    let path = package_dir.join(RECORD);
    if !path.is_file() {
        return Ok(Record {
            schema: 1,
            derived: BTreeMap::new(),
        });
    }
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::io("reading", &path, e))?;
    serde_json::from_str(&text).map_err(|e| DocError::Derived {
        kind: "record",
        reference: RECORD.to_owned(),
        message: format!("`{}` does not parse: {e}", path.display()),
    })
}

/// Write the record, pretty and newline-terminated so a diff of it reads.
fn write_record(package_dir: &Path, record: &Record) -> Result<()> {
    let path = package_dir.join(RECORD);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DocError::io("creating", parent, e))?;
    }
    let mut text = serde_json::to_string_pretty(record).map_err(|e| DocError::Derived {
        kind: "record",
        reference: RECORD.to_owned(),
        message: format!("cannot serialise the record: {e}"),
    })?;
    text.push('\n');
    std::fs::write(&path, text).map_err(|e| DocError::io("writing", &path, e))
}

/// Lift every `derived` block off one page, in document order.
fn collect(page: &Page) -> Vec<(DerivedKind, String)> {
    fn from_blocks(blocks: &[BlockNode], out: &mut Vec<(DerivedKind, String)>) {
        for node in blocks {
            if let Block::Derived { kind, reference } = &node.block {
                out.push((*kind, reference.clone()));
            }
        }
    }
    fn from_section(section: &Section, out: &mut Vec<(DerivedKind, String)>) {
        from_blocks(&section.blocks, out);
        for sub in &section.sections {
            from_section(sub, out);
        }
    }
    let mut out = Vec::new();
    from_blocks(&page.doc.preamble, &mut out);
    for section in &page.doc.sections {
        from_section(section, &mut out);
    }
    out
}

/// SHA-256 in hex. A record row keeps a hash and a size, never the text:
/// the whole point is that the text does not live in the repository.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests;
