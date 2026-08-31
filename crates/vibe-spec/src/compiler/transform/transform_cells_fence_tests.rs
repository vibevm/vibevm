//! The transform module tree stays exhaustively classified.
//!
//! Its own cell rather than a fourth test beside the per-cell fences,
//! because it makes a different claim: not "this cell obeys its family" but
//! "every cell HAS one". A production cell that ships undeclared, or
//! declared and unclassified, is what this test exists to make impossible —
//! and it is the assertion a future atom adding a cell will meet first.

use std::collections::BTreeSet;

use super::fence_families::{
    HEADER_RULES, LOWERING_RULES, MINIFY_RULES, NATIVE_IDENTITY_RULES, NATIVE_MANAGER_RULES,
    NATIVE_SCHEDULE_RULES, PLAN_CARRIER_RULES, SELECTOR_RULES, WRAPPER_RULES, offenders,
};

/// The rule families stay exhaustive over the module tree: the production
/// transform cells are exactly the fifteen declared `pub(crate) mod`s, every
/// cfg-test cell is declared too, and no undeclared `.rs` sibling can ship
/// unclassified (a new production cell must be added to a family here).
#[test]
fn the_module_tree_declares_every_transform_cell_under_a_rule_family() {
    let module_tree =
        syn::parse_file(include_str!("mod.rs")).expect("transform/mod.rs parses as Rust");
    let mut production = BTreeSet::new();
    let mut test_only = BTreeSet::new();
    for item in &module_tree.items {
        let syn::Item::Mod(item_mod) = item else {
            continue;
        };
        let is_test = item_mod.attrs.iter().any(|attribute| {
            matches!(&attribute.meta, syn::Meta::List(list)
                if list.path.is_ident("cfg")
                    && list.tokens.to_string().contains("test"))
        });
        if is_test {
            test_only.insert(item_mod.ident.to_string());
        } else {
            production.insert(item_mod.ident.to_string());
        }
    }
    assert_eq!(
        production,
        BTreeSet::from([
            "behavior".to_owned(),
            "config".to_owned(),
            // T10B's two production cells: the lowering entry and the
            // effective-configuration half it delegates to.
            "config_lowering".to_owned(),
            "emitted_reconstruction".to_owned(),
            "fault".to_owned(),
            // T10C's one production cell: the ACTIVE list a nonempty plan
            // records into its artifact.
            "header".to_owned(),
            "lane_admission".to_owned(),
            "lowering".to_owned(),
            "native_identity".to_owned(),
            "native_manager".to_owned(),
            "native_schedule".to_owned(),
            "plan".to_owned(),
            "plan_digest".to_owned(),
            "plan_validate".to_owned(),
            "registry".to_owned(),
            "schedule".to_owned(),
            "selector_admission".to_owned(),
            // R4.2's one production cell: the first real behavior and the
            // segmented emitted-tape adapter it drives.
            "xml_minify_binding".to_owned(),
        ]),
        "a new production transform cell must be declared AND classified"
    );
    assert_eq!(
        test_only,
        BTreeSet::from([
            "carriage".to_owned(),
            "config_tests".to_owned(),
            // The manifest DAG proof, split out of `plan_fence_tests` at its
            // file-budget seam and along its own responsibility line.
            "dependency_dag_fence_tests".to_owned(),
            // The fence families and their AST classifier, split out of
            // `schedule_fence_tests` at its file-budget seam.
            "fence_families".to_owned(),
            // T10C's test cells.
            "header_e2e_tests".to_owned(),
            "header_tests".to_owned(),
            // T10B's test cells.
            "lowering_e2e_tests".to_owned(),
            "lowering_tests".to_owned(),
            "lowering_worlds".to_owned(),
            "native_identity_tests".to_owned(),
            "native_fence_tests".to_owned(),
            "native_manager_hostile_tests".to_owned(),
            "native_manager_matrix_tests".to_owned(),
            "native_manager_test_support".to_owned(),
            "plan_digest_tests".to_owned(),
            "plan_fence_tests".to_owned(),
            "plan_refusal_tests".to_owned(),
            "plan_test_support".to_owned(),
            "plan_tests".to_owned(),
            "plan_visibility_fence_tests".to_owned(),
            "registry_fence_tests".to_owned(),
            "registry_test_support".to_owned(),
            "registry_tests".to_owned(),
            "schedule_emitted_tests".to_owned(),
            "schedule_execution_tests".to_owned(),
            "schedule_execution_vehicles".to_owned(),
            "schedule_fence_tests".to_owned(),
            "schedule_lane_tests".to_owned(),
            "schedule_lane_vehicles".to_owned(),
            "schedule_selector_tests".to_owned(),
            "schedule_selector_vehicles".to_owned(),
            "schedule_selector_worlds".to_owned(),
            "schedule_separator_tests".to_owned(),
            "schedule_tests".to_owned(),
            "selector_admission_tests".to_owned(),
            "transform_cells_fence_tests".to_owned(),
            // R4.2's test cells.
            "xml_minify_binding_e2e_tests".to_owned(),
            "xml_minify_binding_tests".to_owned(),
        ]),
        "a new test cell must be declared too — undeclared files do not compile"
    );

    // The classification itself: the wrapper cell, the T6c lane-admission
    // gate, the T9 emitted-reconstruction cell and the T8 fault family under
    // wrapper rules, the T8 admission cell under its own, the plan cells
    // under the stronger carrier rules. The gates, the reconstruction cell
    // and the fault family belong to the wrapper family because they hold the
    // wrapper's own posture — no manifest/collector/row/path/codec surface,
    // no upward builtin spelling, no kernel selector, and no fault eliminated
    // by panic — while legitimately boxing CONCRETE error types.
    assert!(offenders(include_str!("schedule.rs"), &WRAPPER_RULES).is_empty());
    assert!(offenders(include_str!("native_schedule.rs"), &NATIVE_SCHEDULE_RULES).is_empty());
    assert!(offenders(include_str!("native_manager.rs"), &NATIVE_MANAGER_RULES).is_empty());
    assert!(offenders(include_str!("native_identity.rs"), &NATIVE_IDENTITY_RULES).is_empty());
    assert!(offenders(include_str!("lane_admission.rs"), &WRAPPER_RULES).is_empty());
    assert!(offenders(include_str!("emitted_reconstruction.rs"), &WRAPPER_RULES).is_empty());
    assert!(offenders(include_str!("fault.rs"), &WRAPPER_RULES).is_empty());
    assert!(offenders(include_str!("selector_admission.rs"), &SELECTOR_RULES).is_empty());
    assert!(offenders(include_str!("plan.rs"), &PLAN_CARRIER_RULES).is_empty());
    // T10B: the lowering cell under its own family — the one production cell
    // permitted to name a kernel ROW, and still forbidden every collector
    // spelling. Its effective-configuration half owns no behavior channel
    // either, so the plan-carrier family binds it here; the `toml` permission
    // R4.2 gave it is granted by name in `plan_fence_tests`, where the common
    // parser/serializer set lives.
    assert!(offenders(include_str!("lowering.rs"), &LOWERING_RULES).is_empty());
    assert!(offenders(include_str!("config_lowering.rs"), &PLAN_CARRIER_RULES).is_empty());
    // T10C: the header cell under its own family — the one production cell
    // permitted to name the shared generated-comment codec, because spelling
    // the ACTIVE list is exactly what it exists to do. It is held to more
    // besides: a pure value builder over a built plan, owning no behavior
    // channel and reading no collector, and — the rule §7.1 states by name —
    // naming no OTHER percent codec, so one identity cannot acquire a second
    // spelling here.
    assert!(offenders(include_str!("header.rs"), &HEADER_RULES).is_empty());
    // R4.2: the binding cell under its own family — the one production cell
    // permitted to name the EMIT cell's framing, because reading that framing
    // back off a tape is exactly what it exists to do. The codec stays out:
    // the binding asks `framing` for a hoisted origin rather than decoding a
    // comment itself, so there is one framing grammar and one codec call site.
    assert!(offenders(include_str!("xml_minify_binding.rs"), &MINIFY_RULES).is_empty());
    // The reconstruction cell is held to MORE than its family requires, and
    // the extra is asserted rather than trusted: it is a pure value builder,
    // so — exactly like the selector admission cell — it owns no behavior
    // channel of any spelling. That ban belongs to the plan-carrier family,
    // so both are checked; together they say "no behavior channel AND no
    // fault eliminated by panic", which neither family says alone.
    assert!(
        offenders(
            include_str!("emitted_reconstruction.rs"),
            &PLAN_CARRIER_RULES
        )
        .is_empty()
    );
}
