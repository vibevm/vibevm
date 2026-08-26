//! Decompilers for the committed boot artifacts (PROP-036 §2.8, §3).
//!
//! Two pure readers, no filesystem access — the caller hands in the file
//! text:
//!
//! - [`decompile_static`] parses the static lane's on-disk `vibe:static`
//!   open-marker format into its contributions. This is a **dedicated**
//!   decompiler — it is NOT `vibe_spec::decompile()`, which parses the
//!   distinct `vibe:begin`/`vibe:end` compiler format and returns empty on
//!   `STATIC.md` (PROP-036 §2.8).
//! - [`read_index`] parses the generated `INDEX.md` TOML manifest into its
//!   ordered `[[entry]]` list.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#static-decompile");

use anyhow::{Context, Result};
use serde::Deserialize;
use specmark::spec;
use thiserror::Error;
use vibe_specdoc::{XmlCommentCodecError, decode_generated_xml_comment};

use super::model::{EmbedSpan, IndexEntry, IndexKind, StaticContribution};

const COMMENT_OPEN: &str = "<!--";
const COMMENT_CLOSE: &str = "-->";
const MARKER_PREFIX: &str = "vibe:static ";
/// The `origin — path` separator: space, U+2014 em-dash, space
/// (`crates/vibe-workspace/src/boot_artifacts.rs`).
const MARKER_SEP: &str = " \u{2014} ";

/// Failures specific to decompiling the committed static-lane artifact.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#static-decompile")]
pub enum StaticDecompileError {
    /// A complete generated comment used an invalid c1 wire spelling.
    #[error(
        "generated XML comment cannot be decoded: {source} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#static-decompile; \
         fix: regenerate the committed static lane with the current vibe binary)"
    )]
    Codec {
        #[source]
        source: XmlCommentCodecError,
    },
    /// Canonical c1 decoded, but its reserved provenance payload was invalid.
    #[error(
        "malformed generated `vibe:static` provenance at line {line}: {reason}; \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#static-decompile; \
         fix: regenerate it as `vibe:static <nonempty origin> — <nonempty path>`)"
    )]
    MalformedGeneratedProvenance { line: usize, reason: &'static str },
    /// Any unclosed comment makes the committed lane structurally ambiguous.
    #[error(
        "unterminated XML comment beginning at line {line}; \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#static-decompile; \
         fix: close it with `-->` or regenerate the committed static lane)"
    )]
    UnterminatedXmlComment { line: usize },
}

impl From<XmlCommentCodecError> for StaticDecompileError {
    fn from(source: XmlCommentCodecError) -> Self {
        Self::Codec { source }
    }
}

/// A parsed `<!-- vibe:static {origin} — {path} -->` marker line.
struct Marker {
    origin: String,
    source_path: String,
    /// 0-based first and last line indices of the marker within the file.
    start_line: usize,
    end_line: usize,
}

fn parse_legacy_marker(payload: &str, start_line: usize, end_line: usize) -> Option<Marker> {
    let inner = payload.strip_prefix(MARKER_PREFIX)?;
    let (origin, path) = inner.split_once(MARKER_SEP)?;
    Some(Marker {
        origin: origin.trim().to_string(),
        source_path: path.trim().to_string(),
        start_line,
        end_line,
    })
}

fn parse_generated_marker(
    payload: &str,
    start_line: usize,
    end_line: usize,
) -> std::result::Result<Marker, StaticDecompileError> {
    let Some(inner) = payload.strip_prefix(MARKER_PREFIX) else {
        return Err(StaticDecompileError::MalformedGeneratedProvenance {
            line: start_line + 1,
            reason: "the reserved `vibe:static ` prefix is missing",
        });
    };
    let Some((origin, path)) = inner.split_once(MARKER_SEP) else {
        return Err(StaticDecompileError::MalformedGeneratedProvenance {
            line: start_line + 1,
            reason: "the exact ` space + U+2014 em dash + space ` separator is missing",
        });
    };
    let origin = origin.trim();
    let path = path.trim();
    if origin.is_empty() {
        return Err(StaticDecompileError::MalformedGeneratedProvenance {
            line: start_line + 1,
            reason: "origin is empty after trimming",
        });
    }
    if path.is_empty() {
        return Err(StaticDecompileError::MalformedGeneratedProvenance {
            line: start_line + 1,
            reason: "path is empty after trimming",
        });
    }
    Ok(Marker {
        origin: origin.to_string(),
        source_path: path.to_string(),
        start_line,
        end_line,
    })
}

