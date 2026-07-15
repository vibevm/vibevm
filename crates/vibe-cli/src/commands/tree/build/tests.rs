//! Characterization oracle for the load-classification engine (PROP-036
//! §2.3–§2.5; scaffold-d/h).
//!
//! `classify_origin` is a pure decision table — the analyzer's non-obvious
//! dynamic. This is its runnable reference: every documented row enumerated,
//! so a weak reader **steps through** the classification instead of predicting
//! it (MANIFESTO §3 — execution-prediction is where weak models collapse). A
//! change to the table that is not reflected here fails the cell's own tests.

use super::*;
use vibe_core::manifest::LinkType;

/// One row of the decision table: the six inputs → `(transitive, origin)`.
#[cfg(test)]
fn row(
    load: LoadType,
    when: bool,
    declarer: bool,
    in_closure: bool,
    declared: Option<LinkType>,
    suggested: Option<LinkType>,
) -> (bool, LoadOrigin) {
    classify_origin(load, when, declarer, in_closure, declared, suggested)
}

#[test]
fn none_lane_is_always_default_none() {
    // A package in neither lane: no origin to attribute.
    assert_eq!(
        row(LoadType::None, false, false, false, None, None),
        (false, LoadOrigin::None)
    );
}

#[test]
fn dynamic_when_gate_wins_over_everything() {
    // A `when`-gated dynamic entry is WhenForced regardless of declaration.
    assert_eq!(
        row(
            LoadType::Dynamic,
            true,
            false,
            false,
            Some(LinkType::Dynamic),
            None
        ),
        (false, LoadOrigin::WhenForced)
    );
}

#[test]
fn dynamic_declared_then_default() {
    assert_eq!(
        row(
            LoadType::Dynamic,
            false,
            false,
            false,
            Some(LinkType::Dynamic),
            None
        ),
        (false, LoadOrigin::Declared)
    );
    assert_eq!(
        row(LoadType::Dynamic, false, false, false, None, None),
        (false, LoadOrigin::Default)
    );
}

#[test]
fn static_transitive_declarer_owns_its_static_ness() {
    // The declarer of a `static-transitive` edge is attributed Declared — its
    // static-ness is its own, not the closure's.
    assert_eq!(
        row(
            LoadType::Static,
            false,
            true,
            false,
            Some(LinkType::StaticTransitive),
            None
        ),
        (false, LoadOrigin::Declared)
    );
}

#[test]
fn static_in_closure_not_self_suggested_is_transitive() {
    // Pulled into a static-transitive closure, not statically suggested on its
    // own — the transitive origin, and the only `transitive = true` row.
    assert_eq!(
        row(LoadType::Static, false, false, true, None, None),
        (true, LoadOrigin::StaticTransitive)
    );
}

#[test]
fn static_precedence_declared_then_suggested_then_default() {
    assert_eq!(
        row(
            LoadType::Static,
            false,
            false,
            false,
            Some(LinkType::Static),
            None
        ),
        (false, LoadOrigin::Declared)
    );
    assert_eq!(
        row(
            LoadType::Static,
            false,
            false,
            false,
            None,
            Some(LinkType::Static)
        ),
        (false, LoadOrigin::Suggested)
    );
    assert_eq!(
        row(LoadType::Static, false, false, false, None, None),
        (false, LoadOrigin::Default)
    );
    // `static-hard` counts as a static link on both the declared and suggested
    // tiers (PROP-038 §2.3) — the classifier must not treat it as non-static.
    assert_eq!(
        row(
            LoadType::Static,
            false,
            false,
            false,
            Some(LinkType::StaticHard),
            None
        ),
        (false, LoadOrigin::Declared)
    );
    assert_eq!(
        row(
            LoadType::Static,
            false,
            false,
            false,
            None,
            Some(LinkType::StaticHard)
        ),
        (false, LoadOrigin::Suggested)
    );
}
