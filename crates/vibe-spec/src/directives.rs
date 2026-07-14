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
}

impl Directives {
    /// Scan a document for directives and in-place uses. Infallible: malformed
    /// directives land in [`errors`](Self::errors), not a `Result`.
    pub fn parse(source: &str) -> Self {
        let lines: Vec<String> = source.lines().map(String::from).collect();
        let fenced = fence_mask(&lines);
        let mut out = Directives::default();

        for (i, line) in lines.iter().enumerate() {
            if fenced[i] {
                continue;
            }
            if let Some((kind, rest)) = directive_prefix(line.trim_start()) {
                out.push_directive(kind, rest, i);
            }
            out.scan_in_place(line, i);
        }
        out
    }

    fn push_directive(&mut self, kind: DirectiveKind, rest: &str, line: usize) {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        match tokens.iter().position(|t| t.starts_with("spec://")) {
            None => self.errors.push(DirectiveError {
                line,
                message: format!("{} directive has no spec:// address", kind.keyword()),
            }),
            Some(idx) => match SpecAddress::parse(tokens[idx]) {
                Ok(address) => self.directives.push(Directive {
                    kind,
                    options: tokens[..idx].join(" "),
                    address,
                    line,
                }),
                Err(e) => self.errors.push(DirectiveError {
                    line,
                    message: format!("{} has a bad address: {e}", kind.keyword()),
                }),
            },
        }
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

/// The address run of an `@spec` starting at `spec://`: everything up to
/// whitespace or a closing bracket/quote, with trailing sentence punctuation
/// trimmed (so `(@spec://a/b#c).` yields `spec://a/b#c`).
fn address_run(s: &str) -> &str {
    let end = s
        .find(|c: char| c.is_whitespace() || matches!(c, ')' | ']' | '>' | '"' | '\'' | '`' | '|'))
        .unwrap_or(s.len());
    s[..end].trim_end_matches(['.', ',', ';', ':', '!', '?'])
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
        let d = Directives::parse("#use spec://vibevm/a/b#Bad\n");
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
}
