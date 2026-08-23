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

/// PROP-045 ##PROJECTION-READ: an `.xml` boot source's directives are
/// collected from the projection — the directive rides in unit text, the
/// projection preserves it, and the in-place listing names the `.xml`
/// path it came from.
#[test]
fn in_place_directives_collect_from_an_xml_boot_source() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join("spec/boot")).expect("mkdir");
    std::fs::write(
        root.join("spec/boot/dyn.xml"),
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
         <p>#use spec://org.example/lib/boot</p>\n</spec>\n",
    )
    .expect("write xml");
    let specs = collect_in_place(root, &["spec/boot/dyn.xml".to_string()]);
    assert_eq!(specs.len(), 1, "{specs:?}");
    assert_eq!(specs[0].file, "spec/boot/dyn.xml");
    assert_eq!(specs[0].carrier, Carrier::Use);
    assert_eq!(specs[0].address.to_string(), "spec://org.example/lib/boot");
}

/// PROP-050 ##VIBE-WHY: the suffix is quiet exactly for the arrivals a
/// reader could already tell from the tree — a root edge (rule (1)) and a
/// lock with no visibility fields at all. A chain admission names its
/// rule; an override names the node that rewrote the decisive edge.
#[test]
fn provenance_suffix_is_quiet_for_plain_arrivals() {
    assert_eq!(provenance_suffix(None, None), "");
    assert_eq!(provenance_suffix(Some("root-edge"), None), "");
}

#[test]
fn provenance_suffix_names_chain_rule_and_override_coordinate() {
    assert_eq!(
        provenance_suffix(Some("friends-chain"), None),
        " [friends-chain]"
    );
    assert_eq!(
        provenance_suffix(Some("public-chain"), None),
        " [public-chain]"
    );
    assert_eq!(
        provenance_suffix(Some("root-edge"), Some("org.x/root")),
        " [via-override: org.x/root]"
    );
    assert_eq!(
        provenance_suffix(Some("friends-chain"), Some("org.x/root")),
        " [friends-chain] [via-override: org.x/root]"
    );
}

/// The text renderer carries the suffix on the member row (PROP-050
/// ##VIBE-WHY): a friends-chain member renders annotated, a root-edge
/// neighbour renders exactly as before — the annotation is text-render
/// only and never widens the frozen v1 JSON contract.
#[test]
fn plain_render_carries_the_provenance_suffix() {
    let mut chained = fixture_package("org.x/wal");
    chained.provenance_suffix = " [friends-chain]".to_string();
    let plain_root_edge = fixture_package("org.x/api");

    let tree = PackageTree {
        schema_version: SCHEMA_VERSION,
        generated_at: None,
        tool_version: None,
        project: Project {
            root: "/tmp/x".to_string(),
            name: Some("x".to_string()),
            is_workspace: false,
            self_coord: "org.vibevm.core/vibevm".to_string(),
        },
        roots: vec!["org.x/api".to_string()],
        packages: vec![chained, plain_root_edge],
        boot: Boot {
            static_md: None,
            index_md: IndexLane {
                present: false,
                path: "spec/boot/INDEX.md".to_string(),
                static_pointer: None,
                entries: Vec::new(),
            },
        },
        in_place_specs: Vec::new(),
        diagnostics: Vec::new(),
    };
    let out = crate::commands::tree::plain::render(&tree);
    assert!(
        out.lines()
            .any(|line| line.contains("org.x/wal [friends-chain]")),
        "the chain-admitted member carries its suffix:\n{out}"
    );
    assert!(
        out.lines()
            .any(|line| line.starts_with("org.x/api") && !line.contains('[')),
        "the root-edge member stays unannotated:\n{out}"
    );
}

/// A minimal `Package` for the render test — load type `None`, no deps,
/// and a `provenance_suffix` the caller fills in.
fn fixture_package(id: &str) -> Package {
    let (group, name) = id.split_once('/').expect("group/name id");
    Package {
        id: id.to_string(),
        group: group.to_string(),
        name: name.to_string(),
        kind: "flow".to_string(),
        version: "0.1.0".to_string(),
        content_hash: None,
        source: None,
        load: Load {
            load_type: LoadType::None,
            transitive: false,
            declared: None,
            origin: LoadOrigin::None,
            in_static_md: false,
            in_index_md: false,
            boot_path: None,
        },
        condition: Condition::absent(),
        dependencies: Vec::new(),
        provenance_suffix: String::new(),
    }
}
