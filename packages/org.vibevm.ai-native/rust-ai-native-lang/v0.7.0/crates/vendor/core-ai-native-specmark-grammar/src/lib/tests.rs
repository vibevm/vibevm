//! Grammar unit tests, out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path] mod tests;`, so `use super::*` is
//! unchanged from the inline form.

use super::*;
use quote::quote;

const URI: &str = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#req-conditional-fixpoint";

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
        "http://x/y#a",           // wrong scheme
        "spec://org.vibevm.core/vibevm#a",        // no doc-path
        "spec://org.vibevm.core/vibevm/x",        // no fragment
        "spec://org.vibevm.core/vibevm/x#",       // empty anchor
        "spec://org.vibevm.core/vibevm/x#a b",    // whitespace
        "spec://org.vibevm.core/vibevm/x#a~rx",   // non-integer pin
        "spec://org.vibevm.core/vibevm/x#a~r0",   // r0
        "spec://org.vibevm.core/vibevm/x#a#b",    // two fragments
        "spec://org.vibevm.core/vibevm/x#-a",     // leading dash
        "spec://org.vibevm.core/vibevm/x#_lead",  // leading underscore: the head must be a letter
        "spec://org.vibevm.core/vibevm/x#9lives", // digit head, likewise
        "spec://org.vibevm.core/vibevm/x#a.b",    // `.` is not an id character
    ] {
        assert!(parse_spec_uri(bad).is_err(), "should reject `{bad}`");
    }
}

/// A normative `UPPER-SLUG` fact is addressable as a URI — the sentence
/// `##FACT-ID-GRAMMAR` already states, now implemented. `#A-b` moved here
/// from the rejection set above: the owner ruled the behaviour changes.
#[test]
fn uri_accepts_an_upper_fact_anchor() {
    let u = parse_spec_uri("spec://org.vibevm.core/vibevm/x#A-b").unwrap();
    assert_eq!(u.anchor, "A-b");
    assert_eq!(u.pinned_r, None);
    assert_eq!(u.without_pin(), "spec://org.vibevm.core/vibevm/x#A-b");

    // The revision pin composes with it unchanged.
    let p = parse_spec_uri("spec://org.vibevm.core/vibevm/x#A-b~r2").unwrap();
    assert_eq!(p.anchor, "A-b");
    assert_eq!(p.pinned_r, Some(2));
    assert_eq!(p.without_pin(), "spec://org.vibevm.core/vibevm/x#A-b");

    // A real minted fact id, underscores and all.
    let f =
        parse_spec_uri("spec://org.vibevm.ai-native/core-ai-native/00-MANIFESTO#R_040").unwrap();
    assert_eq!(f.anchor, "R_040");

    // The id grammar constrains only the head character, so a dash may sit
    // anywhere after it — `#a-` is a URI the kebab law would have refused.
    // Only the head rule still bites.
    assert!(parse_spec_uri("spec://org.vibevm.core/vibevm/x#a-").is_ok());
    assert!(parse_spec_uri("spec://org.vibevm.core/vibevm/x#-a").is_err());
}

/// Every string a document may mint a name from, with the one verdict both
/// validators owe it. `is_valid_anchor` delegates to `is_valid_fact_id`, so
/// the table is shared rather than doubled: two lists would let the two laws
/// drift the moment one list was edited and the other was not.
const ID_TABLE: &[(&str, bool)] = &[
    // The kebab register — a heading anchor's house style, still legal.
    ("req-conditional-fixpoint", true),
    ("my-fact", true),
    ("root", true),
    ("a1", true),
    ("a", true),
    // The id register — legal for a heading anchor since DRIFT-034.
    ("FACT-A", true),
    ("A-b", true),
    ("Mixed-Case", true),
    ("Some_Anchor", true),
    ("R_040", true),
    ("Z9", true),
    ("x-y_z-1", true),
    // Only the head rule bites: a trailing dash is fine, a leading one is not.
    ("a-", true),
    ("-leading", false),
    ("_lead", false),
    // A digit head — the one direction the widening *narrowed*.
    ("9lives", false),
    // Outside the charset entirely.
    ("has space", false),
    ("a!", false),
    ("a.b", false),
    ("café", false),
    ("", false),
];

/// A heading anchor and a `##<ID>` fact name the same address space, so the
/// two validators must not merely happen to agree — one calls the other. The
/// shared table is the assertion that they do.
#[test]
fn the_two_validators_agree_on_every_input() {
    for &(s, want) in ID_TABLE {
        assert_eq!(is_valid_fact_id(s), want, "is_valid_fact_id(`{s}`)");
        assert_eq!(
            is_valid_anchor(s),
            is_valid_fact_id(s),
            "the two laws disagree on `{s}` — `is_valid_anchor` must delegate"
        );
    }
    // A URI's anchor position takes the same law, so an accepted id is an
    // addressable one.
    for &(s, want) in ID_TABLE {
        assert_eq!(
            parse_spec_uri(&format!("spec://org.vibevm.core/vibevm/x#{s}")).is_ok(),
            want,
            "URI anchor `{s}`"
        );
    }
}

/// The widening is not purely additive, and the asymmetry is the part a
/// future reader will not guess. Kebab admitted a digit head; the id grammar
/// requires a letter. Pinned in **both** directions so neither drifts back.
#[test]
fn a_digit_head_is_the_one_thing_the_widening_took_away() {
    // Rejected now, accepted under the kebab law.
    for was_kebab in ["9lives", "2026-07-07", "1", "0-a"] {
        assert!(
            !is_valid_anchor(was_kebab),
            "`{was_kebab}` is digit-headed and no longer an anchor"
        );
    }
    // Accepted now, rejected under the kebab law — the other direction.
    for now_legal in ["Some_Anchor", "a-", "FACT-A", "R_040"] {
        assert!(
            is_valid_anchor(now_legal),
            "`{now_legal}` is a legal id and so a legal anchor"
        );
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

// The former `fact_id_grammar_is_wider_than_the_heading_anchor_law` lived
// here with its own accept/reject lists. It is gone, not moved: its premise
// (a fact id is wider than an anchor) stopped being true, and its two lists
// were a second copy of the input set. `ID_TABLE` above carries every string
// it asserted, once.
