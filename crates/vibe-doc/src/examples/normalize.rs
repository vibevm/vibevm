//! The normalisation pipeline (PROP-057 `##PIPE-EXAMPLE-RUNNER`).
//!
//! Both sides of a comparison — the captured stream and the golden on the
//! page — go through the SAME rules, in a fixed order, and what comes out
//! is compared byte for byte. The order is load-bearing: the path
//! replacements run before slashes are unified (a replacement that ran
//! second would never find the native spelling), and the block sort runs
//! after every textual replacement (sorting first would order lines that
//! are about to change).
//!
//! Every rule is a named class declared in the fixture. There is no
//! «fuzzy» comparison and no match template: a line the product does not
//! promise to keep stable is closed by a rule a reviewer can read, or the
//! product is fixed. That is the whole difference between normalising a
//! difference and hiding one.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER");

use std::path::Path;

use regex::Regex;

use crate::error::{DocError, Result};
use crate::examples::fixture::NormalizeDecl;

/// The three paths whose real spellings must never reach a page, with the
/// placeholder each becomes. Order matters and is the caller's: the most
/// specific path is replaced first, because the sandbox lives inside the
/// user's profile and the repository may too.
#[derive(Debug, Clone, Default)]
pub struct Placeholders {
    /// The sandbox root → `<TMP>`.
    pub sandbox: Option<String>,
    /// The source tree → `<REPO>`.
    pub repo: Option<String>,
    /// The real user home → `<HOME>`.
    pub home: Option<String>,
}

impl Placeholders {
    fn pairs(&self) -> Vec<(&str, &'static str)> {
        let mut out: Vec<(&str, &'static str)> = Vec::new();
        if let Some(p) = &self.sandbox {
            out.push((p.as_str(), "<TMP>"));
        }
        if let Some(p) = &self.repo {
            out.push((p.as_str(), "<REPO>"));
        }
        if let Some(p) = &self.home {
            out.push((p.as_str(), "<HOME>"));
        }
        out
    }
}

/// A fixture's rules, compiled once per run.
#[derive(Debug)]
pub struct Normalizer {
    decl: NormalizeDecl,
    replace: Vec<(Regex, String)>,
    sort_blocks: Vec<Regex>,
    version: Regex,
    ansi: Regex,
    places: Placeholders,
}

impl Normalizer {
    /// Compile the fixture's declaration. A pattern that is not a usable
    /// line form is refused here, with the fixture that wrote it — never
    /// at the moment a stream happens to reach it.
    pub fn compile(
        fixture: &str,
        decl: &NormalizeDecl,
        places: Placeholders,
    ) -> Result<Normalizer> {
        let bad = |pattern: &str, e: regex::Error| DocError::Pattern {
            fixture: fixture.to_owned(),
            pattern: pattern.to_owned(),
            message: e.to_string(),
        };
        let mut replace = Vec::new();
        for r in &decl.replace {
            let re = Regex::new(&format!("(?m){}", r.pattern)).map_err(|e| bad(&r.pattern, e))?;
            replace.push((re, r.with.clone()));
        }
        let mut sort_blocks = Vec::new();
        for p in &decl.sort_blocks {
            sort_blocks.push(Regex::new(p).map_err(|e| bad(p, e))?);
        }
        // `vibe 1.2.3` and `vibe.exe 1.2.3` — the PRODUCT's version, tied
        // to the word `vibe`. A package version (`wal@1.0.0`) is content
        // and is deliberately out of reach of this rule.
        let version = Regex::new(r"(?m)\bvibe(\.exe)? \d+\.\d+\.\d+(-[0-9A-Za-z.\-]+)?")
            .map_err(|e| bad("<product version>", e))?;
        let ansi = Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(\x07|\x1b\\)")
            .map_err(|e| bad("<ansi>", e))?;
        Ok(Normalizer {
            decl: decl.clone(),
            replace,
            sort_blocks,
            version,
            ansi,
            places,
        })
    }

