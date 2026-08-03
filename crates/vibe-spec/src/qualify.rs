//! The qualify cell — origin-qualified label rewrite (B-011 / PROP-035 §8
//! phase 5, `##PIPE-QUALIFY`).
//!
//! When `vibe` splices a contribution into the compiled `STATIC.md`, every
//! label the contribution defines collides with its siblings' (`{#root}`
//! alone repeats 26× across the lane — the B-011 commission). This cell
//! rewrites **one** contribution's labels into a globally-unique qualified
//! form derived purely from the contribution's `<group>/<name>` origin, and
//! rewrites the contribution's own intra-document references to match — so the
//! compiled lane is collision-free *by construction*, and a label's meaning is
//! fixed when its package is authored, never when the world is assembled
//! (design `spec/design/deterministic-loading-aliasing.md` §3; the normative
//! twin PROP-035 §8 phase 5 / §11 `##COMPILED-LABELS-ARE-QUALIFIED`).
//!
//! Three rewrite rules, exactly:
//!
//! - heading anchor definitions `{#x}` on heading lines → `{#<slug>--x}`;
//! - fact-id definitions `##X` (the lead token of a paragraph or list item,
//!   outside fences) → `##<slug>--X`;
//! - intra-document markdown links `(#x)` → `(#<slug>--x)`, but **only** when
//!   `x` is a label this same contribution defines (definitions are collected
//!   first, then references rewritten against that set — an unknown `(#y)` is
//!   left untouched; resolving it is the compiler's lookup rule, not this
//!   cell's).
//!
//! Never touched: fenced code blocks (the shared [`doctree::fence_mask`]),
//! inline-code spans (backticked prose), full `spec://…` addresses,
//! `@spec://…` in-place uses, and directive lines (`#use` / `#embed` /
//! `#source`). The slug is always lowercase; the original label tail keeps its
//! case, so the normative-vs-lead casing convention stays readable in the tail.
//!
//! The cell is pure: [`qualify_contribution`] takes one contribution's text
//! and its origin and returns the rewritten text plus the rename map (document
//! order, deduplicated). It has no view of any sibling contribution, which is
//! what makes late lane additions append-only (design §6).

use std::collections::HashSet;

use crate::doctree::fence_mask;

/// The origin slug: `org.vibevm.world/wal` → `org-vibevm-world--wal`.
///
/// Lowercased; dots in the group become `-`; the group/name joiner (the `/`)
/// becomes `--` (legal under the `[A-Za-z][A-Za-z0-9_-]*` anchor grammar, and
/// distinct from a dot because `.` already means tree-path descent in
/// `spec://` addresses). A ` [shared by …]` provenance suffix — which a
/// hoisted entry may append to the origin — is dropped by taking the first
/// whitespace-separated token (the same rule `normal_seed` applies in
/// `vibe-workspace::boot_artifacts::normal`). A coordinate with no `/` (a
/// single host-like token such as `vibevm`) has no joiner: its slug is just the
/// lowercased token.
pub fn origin_slug(origin: &str) -> String {
    let coord = origin.split_whitespace().next().unwrap_or("");
    coord
        .to_ascii_lowercase()
        .replace('.', "-")
        .replace('/', "--")
}

/// One renamed label: `original` → `qualified`, for the compiled lane's
/// tombstone table (design §6.1 layer 2; PROP-035 §11
/// `##STATIC-TOMBSTONE-TABLE`). The map is in document order, deduplicated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameEntry {
    /// The label as the contribution authored it (e.g. `root`, `FACT-A`).
    pub original: String,
    /// The origin-qualified form (e.g. `org-vibevm-world--wal--root`).
    pub qualified: String,
}

