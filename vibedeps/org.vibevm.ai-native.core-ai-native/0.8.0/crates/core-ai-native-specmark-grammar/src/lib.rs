//! The specmark tag grammar — PROP-014 §2.3, one place.
//!
//! Both consumers parse through this crate so the grammar cannot fork:
//!
//! - `specmark` (the proc-macro crate) parses attribute tokens at compile
//!   time and turns errors into `compile_error!` diagnostics;
//! - `specmap-core`'s scanner parses the same tokens out of `syn`-read
//!   source files when building `specmap.json`.
//!
//! Grammar (one edge per attribute; attributes repeat for multiple edges):
//!
//! ```text
//! #[spec( <verb> = "<spec-uri>" [, r = <N>] [, reason = "<text>"] )]
//! #[verifies("<spec-uri>" [, r = <N>])]            // sugar for tests
//! specmark::scope!("<spec-uri>" [, r = <N>]);      // module-level marker
//! ```
//!
//! Rules enforced here: the verb set is closed; the URI must be
//! `spec://<package>/<doc-path>#<anchor>[~r<N>]` whose anchor is an id
//! `[A-Za-z][A-Za-z0-9_-]*` — one document address space holds both
//! registers, so code cites a normative `##UPPER-SLUG` fact exactly as it
//! cites a kebab heading anchor (PROP-014 §2.1);
//! `r` is a positive integer; `reason` is mandatory for `deviates` and
//! rejected on every other verb; a revision pinned both in the URI (`~rN`)
//! and as `r = N` must agree.

use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitInt, LitStr, Token};

/// The closed verb set (PROP-014 §2.3).
///
/// ```
/// use core_ai_native_specmark_grammar::Verb;
/// assert_eq!(Verb::parse("implements"), Some(Verb::Implements));
/// assert_eq!(Verb::Implements.as_str(), "implements");
/// assert_eq!(Verb::parse("fulfills"), None); // the verb set is closed
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verb {
    Implements,
    Verifies,
    Documents,
    Deviates,
    Informs,
}

impl Verb {
    pub fn as_str(self) -> &'static str {
        match self {
            Verb::Implements => "implements",
            Verb::Verifies => "verifies",
            Verb::Documents => "documents",
            Verb::Deviates => "deviates",
            Verb::Informs => "informs",
        }
    }

    pub fn parse(s: &str) -> Option<Verb> {
        Some(match s {
            "implements" => Verb::Implements,
            "verifies" => Verb::Verifies,
            "documents" => Verb::Documents,
            "deviates" => Verb::Deviates,
            "informs" => Verb::Informs,
            _ => return None,
        })
    }
}

/// A parsed `spec://` URI (PROP-014 §2.1).
///
/// ```
/// use core_ai_native_specmark_grammar::parse_spec_uri;
/// let uri = parse_spec_uri("spec://org.vibevm.core/vibevm/common/PROP-000#commits~r2").unwrap();
/// assert_eq!(uri.package, "org.vibevm.core");
/// assert_eq!(uri.anchor, "commits");
/// assert_eq!(uri.pinned_r, Some(2));
/// // `without_pin` drops the `~rN` to recover the unit's canonical address.
/// assert_eq!(uri.without_pin(), "spec://org.vibevm.core/vibevm/common/PROP-000#commits");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecUri {
    /// The URI exactly as written, including any `~rN` pin.
    pub raw: String,
    pub package: String,
    pub doc_path: String,
    pub anchor: String,
    /// A `~rN` revision pin carried inside the URI itself.
    pub pinned_r: Option<u32>,
}

impl SpecUri {
    /// The URI without a revision pin — the unit's canonical address.
    pub fn without_pin(&self) -> String {
        format!("spec://{}/{}#{}", self.package, self.doc_path, self.anchor)
    }
}

/// Validate a **heading** anchor: the id grammar `[A-Za-z][A-Za-z0-9_-]*`.
///
/// A heading's `{#anchor}` and a paragraph's `##<ID>` fact both mint a name
/// into the document's one address space (PROP-014 §2.1), so one grammar
/// governs both: a heading anchor may be written every way a fact id may.
/// The kebab convention survives as a convention — `UPPER-SLUG` for a
/// normative fact, `kebab-case` for a service unit — and the register is what
/// tells a reader which grain is being cited, but it is no longer enforced
/// here.
///
/// This **calls** [`is_valid_fact_id`] rather than restating it. Two
/// predicates that agree today are two predicates that drift tomorrow.
///
/// The move is not a pure widening: kebab admitted a digit head and the id
/// grammar does not, so `9lives` goes accepted → rejected while `Some_Anchor`
/// and `a-` go the other way. Measured first — no heading anchor in the
/// scanned corpus is digit-headed.
///
/// ```
/// use core_ai_native_specmark_grammar::is_valid_anchor;
/// assert!(is_valid_anchor("req-conditional-fixpoint"));
/// assert!(is_valid_anchor("Mixed-Case")); // the id register, now legal
/// assert!(!is_valid_anchor("9lives")); // the head must be a letter
/// assert!(!is_valid_anchor("-leading")); // …and a dash is no letter
/// ```
pub fn is_valid_anchor(anchor: &str) -> bool {
    is_valid_fact_id(anchor)
}

