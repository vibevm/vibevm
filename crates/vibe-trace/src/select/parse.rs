//! The grammar half of the query language (E-A5B-QUERYLANG): the parser,
//! the parsed-query value, and its parse errors. Split out of [`super`]
//! along the responsibility seam "parse the string" vs "walk the graph" so
//! each half stays under the per-file budget — the same reason `search/`
//! and `fragment/` keep their tests beside the implementation.
//!
//! The grammar (version 1) is a whitespace-separated conjunction of
//! `name:value` predicates — no `OR`, no parentheses, no precedence. Every
//! parse error names the offending token and lists the expected; there is no
//! silent fall-through to "everything" (§2.2 points 9–10).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#map-select");

use std::collections::HashSet;
use std::fmt;

use serde::Serialize;
use specmap_core::generated::specmap::EdgeVerb;

/// The largest depth the walk may go. `depth:4` is a parse error (§2.2
/// point 4) — three hops already reaches across the whole bipartite graph
/// from any seed in practice, and an unbounded walk has no honest ceiling.
pub(super) const MAX_DEPTH: u32 = 3;

/// The predicate names the grammar recognises, in the order they appear in
/// the header and in "expected one of …" error messages.
const PREDICATES: &[&str] = &[
    "uri:", "symbol:", "kind:", "scope:", "has:", "lacks:", "depth:",
];

/// The five traceability verbs an `Edge` may carry, as the grammar spells them.
const VERBS: &[&str] = &["implements", "verifies", "documents", "deviates", "informs"];

/// A traceability verb, as the grammar spells it. The generated [`EdgeVerb`]
/// derives only `Serialize`/`Deserialize`, so this local enum carries the
/// `Debug`/`Clone`/`Copy`/`PartialEq` a parsed query needs — it is the
/// grammar's own closed vocabulary, converted from an [`EdgeVerb`] at the
/// graph boundary ([`Verb::from_edge`]) and never leaked back out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Verb {
    #[serde(rename = "implements")]
    Implements,
    #[serde(rename = "verifies")]
    Verifies,
    #[serde(rename = "documents")]
    Documents,
    #[serde(rename = "deviates")]
    Deviates,
    #[serde(rename = "informs")]
    Informs,
}

impl Verb {
    /// Parse a grammar spelling, or `None` if it is not one of the five verbs.
    fn parse(s: &str) -> Option<Verb> {
        match s {
            "implements" => Some(Verb::Implements),
            "verifies" => Some(Verb::Verifies),
            "documents" => Some(Verb::Documents),
            "deviates" => Some(Verb::Deviates),
            "informs" => Some(Verb::Informs),
            _ => None,
        }
    }

    /// The grammar spelling (mirrors the `EdgeVerb` JTD renames).
    fn as_str(self) -> &'static str {
        match self {
            Verb::Implements => "implements",
            Verb::Verifies => "verifies",
            Verb::Documents => "documents",
            Verb::Deviates => "deviates",
            Verb::Informs => "informs",
        }
    }

    /// The verb an [`Edge`](specmap_core::generated::specmap::Edge) carries,
    /// lifted into this enum for comparison. Takes the edge verb by reference
    /// — the generated [`EdgeVerb`] is neither `Copy` nor `Clone`, so it
    /// cannot be moved out of a borrowed edge.
    pub(super) fn from_edge(ev: &EdgeVerb) -> Verb {
        match ev {
            EdgeVerb::Implements => Verb::Implements,
            EdgeVerb::Verifies => Verb::Verifies,
            EdgeVerb::Documents => Verb::Documents,
            EdgeVerb::Deviates => Verb::Deviates,
            EdgeVerb::Informs => Verb::Informs,
        }
    }
}

