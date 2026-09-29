//! Markdown side of the scanner: anchored spec units.
//!
//! A unit is the span from an anchored heading (`### Title {#anchor}`)
//! to the next same-or-higher heading — anchored or not (GUIDE-SPEC-
//! AUTHORING §1). The first non-blank body line may be a kind line:
//! `` `req r2` ``, `` `req r1 planned` ``, `` `req r2 disputed(#other)` ``
//! — optionally followed by prose on the same line. Units without a kind
//! line are legacy-unmarked and still inventoried (full node inventory,
//! PROP-014 §4 Phase 0).

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#spec-units");

use crate::generated::specmap::{SpecUnit, SpecUnitKind, SpecUnitStatus, Warning};
use specmark_grammar::is_valid_anchor;

use crate::config::SectionGrain;
use crate::content_hash;

/// Pure line-classification primitives (heading / fence / list-item /
/// `##<ID>` fact-anchor detection) — the syntactic helper layer this pass
/// composes. Lives in the `lines` child module so the main file stays within
/// the file-length budget (same split as `mdspec/tests.rs`).
mod lines;
use lines::{fact_anchor_at, fence_mask, heading_level, list_item_content, parse_heading};

/// Enumerable markdown exclusion (`Config::spec_exclude`): compiles the
/// patterns once, tests every candidate file, and reports stale / invalid
/// patterns. A child module for the file-length budget, like `lines`.
mod excludes;

/// The tree walks (`scan_spec_tree` / `scan_external_units`) — the seam
/// where the two serialisations' readers meet. A child module for the
/// file-length budget, like `lines` and `excludes`; re-exported so the
/// crate's callers (and the index builder) keep their paths.
mod walk;
pub use walk::{scan_external_units, scan_spec_tree};

/// Parsed kind line: `` `<kind> r<N>[ <status>]` `` + optional same-line prose.
/// `pub(crate)` — the XML reader (`xmlspec`) applies the SAME grammar to a
/// leading `<p>`'s first line, so the two frontends cannot fork on what a
/// kind declaration is.
pub(crate) struct KindLine {
    pub(crate) kind: SpecUnitKind,
    pub(crate) revision: u32,
    pub(crate) status: Option<SpecUnitStatus>,
    pub(crate) disputes: Option<String>,
}

/// Parse the backticked declaration if the line starts with one.
/// `Ok(None)` — the line is not a kind line at all; `Err` — it looks
/// like one but is malformed (warned, not fatal).
pub(crate) fn parse_kind_line(line: &str) -> Result<Option<KindLine>, String> {
    let trimmed = line.trim_start();
    let Some(rest) = trimmed.strip_prefix('`') else {
        return Ok(None);
    };
    let Some(close) = rest.find('`') else {
        return Ok(None);
    };
    let decl = &rest[..close];
    let mut words = decl.split_whitespace();
    let Some(kind_word) = words.next() else {
        return Ok(None);
    };
    let kind = match kind_word {
        "prop" => SpecUnitKind::Prop,
        "req" => SpecUnitKind::Req,
        "design" => SpecUnitKind::Design,
        "guide" => SpecUnitKind::Guide,
        // A backticked span that doesn't start with a kind word is
        // ordinary inline code, not a kind line.
        _ => return Ok(None),
    };
    let Some(rev_word) = words.next() else {
        return Err(format!("kind line `{decl}` is missing the `r<N>` revision"));
    };
    let revision: u32 = rev_word
        .strip_prefix('r')
        .and_then(|d| d.parse().ok())
        .filter(|&n| n >= 1)
        .ok_or_else(|| {
            format!(
                "kind line `{decl}` has a malformed revision `{rev_word}` (expected `r<N>`, N ≥ 1)"
            )
        })?;
    let (status, disputes) = match words.next() {
        None => (None, None),
        Some("planned") => (Some(SpecUnitStatus::Planned), None),
        Some(w) if w.starts_with("disputed(#") && w.ends_with(')') => {
            let other = &w["disputed(#".len()..w.len() - 1];
            if !is_valid_anchor(other) {
                return Err(format!(
                    "kind line `{decl}`: disputed(...) must name an anchor id `[A-Za-z][A-Za-z0-9_-]*`, got `{other}`"
                ));
            }
            (Some(SpecUnitStatus::Disputed), Some(other.to_string()))
        }
        Some(w) => {
            return Err(format!(
                "kind line `{decl}` has an unknown status `{w}` (expected `planned` or `disputed(#anchor)`)"
            ));
        }
    };
    if words.next().is_some() {
        return Err(format!("kind line `{decl}` carries trailing tokens"));
    }
    Ok(Some(KindLine {
        kind,
        revision,
        status,
        disputes,
    }))
}