/// Validate a fact-anchor id: `[A-Za-z][A-Za-z0-9_-]*` (PROP-014 §2.1).
///
/// A fact anchor — a `##<ID>` written as the first token of a paragraph or
/// list item — takes an ASCII-alphabetic first character, then ASCII
/// alphanumerics, `-`, or `_`. Both id registers validate here —
/// `UPPER-SLUG` names a normative fact, `kebab-case` a service one — the
/// register is convention, not enforced. This is the whole of the document's
/// name grammar: [`is_valid_anchor`] delegates here, so a heading anchor and
/// a fact id are one language.
///
/// ```
/// use core_ai_native_specmark_grammar::{is_valid_anchor, is_valid_fact_id};
/// assert!(is_valid_fact_id("FACT-A")); // normative UPPER-SLUG register
/// assert!(is_valid_fact_id("my-fact")); // service kebab-case register
/// assert!(is_valid_fact_id("R_040")); // underscores and digits allowed
/// assert!(!is_valid_fact_id("9lives")); // must start with a letter
/// assert!(!is_valid_fact_id("has space"));
/// assert!(!is_valid_fact_id(""));
/// // One grammar, two grains: a heading anchor is held to exactly this.
/// assert!(is_valid_anchor("FACT-A"));
/// ```
pub fn is_valid_fact_id(id: &str) -> bool {
    let mut chars = id.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Parse and validate a `spec://` URI string.
///
/// The anchor position takes the id grammar ([`is_valid_fact_id`]) — the one
/// law a heading anchor is held to as well: `##FACT-ID-GRAMMAR` states a fact
/// is addressable as `spec://…#<ID>`, so a normative `UPPER-SLUG` is citable
/// from code exactly as a heading anchor is.
///
/// ```
/// use core_ai_native_specmark_grammar::parse_spec_uri;
/// let uri = parse_spec_uri("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#mirror").unwrap();
/// assert_eq!(uri.doc_path, "vibevm/modules/vibe-registry/PROP-002");
/// assert_eq!(uri.anchor, "mirror");
/// // A normative fact is addressable in the same one space.
/// let f = parse_spec_uri("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#FACT-ID-GRAMMAR").unwrap();
/// assert_eq!(f.anchor, "FACT-ID-GRAMMAR");
/// // A missing `#anchor` fragment is rejected.
/// assert!(parse_spec_uri("spec://org.vibevm.core/vibevm/x").is_err());
/// ```
pub fn parse_spec_uri(raw: &str) -> Result<SpecUri, String> {
    let rest = raw
        .strip_prefix("spec://")
        .ok_or_else(|| format!("spec URI must start with `spec://`, got `{raw}`"))?;
    if rest.chars().any(char::is_whitespace) {
        return Err(format!("spec URI must not contain whitespace: `{raw}`"));
    }
    let (path, frag) = rest
        .split_once('#')
        .ok_or_else(|| format!("spec URI must carry a `#<anchor>` fragment: `{raw}`"))?;
    if frag.contains('#') {
        return Err(format!("spec URI has more than one `#`: `{raw}`"));
    }
    let (package, doc_path) = path
        .split_once('/')
        .ok_or_else(|| format!("spec URI path must be `<package>/<doc-path>`: `{raw}`"))?;
    if package.is_empty() || doc_path.is_empty() {
        return Err(format!(
            "spec URI package and doc-path must be non-empty: `{raw}`"
        ));
    }
    let (anchor, pinned_r) = match frag.split_once('~') {
        None => (frag, None),
        Some((anchor, pin)) => {
            let digits = pin
                .strip_prefix('r')
                .ok_or_else(|| format!("revision pin must be `~r<N>`, got `~{pin}` in `{raw}`"))?;
            let n: u32 = digits
                .parse()
                .map_err(|_| format!("revision pin must be an integer: `~r{digits}` in `{raw}`"))?;
            if n == 0 {
                return Err(format!(
                    "revisions start at r1; `~r0` is invalid in `{raw}`"
                ));
            }
            (anchor, Some(n))
        }
    };
    if !is_valid_fact_id(anchor) {
        return Err(format!(
            "anchor must be an id `[A-Za-z][A-Za-z0-9_-]*`, got `#{anchor}` in `{raw}`"
        ));
    }
    Ok(SpecUri {
        raw: raw.to_string(),
        package: package.to_string(),
        doc_path: doc_path.to_string(),
        anchor: anchor.to_string(),
        pinned_r,
    })
}

/// One validated edge declaration, whatever carrier syntax it arrived in.
///
/// ```
/// use core_ai_native_specmark_grammar::{EdgeSpec, Verb, parse_spec_uri};
/// let edge = EdgeSpec {
///     verb: Verb::Implements,
///     uri: parse_spec_uri("spec://org.vibevm.core/vibevm/common/PROP-000#commits").unwrap(),
///     r: None,
///     reason: None,
/// };
/// assert_eq!(edge.verb.as_str(), "implements");
/// ```
#[derive(Debug, Clone)]
pub struct EdgeSpec {
    pub verb: Verb,
    pub uri: SpecUri,
    /// The effective revision pin (`r = N` or the URI's `~rN`).
    pub r: Option<u32>,
    /// Present iff `verb == Deviates`.
    pub reason: Option<String>,
}

fn parse_r_value(input: ParseStream) -> syn::Result<u32> {
    let lit: LitInt = input.parse()?;
    let n: u32 = lit.base10_parse()?;
    if n == 0 {
        return Err(syn::Error::new(
            lit.span(),
            "revisions start at r1; `r = 0` is invalid",
        ));
    }
    Ok(n)
}

fn reconcile_pins(uri: &SpecUri, attr_r: Option<u32>, err_span: Span) -> syn::Result<Option<u32>> {
    match (uri.pinned_r, attr_r) {
        (Some(a), Some(b)) if a != b => Err(syn::Error::new(
            err_span,
            format!(
                "revision pinned twice and differing: URI says `~r{a}`, attribute says `r = {b}`"
            ),
        )),
        (a, b) => Ok(b.or(a)),
    }
}

/// Argument grammar of `#[spec(...)]` — parsed from the attribute's
/// tokens; the validated [`EdgeSpec`] is the payload.
///
/// ```
/// use core_ai_native_specmark_grammar::{SpecArgs, Verb};
/// let args: SpecArgs =
///     syn::parse_str(r#"implements = "spec://org.vibevm.core/vibevm/common/PROP-000#commits", r = 2"#).unwrap();
/// assert_eq!(args.edge.verb, Verb::Implements);
/// assert_eq!(args.edge.r, Some(2));
/// ```
#[derive(Debug)]
pub struct SpecArgs {
    pub edge: EdgeSpec,
}

impl Parse for SpecArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let verb_ident: Ident = input.parse().map_err(|e| {
            syn::Error::new(
                e.span(),
                "expected `#[spec(<verb> = \"<spec-uri>\", ...)]` — \
                 verbs: implements, verifies, documents, deviates, informs",
            )
        })?;
        let verb = Verb::parse(&verb_ident.to_string()).ok_or_else(|| {
            syn::Error::new(
                verb_ident.span(),
                format!(
                    "unknown specmark verb `{verb_ident}`; expected one of \
                     implements, verifies, documents, deviates, informs"
                ),
            )
        })?;
        input.parse::<Token![=]>()?;
        let uri_lit: LitStr = input.parse()?;
        let uri =
            parse_spec_uri(&uri_lit.value()).map_err(|e| syn::Error::new(uri_lit.span(), e))?;

        let mut r: Option<u32> = None;
        let mut reason: Option<String> = None;
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break; // tolerate a trailing comma
            }
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "r" => {
                    if r.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate `r` key"));
                    }
                    r = Some(parse_r_value(input)?);
                }
                "reason" => {
                    if reason.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate `reason` key"));
                    }
                    let lit: LitStr = input.parse()?;
                    let v = lit.value();
                    if v.trim().is_empty() {
                        return Err(syn::Error::new(lit.span(), "`reason` must not be empty"));
                    }
                    reason = Some(v);
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown specmark key `{other}`; expected `r` or `reason`"),
                    ));
                }
            }
        }
        if !input.is_empty() {
            return Err(input.error("unexpected trailing tokens in `#[spec(...)]`"));
        }

        match (verb, &reason) {
            (Verb::Deviates, None) => {
                return Err(syn::Error::new(
                    verb_ident.span(),
                    "`deviates` requires `reason = \"…\"` (PROP-014 §2.3)",
                ));
            }
            (v, Some(_)) if v != Verb::Deviates => {
                return Err(syn::Error::new(
                    verb_ident.span(),
                    "`reason` is only meaningful on `deviates`",
                ));
            }
            _ => {}
        }
        let r = reconcile_pins(&uri, r, uri_lit.span())?;
        Ok(SpecArgs {
            edge: EdgeSpec {
                verb,
                uri,
                r,
                reason,
            },
        })
    }
}