/// Qualify one contribution's labels under its origin (PROP-035 §8 phase 5).
///
/// Returns the rewritten text and the rename map in document order, with each
/// label appearing once (first occurrence wins on a repeat). The rewrite is a
/// pure function of `(text, origin)` — independent of splice order and of any
/// sibling contribution — which is the append-only property (design §6).
pub fn qualify_contribution(text: &str, origin: &str) -> (String, Vec<RenameEntry>) {
    let slug = origin_slug(origin);
    // Split on `'\n'` (not `lines()`) so a trailing newline round-trips
    // byte-for-byte; `lines()` would discard it.
    let lines: Vec<String> = text.split('\n').map(String::from).collect();
    let fenced = fence_mask(&lines);

    // Pass 1 — collect the contribution's defined labels (heading anchors and
    // fact ids share one namespace, PROP-035 §7.3) so a reference can be
    // rewritten against the full set, including forward references.
    let mut defined: HashSet<String> = HashSet::new();
    let mut renames: Vec<RenameEntry> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if fenced[i] || is_directive_line(line) {
            continue;
        }
        if heading_level(line).is_some() {
            if let Some((_, inner)) = heading_anchor(line) {
                note_definition(inner.to_string(), &slug, &mut defined, &mut renames);
            }
        } else if let Some((_, id)) = fact_id(line) {
            note_definition(id.to_string(), &slug, &mut defined, &mut renames);
        }
    }

    // Pass 2 — rewrite definitions and references against the collected set.
    // Detection is pure, so re-running the same helpers yields the same spans
    // pass 1 recorded (and nothing else).
    let mut out_lines: Vec<String> = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        if fenced[i] || is_directive_line(line) {
            out_lines.push(line.clone());
            continue;
        }
        let mut rewritten = line.clone();
        if heading_level(&rewritten).is_some() {
            if let Some((range, inner)) = heading_anchor(&rewritten) {
                rewritten.replace_range(range, &format!("{slug}--{inner}"));
            }
        } else if let Some((range, id)) = fact_id(&rewritten) {
            rewritten.replace_range(range, &format!("{slug}--{id}"));
        }
        rewritten = rewrite_links(&rewritten, &slug, &defined);
        out_lines.push(rewritten);
    }

    (out_lines.join("\n"), renames)
}

/// Record a definition: insert into the dedup set and, on first sight, append a
/// rename entry (so the map follows document order of first occurrence).
fn note_definition(
    label: String,
    slug: &str,
    defined: &mut HashSet<String>,
    renames: &mut Vec<RenameEntry>,
) {
    if defined.insert(label.clone()) {
        renames.push(RenameEntry {
            qualified: format!("{slug}--{label}"),
            original: label,
        });
    }
}

/// Whether a line is an ATX heading: 1–6 `#` then a space (the vendored
/// engine's rule, mirrored from `doctree::parse_heading`). A glued `##X` with
/// no space after the hashes is a fact id, not a heading.
fn heading_level(line: &str) -> Option<u8> {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if (1..=6).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ') {
        Some(hashes as u8)
    } else {
        None
    }
}

/// The `{#anchor}` on a line, if any: the byte range of the anchor's inner
/// text (between `{#` and the first `}`) and that inner text. Mirrors
/// `doctree::split_anchor`'s "first `{#`, then its `}`" scan, but returns the
/// span so the caller can splice in place. The inner need only be non-empty —
/// a tree-path anchor (`{#a.b}`) is qualified too, and stays reversible.
fn heading_anchor(line: &str) -> Option<(std::ops::Range<usize>, &str)> {
    let open = line.find("{#")?;
    let close_rel = line[open + 2..].find('}')?;
    let close = open + 2 + close_rel;
    let inner = &line[open + 2..close];
    (!inner.is_empty()).then_some((open + 2..close, inner))
}

/// The optional leading list marker's content offset on a content line: past
/// leading whitespace, and past a `- `/`* `/`+ ` or `N. `/`N) ` marker if one
/// opens the line. Mirrors `facts::list_item_content` (the two scanners hold
/// the convention separately, by the crate's separability seam).
fn content_offset(line: &str) -> usize {
    let lead = line.len() - line.trim_start().len();
    let rest = &line[lead..];
    for pre in ["- ", "* ", "+ "] {
        if rest.starts_with(pre) {
            return lead + pre.len();
        }
    }
    let digits = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
    if (1..=9).contains(&digits) {
        let after = &rest[digits..];
        if after.starts_with(". ") || after.starts_with(") ") {
            return lead + digits + 2;
        }
    }
    lead
}