/// Decode generated c1 comments while retaining legacy Markdown markers.
/// Reserved c1 comments are scanned as complete comments rather than lines:
/// the generated preamble and tombstone legitimately span several lines.
fn collect_markers(text: &str) -> std::result::Result<Vec<Marker>, StaticDecompileError> {
    let mut markers = Vec::new();
    let mut cursor = 0;
    let mut cursor_line = 0;

    while let Some(relative_start) = text[cursor..].find(COMMENT_OPEN) {
        let start = cursor + relative_start;
        let start_line = cursor_line + text[cursor..start].bytes().filter(|b| *b == b'\n').count();
        let comment_tail = &text[start..];
        let Some(relative_close) = comment_tail.find(COMMENT_CLOSE) else {
            return Err(StaticDecompileError::UnterminatedXmlComment {
                line: start_line + 1,
            });
        };
        let end = start + relative_close + COMMENT_CLOSE.len();
        let comment = &text[start..end];
        let end_line = start_line + comment.bytes().filter(|b| *b == b'\n').count();

        match decode_generated_xml_comment(comment)? {
            Some(payload) => {
                if payload.starts_with(MARKER_PREFIX) {
                    markers.push(parse_generated_marker(&payload, start_line, end_line)?);
                }
            }
            None => {
                let legacy = comment
                    .strip_prefix("<!-- ")
                    .and_then(|value| value.strip_suffix(" -->"));
                if let Some(marker) =
                    legacy.and_then(|payload| parse_legacy_marker(payload, start_line, end_line))
                {
                    markers.push(marker);
                }
            }
        }

        cursor_line = end_line;
        cursor = end;
    }
    Ok(markers)
}