/// The `duplicate-anchor` warning, shared by heading anchors and `##<ID>`
/// fact anchors — one id per document, whichever grain mints it first
/// (PROP-014 §2.1, one address space per document). `pub(crate)` — the XML
/// reader mints into the SAME namespace and must warn with the same words.
pub(crate) fn duplicate_anchor_warning(anchor: &str, file: &str, line: u32) -> Warning {
    Warning {
        code: "duplicate-anchor".to_string(),
        message: format!(
            "anchor `{{#{anchor}}}` already used earlier in this file — \
             spec://…#{anchor} is ambiguous"
        ),
        file: file.to_string(),
        line,
    }
}

/// Segment one text block (`lines[start..end]` — no blank, heading, or
/// fenced line inside) into untyped fact units.
///
/// A `##<ID>` mints a unit when it is the first token of the block's lead
/// paragraph or of any list item; a nested item is its own unit, and a
/// plain line continues the paragraph/item above it. The span is the
/// segment's own lines (continuations included) and the unit is untyped —
/// no `kind:`/revision line applies to a fact (PROP-014 §2.1). Fact ids
/// share the document's `seen_anchors`, so a duplicate — fact-vs-fact or
/// fact-vs-heading — warns exactly as a heading collision does. Returns the
/// block's units and warnings for the caller to append in document order.
fn segment_block_facts(
    lines: &[&str],
    start: usize,
    end: usize,
    file: &str,
    namespace: &str,
    doc_path: &str,
    seen_anchors: &mut Vec<String>,
) -> (Vec<SpecUnit>, Vec<Warning>) {
    let len = end - start;
    // `Some(off)` — the byte offset of a list item's content; `None` — a
    // plain line (a paragraph line, or an item's continuation).
    let markers: Vec<Option<usize>> = (start..end).map(|k| list_item_content(lines[k])).collect();

    // Each segment: (anchoring line, marker offset on it, span [lo, hi)).
    let mut segments: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut k = 0;
    // Lead: the plain lines before the first list item form one paragraph.
    if matches!(markers.first(), Some(None)) {
        let mut e = 0;
        while e + 1 < len && markers[e + 1].is_none() {
            e += 1;
        }
        segments.push((start, 0, start, start + e + 1));
        k = e + 1;
    }
    // Every later segment opens on a list item; the following plain lines
    // are its continuation, up to the next item.
    while k < len {
        let Some(off) = markers[k] else { break };
        let mut e = k;
        while e + 1 < len && markers[e + 1].is_none() {
            e += 1;
        }
        segments.push((start + k, off, start + k, start + e + 1));
        k = e + 1;
    }

    let mut units = Vec::new();
    let mut warnings = Vec::new();
    for (anchor_line, marker_off, span_lo, span_hi) in segments {
        let Some((id, heading)) = fact_anchor_at(lines[anchor_line], marker_off) else {
            continue;
        };
        let line_no = (anchor_line + 1) as u32;
        if seen_anchors.contains(&id) {
            warnings.push(duplicate_anchor_warning(&id, file, line_no));
        } else {
            seen_anchors.push(id.clone());
        }
        let span_text = lines[span_lo..span_hi].join("\n");
        units.push(SpecUnit {
            uri: format!("spec://{namespace}/{doc_path}#{id}"),
            docPath: doc_path.to_string(),
            file: file.to_string(),
            anchor: id,
            heading,
            contentHash: content_hash(&span_text),
            line: line_no,
            kind: None,
            revision: None,
            status: None,
            disputes: None,
        });
    }
    (units, warnings)
}