/// A parsed `select` query: the conjunction of predicates a caller handed in
/// (УТОЧНИ-3). Mirrors the floor's
/// [`Filters`](crate::search::Filters) shape for `uri`/`symbol`/`kind` (so a
/// reader sees the same fields mean the same thing) and adds the four
/// predicates the floor has no analogue for. Printable via
/// [`header`](Self::header), which renders the parsed state — not the raw
/// string — into the text answer.
#[derive(Debug, Clone, Serialize)]
pub struct ParsedQuery {
    /// Exact `spec://…#anchor` URI (spec units only). The floor's `uri`.
    pub uri: Option<String>,
    /// Substring of a code symbol (code items only). The floor's `symbol`.
    pub symbol: Option<String>,
    /// Unified node kind. The floor's `kind`.
    pub kind: Option<String>,
    /// Prefix of a spec unit's `uri` (spec units only).
    pub scope: Option<String>,
    /// Keep only seeds an edge of this verb touches.
    pub has: Option<Verb>,
    /// Keep only seeds no edge of this verb touches.
    pub lacks: Option<Verb>,
    /// Undirected walk depth (0..=3). Defaults to 0 — no walk, seeds only.
    pub depth: u32,
}

impl ParsedQuery {
    /// Render the parsed predicates as the text header does (§2.3): the set
    /// predicates joined by ` AND `, `depth` always shown. Mirrors the floor's
    /// `uri=… symbol~"…" kind=…` conventions, extending them with
    /// `scope~"…" has=<verb> lacks=<verb>`.
    pub fn header(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(uri) = &self.uri {
            parts.push(format!("uri={uri}"));
        }
        if let Some(sym) = &self.symbol {
            parts.push(format!("symbol~\"{sym}\""));
        }
        if let Some(k) = &self.kind {
            parts.push(format!("kind={k}"));
        }
        if let Some(s) = &self.scope {
            parts.push(format!("scope~\"{s}\""));
        }
        if let Some(v) = self.has {
            parts.push(format!("has={}", v.as_str()));
        }
        if let Some(v) = self.lacks {
            parts.push(format!("lacks={}", v.as_str()));
        }
        parts.push(format!("depth={}", self.depth));
        parts.join(" AND ")
    }
}

/// A failed parse. Every message names the offending token and lists what was
/// expected (§2.2 point 10) — there is no silent fall-through to "everything".
/// The tone mirrors `vibe query --limit 0`: backtick the token, state the rule,
/// give the bound (УТОЧНИ-4).
#[derive(Debug, Clone)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    fn new(message: String) -> Self {
        ParseError { message }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse a `select` query string into a [`ParsedQuery`].
///
/// The grammar is a whitespace-separated conjunction of `name:value`
/// predicates (§2.1). An empty query, an unknown predicate, an unknown verb,
/// a non-numeric or out-of-range depth, a predicate with no value, and a
/// repeated predicate are all errors that name the token and list the
/// expected — never a silent "match everything" (§2.2 points 9–10).
///
/// ```
/// use vibe_trace::select::parse;
///
/// let q = parse("lacks:verifies scope:spec://demo/D depth:2").unwrap();
/// assert_eq!(q.scope.as_deref(), Some("spec://demo/D"));
/// assert!(q.lacks.is_some());
/// assert_eq!(q.depth, 2);
/// // The four error classes — each names the token and lists the expected.
/// parse("").expect_err("an empty query is an error, not \"everything\"");
/// parse("bogus:x").expect_err("unknown predicate");
/// parse("has:maybe").expect_err("unknown verb");
/// parse("depth:4").expect_err("depth out of range");
/// parse("uri:a uri:b").expect_err("a repeated predicate");
/// ```
pub fn parse(query: &str) -> Result<ParsedQuery, ParseError> {
    let mut p = ParsedQuery {
        uri: None,
        symbol: None,
        kind: None,
        scope: None,
        has: None,
        lacks: None,
        depth: 0,
    };
    let mut seen: HashSet<&'static str> = HashSet::new();
    for tok in query.split_ascii_whitespace() {
        // `name:value` — split on the FIRST colon so a value may itself hold
        // colons (a `spec://…` URI, a `scope:spec://…` prefix).
        let (name, value) = tok.split_once(':').ok_or_else(|| unknown_predicate(tok))?;
        match name {
            "uri" => {
                record(&mut seen, "uri")?;
                p.uri = Some(take_value("uri", value)?.to_owned());
            }
            "symbol" => {
                record(&mut seen, "symbol")?;
                p.symbol = Some(take_value("symbol", value)?.to_owned());
            }
            "kind" => {
                record(&mut seen, "kind")?;
                p.kind = Some(take_value("kind", value)?.to_owned());
            }
            "scope" => {
                record(&mut seen, "scope")?;
                p.scope = Some(take_value("scope", value)?.to_owned());
            }
            "has" => {
                record(&mut seen, "has")?;
                let v = take_value("has", value)?;
                p.has = Some(Verb::parse(v).ok_or_else(|| unknown_verb("has", v))?);
            }
            "lacks" => {
                record(&mut seen, "lacks")?;
                let v = take_value("lacks", value)?;
                p.lacks = Some(Verb::parse(v).ok_or_else(|| unknown_verb("lacks", v))?);
            }
            "depth" => {
                record(&mut seen, "depth")?;
                let v = take_value("depth", value)?;
                let n: u32 = v.parse().map_err(|_| bad_depth(v))?;
                if n > MAX_DEPTH {
                    return Err(depth_out_of_range(n));
                }
                p.depth = n;
            }
            _ => return Err(unknown_predicate(tok)),
        }
    }
    if seen.is_empty() {
        return Err(empty_query());
    }
    Ok(p)
}