/// Decompile static-lane text into its ordered contributions (PROP-036 §2.8).
///
/// Each marker opens a region running to the next marker or EOF; the region
/// yields the source `origin` (`group/name` or a host rel-path) and `path`.
/// A region's `bytes`/`lines` measure its body — the text after the marker
/// line up to (not including) the next marker. Nested
/// `<!-- embed: {addr} -->` … `<!-- /embed: {addr} -->` pairs become
/// [`EmbedSpan`]s with file-relative 1-based line numbers.
pub fn decompile_static(
    text: &str,
) -> std::result::Result<Vec<StaticContribution>, StaticDecompileError> {
    let lines: Vec<&str> = text.lines().collect();
    let markers = collect_markers(text)?;

    let mut out = Vec::with_capacity(markers.len());
    for (order, marker) in markers.iter().enumerate() {
        // Region body: from the line after the marker to the line before the
        // next marker (or EOF).
        let body_start = marker.end_line + 1;
        let body_end = markers
            .get(order + 1)
            .map(|m| m.start_line)
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
    Ok(out)
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
    let toml: IndexToml = toml::from_str(text).with_context(|| {
        format!(
            "parsing the generated {} manifest",
            vibe_core::machine_json_path(&vibe_core::layout::current_boot_index())
        )
    })?;
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

/// Map a boot-file path to the `(group, name)` of the package that owns it,
/// or `None` for a host-authored path. A materialised slot path lives under
/// the live layout's deps root (`<deps root>/<group>.<name>/<version>/…`);
/// the second component encodes the package identity.
pub fn slot_package(path: &str) -> Option<(String, String)> {
    // Component-wise strip, so both separator spellings work and the R4
    // flip's two-component root matches in one comparison.
    let rest = std::path::Path::new(path)
        .strip_prefix(vibe_core::layout::current_vibedeps_root())
        .ok()?;
    let rest = rest.to_string_lossy().replace('\\', "/");
    let slot = rest.split('/').next()?;
    // The slot is identity-keyed, `<group>.<name>` (PROP-022 §2.1): the name
    // is a single dot-free LDH label, so the last dot is always the boundary.
    let (group, name) = slot.rsplit_once('.')?;
    Some((group.to_string(), name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c1(payload: &str) -> String {
        format!(
            "<!-- vibe:c1 {} -->",
            vibe_specdoc::encode_generated_xml_comment(payload)
        )
    }

    /// A materialised slot file on the live layout:
    /// `<deps root>/<slot>/<version>/<tail>`. Built from the layout
    /// module so every fixture here rides the R4 flip unchanged.
    fn slot_file(slot: &str, version: &str, tail: impl AsRef<std::path::Path>) -> String {
        vibe_core::machine_json_path(
            &vibe_core::layout::current_vibedeps_root()
                .join(slot)
                .join(version)
                .join(tail.as_ref()),
        )
    }

    /// A package's boot-lane file inside its slot — the tail a
    /// materialised boot snippet carries.
    fn slot_boot(slot: &str, version: &str, file: &str) -> String {
        slot_file(
            slot,
            version,
            vibe_core::layout::current_specs_root()
                .join(vibe_core::layout::BOOT_DIR)
                .join(file),
        )
    }

    /// A host boot-lane file on the live layout.
    fn host_boot(file: &str) -> String {
        vibe_core::machine_json_path(&vibe_core::layout::current_boot_dir().join(file))
    }

    #[test]
    fn decompiles_two_contributions_with_bodies() {
        let one = slot_boot("org.vibevm.world.addressable-specs", "0.1.0", "15.md");
        let two = slot_boot("org.vibevm.world.redbook", "0.2.0", "03.md");
        let text = format!(
            "\
<!-- header -->

<!-- vibe:static org.vibevm.world/addressable-specs \u{2014} {one} -->

# Addressable Specs

body line
<!-- vibe:static org.vibevm.world/redbook \u{2014} {two} -->

# Redbook
"
        );
        let c = decompile_static(&text).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].origin, "org.vibevm.world/addressable-specs");
        assert_eq!(c[0].source_path, one);
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
        let core = host_boot("00-core.md");
        let text = format!("<!-- vibe:static {core} \u{2014} {core} -->\nbody\n");
        let c = decompile_static(&text).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].origin, core);
    }

    #[test]
    fn a_non_marker_line_is_not_a_contribution() {
        // A plain heading or an em-dash-free comment is not a marker.
        let text = "# Heading\n<!-- vibe:static no-separator-here -->\ntext\n";
        assert!(decompile_static(text).unwrap().is_empty());
    }

    #[test]
    fn attributes_a_nested_embed_span() {
        let spec = slot_file("org.vibevm.world.x", "0.1.0", "b.md");
        let text = format!(
            "\
<!-- vibe:static org.vibevm.world/x \u{2014} {spec} -->

<!-- embed: spec://org.vibevm.core/vibevm/a/b#c -->
inner
<!-- /embed: spec://org.vibevm.core/vibevm/a/b#c -->
"
        );
        let c = decompile_static(&text).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].embeds.len(), 1);
        assert_eq!(
            c[0].embeds[0].address,
            "spec://org.vibevm.core/vibevm/a/b#c"
        );
        assert!(c[0].embeds[0].start_line < c[0].embeds[0].end_line);
    }

    #[test]
    fn reads_index_entries_in_order() {
        let static_lane =
            vibe_core::machine_json_path(&vibe_core::layout::current_boot_static_md());
        let core = host_boot("00-core.md");
        let dyn_entry = slot_boot("org.vibevm.ai-native.rust-ai-native-lang", "0.7.0", "20.md");
        let text = format!(
            "\
schema = 1
static = \"{static_lane}\"

[[entry]]
path = \"{core}\"
kind = \"static\"

[[entry]]
path = \"{dyn_entry}\"
kind = \"dynamic\"
when = \"os:linux\"
"
        );
        let parsed = read_index(&text).unwrap();
        assert_eq!(parsed.static_pointer.as_deref(), Some(static_lane.as_str()));
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].order, 0);
        assert_eq!(parsed.entries[0].kind, IndexKind::Static);
        assert_eq!(parsed.entries[1].kind, IndexKind::Dynamic);
        assert_eq!(parsed.entries[1].when.as_deref(), Some("os:linux"));
    }

    #[test]
    fn maps_a_slot_path_to_its_package() {
        assert_eq!(
            slot_package(&slot_boot(
                "org.vibevm.ai-native.rust-ai-native-lang",
                "0.7.0",
                "20.md"
            )),
            Some((
                "org.vibevm.ai-native".to_string(),
                "rust-ai-native-lang".to_string()
            ))
        );
        assert_eq!(
            slot_package(&slot_boot("org.vibevm.world.redbook", "0.2.0", "03.md")),
            Some(("org.vibevm.world".to_string(), "redbook".to_string()))
        );
        // A host-authored path maps to no package.
        assert_eq!(slot_package(&host_boot("00-core.md")), None);
    }

    #[test]
    fn c1_provenance_decodes_exact_origin_and_path() {
        let origin = "org.demo/a--b-%雪";
        let path = "dir/a--b-/x%2D&雪.xml";
        let logical = format!("vibe:static {origin}{MARKER_SEP}{path}");
        let text = format!("{}\n\n<spec/>\n", c1(&logical));
        let contributions = decompile_static(&text).unwrap();
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].origin, origin);
        assert_eq!(contributions[0].source_path, path);
    }

    #[test]
    fn malformed_unknown_and_noncanonical_c1_fail_instead_of_disappearing() {
        for text in [
            "<!-- vibe:c2 vibe:static origin — path -->\n",
            "<!-- vibe:c1 vibe:static origin — path% -->\n",
            "<!-- vibe:c1 vibe:static %41 — path -->\n",
        ] {
            let error = decompile_static(text).expect_err(text);
            assert!(
                matches!(error, StaticDecompileError::Codec { .. }),
                "{error}"
            );
        }
    }

    #[test]
    fn canonical_c1_provenance_requires_separator_and_nonempty_fields() {
        for (payload, reason) in [
            ("vibe:static origin", "separator"),
            ("vibe:static origin - path", "separator"),
            ("vibe:static  — path", "origin is empty"),
            ("vibe:static origin —  ", "path is empty"),
        ] {
            let text = c1(payload);
            let error = decompile_static(&text).expect_err(payload);
            assert!(
                matches!(
                    error,
                    StaticDecompileError::MalformedGeneratedProvenance { .. }
                ),
                "{payload:?}: {error}"
            );
            assert!(error.to_string().contains(reason), "{payload:?}: {error}");
        }
    }

    #[test]
    fn canonical_non_provenance_c1_genres_remain_skippable() {
        for payload in [
            "generated header",
            "RESOLUTION RULES — body",
            "RENAMED ANCHORS (short → heirs)",
            "vibe:hoisted org.demo/pkg — root",
            "vibe:transforms xml-minify",
        ] {
            assert!(
                decompile_static(&c1(payload)).unwrap().is_empty(),
                "{payload}"
            );
        }
    }

    #[test]
    fn every_unterminated_comment_is_a_typed_error_regardless_of_c1_spacing() {
        for text in [
            "<!-- vibe:c1 unterminated",
            "<!--vibe:c1 unterminated",
            "<!--  vibe:c1 unterminated",
            "<!--\nvibe:c1 unterminated",
            "<!-- authored but unterminated",
        ] {
            let error = decompile_static(text).expect_err(text);
            assert!(
                matches!(
                    error,
                    StaticDecompileError::UnterminatedXmlComment { line: 1 }
                ),
                "{text:?}: {error}"
            );
        }
    }

    #[test]
    fn multiline_c1_provenance_keeps_fields_and_body_line_accounting() {
        let origin = "org.demo/\nalpha";
        let path = "dir/\nentry.xml";
        let payload = format!("vibe:static {origin}{MARKER_SEP}{path}");
        let text = format!("{}\nBODY-A\nBODY-B\n", c1(&payload));

        let contributions = decompile_static(&text).unwrap();
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].origin, origin);
        assert_eq!(contributions[0].source_path, path);
        assert_eq!(contributions[0].lines, 2);
        assert_eq!(contributions[0].bytes, "BODY-A\nBODY-B\n".len() as u64);
    }
}