/// A `##<ID>` fact-id definition at the line's content offset, if the line
/// opens one: `##`, then a valid id `[A-Za-z][A-Za-z0-9_-]*`, then whitespace or
/// EOL. Returns the id's byte range and the id.
///
/// Recognized **only at the lead position** — a `##X` mid-line is prose, not a
/// definition (R3). A `###`/`####` run is a heading (space-required) and routed
/// to the heading path before this is reached; a glued `###X` is neither
/// heading nor fact (`strip_prefix("##")` leaves `#X` → empty id → `None`).
fn fact_id(line: &str) -> Option<(std::ops::Range<usize>, &str)> {
    let start = content_offset(line);
    let rest = line.get(start..)?;
    let after = rest.strip_prefix("##")?;
    let id_len = after
        .bytes()
        .take_while(|&b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        .count();
    if id_len == 0 {
        return None;
    }
    let head_letter = after.as_bytes()[0].is_ascii_alphabetic();
    let terminated = after
        .as_bytes()
        .get(id_len)
        .is_none_or(|b| b.is_ascii_whitespace());
    if head_letter && terminated {
        let id_start = start + 2;
        Some((id_start..id_start + id_len, &after[..id_len]))
    } else {
        None
    }
}

/// Whether a line is a directive line (`#use` / `#embed` / `#source` at the
/// start, followed by whitespace or EOL) — mirrored from
/// `directives::directive_prefix`. Such lines are never touched.
fn is_directive_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    for kw in ["#embed", "#use", "#source"] {
        if let Some(rest) = trimmed.strip_prefix(kw)
            && (rest.is_empty() || rest.starts_with(char::is_whitespace))
        {
            return true;
        }
    }
    false
}

/// Rewrite `(#x)` references in `line` to `(#<slug>--x)` where `x` is in
/// `defined`, leaving unknown references and anything inside an inline-code
/// span untouched. Inline code is handled by toggling an `in_code` flag on each
/// unescaped backtick (R2: the cheap correct treatment — skip-while-scanning,
/// since `##X` and `{#x}` are positionally immune to inline code and only the
/// `(#x)` reference scan needs the guard).
fn rewrite_links(line: &str, slug: &str, defined: &HashSet<String>) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut last = 0usize; // first not-yet-flushed byte (exclusive boundary)
    let mut i = 0usize;
    let mut in_code = false;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'`' {
            in_code = !in_code;
            i += 1;
            continue;
        }
        if !in_code
            && b == b'('
            && bytes.get(i + 1) == Some(&b'#')
            && let Some((id, after_id)) = read_anchor_id(bytes, i + 2)
            && bytes.get(after_id) == Some(&b')')
            && defined.contains(id)
        {
            out.push_str(&line[last..i]);
            out.push_str("(#");
            out.push_str(slug);
            out.push_str("--");
            out.push_str(id);
            out.push(')');
            last = after_id + 1;
            i = after_id + 1;
            continue;
        }
        i += 1;
    }
    out.push_str(&line[last..]);
    out
}

