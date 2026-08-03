//! The directive scanner (PROP-035 §7).
//!
//! Finds the preprocessor directives in a document and parses their addresses:
//!
//! - `#embed [options] <spec://…>` — a macro splice (§7.1);
//! - `#use [options] <spec://…>` — a dependency edge (§7.2);
//! - `#source [options] <spec://…>` — a contract→impl edge (§7.3);
//! - `@spec://…` — an in-place use, mandatory to read (§7.4).
//!
//! A directive keyword (`#embed` / `#use` / `#source`) is recognised only at
//! the start of a line (after leading whitespace) and only when followed by
//! whitespace — so it never collides with a Markdown heading (`# text`, which
//! needs a space after the `#`) nor with prose. Directives and `@spec` inside
//! fenced code blocks are ignored, exactly as headings are.
//!
//! Scanning stops at parsing; associating a directive with the node it sits in,
//! ordering the use-graph, and expanding embeds are the pipeline's job (§8),
//! which uses the line numbers recorded here.
//!
//! A **bare** `spec://…` (no `@`) is a discretionary reference, not a mandatory
//! in-place use, so it is deliberately *not* collected here (PROP-035 §7.4).

use std::collections::BTreeMap;

use crate::address::SpecAddress;
use crate::doctree::fence_mask;

/// Which preprocessor directive a line carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectiveKind {
    Embed,
    Use,
    Source,
}

impl DirectiveKind {
    /// The directive keyword, `#`-prefixed.
    pub fn keyword(self) -> &'static str {
        match self {
            DirectiveKind::Embed => "#embed",
            DirectiveKind::Use => "#use",
            DirectiveKind::Source => "#source",
        }
    }
}

/// A parsed `#embed` / `#use` / `#source` directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directive {
    pub kind: DirectiveKind,
    /// The raw options between the keyword and the address (may be empty).
    pub options: String,
    pub address: SpecAddress,
    /// 0-based source line.
    pub line: usize,
}

/// An `@spec://…` in-place use (§7.4) — mandatory to read on first encounter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InPlaceUse {
    pub address: SpecAddress,
    /// 0-based source line.
    pub line: usize,
}

/// A malformed directive or `@spec`, reported rather than fatal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectiveError {
    pub line: usize,
    pub message: String,
}

/// Everything a directive scan finds, in document order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Directives {
    pub directives: Vec<Directive>,
    pub in_place_uses: Vec<InPlaceUse>,
    pub errors: Vec<DirectiveError>,
    /// The file's `#use … as <Alias>` bindings (B-011): alias name → the
    /// declared address. `BTreeMap` so iteration is deterministic for the
    /// downstream tombstone/preamble emit. Whole-file scope: pass 1 collects
    /// every declaration before pass 2 resolves `@!X`, so a declaration may
    /// appear anywhere in the file. A duplicate name is reported in
    /// [`errors`](Self::errors); the first declaration wins here.
    pub aliases: BTreeMap<String, SpecAddress>,
}

impl Directives {
    /// Scan a document for directives and in-place uses. Infallible: malformed
    /// directives land in [`errors`](Self::errors), not a `Result`.
    pub fn parse(source: &str) -> Self {
        let lines: Vec<String> = source.lines().map(String::from).collect();
        let fenced = fence_mask(&lines);
        let commented = comment_mask(&lines);
        let masked = |i: usize| fenced[i] || commented[i];
        let mut out = Directives::default();

        // Pass 1 — fences + directive lines: collect directives AND alias
        // declarations. The alias table is completed here, so its declaration
        // position in the file does not matter (whole-file scope, B-011 §4).
        // `alias_first_line` is parse scratch for the duplicate-alias report.
        let mut alias_first_line: BTreeMap<String, usize> = BTreeMap::new();
        for (i, line) in lines.iter().enumerate() {
            if masked(i) {
                continue;
            }
            if let Some((kind, rest)) = directive_prefix(line.trim_start()) {
                out.push_directive(kind, rest, i, &mut alias_first_line);
            }
        }

        // Pass 2 — in-place scans (`@spec://`, `@!`) resolve against the
        // completed alias table. The fence and comment masks govern both.
        for (i, line) in lines.iter().enumerate() {
            if masked(i) {
                continue;
            }
            out.scan_in_place(line, i);
            out.scan_alias_use(line, i);
        }
        out
    }

