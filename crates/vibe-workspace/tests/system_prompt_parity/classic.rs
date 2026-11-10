//! The **reference stream**: today's file-reading boot, read the way the
//! published contract says an agent reads it.
//!
//! This module is the differential test's other half, and its whole value is
//! that it shares **no code** with what it is compared against. `PLAN.md` P7
//! states the construction rule and calls it a reviewer's first question: if
//! the reference were built from the composer, the plan, the marker grammar
//! or `resolve_static_path`, the test would prove only that one function
//! agrees with itself. So everything here is re-implemented from the
//! contract as published:
//!
//! - the entrypoint is `AGENTS.md` (or the declared spelling) read whole;
//! - the static lane is whichever of `STATIC.xml` / `STATIC.md` is on disk,
//!   decided by two `Path::exists` calls rather than by the resolver;
//! - the INDEX is `INDEX.md` parsed **as a TOML document**, which is what
//!   `CLAUDE.md`'s own session-boot block tells a reader it is, and its
//!   `[[entry]]` rows are taken in file order with their `path`, `kind` and
//!   optional `when` verbatim;
//! - the managed block's byte range is found by scanning for a line that
//!   trims to exactly `<vibevm>` / `</vibevm>`, which is the rule PROP-012
//!   publishes, not a call into `locate_block`.
//!
//! The only crate-level thing it uses is `vibe_core::layout`, for the names
//! of the directories — a reference that invented its own spelling of
//! `vibevm/vibespecs/boot/` would be testing the wrong tree.

use std::path::{Path, PathBuf};

/// One `[[entry]]` row of `INDEX.md`, as the file spells it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRow {
    /// `path`, verbatim — workspace-root-relative in the generated artifact.
    pub path: String,
    /// `kind` — `"static"` or `"dynamic"`, the row's read timing.
    pub kind: String,
    /// `when`, when the row carries one.
    pub when: Option<String>,
}

/// The boot a session performs today, as bytes and in order.
#[derive(Debug, Clone)]
pub struct ClassicBoot {
    /// The selected entrypoint's path, workspace-root-relative.
    pub entrypoint_rel: String,
    /// The entrypoint's bytes, whole.
    pub entrypoint: Vec<u8>,
    /// The static lane's workspace-root-relative path and bytes, when a
    /// lane was published at all.
    pub lane: Option<(String, Vec<u8>)>,
    /// `INDEX.md`'s bytes, for the plan-identity comparison.
    pub index_bytes: Vec<u8>,
    /// The `[[entry]]` rows, in file order.
    pub rows: Vec<IndexRow>,
}

impl ClassicBoot {
    /// Read the whole classic boot of the node rooted at `node_dir`, whose
    /// paths are relative to `workspace_root` (the two are the same
    /// directory for a root node).
    pub fn read(workspace_root: &Path, node_dir: &Path, entrypoint_rel: &str) -> Self {
        let boot = node_dir.join(vibe_core::layout::current_boot_dir());
        let index_path = boot.join(vibe_core::layout::INDEX_MD);
        let index_bytes = std::fs::read(&index_path)
            .unwrap_or_else(|error| panic!("the published INDEX at {index_path:?}: {error}"));
        Self {
            entrypoint_rel: entrypoint_rel.to_owned(),
            entrypoint: std::fs::read(node_dir.join(entrypoint_rel))
                .unwrap_or_else(|error| panic!("the entrypoint `{entrypoint_rel}`: {error}")),
            lane: lane_on_disk(&boot).map(|path| {
                let bytes = std::fs::read(&path).expect("the published static lane");
                (relative(workspace_root, &path), bytes)
            }),
            rows: parse_index(&index_bytes),
            index_bytes,
        }
    }

    /// The rows a session reads unconditionally — band U's identity and
    /// order, straight off the published artifact.
    pub fn unconditional(&self) -> Vec<&IndexRow> {
        self.rows.iter().filter(|row| row.when.is_none()).collect()
    }

    /// The managed `<vibevm>` block's byte span inside the entrypoint, by
    /// the published rule: an exactly standalone marker line, and the span
    /// runs from the first byte of the open marker's line to the byte after
    /// the close marker's line (PROP-012 §2.2).
    pub fn managed_block(&self) -> Option<std::ops::Range<usize>> {
        standalone_block(&self.entrypoint)
    }
}

/// Which `STATIC.*` spelling is on disk. Two `exists` calls rather than the
/// resolver under test; both present is the one case the reference refuses to
/// guess about, exactly as the published rule says the generator owns one.
fn lane_on_disk(boot_dir: &Path) -> Option<PathBuf> {
    let xml = boot_dir.join(vibe_core::layout::STATIC_XML);
    let md = boot_dir.join(vibe_core::layout::STATIC_MD);
    match (xml.exists(), md.exists()) {
        (true, true) => panic!("both `STATIC` spellings are on disk; the generator owns one"),
        (true, false) => Some(xml),
        (false, true) => Some(md),
        (false, false) => None,
    }
}