/// The canonical citation path used inside `spec://` URIs — the house
/// style every existing citation in the repo already uses (CLAUDE.md:
/// `spec://org.vibevm.core/vibevm/common/PROP-000#commits`): relative to
/// `vibevm/vibespecs/` (or the legacy `spec/` prefix accepted by this pure
/// parser), the serialisation extension (`.md` or `.xml` — a document's
/// address does not depend on which form it is written in) stripped, and a
/// filename carrying a document id truncated to it
/// (`modules/vibe-resolver/PROP-003-dep-evolution.md`
/// → `modules/vibe-resolver/PROP-003`). Files without a document id
/// keep their full stem (`boot/00-core`, `WAL`).
pub fn canonical_doc_path(file: &str) -> String {
    // PROP-052: strip the longer physical root first so the move to
    // `vibevm/vibespecs` never leaks into stable logical `spec://` addresses.
    let rel = file
        .strip_prefix("vibevm/vibespecs/")
        .or_else(|| file.strip_prefix("spec/"))
        .unwrap_or(file);
    let (dir, name) = match rel.rsplit_once('/') {
        Some((d, n)) => (Some(d), n),
        None => (None, rel),
    };
    let stem = name
        .strip_suffix(".md")
        .or_else(|| name.strip_suffix(".xml"))
        .unwrap_or(name);
    let mut parts = stem.split('-');
    let id = match (parts.next(), parts.next()) {
        (Some(kind @ ("PROP" | "FEAT")), Some(num))
            if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) =>
        {
            Some(format!("{kind}-{num}"))
        }
        _ => None,
    };
    let canonical_name = id.unwrap_or_else(|| stem.to_string());
    match dir {
        Some(d) => format!("{d}/{canonical_name}"),
        None => canonical_name,
    }
}

/// Parse one markdown document into units + warnings.
///
/// `file` is the forward-slash repo-relative path on disk; the URI
/// doc-path is derived via [`canonical_doc_path`], and `namespace` is
/// the `spec://<namespace>/…` segment the units are minted under
/// ([`Config::namespace`] for the project's own tree, an
/// [`ExternalSpec`](crate::config::ExternalSpec)'s namespace for an
/// installed package's tree).
/// The bare three-arg seam the unit tests call: the `long-section` quality
/// check is **disabled** here (threshold `0`), so existing assertions on "no
/// warnings" stay green. The scan entry points route through
/// [`parse_units_with`] carrying the live config.
pub fn parse_units(file: &str, text: &str, namespace: &str) -> (Vec<SpecUnit>, Vec<Warning>) {
    parse_units_with(file, text, namespace, 0, SectionGrain::Leaf)
}