    fn push_directive(
        &mut self,
        kind: DirectiveKind,
        rest: &str,
        line: usize,
        alias_first_line: &mut BTreeMap<String, usize>,
    ) {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        let Some(idx) = tokens.iter().position(|t| t.starts_with("spec://")) else {
            self.errors.push(DirectiveError {
                line,
                message: format!("{} directive has no spec:// address", kind.keyword()),
            });
            return;
        };
        let address = match SpecAddress::parse(tokens[idx]) {
            Ok(address) => address,
            Err(e) => {
                self.errors.push(DirectiveError {
                    line,
                    message: format!("{} has a bad address: {e}", kind.keyword()),
                });
                return;
            }
        };

        // R5 (B-011): a `spec://` address that names a generated static lane
        // (`…/boot/STATIC`) is an illegal citation target — the lane is a cache,
        // source-of-truth is the package source. Rejected at the single address
        // chokepoint, before any directive-kind or alias handling runs, so it is
        // caught for every directive AND every `#use … as` binding alike
        // (PROP-035 §11 `##COMPILED-LANE-IS-NOT-A-CITATION-TARGET`).
        if let Some(message) = lane_citation_error(&address) {
            self.errors.push(DirectiveError { line, message });
            return;
        }

        // The token tail after the address was silently ignored pre-B-011; it
        // is now reported for every directive kind. The only legal tail is
        // `#use`'s `as <Alias>` clause (refinement point R1).
        let tail: &[&str] = &tokens[idx + 1..];
        let alias_name = match classify_tail(kind, tail) {
            Ok(name) => name,
            Err(message) => {
                self.errors.push(DirectiveError { line, message });
                return;
            }
        };

        if let Some(name) = alias_name {
            // `#use … as <Alias>`: record the binding. A duplicate name is
            // reported (both lines, 1-based for the human reader) but does not
            // overwrite the first declaration, and does not block the
            // directive — which is still a valid dependency edge.
            if let Some(&first) = alias_first_line.get(name) {
                self.errors.push(DirectiveError {
                    line,
                    message: format!(
                        "duplicate alias `{name}`: first declared on line {}, again on line {}",
                        first + 1,
                        line + 1,
                    ),
                });
            } else {
                alias_first_line.insert(name.to_string(), line);
                self.aliases.insert(name.to_string(), address.clone());
            }
        }

        self.directives.push(Directive {
            kind,
            options: tokens[..idx].join(" "),
            address,
            line,
        });
    }

    fn scan_in_place(&mut self, line: &str, line_no: usize) {
        for (pos, _) in line.match_indices("@spec://") {
            let run = address_run(&line[pos + 1..]); // skip the '@'
            match SpecAddress::parse(run) {
                Ok(address) => {
                    // R5 (B-011): the lane is not a citation target — reject an
                    // `@spec://…/boot/STATIC` the same way a directive address is
                    // rejected above, at the one chokepoint both share.
                    if let Some(message) = lane_citation_error(&address) {
                        self.errors.push(DirectiveError {
                            line: line_no,
                            message,
                        });
                    } else {
                        self.in_place_uses.push(InPlaceUse {
                            address,
                            line: line_no,
                        });
                    }
                }
                Err(e) => self.errors.push(DirectiveError {
                    line: line_no,
                    message: format!("bad @spec in-place use: {e}"),
                }),
            }
        }
    }

