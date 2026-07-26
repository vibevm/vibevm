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

use std::path::Path;

use crate::generated::specmap::{SpecUnit, SpecUnitKind, SpecUnitStatus, Warning};
use specmark_grammar::{is_valid_anchor, is_valid_fact_id};
use walkdir::WalkDir;

use crate::config::Config;
use crate::{content_hash, fwd};

/// A heading line: 1–6 `#`, a space, text, trailing `{#anchor}`.
fn parse_heading(line: &str) -> Option<(usize, String, String)> {
    let trimmed = line.trim_end();
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    let rest = rest.trim_start();
    let open = rest.rfind("{#")?;
    if !rest.ends_with('}') {
        return None;
    }
    let anchor = &rest[open + 2..rest.len() - 1];
    let heading = rest[..open].trim_end().to_string();
    Some((hashes, heading, anchor.to_string()))
}

/// Any heading line (anchored or not) — unit spans end at these.
fn heading_level(line: &str) -> Option<usize> {
    let trimmed = line.trim_end();
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    trimmed[hashes..].starts_with(' ').then_some(hashes)
}

/// Parsed kind line: `` `<kind> r<N>[ <status>]` `` + optional same-line prose.
struct KindLine {
    kind: SpecUnitKind,
    revision: u32,
    status: Option<SpecUnitStatus>,
    disputes: Option<String>,
}

