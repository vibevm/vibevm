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
        let mut out = Directives::default();

        // Pass 1 — fences + directive lines: collect directives AND alias
        // declarations. The alias table is completed here, so its declaration
        // position in the file does not matter (whole-file scope, B-011 §4).
        // `alias_first_line` is parse scratch for the duplicate-alias report.
        let mut alias_first_line: BTreeMap<String, usize> = BTreeMap::new();
        for (i, line) in lines.iter().enumerate() {
            if fenced[i] {
                continue;
            }
            if let Some((kind, rest)) = directive_prefix(line.trim_start()) {
                out.push_directive(kind, rest, i, &mut alias_first_line);
            }
        }

        // Pass 2 — in-place scans (`@spec://`, `@!`) resolve against the
        // completed alias table. The fence mask still governs both.
        for (i, line) in lines.iter().enumerate() {
            if fenced[i] {
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
                Ok(address) => self.in_place_uses.push(InPlaceUse {
                    address,
                    line: line_no,
                }),
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
fn identifier_run(s: &str) -> &str {
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
mod tests {
    use super::*;

    #[test]
    fn parses_the_three_directives() {
        let src = "\
#embed spec://vibevm/a/b#x
#use spec://org.vibevm.demo/lib/contract/API#root
#source spec://vibevm/c/d#y
";
        let d = Directives::parse(src);
        assert_eq!(d.directives.len(), 3);
        assert_eq!(d.errors, vec![]);
        assert_eq!(d.directives[0].kind, DirectiveKind::Embed);
        assert_eq!(d.directives[1].kind, DirectiveKind::Use);
        assert_eq!(d.directives[2].kind, DirectiveKind::Source);
        assert_eq!(d.directives[0].address.doc_path, "a/b");
        assert_eq!(d.directives[1].line, 1);
    }

    #[test]
    fn parses_options() {
        let d = Directives::parse("#embed once spec://vibevm/a/b#x\n");
        assert_eq!(d.directives[0].options, "once");
        assert_eq!(d.directives[0].address.doc_path, "a/b");
    }

    #[test]
    fn collects_in_place_uses_from_prose() {
        let src = "See @spec://vibevm/common/PROP-000#commits for the rules.\n";
        let d = Directives::parse(src);
        assert_eq!(d.in_place_uses.len(), 1);
        assert_eq!(d.in_place_uses[0].address.doc_path, "common/PROP-000");
        assert_eq!(d.in_place_uses[0].address.anchor, vec!["commits"]);
    }

    #[test]
    fn trims_brackets_and_sentence_punctuation() {
        let d = Directives::parse("(@spec://vibevm/a/b#c).\n");
        assert_eq!(d.in_place_uses.len(), 1);
        assert_eq!(
            d.in_place_uses[0].address.without_pin(),
            "spec://vibevm/a/b#c"
        );
    }

    /// R2 probe (B-011): what does the existing `@spec://` scan do inside inline
    /// backticks? The closing backtick terminates `address_run`, so the run
    /// parses — the use IS collected. The `@!` scanner must mirror this. This
    /// test pins the discovered behaviour so the mirror stays honest.
    #[test]
    fn probe_spec_in_inline_backticks_is_collected() {
        let d = Directives::parse("see `@spec://vibevm/a/b#c` here\n");
        assert_eq!(d.in_place_uses.len(), 1);
        assert_eq!(d.in_place_uses[0].address.doc_path, "a/b");
        assert_eq!(d.in_place_uses[0].address.anchor, vec!["c"]);
    }

    #[test]
    fn multiple_in_place_uses_on_one_line() {
        let d = Directives::parse("@spec://vibevm/a#x and @spec://vibevm/b#y\n");
        assert_eq!(d.in_place_uses.len(), 2);
    }

    #[test]
    fn bare_spec_is_not_an_in_place_use() {
        // No `@` sigil → discretionary reference, not collected.
        let d = Directives::parse("see spec://vibevm/a/b#c here\n");
        assert!(d.in_place_uses.is_empty());
        assert!(d.directives.is_empty());
    }

    #[test]
    fn directives_in_fences_are_ignored() {
        let src = "\
#use spec://vibevm/real#x
```
#use spec://vibevm/fake#y
@spec://vibevm/fake#z
```
";
        let d = Directives::parse(src);
        assert_eq!(d.directives.len(), 1);
        assert_eq!(d.directives[0].address.doc_path, "real");
        assert!(d.in_place_uses.is_empty());
    }

    #[test]
    fn heading_is_not_a_directive() {
        // A real heading (`# text`, space after `#`) is not a directive.
        let d = Directives::parse("# Use the thing {#use-it}\nbody\n");
        assert!(d.directives.is_empty());
    }

    #[test]
    fn bad_address_is_reported() {
        // A digit-headed anchor: `#Bad` is a legal fact id and no longer serves.
        let d = Directives::parse("#use spec://vibevm/a/b#9lives\n");
        assert!(d.directives.is_empty());
        assert_eq!(d.errors.len(), 1);
        assert!(d.errors[0].message.contains("bad address"));
    }

    #[test]
    fn directive_without_address_is_reported() {
        let d = Directives::parse("#embed nothing-here\n");
        assert_eq!(d.errors.len(), 1);
        assert!(d.errors[0].message.contains("no spec:// address"));
    }

    // ---- B-011: the `as` clause and the `@!X` sigil ------------------------

    #[test]
    fn as_clause_binds_alias() {
        let d = Directives::parse("#use spec://vibevm/a/b#c as root\n");
        assert_eq!(d.directives.len(), 1);
        assert_eq!(d.directives[0].kind, DirectiveKind::Use);
        assert_eq!(d.directives[0].options, "");
        assert_eq!(d.directives[0].address.doc_path, "a/b");
        assert_eq!(d.errors, vec![]);
        let addr = d.aliases.get("root").expect("alias `root` declared");
        assert_eq!(addr.without_pin(), "spec://vibevm/a/b#c");
    }

    #[test]
    fn options_before_address_still_parse_with_as() {
        let d = Directives::parse("#use once spec://vibevm/a/b#c as root\n");
        assert_eq!(d.directives[0].options, "once");
        assert_eq!(d.directives[0].address.doc_path, "a/b");
        assert_eq!(
            d.aliases.get("root").map(|a| a.doc_path.clone()),
            Some("a/b".to_string())
        );
    }

    #[test]
    fn duplicate_alias_is_reported_and_first_wins() {
        let src = "\
#use spec://vibevm/a/b#c as root
#use spec://vibevm/x/y#z as root
";
        let d = Directives::parse(src);
        // Both directives still land — each is a valid dependency edge.
        assert_eq!(d.directives.len(), 2);
        // ...but the alias table keeps the first declaration.
        assert_eq!(
            d.aliases.get("root").map(|a| a.doc_path.clone()),
            Some("a/b".to_string())
        );
        // Exactly one error naming the alias and both lines (1-based).
        assert_eq!(d.errors.len(), 1);
        let msg = &d.errors[0].message;
        assert!(msg.contains("duplicate alias"), "{msg}");
        assert!(msg.contains("`root`"), "{msg}");
        assert!(msg.contains("line 1"), "{msg}");
        assert!(msg.contains("line 2"), "{msg}");
    }

    #[test]
    fn trailing_tokens_after_address_error_for_every_kind() {
        // R1: the silently-ignored tail is now an error, for each directive kind.
        for kw in ["#use", "#embed", "#source"] {
            let src = format!("{kw} spec://vibevm/a/b#c junk after\n");
            let d = Directives::parse(&src);
            assert!(
                d.directives.is_empty(),
                "{kw}: directive must not land on trailing junk"
            );
            assert_eq!(d.errors.len(), 1, "{kw}: expected one error");
            assert!(
                d.errors[0]
                    .message
                    .contains("unexpected tokens after address"),
                "{kw}: {}",
                d.errors[0].message
            );
        }
    }

    #[test]
    fn as_clause_on_embed_and_source_errors() {
        // `as` is a `#use` clause; on `#embed`/`#source` it is a defect.
        for kw in ["#embed", "#source"] {
            let src = format!("{kw} spec://vibevm/a/b#c as root\n");
            let d = Directives::parse(&src);
            assert!(d.directives.is_empty(), "{kw} … as: must not land");
            assert_eq!(d.errors.len(), 1, "{kw} … as: expected one error");
            assert!(
                d.errors[0].message.contains("`as` is a `#use` clause"),
                "{kw}: {}",
                d.errors[0].message
            );
        }
    }

    #[test]
    fn as_clause_without_name_or_with_bad_name_errors() {
        // `as` with no name.
        let d = Directives::parse("#use spec://vibevm/a/b#c as\n");
        assert!(d.directives.is_empty());
        assert_eq!(d.errors.len(), 1);
        assert!(d.errors[0].message.contains("needs an alias name"));

        // `as` with a digit-headed name (violates the identifier grammar).
        let d = Directives::parse("#use spec://vibevm/a/b#c as 9bad\n");
        assert!(d.directives.is_empty());
        assert_eq!(d.errors.len(), 1);
        assert!(d.errors[0].message.contains("not a valid identifier"));

        // `as` with more than one name.
        let d = Directives::parse("#use spec://vibevm/a/b#c as a b\n");
        assert!(d.directives.is_empty());
        assert_eq!(d.errors.len(), 1);
        assert!(d.errors[0].message.contains("exactly one alias name"));
    }

    #[test]
    fn at_bang_resolves_alias_declared_later_in_file() {
        // Whole-file scope (B-011 §4): the `@!root` use appears BEFORE the
        // `as root` declaration — pass 2 resolves against the completed table.
        let src = "\
Sees @!root here on line 1.
#use spec://vibevm/a/b#c as root
";
        let d = Directives::parse(src);
        assert_eq!(d.errors, vec![]);
        assert_eq!(d.in_place_uses.len(), 1);
        assert_eq!(d.in_place_uses[0].address.doc_path, "a/b");
        assert_eq!(d.in_place_uses[0].address.anchor, vec!["c"]);
        assert_eq!(d.in_place_uses[0].line, 0);
    }

    #[test]
    fn at_bang_undeclared_alias_lists_known_aliases() {
        let src = "\
#use spec://vibevm/a/b#c as root
Refers to @!missing and @!root.
";
        let d = Directives::parse(src);
        // `@!root` resolves; `@!missing` is the one error.
        assert_eq!(d.in_place_uses.len(), 1);
        assert_eq!(d.errors.len(), 1);
        let msg = &d.errors[0].message;
        assert!(msg.contains("undeclared alias"), "{msg}");
        assert!(msg.contains("@!missing"), "{msg}");
        assert!(msg.contains("root"), "{msg}"); // known alias is listed
    }

    #[test]
    fn at_bang_undeclared_with_no_aliases_says_none_declared() {
        let d = Directives::parse("Refers to @!ghost.\n");
        assert_eq!(d.errors.len(), 1);
        assert!(d.errors[0].message.contains("undeclared alias"));
        assert!(d.errors[0].message.contains("none declared"));
    }

    #[test]
    fn at_bang_in_fences_is_ignored() {
        let src = "\
#use spec://vibevm/a/b#c as root
```
@!root inside a fence
@spec://vibevm/fake#z
```
";
        let d = Directives::parse(src);
        assert_eq!(d.in_place_uses.len(), 0);
        assert_eq!(d.errors, vec![]);
    }

    #[test]
    fn at_bang_in_inline_backticks_mirrors_spec() {
        // R2: `@!` mirrors `@spec://` — inline backticks do not suppress
        // collection (only fenced blocks do, via the fence mask). The closing
        // backtick terminates the run, so both sigils are collected.
        let src = "\
#use spec://vibevm/a/b#c as root
see `@!root` in code, like `@spec://vibevm/a/b#c` is.
";
        let d = Directives::parse(src);
        assert_eq!(d.errors, vec![]);
        assert_eq!(d.in_place_uses.len(), 2);
        assert_eq!(d.in_place_uses[0].address.doc_path, "a/b");
        assert_eq!(d.in_place_uses[1].address.doc_path, "a/b");
    }

    #[test]
    fn at_bang_adjacent_punctuation_trims_to_identifier() {
        // R3: the identifier grammar naturally terminates the name; trailing
        // punctuation is prose, consistent with `address_run`'s trimming.
        let src = "\
#use spec://vibevm/a/b#c as root
Trailing dot @!root. and parenthesised (@!root) both resolve.
";
        let d = Directives::parse(src);
        assert_eq!(d.errors, vec![]);
        assert_eq!(d.in_place_uses.len(), 2);
        for u in &d.in_place_uses {
            assert_eq!(u.address.doc_path, "a/b");
            assert_eq!(u.address.anchor, vec!["c"]);
        }
    }

    #[test]
    fn multiple_at_bang_uses_on_one_line() {
        let src = "\
#use spec://vibevm/a#x as a
#use spec://vibevm/b#y as b
@a and @!a and @!b here
";
        let d = Directives::parse(src);
        // `@a` is neither `@spec://` nor `@!` — not collected. Two `@!` uses.
        assert_eq!(d.errors, vec![]);
        assert_eq!(d.in_place_uses.len(), 2);
    }
}