/// [`parse_units`] carrying the `long-section` quality policy.
/// `max_section_lines` is inclusive (a section reaching it fires) and `0`
/// disables it; `grain` selects whether only leaf sections or every section
/// is measured.
fn parse_units_with(
    file: &str,
    text: &str,
    namespace: &str,
    max_section_lines: usize,
    grain: SectionGrain,
) -> (Vec<SpecUnit>, Vec<Warning>) {
    let doc_path = canonical_doc_path(file);
    let lines: Vec<&str> = text.lines().collect();
    let fenced = fence_mask(&lines);
    let mut units = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_anchors: Vec<String> = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        if fenced[i] {
            i += 1;
            continue;
        }
        let Some((level, heading, anchor)) = parse_heading(lines[i]) else {
            // Not an anchored heading. Blank lines and unanchored headings
            // mint no unit; every other non-fenced line opens a text block
            // that may carry `##<ID>` fact anchors (PROP-014 §2.1). Scanning
            // in document order keeps the `seen_anchors` dedup line-ordered
            // across headings and facts alike.
            if lines[i].trim().is_empty() || heading_level(lines[i]).is_some() {
                i += 1;
                continue;
            }
            let mut end = i;
            while end < lines.len()
                && !fenced[end]
                && !lines[end].trim().is_empty()
                && heading_level(lines[end]).is_none()
            {
                end += 1;
            }
            let (mut u, mut w) = segment_block_facts(
                &lines,
                i,
                end,
                file,
                namespace,
                &doc_path,
                &mut seen_anchors,
            );
            units.append(&mut u);
            warnings.append(&mut w);
            i = end;
            continue;
        };
        let heading_line_no = (i + 1) as u32;

        if !is_valid_anchor(&anchor) {
            warnings.push(Warning {
                code: "invalid-anchor".to_string(),
                message: format!(
                    "anchor `{{#{anchor}}}` is not an id `[A-Za-z][A-Za-z0-9_-]*`; unit skipped"
                ),
                file: file.to_string(),
                line: heading_line_no,
            });
            i += 1;
            continue;
        }
        if seen_anchors.contains(&anchor) {
            warnings.push(duplicate_anchor_warning(&anchor, file, heading_line_no));
        } else {
            seen_anchors.push(anchor.clone());
        }

        // Span: heading line up to (exclusive) the next same-or-higher
        // heading, anchored or not. Fenced lines never terminate a span.
        let mut end = i + 1;
        while end < lines.len() {
            if !fenced[end]
                && let Some(l) = heading_level(lines[end])
                && l <= level
            {
                break;
            }
            end += 1;
        }
        let body_lines = &lines[i..end];
        let span_text = body_lines.join("\n");

        // Kind line: first non-blank line after the heading.
        let mut kind: Option<SpecUnitKind> = None;
        let mut revision: Option<u32> = None;
        let mut status: Option<SpecUnitStatus> = None;
        let mut disputes: Option<String> = None;
        if let Some((off, first)) = lines[i + 1..end]
            .iter()
            .enumerate()
            .find(|(_, l)| !l.trim().is_empty())
        {
            match parse_kind_line(first) {
                Ok(Some(kl)) => {
                    kind = Some(kl.kind);
                    revision = Some(kl.revision);
                    status = kl.status;
                    disputes = kl.disputes;
                }
                Ok(None) => {}
                Err(msg) => warnings.push(Warning {
                    code: "malformed-kind-line".to_string(),
                    message: msg,
                    file: file.to_string(),
                    line: (i + 1 + off + 1) as u32,
                }),
            }
        }

        // Long-section quality warning (§3.3): a leaf section — one with no
        // nested subsection — past the threshold reads poorly and churns
        // often. A container section is long only because the document is,
        // which says nothing about discipline, so at `leaf` grain (the
        // default) it is not measured. Because the span already ends at the
        // next same-or-higher heading, any heading left inside the body is
        // strictly deeper — so "no heading in the body" *is* the leaf test.
        // Fenced headings are code samples, not structure, and are ignored.
        if max_section_lines != 0 {
            let is_leaf = (i + 1..end).all(|k| fenced[k] || heading_level(lines[k]).is_none());
            if grain == SectionGrain::All || is_leaf {
                let section_lines = end - i;
                if section_lines >= max_section_lines {
                    warnings.push(Warning {
                        code: "long-section".to_string(),
                        message: format!(
                            "section `{heading}` spans {section_lines} lines \
                             (threshold {max_section_lines}) — long sections read \
                             poorly and churn often; split into smaller leaves"
                        ),
                        file: file.to_string(),
                        line: heading_line_no,
                    });
                }
            }
        }

        units.push(SpecUnit {
            uri: format!("spec://{namespace}/{doc_path}#{anchor}"),
            docPath: doc_path.clone(),
            file: file.to_string(),
            anchor,
            heading,
            contentHash: content_hash(&span_text),
            line: heading_line_no,
            kind: kind.map(Box::new),
            revision: revision.map(Box::new),
            status: status.map(Box::new),
            disputes: disputes.map(Box::new),
        });
        i += 1;
    }
    (units, warnings)
}

#[cfg(test)]
mod tests;
