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
//! `spec://<package>/<doc-path>#<anchor>[~r<N>]` with a kebab-case anchor;
//! `r` is a positive integer; `reason` is mandatory for `deviates` and
//! rejected on every other verb; a revision pinned both in the URI (`~rN`)
//! and as `r = N` must agree.

use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitInt, LitStr, Token};

/// The closed verb set (PROP-014 §2.3).
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

/// Validate a kebab-case anchor: `[a-z0-9]+(-[a-z0-9]+)*`.
pub fn is_valid_anchor(anchor: &str) -> bool {
    if anchor.is_empty() {
        return false;
    }
    anchor.split('-').all(|seg| {
        !seg.is_empty()
            && seg
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    })
}

/// Parse and validate a `spec://` URI string.
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
    if !is_valid_anchor(anchor) {
        return Err(format!(
            "anchor must be kebab-case `[a-z0-9]+(-[a-z0-9]+)*`, got `#{anchor}` in `{raw}`"
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

/// Argument grammar of `#[spec(...)]`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    const URI: &str = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-fixpoint";

    #[test]
    fn uri_parses_with_all_parts() {
        let u = parse_spec_uri(URI).unwrap();
        assert_eq!(u.package, "vibevm");
        assert_eq!(u.doc_path, "modules/vibe-resolver/PROP-003");
        assert_eq!(u.anchor, "req-conditional-fixpoint");
        assert_eq!(u.pinned_r, None);
        assert_eq!(u.without_pin(), URI);
    }

    #[test]
    fn uri_parses_revision_pin() {
        let u = parse_spec_uri(&format!("{URI}~r2")).unwrap();
        assert_eq!(u.pinned_r, Some(2));
        assert_eq!(u.without_pin(), URI);
    }

    #[test]
    fn uri_rejections() {
        for bad in [
            "http://x/y#a",         // wrong scheme
            "spec://vibevm#a",      // no doc-path
            "spec://vibevm/x",      // no fragment
            "spec://vibevm/x#A-b",  // uppercase anchor
            "spec://vibevm/x#a b",  // whitespace
            "spec://vibevm/x#a~rx", // non-integer pin
            "spec://vibevm/x#a~r0", // r0
            "spec://vibevm/x#a#b",  // two fragments
            "spec://vibevm/x#-a",   // leading dash
            "spec://vibevm/x#a-",   // trailing dash
        ] {
            assert!(parse_spec_uri(bad).is_err(), "should reject `{bad}`");
        }
    }

    #[test]
    fn spec_args_happy_path() {
        let args: SpecArgs = syn::parse2(quote! { implements = #URI, r = 2 }).unwrap();
        assert_eq!(args.edge.verb, Verb::Implements);
        assert_eq!(args.edge.r, Some(2));
        assert_eq!(args.edge.reason, None);
    }

    #[test]
    fn spec_args_deviates_requires_reason() {
        let err = syn::parse2::<SpecArgs>(quote! { deviates = #URI, r = 1 }).unwrap_err();
        assert!(err.to_string().contains("requires `reason"), "{err}");
        let ok: SpecArgs = syn::parse2(
            quote! { deviates = #URI, r = 1, reason = "boolean composition unimplemented" },
        )
        .unwrap();
        assert_eq!(
            ok.edge.reason.as_deref(),
            Some("boolean composition unimplemented")
        );
    }

    #[test]
    fn spec_args_reason_rejected_on_other_verbs() {
        let err =
            syn::parse2::<SpecArgs>(quote! { implements = #URI, reason = "nope" }).unwrap_err();
        assert!(
            err.to_string().contains("only meaningful on `deviates`"),
            "{err}"
        );
    }

    #[test]
    fn spec_args_unknown_verb_and_key() {
        let err = syn::parse2::<SpecArgs>(quote! { fulfills = #URI }).unwrap_err();
        assert!(err.to_string().contains("unknown specmark verb"), "{err}");
        let err = syn::parse2::<SpecArgs>(quote! { implements = #URI, rev = 2 }).unwrap_err();
        assert!(err.to_string().contains("unknown specmark key"), "{err}");
    }

    #[test]
    fn spec_args_pin_conflict_and_agreement() {
        let pinned = format!("{URI}~r3");
        let err = syn::parse2::<SpecArgs>(quote! { implements = #pinned, r = 2 }).unwrap_err();
        assert!(err.to_string().contains("pinned twice"), "{err}");
        let ok: SpecArgs = syn::parse2(quote! { implements = #pinned, r = 3 }).unwrap();
        assert_eq!(ok.edge.r, Some(3));
        let ok: SpecArgs = syn::parse2(quote! { implements = #pinned }).unwrap();
        assert_eq!(ok.edge.r, Some(3));
    }

    #[test]
    fn uri_args_for_verifies_and_scope() {
        let v: UriArgs = syn::parse2(quote! { #URI, r = 2 }).unwrap();
        let e = v.into_verifies_edge();
        assert_eq!(e.verb, Verb::Verifies);
        assert_eq!(e.r, Some(2));

        let s: UriArgs = syn::parse2(quote! { #URI }).unwrap();
        let e = s.into_scope_edge();
        assert_eq!(e.verb, Verb::Implements);
        assert_eq!(e.r, None);
    }

    #[test]
    fn spec_args_rejects_zero_revision_and_empty_reason() {
        let err = syn::parse2::<SpecArgs>(quote! { implements = #URI, r = 0 }).unwrap_err();
        assert!(err.to_string().contains("start at r1"), "{err}");
        let err = syn::parse2::<SpecArgs>(quote! { deviates = #URI, reason = "  " }).unwrap_err();
        assert!(err.to_string().contains("must not be empty"), "{err}");
    }
}