/// Parse the backticked declaration if the line starts with one.
/// `Ok(None)` — the line is not a kind line at all; `Err` — it looks
/// like one but is malformed (warned, not fatal).
fn parse_kind_line(line: &str) -> Result<Option<KindLine>, String> {
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
                    "kind line `{decl}`: disputed(...) must name a kebab-case anchor, got `{other}`"
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

/// Per-line "inside a fenced code block" mask. A line whose trimmed
/// start is ``` or ~~~ toggles the fence; heading detection is
/// suppressed inside fences so worked examples in guides do not leak
/// into the unit inventory.
fn fence_mask(lines: &[&str]) -> Vec<bool> {
    let mut mask = Vec::with_capacity(lines.len());
    let mut in_fence = false;
    for line in lines {
        let t = line.trim_start();
        let is_boundary = t.starts_with("```") || t.starts_with("~~~");
        if is_boundary {
            // The boundary line itself counts as fenced content.
            mask.push(true);
            in_fence = !in_fence;
        } else {
            mask.push(in_fence);
        }
    }
    mask
}

/// Byte offset of a list item's content when the line opens one
/// (`- ` / `* ` / `+ ` / `N. ` / `N) ` at any indent), else `None`.
///
/// A list item is where the finest fact grain lives: a `##<ID>` written as
/// the item's first token mints its own unit (PROP-014 §2.1). This mirrors
/// — without sharing code (PROP-014 §2.9 separability; the convention is
/// held by tests on both sides) — the host Progress-Control scanner's list
/// recognition.
fn list_item_content(line: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start().len();
    let rest = &line[indent..];
    for pre in ["- ", "* ", "+ "] {
        if rest.starts_with(pre) {
            return Some(indent + pre.len());
        }
    }
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if (1..=9).contains(&digits) {
        let after = &rest[digits..];
        if after.starts_with(". ") || after.starts_with(") ") {
            return Some(indent + digits + 2);
        }
    }
    None
}

/// A `##<ID>` fact anchor at `line[start..]` (leading whitespace skipped):
/// `##`, a valid fact id, then whitespace or end-of-line. Returns the id
/// and the trimmed remainder of the line — the fact's lead text, kept as
/// the unit heading.
///
/// `##` followed by an invalid id — a non-letter head (`##9bad`) or an id
/// run glued to a non-space glyph (`##bad!`) — is ordinary prose: `None`,
/// and (unlike a malformed heading anchor) no warning (PROP-014 §2.1). The
/// id charset is the wider fact grammar ([`is_valid_fact_id`]), not the
/// kebab heading law.
fn fact_anchor_at(line: &str, start: usize) -> Option<(String, String)> {
    let seg = &line[start..];
    let lead_ws = seg.len() - seg.trim_start().len();
    let rest = seg[lead_ws..].strip_prefix("##")?;
    let id_len = rest
        .chars()
        .take_while(|&c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        .count();
    if id_len == 0 {
        return None;
    }
    let id = &rest[..id_len];
    let after = &rest[id_len..];
    if !is_valid_fact_id(id) || after.chars().next().is_some_and(|c| !c.is_whitespace()) {
        return None;
    }
    Some((id.to_string(), after.trim().to_string()))
}

/// The `duplicate-anchor` warning, shared by heading anchors and `##<ID>`
/// fact anchors — one id per document, whichever grain mints it first
/// (PROP-014 §2.1, one address space per document).
fn duplicate_anchor_warning(anchor: &str, file: &str, line: u32) -> Warning {
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
/// `spec://vibevm/common/PROP-000#commits`): relative to `spec/`, the
/// `.md` extension stripped, and a filename carrying a document id
/// truncated to it (`modules/vibe-resolver/PROP-003-dep-evolution.md`
/// → `modules/vibe-resolver/PROP-003`). Files without a document id
/// keep their full stem (`boot/00-core`, `WAL`).
pub fn canonical_doc_path(file: &str) -> String {
    let rel = file.strip_prefix("spec/").unwrap_or(file);
    let (dir, name) = match rel.rsplit_once('/') {
        Some((d, n)) => (Some(d), n),
        None => (None, rel),
    };
    let stem = name.strip_suffix(".md").unwrap_or(name);
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
pub fn parse_units(file: &str, text: &str, namespace: &str) -> (Vec<SpecUnit>, Vec<Warning>) {
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
                message: format!("anchor `{{#{anchor}}}` is not kebab-case; unit skipped"),
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

/// Walk each `<spec_root>/**/*.md` under the repo root, then the explicit
/// [`Config::root_spec_docs`]. Deterministic order.
pub fn scan_spec_tree(root: &Path, cfg: &Config) -> (Vec<SpecUnit>, Vec<Warning>) {
    let mut units = Vec::new();
    let mut warnings = Vec::new();
    for spec_root_rel in &cfg.spec_roots {
        let spec_root = root.join(spec_root_rel);
        for entry in WalkDir::new(&spec_root)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let rel = path.strip_prefix(root).unwrap_or(path);
            let file_rel = fwd(rel);
            match std::fs::read_to_string(path) {
                Ok(text) => {
                    let (mut u, mut w) = parse_units(&file_rel, &text, &cfg.namespace);
                    units.append(&mut u);
                    warnings.append(&mut w);
                }
                Err(e) => warnings.push(Warning {
                    code: "unreadable-file".to_string(),
                    message: format!("could not read: {e}"),
                    file: file_rel,
                    line: 0,
                }),
            }
        }
    }
    for name in &cfg.root_spec_docs {
        let path = root.join(name);
        if !path.exists() {
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let (mut u, mut w) = parse_units(name, &text, &cfg.namespace);
                units.append(&mut u);
                warnings.append(&mut w);
            }
            Err(e) => warnings.push(Warning {
                code: "unreadable-file".to_string(),
                message: format!("could not read: {e}"),
                file: name.clone(),
                line: 0,
            }),
        }
    }
    (units, warnings)
}

/// Scan each [`Config::external_specs`] tree — an installed package's spec
/// directory — and mint its units under that package's namespace. These
/// units participate in **resolution only** (dangling suppression, suspect
/// revisions, queries); the caller never serialises them into the project's
/// own index, and their parse warnings are the package's business, not this
/// project's, so they are dropped. A missing root is reported to stderr and
/// skipped (the package may simply not be installed yet), never a failure.
pub fn scan_external_units(root: &Path, cfg: &Config) -> Vec<SpecUnit> {
    let mut units = Vec::new();
    for ext in &cfg.external_specs {
        let base = root.join(&ext.root);
        if !base.is_dir() {
            eprintln!(
                "specmap: external spec root `{}` (namespace `{}`) not found — \
                 skipped; install the package to resolve its units",
                ext.root, ext.namespace
            );
            continue;
        }
        for entry in WalkDir::new(&base)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            // Doc-paths are minted relative to the external tree itself, so
            // `<ext.root>/mechanisms/X.md` reads `spec://<ns>/mechanisms/X#…`.
            let rel = path.strip_prefix(&base).unwrap_or(path);
            if let Ok(text) = std::fs::read_to_string(path) {
                let (mut u, _w) = parse_units(&fwd(rel), &text, &ext.namespace);
                units.append(&mut u);
            }
        }
    }
    units
}

#[cfg(test)]
mod tests;
