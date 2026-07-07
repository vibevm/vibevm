//! Markdown side of the scanner: anchored spec units.
//!
//! A unit is the span from an anchored heading (`### Title {#anchor}`)
//! to the next same-or-higher heading — anchored or not (GUIDE-SPEC-
//! AUTHORING §1). The first non-blank body line may be a kind line:
//! `` `req r2` ``, `` `req r1 planned` ``, `` `req r2 disputed(#other)` ``
//! — optionally followed by prose on the same line. Units without a kind
//! line are legacy-unmarked and still inventoried (full node inventory,
//! PROP-014 §4 Phase 0).

specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#spec-units");

use std::path::Path;

use crate::generated::specmap::{SpecUnit, SpecUnitKind, SpecUnitStatus, Warning};
use specmark_grammar::is_valid_anchor;
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
            i += 1;
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
            warnings.push(Warning {
                code: "duplicate-anchor".to_string(),
                message: format!(
                    "anchor `{{#{anchor}}}` already used earlier in this file — \
                     spec://…#{anchor} is ambiguous"
                ),
                file: file.to_string(),
                line: heading_line_no,
            });
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
mod tests {
    use super::*;

    const DOC: &str = "spec/test/DOC.md";
    const NS: &str = "project";

    fn fmt_warnings(w: &[Warning]) -> String {
        w.iter()
            .map(|x| format!("{}:{} [{}] {}", x.file, x.line, x.code, x.message))
            .collect::<Vec<_>>()
            .join("; ")
    }

    #[test]
    fn anchored_heading_becomes_a_unit_with_span_hash() {
        let text = "# Title {#root}\n\nbody one\n\n## Sub {#sub-part}\n\nbody two\n\n## Next {#next-part}\nafter\n";
        let (units, warnings) = parse_units(DOC, text, NS);
        assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
        assert_eq!(units.len(), 3);
        assert_eq!(units[0].anchor, "root");
        assert_eq!(units[0].uri, "spec://project/test/DOC#root");
        assert_eq!(units[0].docPath, "test/DOC");
        assert_eq!(units[0].file, DOC);
        assert_eq!(units[0].line, 1);
        // The root unit spans the whole document (no same-or-higher
        // heading follows); the sub unit ends before `## Next`.
        assert_eq!(units[1].anchor, "sub-part");
        assert_eq!(units[2].anchor, "next-part");
        assert_ne!(units[1].contentHash, units[2].contentHash);
    }

    #[test]
    fn unanchored_heading_ends_a_span_but_is_not_a_unit() {
        let text = "## A {#a}\nbody\n## Plain heading\nmore\n## B {#b}\nbody b\n";
        let (units, _) = parse_units(DOC, text, NS);
        assert_eq!(units.len(), 2);
        // A's span must stop at `## Plain heading`.
        let a_hash = units[0].contentHash.clone();
        let (units2, _) = parse_units(DOC, "## A {#a}\nbody\n", NS);
        assert_eq!(a_hash, units2[0].contentHash);
    }

    #[test]
    fn kind_line_parses_kind_revision_status() {
        let text = "### R {#req-x}\n`req r2`\n\nMUST hold.\n\n### P {#req-y}\n`req r1 planned`\n\n### D {#req-z}\n`req r3 disputed(#req-x)` — see the pair.\n";
        let (units, warnings) = parse_units(DOC, text, NS);
        assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
        assert!(matches!(units[0].kind.as_deref(), Some(SpecUnitKind::Req)));
        assert_eq!(units[0].revision.as_deref(), Some(&2));
        assert!(units[0].status.is_none());
        assert!(matches!(
            units[1].status.as_deref(),
            Some(SpecUnitStatus::Planned)
        ));
        assert!(matches!(
            units[2].status.as_deref(),
            Some(SpecUnitStatus::Disputed)
        ));
        assert_eq!(units[2].disputes.as_deref(), Some(&"req-x".to_string()));
    }

    #[test]
    fn ordinary_inline_code_is_not_a_kind_line() {
        let text = "### T {#t}\n`vibe install` does things.\n";
        let (units, warnings) = parse_units(DOC, text, NS);
        assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
        assert!(units[0].kind.is_none());
    }

