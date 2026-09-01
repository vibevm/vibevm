//! Unit tests for boot-graph fingerprints ([`super`]) — the Merkle change
//! detector that drives dirty-subgraph regeneration (PROP-038 §2.7), and the
//! owner-plan frame R4 architecture §7.1 adds to it.

use super::*;
use crate::boot::hybrid::testkit::{id, table, unit};
use vibe_core::manifest::LinkType;

#[cfg(test)]
fn vers(names: &[&str]) -> HashMap<UnitId, String> {
    names.iter().map(|n| (id(n), "1.0.0".to_string())).collect()
}

/// No owner activates anything — the historical input, and the shape every
/// pre-T10C assertion is stated under.
#[cfg(test)]
fn no_plans() -> HashMap<UnitId, String> {
    HashMap::new()
}

/// One synthetic owner-plan digest for `name`. Synthetic on purpose: this
/// cell tests the FRAMING, so it drives the frame directly with a literal hex
/// rather than standing up a world. The integration form — a real activated
/// owner whose recorded fingerprint moves while a sibling's does not — lives
/// in `install::tests_minify_units`.
#[cfg(test)]
fn plan_for(name: &str, digest: &str) -> HashMap<UnitId, String> {
    [(id(name), digest.to_string())].into_iter().collect()
}

#[test]
fn fingerprint_is_deterministic() {
    let t = table(vec![
        unit("root", &[("a", LinkType::Static)]),
        unit("a", &[]),
    ]);
    let v = vers(&["root", "a"]);
    assert_eq!(
        fingerprints(&t, &v, &no_plans()),
        fingerprints(&t, &v, &no_plans())
    );
}

#[test]
fn a_version_change_flips_the_fingerprint_up_the_static_chain() {
    let t = table(vec![
        unit("root", &[("a", LinkType::Static)]),
        unit("a", &[]),
    ]);
    let v1 = vers(&["root", "a"]);
    let mut v2 = v1.clone();
    v2.insert(id("a"), "2.0.0".to_string());
    let f1 = fingerprints(&t, &v1, &no_plans());
    let f2 = fingerprints(&t, &v2, &no_plans());
    assert_ne!(f1[&id("a")], f2[&id("a")]);
    assert_ne!(
        f1[&id("root")],
        f2[&id("root")],
        "a static parent propagates the change"
    );
}

#[test]
fn a_link_type_switch_flips_the_fingerprint() {
    let t1 = table(vec![
        unit("root", &[("a", LinkType::Static)]),
        unit("a", &[]),
    ]);
    let t2 = table(vec![
        unit("root", &[("a", LinkType::Dynamic)]),
        unit("a", &[]),
    ]);
    let v = vers(&["root", "a"]);
    assert_ne!(
        fingerprints(&t1, &v, &no_plans())[&id("root")],
        fingerprints(&t2, &v, &no_plans())[&id("root")],
        "a dynamic<->static switch flips the parent"
    );
}

#[test]
fn a_dynamic_boundary_isolates_a_change_behind_it() {
    // root→a dynamic, a→b static. A change to b flips a (its static parent)
    // but NOT root — the dynamic edge to a breaks propagation (PROP-038 §2.7).
    let t = table(vec![
        unit("root", &[("a", LinkType::Dynamic)]),
        unit("a", &[("b", LinkType::Static)]),
        unit("b", &[]),
    ]);
    let v1 = vers(&["root", "a", "b"]);
    let mut v2 = v1.clone();
    v2.insert(id("b"), "2.0.0".to_string());
    let f1 = fingerprints(&t, &v1, &no_plans());
    let f2 = fingerprints(&t, &v2, &no_plans());
    assert_ne!(f1[&id("a")], f2[&id("a")], "b's static parent a changes");
    assert_eq!(
        f1[&id("root")],
        f2[&id("root")],
        "the dynamic edge breaks propagation"
    );
}

#[test]
fn adding_a_static_edge_flips_the_fingerprint() {
    let t1 = table(vec![unit("root", &[]), unit("a", &[])]);
    let t2 = table(vec![
        unit("root", &[("a", LinkType::Static)]),
        unit("a", &[]),
    ]);
    let v = vers(&["root", "a"]);
    assert_ne!(
        fingerprints(&t1, &v, &no_plans())[&id("root")],
        fingerprints(&t2, &v, &no_plans())[&id("root")]
    );
}

/// HISTORICAL BYTE IDENTITY, stated as a literal (R4 architecture §7.1).
///
/// One fixed fixture unit's fingerprint, computed with no plan digests,
/// spelled out as the exact hex it has always had. The literal was obtained
/// from an INDEPENDENT model of the documented pre-frame Merkle body — the
/// module doc's field order, hashed by hand outside this crate — not copied
/// off a run, so it is an oracle and not a snapshot. A relative assertion —
/// "same as itself", "same as its twin" — would stay green if a frame were
/// appended unconditionally, because both sides would move together. This one
/// cannot: a frame that fires when the map is empty changes this literal, and
/// so does any reordering or relabelling of the Merkle body.
#[test]
fn the_planless_fingerprint_of_a_fixed_unit_is_its_exact_historical_literal() {
    let t = table(vec![
        unit("root", &[("a", LinkType::Static), ("b", LinkType::Dynamic)]),
        unit("a", &[]),
        unit("b", &[]),
    ]);
    let v = vers(&["root", "a", "b"]);
    let fps = fingerprints(&t, &v, &no_plans());
    assert_eq!(
        fps[&id("root")],
        "5fd0979d1464243bb0012c26a71c41184f8582297c534a882fdc6a67ae4b7247",
        "the planless boot-graph fingerprint is frozen: no frame is written \
         for a unit whose owner activates nothing"
    );
}