/// Read an anchor id `[A-Za-z][A-Za-z0-9_-]*` starting at `start`, returning
/// the id (a `&str` slice of the line — ASCII, so valid UTF-8) and the byte
/// position just past it. `None` if the head byte is not an ASCII letter.
fn read_anchor_id(bytes: &[u8], start: usize) -> Option<(&str, usize)> {
    let head = *bytes.get(start)?;
    if !head.is_ascii_alphabetic() {
        return None;
    }
    let mut end = start + 1;
    while let Some(&b) = bytes.get(end)
        && (b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        end += 1;
    }
    // The bytes are ASCII by construction, so this never fails.
    let id = std::str::from_utf8(&bytes[start..end]).expect("ascii anchor id");
    Some((id, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- 1. Golden — every category, exact output ---------------------------

    #[test]
    fn golden_contribution_rewrites_every_category() {
        let slug = "org-vibevm-world--wal";
        // Exercises: a heading anchor, a paragraph fact, a list-item fact,
        // intra-links to a heading and to a fact, an unknown intra-link,
        // a fenced block carrying fake labels (incl. a (#root) that IS
        // defined — it must stay unrewritten inside the fence), an inline-code
        // span carrying a defined (#root), a full spec:// address, an @spec://
        // in-place use, and a directive line.
        let input = "\
# Heading One {#root}

##FACT-ONE The first fact.

See [the link](#root) and [the fact](#FACT-ONE) and [none](#missing).

- ##FACT-TWO A list fact.

```
# fake ##root and (#never) and (#root)
```

Inline code: `##root` and `(#root)`.

@spec://org.vibevm.world/wal/doc#root is a use.

#use spec://org.vibevm.world/wal/doc#root
";
        let expected = format!(
            "\
# Heading One {{#{slug}--root}}

##{slug}--FACT-ONE The first fact.

See [the link](#{slug}--root) and [the fact](#{slug}--FACT-ONE) and [none](#missing).

- ##{slug}--FACT-TWO A list fact.

```
# fake ##root and (#never) and (#root)
```

Inline code: `##root` and `(#root)`.

@spec://org.vibevm.world/wal/doc#root is a use.

#use spec://org.vibevm.world/wal/doc#root
"
        );

        let (out, renames) = qualify_contribution(input, "org.vibevm.world/wal");

        assert_eq!(out, expected);
        assert_eq!(
            renames
                .iter()
                .map(|r| r.original.as_str())
                .collect::<Vec<_>>(),
            vec!["root", "FACT-ONE", "FACT-TWO"],
            "rename map is in document order"
        );
        assert_eq!(
            renames
                .iter()
                .map(|r| r.qualified.as_str())
                .collect::<Vec<_>>(),
            vec![
                "org-vibevm-world--wal--root",
                "org-vibevm-world--wal--FACT-ONE",
                "org-vibevm-world--wal--FACT-TWO",
            ]
        );
    }

    // ---- 2. Prefix-reversibility -------------------------------------------

    #[test]
    fn stripping_the_prefix_restores_the_original() {
        // Idempotence-by-construction is not claimed; instead the prefix is
        // reversible: applying the rename map in reverse (longest qualified
        // first, so a label that prefixes another is handled cleanly) restores
        // the original byte-for-byte.
        let input = "# H {#root}\n##FACT-A the statement\nSee [r](#root) and [f](#FACT-A).\n";
        let (qualified, renames) = qualify_contribution(input, "org.vibevm.world/wal");

        // The structural form first (it only borrows `qualified`): every
        // change inserts exactly `<slug>--`, and that substring is absent from
        // the input, so removing it restores the original.
        assert_eq!(
            qualified.replace("org-vibevm-world--wal--", ""),
            input,
            "every rewrite is a pure insertion of the slug prefix"
        );

        // And the map-driven form: applying the rename map in reverse (longest
        // qualified first, so a label that prefixes another is handled cleanly)
        // restores the original byte-for-byte.
        let mut sorted = renames.clone();
        sorted.sort_by_key(|r| std::cmp::Reverse(r.qualified.len()));
        let mut restored = qualified;
        for r in &sorted {
            restored = restored.replace(&r.qualified, &r.original);
        }
        assert_eq!(restored, input);
    }

    // ---- 3. Append-independence --------------------------------------------

    #[test]
    fn qualify_is_independent_of_sibling_contributions() {
        // The signature takes ONE contribution's text and its origin — no view
        // of any sibling — so qualifying A is byte-identical whether or not B
        // exists. The property is enforced structurally: there is no
        // cross-input for it to depend on.
        let a = "# A {#root}\n##A-FACT x\n[link](#root)\n";
        let b = "# B {#root}\n##B-FACT y\n[link](#root)\n";

        let (qa, ra) = qualify_contribution(a, "org.vibevm.world/aaa");
        // B is qualified in a separate call A can never observe.
        let _ = qualify_contribution(b, "org.vibevm.world/bbb");
        let (qa2, ra2) = qualify_contribution(a, "org.vibevm.world/aaa");

        assert_eq!(qa, qa2);
        assert_eq!(ra, ra2);
        // A's labels carry A's origin, never B's.
        assert!(qa.contains("org-vibevm-world--aaa--root"));
        assert!(!qa.contains("org-vibevm-world--bbb"));
    }

    // ---- 4. Slug edge cases ------------------------------------------------

    #[test]
    fn origin_slug_edge_cases() {
        // Dotted group: dots -> `-`, the group/name `/` -> `--`.
        assert_eq!(origin_slug("org.vibevm.world/wal"), "org-vibevm-world--wal");
        // A single host-like token (no `/`): slug is the lowercased token, no
        // joiner.
        assert_eq!(origin_slug("vibevm"), "vibevm");
        // A ` [shared by ...]` provenance suffix is dropped (normal_seed's rule).
        assert_eq!(
            origin_slug("org.vibevm.world/wal [shared by a/b]"),
            "org-vibevm-world--wal"
        );
        // Always lowercased.
        assert_eq!(origin_slug("Org.VIBEVM.World/Wal"), "org-vibevm-world--wal");
    }

    #[test]
    fn single_token_origin_qualifies_labels_without_a_joiner() {
        // A no-`/` origin mints a joiner-less slug, so the qualified form is
        // `<token>--<label>` (one `--`, the label joiner — no group/name join).
        let (out, renames) = qualify_contribution("# H {#root}\n", "vibevm");
        assert_eq!(out, "# H {#vibevm--root}\n");
        assert_eq!(renames[0].qualified, "vibevm--root");
    }

    // ---- 5. Empty rename map -----------------------------------------------

    #[test]
    fn contribution_with_no_labels_is_unchanged_with_empty_map() {
        let input = "# A plain heading\n\nSome prose with no labels or links.\n";
        let (out, renames) = qualify_contribution(input, "org.vibevm.world/wal");
        assert!(renames.is_empty());
        assert_eq!(out, input);
    }

    // ---- R3 pinned: a `###` run is never a fact id -------------------------

    #[test]
    fn h3_run_and_glued_triple_hash_are_not_fact_ids() {
        // R3: a spaced `### Heading` is a heading (its anchor IS qualified);
        // a glued `###GLUED` is neither heading nor fact. Neither mints a fact.
        let (out, renames) =
            qualify_contribution("### Heading {#h}\n###GLUED prose\n", "vibevm/wal");
        assert_eq!(out, "### Heading {#vibevm--wal--h}\n###GLUED prose\n");
        assert_eq!(
            renames
                .iter()
                .map(|r| r.original.as_str())
                .collect::<Vec<_>>(),
            vec!["h"]
        );
    }

    #[test]
    fn mid_line_double_hash_is_not_a_fact_definition() {
        // A `##X` that is not the line's lead token is prose, not a definition.
        let (out, renames) =
            qualify_contribution("lead text then ##NOT-A-FACT here\n", "vibevm/wal");
        assert_eq!(out, "lead text then ##NOT-A-FACT here\n");
        assert!(renames.is_empty());
    }

    #[test]
    fn unknown_intra_link_is_left_untouched() {
        // A (#y) whose target this contribution does not define is not ours to
        // resolve — left byte-identical.
        let (out, renames) = qualify_contribution("[x](#missing)\n", "vibevm/wal");
        assert_eq!(out, "[x](#missing)\n");
        assert!(renames.is_empty());
    }
}
