//! Grammar unit tests, out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path] mod tests;`, so `use super::*` is
//! unchanged from the inline form.

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
    let err = syn::parse2::<SpecArgs>(quote! { implements = #URI, reason = "nope" }).unwrap_err();
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
fn cell_args_happy_path_and_rejections() {
    let ok: CellArgs = syn::parse2(
        quote! { seam = "DepSolver", variant = "sat", replaces = "naive", flag = "solver" },
    )
    .unwrap();
    assert_eq!(ok.seam, "DepSolver");
    assert_eq!(ok.variant, "sat");
    assert_eq!(ok.replaces.as_deref(), Some("naive"));
    assert_eq!(ok.flag.as_deref(), Some("solver"));

    let minimal: CellArgs =
        syn::parse2(quote! { seam = "DepProvider", variant = "local" }).unwrap();
    assert_eq!(minimal.replaces, None);
    assert_eq!(minimal.flag, None);

    let err = syn::parse2::<CellArgs>(quote! { variant = "sat" }).unwrap_err();
    assert!(err.to_string().contains("requires `seam"), "{err}");
    let err =
        syn::parse2::<CellArgs>(quote! { seam = "X", variant = "y", colour = "red" }).unwrap_err();
    assert!(err.to_string().contains("unknown cell key"), "{err}");
    let err =
        syn::parse2::<CellArgs>(quote! { seam = "X", variant = "y", seam = "Z" }).unwrap_err();
    assert!(err.to_string().contains("duplicate"), "{err}");
}

#[test]
fn spec_args_rejects_zero_revision_and_empty_reason() {
    let err = syn::parse2::<SpecArgs>(quote! { implements = #URI, r = 0 }).unwrap_err();
    assert!(err.to_string().contains("start at r1"), "{err}");
    let err = syn::parse2::<SpecArgs>(quote! { deviates = #URI, reason = "  " }).unwrap_err();
    assert!(err.to_string().contains("must not be empty"), "{err}");
}