/// `INDEX.md` as the contract describes it: a TOML document whose `[[entry]]`
/// tables are the boot sequence.
fn parse_index(bytes: &[u8]) -> Vec<IndexRow> {
    let text = std::str::from_utf8(bytes).expect("a generated INDEX is UTF-8");
    let document: toml::Value = toml::from_str(text).expect("a generated INDEX is a TOML document");
    let Some(entries) = document.get("entry") else {
        return Vec::new();
    };
    entries
        .as_array()
        .expect("`entry` is an array of tables")
        .iter()
        .map(|entry| IndexRow {
            path: entry["path"]
                .as_str()
                .expect("an entry names a path")
                .to_owned(),
            kind: entry["kind"]
                .as_str()
                .expect("an entry names its read timing")
                .to_owned(),
            when: entry
                .get("when")
                .map(|when| when.as_str().expect("a `when` is a string").to_owned()),
        })
        .collect()
}

/// The byte span of the whole managed block, **both markers included** —
/// from the first byte of the `<vibevm>` line to the byte after the
/// `</vibevm>` line.
///
/// The published rule (PROP-012 §2.2) is that a marker is an *exactly
/// standalone* line: a line whose trimmed content is the marker and nothing
/// else. Anything else is text. Re-implemented here, deliberately, so the
/// comparison does not inherit `locate_block`'s own reading of its own rule.
fn standalone_block(bytes: &[u8]) -> Option<std::ops::Range<usize>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut open: Option<usize> = None;
    let mut close: Option<usize> = None;
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        match line.trim() {
            "<vibevm>" => {
                if open.is_none() {
                    open = Some(offset);
                }
            }
            "</vibevm>" if open.is_some() && close.is_none() => {
                close = Some(offset + line.len());
            }
            _ => {}
        }
        offset += line.len();
    }
    Some(open?..close?)
}

/// One delivered lane segment, as a reader of the delivered text alone sees
/// it: the path and the id its markers carry, and the bytes between them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveredSegment {
    /// The node id the marker pair carries.
    pub node_id: String,
    /// The project-relative path the marker pair names.
    pub path: String,
    /// The 12 hexadecimal digits both lines carry.
    pub hash12: String,
    /// The bytes between the two marker lines.
    pub body: Vec<u8>,
}

/// Split a delivered agent lane into its segments by reading its marker
/// lines — the grammar `HARNESS-DELIVERY.md` §2 publishes, parsed here with
/// no help from `verify_lane_markers`.
///
/// Panics on anything the grammar does not admit, because a reference that
/// quietly tolerated a malformed delivery would make the parity assertion
/// vacuous.
pub fn split_delivered_lane(bytes: &[u8]) -> Vec<DeliveredSegment> {
    let text = std::str::from_utf8(bytes).expect("a composed lane is UTF-8");
    let mut segments: Vec<DeliveredSegment> = Vec::new();
    let mut open: Option<(String, String, String, usize)> = None;
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        let next = offset + line.len();
        if let Some(fields) = marker_fields(line, "vibe:lane-begin") {
            assert!(open.is_none(), "a begin line inside an open segment");
            open = Some((fields.0, fields.1, fields.2, next));
        } else if let Some(fields) = marker_fields(line, "vibe:lane-end") {
            let (node_id, path, hash12, body_start) =
                open.take().expect("an end line with no begin line");
            assert_eq!(
                (node_id.as_str(), path.as_str(), hash12.as_str()),
                (fields.0.as_str(), fields.1.as_str(), fields.2.as_str()),
                "a pair whose two lines disagree"
            );
            segments.push(DeliveredSegment {
                node_id,
                path,
                hash12,
                body: bytes[body_start..offset].to_vec(),
            });
        }
        offset = next;
    }
    assert!(open.is_none(), "a begin line with no end line");
    segments
}

/// The three fields of a marker line, when the line is one.
fn marker_fields(line: &str, keyword: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("<!-- ")?
        .strip_suffix(" -->")?
        .strip_prefix(keyword)?
        .trim_start();
    let fields: Vec<&str> = inner.split_whitespace().collect();
    match fields.as_slice() {
        [node_id, path, hash12] => Some((
            (*node_id).to_owned(),
            (*path).to_owned(),
            (*hash12).to_owned(),
        )),
        _ => panic!("a marker line with {} fields: {trimmed}", fields.len()),
    }
}

/// A SHA-256 in lowercase hex, computed here rather than taken from the
/// crate under test, so a witness's claim is checked against an independent
/// digest.
pub fn sha256_of(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The first 12 hexadecimal digits of [`sha256_of`] — the id a lane marker
/// carries.
pub fn hash12_of(bytes: &[u8]) -> String {
    sha256_of(bytes)[..12].to_owned()
}

/// A path under `root`, forward-slashed, the way a generated artifact spells
/// it.
pub fn relative(root: &Path, path: &Path) -> String {
    let tail = path.strip_prefix(root).expect("a path under the root");
    tail.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}