/// Argument grammar shared by `#[verifies("uri", r = N)]` and
/// `specmark::scope!("uri", r = N)`: a URI literal plus an optional pin.
///
/// ```
/// use core_ai_native_specmark_grammar::{UriArgs, Verb};
/// let args: UriArgs = syn::parse_str(r#""spec://org.vibevm.core/vibevm/common/PROP-000#commits""#).unwrap();
/// // A `scope!` marker defaults its module edge to `implements`.
/// assert_eq!(args.into_scope_edge().verb, Verb::Implements);
/// ```
#[derive(Debug)]
pub struct UriArgs {
    pub uri: SpecUri,
    pub r: Option<u32>,
}

impl Parse for UriArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let uri_lit: LitStr = input.parse()?;
        let uri =
            parse_spec_uri(&uri_lit.value()).map_err(|e| syn::Error::new(uri_lit.span(), e))?;
        let mut r: Option<u32> = None;
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key != "r" {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown key `{key}`; only `r` is accepted here"),
                ));
            }
            if r.is_some() {
                return Err(syn::Error::new(key.span(), "duplicate `r` key"));
            }
            r = Some(parse_r_value(input)?);
        }
        if !input.is_empty() {
            return Err(input.error("unexpected trailing tokens"));
        }
        let r = reconcile_pins(&uri, r, uri_lit.span())?;
        Ok(UriArgs { uri, r })
    }
}