    /// Scan a prose line for `@!<Alias>` in-place uses (B-011 §7.4) — the
    /// aliased twin of `@spec://`. A declared alias resolves to its target
    /// address and is pushed into [`in_place_uses`](Self::in_place_uses) just
    /// like an `@spec://`; an undeclared one is reported with the file's known
    /// aliases (sorted, since `aliases` is a `BTreeMap`). Runs in pass 2, so the
    /// table is already complete — a declaration after the use still binds it.
    fn scan_alias_use(&mut self, line: &str, line_no: usize) {
        for (pos, _) in line.match_indices("@!") {
            let name = identifier_run(&line[pos + 2..]); // skip the `@!`
            if name.is_empty() {
                // `@!` with no identifier following is not a reference (the
                // sigil is grammar only as `@!<id>`); ignore it.
                continue;
            }
            match self.aliases.get(name) {
                Some(address) => self.in_place_uses.push(InPlaceUse {
                    address: address.clone(),
                    line: line_no,
                }),
                None => {
                    let known: Vec<&str> = self.aliases.keys().map(String::as_str).collect();
                    let listing = if known.is_empty() {
                        "none declared".to_string()
                    } else {
                        known.join(", ")
                    };
                    self.errors.push(DirectiveError {
                        line: line_no,
                        message: format!("undeclared alias `@!{name}` (known aliases: {listing})"),
                    });
                }
            }
        }
    }
}

/// If `line` starts with a directive keyword followed by whitespace (or end of
/// line), return the kind and the trimmed remainder.
fn directive_prefix(line: &str) -> Option<(DirectiveKind, &str)> {
    for kind in [
        DirectiveKind::Embed,
        DirectiveKind::Use,
        DirectiveKind::Source,
    ] {
        if let Some(rest) = line.strip_prefix(kind.keyword())
            && (rest.is_empty() || rest.starts_with(char::is_whitespace))
        {
            return Some((kind, rest.trim_start()));
        }
    }
    None
}

/// R5 (B-011, design §6.1 layer 1): the compiled `spec/boot/STATIC.md` lane is a
/// generated cache, not a citation target — source-of-truth is the package
/// source under `vibedeps/`. An address whose document path names it (`boot/STATIC`
/// or `…/boot/STATIC`) is rejected with a PROP-035 §11
/// `##COMPILED-LANE-IS-NOT-A-CITATION-TARGET` citation. The path-boundary check
/// (`== "boot/STATIC"` or `.ends_with("/boot/STATIC")`) avoids matching an
/// unrelated stem that merely ends in those letters.
fn lane_citation_error(addr: &SpecAddress) -> Option<String> {
    let p = &addr.doc_path;
    if p == "boot/STATIC" || p.ends_with("/boot/STATIC") {
        Some(format!(
            "spec:// address targets the compiled static lane `{addr}` — a generated \
             cache, not a citation target (PROP-035 §11 \
             ##COMPILED-LANE-IS-NOT-A-CITATION-TARGET); cite the package source \
             under vibedeps/ instead"
        ))
    } else {
        None
    }
}