/// FRAME SEMANTICS (R4 architecture §7 matrix row 8, plus the propagation
/// law), on one graph, in one test — because the four claims are only
/// meaningful together.
///
/// `root --static--> a`, `root --dynamic--> b`. Framing a's plan must move a
/// and root (a static parent inlines a's zone bytes, which a's plan produced);
/// framing b's plan must move b and NOT root (the dynamic edge carries
/// identity only, so the plan stays behind the boundary); and removing the
/// entry must restore the exact prior value, not merely "a different one".
#[test]
fn a_frame_moves_its_unit_and_its_static_parents_only_and_removing_it_restores_the_literal() {
    let t = table(vec![
        unit("root", &[("a", LinkType::Static), ("b", LinkType::Dynamic)]),
        unit("a", &[]),
        unit("b", &[]),
    ]);
    let v = vers(&["root", "a", "b"]);
    let base = fingerprints(&t, &v, &no_plans());

    let framed_a = fingerprints(&t, &v, &plan_for("a", &"ab".repeat(32)));
    assert_ne!(framed_a[&id("a")], base[&id("a")], "a's own frame moves a");
    assert_ne!(
        framed_a[&id("root")],
        base[&id("root")],
        "a static parent hashes a's FINGERPRINT, so the plan propagates up"
    );
    assert_eq!(
        framed_a[&id("b")],
        base[&id("b")],
        "and reaches nobody else"
    );

    let framed_b = fingerprints(&t, &v, &plan_for("b", &"cd".repeat(32)));
    assert_ne!(framed_b[&id("b")], base[&id("b")], "b's own frame moves b");
    assert_eq!(
        framed_b[&id("root")],
        base[&id("root")],
        "a dynamic parent hashes identity only: the plan stops at the boundary"
    );

    // A different digest is a different fingerprint — otherwise the frame
    // could be present and constant.
    assert_ne!(
        framed_a[&id("a")],
        fingerprints(&t, &v, &plan_for("a", &"ef".repeat(32)))[&id("a")],
        "the frame carries the digest, not merely its presence"
    );

    // Removing the entry restores the EXACT prior value, literal included.
    assert_eq!(
        fingerprints(&t, &v, &no_plans()),
        base,
        "no entry, no frame — the historical fingerprint returns byte-exact"
    );
    assert_eq!(
        base[&id("root")],
        "5fd0979d1464243bb0012c26a71c41184f8582297c534a882fdc6a67ae4b7247"
    );
}

/// OWNER SCOPING AT THE FRAME: the map is keyed per unit, and a unit's frame
/// reads ITS entry and never a sibling's.
///
/// Two sibling units under one root, each given a distinct synthetic digest.
/// Swapping the two entries must move both siblings — if a unit read the
/// map by anything but its own key (a first entry, an insertion order, a
/// single global value), the swap would be invisible.
#[test]
fn a_units_frame_reads_its_own_entry_and_never_another_units() {
    let t = table(vec![
        unit("root", &[("a", LinkType::Static), ("b", LinkType::Static)]),
        unit("a", &[]),
        unit("b", &[]),
    ]);
    let v = vers(&["root", "a", "b"]);
    let left = "ab".repeat(32);
    let right = "cd".repeat(32);

    let straight: HashMap<UnitId, String> = [(id("a"), left.clone()), (id("b"), right.clone())]
        .into_iter()
        .collect();
    let swapped: HashMap<UnitId, String> =
        [(id("a"), right), (id("b"), left)].into_iter().collect();

    let one = fingerprints(&t, &v, &straight);
    let other = fingerprints(&t, &v, &swapped);
    assert_ne!(one[&id("a")], other[&id("a")], "a read a's entry");
    assert_ne!(one[&id("b")], other[&id("b")], "b read b's entry");

    // And a's fingerprint under the straight map is exactly what it is when
    // ONLY a is framed: b's entry contributes nothing to a.
    assert_eq!(
        one[&id("a")],
        fingerprints(&t, &v, &plan_for("a", &"ab".repeat(32)))[&id("a")],
        "a sibling's plan never enters this unit's frame"
    );
}

#[test]
fn pending_raw_frame_propagates_only_through_static_edges_and_removal_restores_base() {
    let t = table(vec![
        unit("root", &[("a", LinkType::Static), ("b", LinkType::Dynamic)]),
        unit("a", &[]),
        unit("b", &[]),
    ]);
    let v = vers(&["root", "a", "b"]);
    let base = fingerprints(&t, &v, &no_plans());
    let pending_a = HashMap::from([(id("a"), NativePendingFrame::new([0xab; 32]))]);
    let framed_a = fingerprints_with_pending(&t, &v, &no_plans(), &pending_a);
    assert_ne!(framed_a[&id("a")], base[&id("a")]);
    assert_ne!(framed_a[&id("root")], base[&id("root")]);

    let pending_b = HashMap::from([(id("b"), NativePendingFrame::new([0xcd; 32]))]);
    let framed_b = fingerprints_with_pending(&t, &v, &no_plans(), &pending_b);
    assert_ne!(framed_b[&id("b")], base[&id("b")]);
    assert_eq!(framed_b[&id("root")], base[&id("root")]);
    assert_eq!(
        fingerprints_with_pending(&t, &v, &no_plans(), &HashMap::new()),
        base
    );
}