impl UriArgs {
    /// A `#[verifies(...)]` edge.
    pub fn into_verifies_edge(self) -> EdgeSpec {
        EdgeSpec {
            verb: Verb::Verifies,
            uri: self.uri,
            r: self.r,
            reason: None,
        }
    }

    /// A `scope!(...)` marker: the module-level default edge is
    /// `implements` (PROP-014 §2.3, scope inheritance).
    pub fn into_scope_edge(self) -> EdgeSpec {
        EdgeSpec {
            verb: Verb::Implements,
            uri: self.uri,
            r: self.r,
            reason: None,
        }
    }
}

/// Argument grammar of `#[cell(...)]` — the cell manifest carried as a
/// structured attribute on the cell's root item (GUIDE-RUST §1; a
/// dedicated `cell.toml` is a later promotion). `seam` and `variant`
/// are mandatory; `replaces` and `flag` are optional. `replaces`
/// obliges a differential oracle against the named variant
/// (GUIDE-RUST §7, R-040).
///
/// ```
/// use core_ai_native_specmark_grammar::CellArgs;
/// let args: CellArgs =
///     syn::parse_str(r#"seam = "DepSolver", variant = "sat", replaces = "naive""#).unwrap();
/// assert_eq!(args.seam, "DepSolver");
/// assert_eq!(args.variant, "sat");
/// assert_eq!(args.replaces.as_deref(), Some("naive"));
/// ```
#[derive(Debug, Clone)]
pub struct CellArgs {
    pub seam: String,
    pub variant: String,
    pub replaces: Option<String>,
    pub flag: Option<String>,
}

impl Parse for CellArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut seam: Option<String> = None;
        let mut variant: Option<String> = None;
        let mut replaces: Option<String> = None;
        let mut flag: Option<String> = None;
        let mut first = true;
        while !input.is_empty() {
            if !first {
                input.parse::<Token![,]>()?;
                if input.is_empty() {
                    break; // tolerate a trailing comma
                }
            }
            first = false;
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let lit: LitStr = input.parse()?;
            let value = lit.value();
            if value.trim().is_empty() {
                return Err(syn::Error::new(lit.span(), "cell keys must not be empty"));
            }
            let slot = match key.to_string().as_str() {
                "seam" => &mut seam,
                "variant" => &mut variant,
                "replaces" => &mut replaces,
                "flag" => &mut flag,
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "unknown cell key `{other}`; expected seam, variant, replaces, flag"
                        ),
                    ));
                }
            };
            if slot.is_some() {
                return Err(syn::Error::new(
                    key.span(),
                    format!("duplicate `{key}` key"),
                ));
            }
            *slot = Some(value);
        }
        let seam = seam.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "`#[cell(...)]` requires `seam = \"…\"`")
        })?;
        let variant = variant.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "`#[cell(...)]` requires `variant = \"…\"`",
            )
        })?;
        Ok(CellArgs {
            seam,
            variant,
            replaces,
            flag,
        })
    }
}

#[cfg(test)]
#[path = "lib/tests.rs"]
mod tests;