/// The verdict on the tokens following a directive's address (B-011): returns
/// the alias name a legal `#use … as <Alias>` clause binds, or an error message
/// describing the defect. The empty tail is always legal and binds nothing.
///
/// Pre-B-011 the tail was silently ignored; every non-empty tail is now an
/// error — the `as` clause is the one exception, and only on `#use`
/// (refinement point R1).
fn classify_tail<'a>(kind: DirectiveKind, tail: &'a [&'a str]) -> Result<Option<&'a str>, String> {
    if tail.is_empty() {
        return Ok(None);
    }
    let starts_with_as = tail.first() == Some(&"as");
    match kind {
        DirectiveKind::Use if starts_with_as => match tail.len() {
            1 => Err("`as` clause needs an alias name".to_string()),
            2 => {
                let name = tail[1];
                if is_alias_name(name) {
                    Ok(Some(name))
                } else {
                    Err(format!(
                        "alias name `{name}` is not a valid identifier \
                         (expected [A-Za-z][A-Za-z0-9_-]*)"
                    ))
                }
            }
            _ => Err("`as` clause takes exactly one alias name".to_string()),
        },
        DirectiveKind::Use => Err(format!(
            "unexpected tokens after address: {}",
            tail.join(" ")
        )),
        // `as` is a `#use` clause; on `#embed`/`#source` it is a defect.
        DirectiveKind::Embed | DirectiveKind::Source if starts_with_as => Err(format!(
            "`as` is a `#use` clause, not valid on {}",
            kind.keyword()
        )),
        DirectiveKind::Embed | DirectiveKind::Source => Err(format!(
            "unexpected tokens after address: {}",
            tail.join(" ")
        )),
    }
}

/// An alias name: an identifier under the anchor-segment grammar
/// `[A-Za-z][A-Za-z0-9_-]*` (PROP-035 §6, the same rule `address.rs` applies
/// per anchor segment). A local twin rather than a re-export: the seam
/// convention (PROP-035 §4) keeps `vibe-spec`'s identifier rule mirrored beside
/// the vendored grammar, not shared across it.
fn is_alias_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A precomputed mask marking lines inside HTML comments (`<!-- … -->`),
/// including the marker lines themselves. A comment is machinery, not
/// authored directive text: the compiled lane's resolution preamble quotes
/// `#use … as X` / `@!X` verbatim inside one, provenance and `vibe:begin`
/// markers carry addresses inside them, and none of that is a declaration
/// or a use. Line-grained like [`fence_mask`] — the scanners it guards are
/// line-oriented, and a directive can only ever start a line, so a
/// mid-line `<!-- -->` on a content line masks nothing it should not.
fn comment_mask(lines: &[String]) -> Vec<bool> {
    let mut mask = vec![false; lines.len()];
    let mut open = false;
    for (i, line) in lines.iter().enumerate() {
        let started_open = open;
        let mut saw_comment = started_open;
        let mut content_outside = false;
        let mut rest = line.as_str();
        loop {
            if open {
                match rest.find("-->") {
                    Some(pos) => {
                        open = false;
                        rest = &rest[pos + 3..];
                    }
                    None => break,
                }
            } else {
                match rest.find("<!--") {
                    Some(pos) => {
                        if !rest[..pos].trim().is_empty() {
                            content_outside = true;
                        }
                        saw_comment = true;
                        open = true;
                        rest = &rest[pos + 4..];
                    }
                    None => {
                        if !rest.trim().is_empty() {
                            content_outside = true;
                        }
                        break;
                    }
                }
            }
        }
        // Masked: a line that starts inside an open comment, leaves one
        // open, or is comment-only. A content line that merely contains a
        // closed inline comment stays scannable.
        mask[i] = started_open || open || (saw_comment && !content_outside);
    }
    mask
}

/// The address run of an `@spec` starting at `spec://`: everything up to
/// whitespace or a closing bracket/quote, with trailing sentence punctuation
/// trimmed (so `(@spec://a/b#c).` yields `spec://a/b#c`).
fn address_run(s: &str) -> &str {
    let end = s
        .find(|c: char| c.is_whitespace() || matches!(c, ')' | ']' | '>' | '"' | '\'' | '`' | '|'))
        .unwrap_or(s.len());
    s[..end].trim_end_matches(['.', ',', ';', ':', '!', '?'])
}

/// The identifier run after `@!`: the longest prefix matching the alias-name
/// grammar `[A-Za-z][A-Za-z0-9_-]*`. Empty when the next char is not a letter
/// — so `@!.text` and `@!9x` bind nothing (a digit cannot start a name).
/// Trailing punctuation naturally terminates the run (`@!X.` → `X`, `(@!X)` →
/// `X`), consistent with [`address_run`]'s trimming philosophy (refinement
/// point R3).
///
/// `pub(crate)` so the static compiler's `@!X` → `@spec://` rewrite (PROP-035
/// §8 phase 5 / B-011) reuses the exact same identifier boundary this scanner
/// already honours — the two never drift apart on what counts as a name.
pub(crate) fn identifier_run(s: &str) -> &str {
    let mut end = 0;
    for (i, c) in s.char_indices() {
        let valid = if i == 0 {
            c.is_ascii_alphabetic()
        } else {
            c.is_ascii_alphanumeric() || c == '-' || c == '_'
        };
        if !valid {
            break;
        }
        end = i + c.len_utf8();
    }
    &s[..end]
}

#[cfg(test)]
mod tests;