    #[test]
    fn malformed_kind_line_warns_but_keeps_the_unit() {
        let text = "### T {#t}\n`req rX`\n";
        let (units, warnings) = parse_units(DOC, text, NS);
        assert_eq!(units.len(), 1);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "malformed-kind-line");
        let text = "### T {#t}\n`req r1 someday`\n";
        let (_, warnings) = parse_units(DOC, text, NS);
        assert_eq!(warnings[0].code, "malformed-kind-line");
    }

    #[test]
    fn duplicate_anchor_in_one_file_warns_and_keeps_both() {
        let text = "## A {#phases}\none\n## B {#phases}\ntwo\n";
        let (units, warnings) = parse_units(DOC, text, NS);
        assert_eq!(units.len(), 2);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "duplicate-anchor");
        assert_eq!(warnings[0].line, 3);
    }

    #[test]
    fn invalid_anchor_warns_and_skips() {
        let text = "## A {#Bad_Anchor}\nbody\n";
        let (units, warnings) = parse_units(DOC, text, NS);
        assert!(units.is_empty());
        assert_eq!(warnings[0].code, "invalid-anchor");
    }

    #[test]
    fn root_spec_docs_are_scanned_and_other_root_md_is_not() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("spec")).unwrap();
        std::fs::write(
            dir.path().join("spec").join("X.md"),
            "## In tree {#in-tree}\nbody\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("ROOT-SPEC.md"),
            "# demo {#root}\n\n## Section 5. The task graph {#task-graph}\nbody\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("README.md"), "# Readme {#root}\n").unwrap();
        let cfg = Config {
            root_spec_docs: vec!["ROOT-SPEC.md".into()],
            ..Config::default()
        };
        let (units, warnings) = scan_spec_tree(dir.path(), &cfg);
        assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
        let uris: Vec<&str> = units.iter().map(|u| u.uri.as_str()).collect();
        assert!(uris.contains(&"spec://project/X#in-tree"));
        assert!(uris.contains(&"spec://project/ROOT-SPEC#root"));
        assert!(uris.contains(&"spec://project/ROOT-SPEC#task-graph"));
        // README-class root markdown stays out of the inventory.
        assert_eq!(units.len(), 3);
    }

    #[test]
    fn external_specs_resolve_under_their_own_namespace_and_are_skipped_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let ext = dir.path().join("vibedeps/some-flow/0.3.0/spec");
        std::fs::create_dir_all(ext.join("mechanisms")).unwrap();
        std::fs::write(
            ext.join("mechanisms/ENGINE-X-v0.1.md"),
            "## Rules {#rules}\n`req r1`\n\nbody\n",
        )
        .unwrap();
        let cfg = Config {
            external_specs: vec![
                crate::config::ExternalSpec {
                    namespace: "some-flow".into(),
                    root: "vibedeps/some-flow/0.3.0/spec".into(),
                },
                // A not-yet-installed package: skipped, never fatal.
                crate::config::ExternalSpec {
                    namespace: "ghost".into(),
                    root: "vibedeps/ghost/1.0.0/spec".into(),
                },
            ],
            ..Config::default()
        };
        let units = scan_external_units(dir.path(), &cfg);
        assert_eq!(units.len(), 1);
        assert_eq!(
            units[0].uri,
            "spec://some-flow/mechanisms/ENGINE-X-v0.1#rules"
        );
        assert_eq!(units[0].revision.as_deref(), Some(&1));
    }

    #[test]
    fn fenced_sample_headings_are_not_units_and_do_not_cut_spans() {
        let text = "## Real {#real-unit}\nbody\n```markdown\n## Sample {#req-sample}\n`req r2`\n```\ntail\n## Next {#next-unit}\n";
        let (units, warnings) = parse_units(DOC, text, NS);
        assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].anchor, "real-unit");
        assert_eq!(units[1].anchor, "next-unit");
        // The fenced sample stays inside real-unit's span (the hash
        // covers it), it just isn't a unit of its own.
        let (units2, _) = parse_units(
            DOC,
            "## Real {#real-unit}\nbody\ntail\n## Next {#next-unit}\n",
            NS,
        );
        assert_ne!(units[0].contentHash, units2[0].contentHash);
    }

    #[test]
    fn hash_is_line_ending_invariant() {
        let lf = "## A {#a}\nbody\n";
        let crlf = "## A {#a}\r\nbody\r\n";
        let (u1, _) = parse_units(DOC, lf, NS);
        let (u2, _) = parse_units(DOC, crlf, NS);
        assert_eq!(u1[0].contentHash, u2[0].contentHash);
    }
}