    /// Run one stream through every enabled rule, in order.
    pub fn apply(&self, text: &str) -> String {
        let mut out = text.to_owned();
        if self.decl.crlf {
            out = out.replace("\r\n", "\n");
        }
        if self.decl.ansi {
            out = self.ansi.replace_all(&out, "").into_owned();
        }
        if self.decl.paths {
            // The Windows verbatim prefix first: `\\?\C:\…` and `C:\…`
            // are the same directory, and which one a command prints is
            // an accident of how it canonicalised the path, not content.
            out = out.replace("\\\\?\\", "").replace("//?/", "");
            out = replace_paths(&out, &self.places);
        }
        if self.decl.slashes {
            // The doubled form first: a JSON string spells one separator
            // as two characters, and turning each of them into a slash
            // would double every separator in a `--json` example.
            out = out.replace("\\\\", "/").replace('\\', "/");
        }
        if self.decl.exe_name {
            out = out.replace("vibe.exe", "vibe");
        }
        if self.decl.product_version {
            out = self
                .version
                .replace_all(&out, "vibe <VERSION>")
                .into_owned();
        }
        for (re, with) in &self.replace {
            out = re.replace_all(&out, with.as_str()).into_owned();
        }
        for re in &self.sort_blocks {
            out = sort_runs(&out, re);
        }
        if self.decl.trailing_space {
            out = trim_trailing(&out);
        }
        out
    }
}

/// Replace each real path with its placeholder, in the three spellings a
/// path reaches a stream in: native (`C:\a\b`), forward (`C:/a/b`) and
/// JSON-escaped (`C:\\a\\b`). The JSON spelling is handled by matching it
/// directly rather than by collapsing every doubled backslash in the
/// stream, which would corrupt escapes that have nothing to do with paths.
///
/// Matching is case-insensitive because Windows hands the same directory
/// back in different cases from different APIs.
fn replace_paths(text: &str, places: &Placeholders) -> String {
    let mut out = text.to_owned();
    for (raw, placeholder) in places.pairs() {
        out = replace_path_spellings(&out, raw, placeholder);
    }
    out
}

/// Replace one path with `with`, in each of the three spellings a path
/// reaches a text in. Public because a copied sandbox must be retargeted
/// by exactly the same rule that normalises a stream — one algorithm, so
/// the two can never disagree about what «the same path» means.
///
/// ```
/// let out = vibe_doc::examples::normalize::replace_path_spellings(
///     r#"{"a":"C:\\t\\x","b":"C:/t/x","c":"C:\t\x"}"#,
///     r"C:\t",
///     "<TMP>",
/// );
/// assert_eq!(out, r#"{"a":"<TMP>\\x","b":"<TMP>/x","c":"<TMP>\x"}"#);
/// ```
pub fn replace_path_spellings(text: &str, path: &str, with: &str) -> String {
    let native = |p: &str| p.replace('/', "\\");
    let forward = |p: &str| p.replace('\\', "/");
    let escaped = |p: &str| native(p).replace('\\', "\\\\");
    // The JSON spelling first, matched literally: it is the only one
    // whose separator is two characters, so folding cannot see it, and
    // replacing the shorter native form first would leave a stray
    // backslash behind.
    let out = replace_ignore_case(text, &escaped(path), &escaped(with));
    // Then the native and forward families TOGETHER, because a real
    // stream mixes them: a settings directory handed in with forward
    // slashes comes back with a native tail (`C:/a/b\c`), and a needle
    // spelled either way alone would match neither. Each occurrence is
    // replaced in the spelling it was found in, so a retargeted file
    // keeps the separator style its reader expects.
    replace_folding_separators(&out, &native(path), &native(with), &forward(with))
}

/// `str::replace`, case-insensitively, without a regular expression (the
/// needle is a path, not a pattern, and a path is full of metacharacters).
fn replace_ignore_case(haystack: &str, needle: &str, with: &str) -> String {
    replace_by(haystack, needle, |_| with.to_owned(), false)
}

/// The same, with `\` and `/` treated as one character, and the
/// replacement chosen by the separator the match actually used.
fn replace_folding_separators(
    haystack: &str,
    needle: &str,
    native_with: &str,
    forward_with: &str,
) -> String {
    replace_by(
        haystack,
        needle,
        |matched: &str| {
            if matched.contains('/') {
                forward_with.to_owned()
            } else {
                native_with.to_owned()
            }
        },
        true,
    )
}

/// The one scanner both replacements run on: fold the haystack and the
/// needle into a comparison key of the same byte length, find every
/// occurrence, and let the caller choose the replacement from the text
/// that actually matched.
fn replace_by(
    haystack: &str,
    needle: &str,
    with: impl Fn(&str) -> String,
    fold_separators: bool,
) -> String {
    if needle.is_empty() {
        return haystack.to_owned();
    }
    let fold = |s: &str| {
        let lowered = s.to_lowercase();
        if fold_separators {
            lowered.replace('\\', "/")
        } else {
            lowered
        }
    };
    let key_hay = fold(haystack);
    let key_needle = fold(needle);
    // Case folding can change byte length; fall back to the exact form
    // when it does, so an index from the folded string never slices the
    // original at a wrong boundary.
    if key_hay.len() != haystack.len() || key_needle.len() != needle.len() {
        return haystack.replace(needle, &with(needle));
    }
    let mut out = String::with_capacity(haystack.len());
    let mut cursor = 0usize;
    while let Some(found) = key_hay[cursor..].find(&key_needle) {
        let start = cursor + found;
        let end = start + needle.len();
        out.push_str(&haystack[cursor..start]);
        out.push_str(&with(&haystack[start..end]));
        cursor = end;
    }
    out.push_str(&haystack[cursor..]);
    out
}

/// Sort the maximal consecutive runs of lines that match `form`, leaving
/// every other line exactly where it was.
fn sort_runs(text: &str, form: &Regex) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut run: Vec<&str> = Vec::new();
    for line in text.split('\n') {
        if form.is_match(line) {
            run.push(line);
            continue;
        }
        if !run.is_empty() {
            run.sort_unstable();
            out.extend(run.iter().map(|s| (*s).to_owned()));
            run.clear();
        }
        out.push(line.to_owned());
    }
    if !run.is_empty() {
        run.sort_unstable();
        out.extend(run.iter().map(|s| (*s).to_owned()));
    }
    out.join("\n")
}

/// Drop trailing whitespace on every line, then the trailing blank lines
/// of the whole stream. A golden on a page cannot carry invisible
/// trailing spaces, so neither may the capture it is compared to.
fn trim_trailing(text: &str) -> String {
    let mut lines: Vec<&str> = text.split('\n').map(|l| l.trim_end()).collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

/// The one place that turns a path into the string the replacements match
/// against: absolute, with the Windows verbatim prefix stripped, because
/// no product output ever spells `\\?\`.
pub fn display_path(path: &Path) -> String {
    let text = path.display().to_string();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
}

#[cfg(test)]
mod tests;
