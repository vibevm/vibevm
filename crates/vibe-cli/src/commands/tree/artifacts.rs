//! Decompilers for the committed boot artifacts (PROP-036 §2.8, §3).
//!
//! Two pure readers, no filesystem access — the caller hands in the file
//! text:
//!
//! - [`decompile_static`] parses `STATIC.md`'s on-disk `vibe:static`
//!   open-marker format into its contributions. This is a **dedicated**
//!   decompiler — it is NOT `vibe_spec::decompile()`, which parses the
//!   distinct `vibe:begin`/`vibe:end` compiler format and returns empty on
//!   `STATIC.md` (PROP-036 §2.8).
//! - [`read_index`] parses the generated `INDEX.md` TOML manifest into its
//!   ordered `[[entry]]` list.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#static-decompile");

use anyhow::{Context, Result};
use serde::Deserialize;

use super::model::{EmbedSpan, IndexEntry, IndexKind, StaticContribution};

/// The opening delimiter of a `STATIC.md` provenance marker.
const MARKER_OPEN: &str = "<!-- vibe:static ";
/// The closing delimiter of a marker line.
const MARKER_CLOSE: &str = " -->";
/// The `origin — path` separator: space, U+2014 em-dash, space
/// (`crates/vibe-workspace/src/boot_artifacts.rs`).
const MARKER_SEP: &str = " \u{2014} ";

/// A parsed `<!-- vibe:static {origin} — {path} -->` marker line.
struct Marker {
    origin: String,
    source_path: String,
    /// 0-based line index of the marker within the file.
    line: usize,
}

/// Parse one line as a `vibe:static` marker, or `None` if it is not one.
fn parse_marker(line: &str, line_idx: usize) -> Option<Marker> {
    let trimmed = line.trim_end();
    let inner = trimmed
        .strip_prefix(MARKER_OPEN)?
        .strip_suffix(MARKER_CLOSE)?;
    let (origin, path) = inner.split_once(MARKER_SEP)?;
    Some(Marker {
        origin: origin.trim().to_string(),
        source_path: path.trim().to_string(),
        line: line_idx,
    })
}

/// Decompile `STATIC.md` text into its ordered contributions (PROP-036 §2.8).
///
/// Each marker opens a region running to the next marker or EOF; the region
/// yields the source `origin` (`group/name` or a host rel-path) and `path`.
/// A region's `bytes`/`lines` measure its body — the text after the marker
/// line up to (not including) the next marker. Nested
/// `<!-- embed: {addr} -->` … `<!-- /embed: {addr} -->` pairs become
/// [`EmbedSpan`]s with file-relative 1-based line numbers.
pub fn decompile_static(text: &str) -> Vec<StaticContribution> {
    let lines: Vec<&str> = text.lines().collect();
    let markers: Vec<Marker> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| parse_marker(l, i))
        .collect();

    let mut out = Vec::with_capacity(markers.len());
    for (order, marker) in markers.iter().enumerate() {
        // Region body: from the line after the marker to the line before the
        // next marker (or EOF).
        let body_start = marker.line + 1;
        let body_end = markers
            .get(order + 1)
            .map(|m| m.line)
            .unwrap_or(lines.len());
        let body = &lines[body_start.min(lines.len())..body_end.min(lines.len())];
        let bytes: u64 = body.iter().map(|l| l.len() as u64 + 1).sum();
        let embeds = scan_embeds(body, body_start);
        out.push(StaticContribution {
            order: order as u64,
            origin: marker.origin.clone(),
            source_path: marker.source_path.clone(),
            bytes,
            lines: body.len() as u64,
            embeds,
        });
    }
    out
}

/// Attribute `<!-- embed: {addr} -->` … `<!-- /embed: {addr} -->` pairs
/// inside a region body. `body_offset` is the 0-based file line index of the
/// region body's first line; emitted spans are 1-based file lines.
fn scan_embeds(body: &[&str], body_offset: usize) -> Vec<EmbedSpan> {
    let mut spans = Vec::new();
    let mut open: Option<(String, usize)> = None;
    for (i, line) in body.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("<!-- /embed:") {
            let addr = rest.trim_end_matches("-->").trim().to_string();
            if let Some((open_addr, start)) = open.take() {
                spans.push(EmbedSpan {
                    address: if addr.is_empty() { open_addr } else { addr },
                    start_line: (body_offset + start + 1) as u64,
                    end_line: (body_offset + i + 1) as u64,
                });
            }
        } else if let Some(rest) = trimmed.strip_prefix("<!-- embed:") {
            let addr = rest.trim_end_matches("-->").trim().to_string();
            open = Some((addr, i));
        }
    }
    spans
}

/// The TOML shape of `INDEX.md` (a subset — only the fields the tree needs).
#[derive(Debug, Deserialize)]
struct IndexToml {
    #[serde(rename = "static")]
    static_pointer: Option<String>,
    #[serde(default, rename = "entry")]
    entries: Vec<IndexEntryToml>,
}

/// One `[[entry]]` table of `INDEX.md`.
#[derive(Debug, Deserialize)]
struct IndexEntryToml {
    path: String,
    kind: String,
    #[serde(default)]
    when: Option<String>,
}

