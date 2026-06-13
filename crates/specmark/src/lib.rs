//! `specmark` — inert traceability tags (PROP-014 §2.3).
//!
//! The attributes are no-ops for the compiler: they parse-validate the
//! grammar (URI shape, verb set, `r` integer, `reason` required for
//! `deviates`), inject a rendered `Spec:` line into rustdoc, and expand
//! to the item unchanged. Their real consumer is the source scanner
//! behind `cargo xtask specmap`, which reads the attributes as AST —
//! no macro expansion involved.
//!
//! ```rust,ignore
//! use specmark::spec;
//!
//! #[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-fixpoint", r = 2)]
//! pub enum ConditionalPredicate { /* … */ }
//! ```
//!
//! Grammar errors surface as compile errors at the attribute span; the
//! grammar itself lives in `specmark-grammar`, shared verbatim with the
//! scanner so the two can never drift.

use proc_macro::TokenStream;
use quote::quote;
use specmark_grammar::{CellArgs, EdgeSpec, SpecArgs, UriArgs};

/// Render the rustdoc line a tag injects.
fn doc_line(edge: &EdgeSpec) -> String {
    let mut line = format!("Spec: {} {}", edge.verb.as_str(), edge.uri.without_pin());
    if let Some(r) = edge.r {
        line.push_str(&format!(" (r{r})"));
    }
    if let Some(reason) = &edge.reason {
        line.push_str(&format!(" — deviation: {reason}"));
    }
    line
}

/// Prepend the rendered `Spec:` doc line to the item, otherwise emit it
/// unchanged.
fn emit_with_doc(edge: &EdgeSpec, item: TokenStream) -> TokenStream {
    let doc = doc_line(edge);
    let item2: proc_macro2::TokenStream = item.into();
    quote! {
        #[doc = #doc]
        #item2
    }
    .into()
}

/// On a grammar error: emit the compile error *and* the original item, so
/// downstream code still sees the item and the user gets one clear
/// diagnostic instead of an error cascade.
fn emit_error(err: syn::Error, item: TokenStream) -> TokenStream {
    let compile_err = err.to_compile_error();
    let item2: proc_macro2::TokenStream = item.into();
    quote! {
        #compile_err
        #item2
    }
    .into()
}

/// `#[spec(<verb> = "<spec-uri>" [, r = N] [, reason = "…"])]`
///
/// One edge per attribute; repeat the attribute for multiple edges.
/// `deviates` requires `reason`; `reason` is rejected on other verbs.
/// The tag is inert — it injects a `Spec:` rustdoc line and expands to
/// the item unchanged.
///
/// ```
/// #[specmark::spec(
///     implements = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-fixpoint",
///     r = 2
/// )]
/// pub struct ConditionalPredicate;
/// ```
#[proc_macro_attribute]
pub fn spec(attr: TokenStream, item: TokenStream) -> TokenStream {
    match syn::parse::<SpecArgs>(attr) {
        Ok(args) => emit_with_doc(&args.edge, item),
        Err(err) => emit_error(err, item),
    }
}

/// `#[verifies("<spec-uri>" [, r = N])]` — sugar for tests; the edge
/// verb is `verifies`.
///
/// ```
/// #[specmark::verifies("spec://vibevm/common/PROP-000#token-secrecy")]
/// fn redaction_is_total() {}
/// ```
#[proc_macro_attribute]
pub fn verifies(attr: TokenStream, item: TokenStream) -> TokenStream {
    match syn::parse::<UriArgs>(attr) {
        Ok(args) => emit_with_doc(&args.into_verifies_edge(), item),
        Err(err) => emit_error(err, item),
    }
}

/// `specmark::scope!("<spec-uri>" [, r = N]);` — module-level inheritance
/// marker: every item in the module gets a default `implements` edge
/// unless it carries its own `#[spec]` (own tags replace the inherited
/// set, PROP-014 §2.3). Expands to nothing; the scanner reads it from
/// source.
///
/// ```
/// specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#mirror");
/// ```
#[proc_macro]
pub fn scope(input: TokenStream) -> TokenStream {
    match syn::parse::<UriArgs>(input) {
        Ok(_) => TokenStream::new(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// `#[cell(seam = "…", variant = "…" [, replaces = "…"] [, flag = "…"])]`
/// — the cell manifest (GUIDE-RUST §1): one module behind one seam,
/// registered in one place, selected by at most one flag. Inert like
/// `#[spec]`; consumers are `cargo xtask conform-lite`'s structure
/// checks. A `replaces` key obliges a differential oracle against the
/// named variant (GUIDE-RUST §7, R-040).
///
/// ```
/// #[specmark::cell(seam = "RepoCreator", variant = "github")]
/// pub struct GitHubCreator;
/// ```
#[proc_macro_attribute]
pub fn cell(attr: TokenStream, item: TokenStream) -> TokenStream {
    match syn::parse::<CellArgs>(attr) {
        Ok(args) => {
            let mut doc = format!("Cell: seam `{}`, variant `{}`", args.seam, args.variant);
            if let Some(replaces) = &args.replaces {
                doc.push_str(&format!(", replaces `{replaces}`"));
            }
            if let Some(flag) = &args.flag {
                doc.push_str(&format!(", flag `{flag}`"));
            }
            let item2: proc_macro2::TokenStream = item.into();
            quote! {
                #[doc = #doc]
                #item2
            }
            .into()
        }
        Err(err) => emit_error(err, item),
    }
}