/// Note that `name` appeared; error if it already did (§2.2 point 5 — a
/// repeated predicate has no value under AND).
fn record(seen: &mut HashSet<&'static str>, name: &'static str) -> Result<(), ParseError> {
    if !seen.insert(name) {
        return Err(repeated_predicate(name));
    }
    Ok(())
}

/// Lift a non-empty value for `pred`, or the "needs a value" error. The
/// returned slice borrows from `value` (the `Err` owns its message).
fn take_value<'a>(pred: &'static str, value: &'a str) -> Result<&'a str, ParseError> {
    if value.is_empty() {
        Err(missing_value(pred))
    } else {
        Ok(value)
    }
}

/// Join `items` as a backtick-wrapped, comma-separated list for messages.
fn listed(items: &[&str]) -> String {
    items
        .iter()
        .map(|i| format!("`{i}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

// --- error constructors (each names the token + lists the expected) --------

fn unknown_predicate(tok: &str) -> ParseError {
    ParseError::new(format!(
        "unknown predicate `{tok}` in the `select` query; expected one of {}.",
        listed(PREDICATES)
    ))
}

fn unknown_verb(pred: &str, v: &str) -> ParseError {
    ParseError::new(format!(
        "unknown verb `{v}` for `{pred}:` in the `select` query; expected one of {}.",
        listed(VERBS)
    ))
}

fn missing_value(pred: &str) -> ParseError {
    match pred {
        "has" | "lacks" => ParseError::new(format!(
            "predicate `{pred}:` needs a verb in the `select` query; expected one of {}.",
            listed(VERBS)
        )),
        "depth" => ParseError::new(
            "predicate `depth:` needs a value in the `select` query; expected an integer 0..=3."
                .into(),
        ),
        _ => ParseError::new(format!(
            "predicate `{pred}:` needs a value in the `select` query."
        )),
    }
}

fn bad_depth(v: &str) -> ParseError {
    ParseError::new(format!(
        "`{v}` is not a valid `depth:` value in the `select` query; expected an integer 0..=3."
    ))
}

fn depth_out_of_range(n: u32) -> ParseError {
    ParseError::new(format!(
        "`depth:{n}` is out of range in the `select` query; expected 0..={MAX_DEPTH}."
    ))
}

fn repeated_predicate(name: &str) -> ParseError {
    ParseError::new(format!(
        "predicate `{name}:` appears more than once in the `select` query; a conjunction is \
         AND-joined, so repeating a predicate (e.g. `uri:a uri:b`) has no value — state it once."
    ))
}

fn empty_query() -> ParseError {
    ParseError::new(format!(
        "the `select` query is empty; an empty query is an error, not \"everything\". For an \
         unfiltered slice of the whole map use `vibe query`. Predicates: {}.",
        listed(PREDICATES)
    ))
}