/// The parsed `INDEX.md` lane: the `static` pointer and the ordered entries.
pub struct IndexParse {
    pub static_pointer: Option<String>,
    pub entries: Vec<IndexEntry>,
}

/// Read the generated `INDEX.md` TOML into its ordered entry list.
pub fn read_index(text: &str) -> Result<IndexParse> {
    let toml: IndexToml =
        toml::from_str(text).context("parsing the generated spec/boot/INDEX.md manifest")?;
    let entries = toml
        .entries
        .into_iter()
        .enumerate()
        .map(|(order, e)| IndexEntry {
            order: order as u64,
            path: e.path,
            kind: if e.kind == "dynamic" {
                IndexKind::Dynamic
            } else {
                IndexKind::Static
            },
            when: e.when,
        })
        .collect();
    Ok(IndexParse {
        static_pointer: toml.static_pointer,
        entries,
    })
}

/// Map a boot-file path to the `(kind, name)` of the package that owns it,
/// or `None` for a host-authored path. A materialised slot path is
/// `vibedeps/<kind>-<name>/<version>/…`; the second component encodes the
/// kind prefix and the package name.
pub fn slot_package(path: &str) -> Option<(&'static str, String)> {
    let rest = path
        .strip_prefix("vibedeps/")
        .or_else(|| path.strip_prefix("vibedeps\\"))?;
    let slot = rest.split(['/', '\\']).next()?;
    for kind in ["flow", "feat", "stack", "tool", "mcp"] {
        if let Some(name) = slot.strip_prefix(&format!("{kind}-")) {
            return Some((kind, name.to_string()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompiles_two_contributions_with_bodies() {
        let text = "\
<!-- header -->

<!-- vibe:static org.vibevm.world/addressable-specs \u{2014} vibedeps/flow-addressable-specs/0.1.0/spec/boot/15.md -->

# Addressable Specs

body line
<!-- vibe:static org.vibevm.world/redbook \u{2014} vibedeps/flow-redbook/0.2.0/spec/boot/03.md -->

# Redbook
";
        let c = decompile_static(text);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].origin, "org.vibevm.world/addressable-specs");
        assert_eq!(
            c[0].source_path,
            "vibedeps/flow-addressable-specs/0.1.0/spec/boot/15.md"
        );
        assert_eq!(c[0].order, 0);
        assert_eq!(c[1].origin, "org.vibevm.world/redbook");
        assert_eq!(c[1].order, 1);
        // The first region's body has non-zero lines (the second marker
        // bounds it).
        assert!(c[0].lines > 0);
        assert!(c[0].embeds.is_empty());
    }

    #[test]
    fn a_host_rel_path_origin_is_kept_verbatim() {
        let text =
            "<!-- vibe:static spec/boot/00-core.md \u{2014} spec/boot/00-core.md -->\nbody\n";
        let c = decompile_static(text);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].origin, "spec/boot/00-core.md");
    }

    #[test]
    fn a_non_marker_line_is_not_a_contribution() {
        // A plain heading or an em-dash-free comment is not a marker.
        let text = "# Heading\n<!-- vibe:static no-separator-here -->\ntext\n";
        assert!(decompile_static(text).is_empty());
    }

    #[test]
    fn attributes_a_nested_embed_span() {
        let text = "\
<!-- vibe:static org.vibevm.world/x \u{2014} vibedeps/flow-x/0.1.0/b.md -->

<!-- embed: spec://vibevm/a/b#c -->
inner
<!-- /embed: spec://vibevm/a/b#c -->
";
        let c = decompile_static(text);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].embeds.len(), 1);
        assert_eq!(c[0].embeds[0].address, "spec://vibevm/a/b#c");
        assert!(c[0].embeds[0].start_line < c[0].embeds[0].end_line);
    }

    #[test]
    fn reads_index_entries_in_order() {
        let text = "\
schema = 1
static = \"spec/boot/STATIC.md\"

[[entry]]
path = \"spec/boot/00-core.md\"
kind = \"static\"

[[entry]]
path = \"vibedeps/stack-rust-ai-native-lang/0.7.0/spec/boot/20.md\"
kind = \"dynamic\"
when = \"os:linux\"
";
        let parsed = read_index(text).unwrap();
        assert_eq!(
            parsed.static_pointer.as_deref(),
            Some("spec/boot/STATIC.md")
        );
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].order, 0);
        assert_eq!(parsed.entries[0].kind, IndexKind::Static);
        assert_eq!(parsed.entries[1].kind, IndexKind::Dynamic);
        assert_eq!(parsed.entries[1].when.as_deref(), Some("os:linux"));
    }

    #[test]
    fn maps_a_slot_path_to_its_package() {
        assert_eq!(
            slot_package("vibedeps/stack-rust-ai-native-lang/0.7.0/spec/boot/20.md"),
            Some(("stack", "rust-ai-native-lang".to_string()))
        );
        assert_eq!(
            slot_package("vibedeps/flow-redbook/0.2.0/spec/boot/03.md"),
            Some(("flow", "redbook".to_string()))
        );
        // A host-authored path maps to no package.
        assert_eq!(slot_package("spec/boot/00-core.md"), None);
    }
}
