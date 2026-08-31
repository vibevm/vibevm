use specmark::verifies;
use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{ExtensionAppliesTo, ExtensionKey, ExtensionUse, ExtensionsControl};

use crate::{
    ExtensionProvider, ExtensionRegistry, RegistryState, SelectorSubject, SyntheticPresetSource,
    collect_extensions, collect_extensions_with_presets,
};

use super::support::{
    declaration, dependency, host, native_declaration, package_key, selected_declaration, world,
};

/// The four compile stages, in the exact order a per-point concatenation
/// would visit them — the rejected alternative §5.3 of the R4 architecture
/// names, kept here so the RED it produces is computed, not asserted from a
/// hand-copied list.
const COMPILE_STAGES: [&str; 4] = [
    "compile:source",
    "compile:document",
    "compile:lane",
    "compile:emitted",
];

fn keys(rows: &[&crate::ExtensionRegistryRow]) -> Vec<String> {
    rows.iter()
        .map(|row| row.key().as_str().to_owned())
        .collect()
}

fn per_point_concatenation(registry: &ExtensionRegistry) -> Vec<String> {
    COMPILE_STAGES
        .iter()
        .flat_map(|stage| {
            let point = stage
                .parse::<ExtensionPoint>()
                .unwrap_or_else(|error| panic!("legal compile stage `{stage}`: {error}"));
            keys(&registry.enabled_at(point))
        })
        .collect()
}

#[test]
fn exhaustive_view_is_lossless_effective_order_with_closed_state_precedence() {
    let mut inactive = declaration("inactive", "compile:source");
    inactive.auto = Some(false);
    inactive.applies_to = Some(ExtensionAppliesTo {
        packages: Some(vec!["org.allowed/*".into()]),
        paths: None,
    });
    let mut mismatch = declaration("mismatch", "compile:source");
    mismatch.applies_to = Some(ExtensionAppliesTo {
        packages: Some(vec!["org.allowed/*".into()]),
        paths: None,
    });
    let mut disabled = declaration("disabled", "compile:source");
    disabled.auto = Some(false);
    disabled.applies_to = Some(ExtensionAppliesTo {
        packages: Some(vec!["org.allowed/*".into()]),
        paths: None,
    });
    let disabled_key = ExtensionKey::authored("org.zed/tools#disabled");
    let registry = collect_extensions(world(
        vec![
            dependency("org.zed", "tools", vec![disabled, inactive, mismatch]),
            dependency(
                "org.aaa",
                "other",
                vec![declaration("effective", "phase:test")],
            ),
        ],
        host(
            vec![declaration("host", "phase:test")],
            ExtensionsControl {
                uses: vec![ExtensionUse {
                    reference: ExtensionKey::authored("org.zed/tools#mismatch"),
                    config: None,
                }],
                disable: vec![disabled_key],
            },
        ),
        None,
    ))
    .unwrap();

    let rows = registry.exhaustive(SelectorSubject::unscoped());
    assert_eq!(rows.len(), registry.rows().len());
    assert_eq!(
        rows.iter()
            .map(|view| view.row.key().as_str())
            .collect::<Vec<_>>(),
        [
            "org.zed/tools#disabled",
            "org.zed/tools#inactive",
            "org.aaa/other#effective",
            "__host__/demo#host",
            "org.zed/tools#mismatch",
        ]
    );
    assert_eq!(
        rows.iter().map(|view| view.state()).collect::<Vec<_>>(),
        [
            RegistryState::Disabled,
            RegistryState::Inactive,
            RegistryState::Effective,
            RegistryState::Effective,
            RegistryState::SelectorMismatch,
        ]
    );
    assert_eq!(rows.iter().filter(|view| view.is_effective()).count(), 2);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn enabled_at_keeps_selector_bearing_rows_that_an_unscoped_plan_drops() {
    let registry = collect_extensions(world(
        vec![dependency(
            "org.zed",
            "tools",
            vec![
                selected_declaration("inactive", Some(vec!["org.allowed/*"]), None),
                selected_declaration("gated", Some(vec!["org.allowed/*"]), None),
                declaration("off", "phase:test"),
            ],
        )],
        host(
            vec![selected_declaration("plain", None, None)],
            ExtensionsControl {
                uses: vec![ExtensionUse {
                    reference: package_key("org.zed", "tools", "gated"),
                    config: None,
                }],
                disable: vec![package_key("org.zed", "tools", "off")],
            },
        ),
        None,
    ))
    .unwrap();

    let compile = "compile:source".parse().unwrap();
    // The unscoped execution plan drops the selector-bearing activated row
    // before any document subject exists…
    assert_eq!(
        registry
            .plan(compile, SelectorSubject::unscoped())
            .iter()
            .map(|row| row.key().as_str())
            .collect::<Vec<_>>(),
        ["__host__/demo#plain"]
    );
    // …while the enabled view retains it, in exact effective order.
    assert_eq!(
        registry
            .enabled_at(compile)
            .iter()
            .map(|row| row.key().as_str())
            .collect::<Vec<_>>(),
        ["__host__/demo#plain", "org.zed/tools#gated"]
    );
    // Disabled rows are excluded at their point exactly as plans exclude
    // them; the disabled `phase:test` row never leaks into the view.
    assert!(
        registry
            .enabled_at("phase:test".parse().unwrap())
            .is_empty()
    );
    // The exhaustive all-view still retains every declaration once.
    assert_eq!(
        registry.exhaustive(SelectorSubject::unscoped()).len(),
        registry.rows().len()
    );
}

/// The compile view is ONE global effective order across every stage, and a
/// per-point concatenation is provably a different sequence.
///
/// The fixture interleaves the stages across every tier an enabled compile
/// row can occupy: a synthetic preset at `compile:lane`, two host
/// declarations at `compile:emitted` then `compile:source`, and two
/// lock-ordered dependencies activated at `compile:document` and
/// `compile:source`. Concatenating source→document→lane→emitted would open
/// with `#src-host` and close with `#emit-host` — a cross-stage sequence
/// §3.4 of the R4 architecture never authored, which a plan digest taken
/// over it would then bless.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn enabled_compile_rows_is_one_global_order_no_per_point_concatenation_reproduces() {
    let preset = SyntheticPresetSource {
        key: ExtensionKey::authored("@vibe/compile/minify"),
        provider: ExtensionProvider::Host(host(Vec::new(), ExtensionsControl::default()).provider),
        declaration: declaration("minify", "compile:lane"),
    };
    let registry = collect_extensions_with_presets(
        world(
            vec![
                dependency(
                    "org.zed",
                    "z-tools",
                    vec![declaration("doc-z", "compile:document")],
                ),
                dependency(
                    "org.aaa",
                    "a-tools",
                    vec![declaration("src-a", "compile:source")],
                ),
            ],
            host(
                vec![
                    declaration("emit-host", "compile:emitted"),
                    declaration("src-host", "compile:source"),
                    declaration("phase-host", "phase:test"),
                ],
                ExtensionsControl {
                    uses: vec![
                        ExtensionUse {
                            reference: package_key("org.zed", "z-tools", "doc-z"),
                            config: None,
                        },
                        ExtensionUse {
                            reference: package_key("org.aaa", "a-tools", "src-a"),
                            config: None,
                        },
                    ],
                    disable: Vec::new(),
                },
            ),
            None,
        ),
        vec![preset],
    )
    .unwrap();

    let global = keys(&registry.enabled_compile_rows());
    assert_eq!(
        global,
        [
            "@vibe/compile/minify",
            "__host__/demo#emit-host",
            "__host__/demo#src-host",
            "org.zed/z-tools#doc-z",
            "org.aaa/a-tools#src-a",
        ],
        "the compile view is the registry's one effective order, restricted"
    );
    // The non-compile host row is present and enabled, and still stays out.
    assert!(registry.enabled_at("phase:test".parse().unwrap()).len() == 1);
    assert!(!global.iter().any(|key| key.ends_with("#phase-host")));

    let concatenated = per_point_concatenation(&registry);
    assert_eq!(
        concatenated,
        [
            "__host__/demo#src-host",
            "org.aaa/a-tools#src-a",
            "org.zed/z-tools#doc-z",
            "@vibe/compile/minify",
            "__host__/demo#emit-host",
        ],
        "the rejected alternative is this exact fabricated sequence"
    );
    assert_ne!(
        global, concatenated,
        "a per-point concatenation moves `#src-host` to the head and \
         `#emit-host` to the tail: same set, invented order"
    );
}

/// `compile:pass` is a compile point, so a pass-tier row is a compile row.
///
/// Pinned separately from the stage-order test because the stage partition
/// and the family membership are different claims: R6 routes the pass tier
/// out of the one lowering, and that act flips exactly this assertion.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_compile_view_carries_the_whole_compile_family_including_the_pass_tier() {
    let mut pass = declaration("late-pass", "compile:pass");
    pass.compiler_internals = Some(true);
    let registry = collect_extensions(world(
        Vec::new(),
        host(
            vec![pass, declaration("src-host", "compile:source")],
            ExtensionsControl::default(),
        ),
        None,
    ))
    .unwrap();

    assert_eq!(
        keys(&registry.enabled_compile_rows()),
        ["__host__/demo#late-pass", "__host__/demo#src-host"],
        "a `compile:pass` row is a compile-point row until R6 routes it apart"
    );
}

/// Disabled and inactive compile rows leave the compile view and stay in the
/// exhaustive one — the observability law the execution filter never breaks.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
fn disabled_and_inactive_compile_rows_leave_the_view_and_stay_queryable() {
    let registry = collect_extensions(world(
        vec![dependency(
            "org.zed",
            "z-tools",
            vec![declaration("never-used", "compile:source")],
        )],
        host(
            vec![
                declaration("off-host", "compile:document"),
                declaration("on-host", "compile:document"),
            ],
            ExtensionsControl {
                uses: Vec::new(),
                disable: vec![ExtensionKey::authored("__host__/demo#off-host")],
            },
        ),
        None,
    ))
    .unwrap();

    assert_eq!(
        keys(&registry.enabled_compile_rows()),
        ["__host__/demo#on-host"],
        "the disabled host row and the unactivated dependency row are out"
    );
    let exhaustive = registry.exhaustive(SelectorSubject::unscoped());
    assert_eq!(exhaustive.len(), registry.rows().len());
    let state = |suffix: &str| {
        exhaustive
            .iter()
            .find(|view| view.row.key().as_str().ends_with(suffix))
            .unwrap_or_else(|| panic!("row `{suffix}` stays queryable"))
            .state()
    };
    assert_eq!(state("#off-host"), RegistryState::Disabled);
    assert_eq!(state("#never-used"), RegistryState::Inactive);
    assert_eq!(state("#on-host"), RegistryState::Effective);
}

/// Owner-runtime indices are views over ONE retained allocation and ONE
/// effective order. Compile dense order is the enumeration of the complete
/// compile subset; native candidates retain native rows from every family and
/// never become a second ordering authority.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#BUILD-PHASE-OWNS-IT")]
fn opaque_indices_preserve_compile_dense_order_native_order_and_row_identity() {
    let build_registry = || {
        let mut selector_native = native_declaration("selector-native", "compile:document");
        selector_native.applies_to = Some(ExtensionAppliesTo {
            packages: Some(vec!["org.selected/*".into()]),
            paths: None,
        });
        collect_extensions(world(
            vec![dependency(
                "org.dep",
                "tools",
                vec![
                    native_declaration("phase-native-dep", "phase:build"),
                    native_declaration("slot-native-dep", "slot:pre-install"),
                    native_declaration("compile-native-activated", "compile:document"),
                    native_declaration("compile-native-inactive", "compile:source"),
                ],
            )],
            host(
                vec![
                    declaration("compile-builtin-host", "compile:emitted"),
                    native_declaration("phase-native-host", "phase:test"),
                    native_declaration("compile-native-host", "compile:source"),
                    selector_native,
                    native_declaration("compile-native-disabled", "compile:document"),
                ],
                ExtensionsControl {
                    uses: vec![ExtensionUse {
                        reference: package_key("org.dep", "tools", "compile-native-activated"),
                        config: None,
                    }],
                    disable: vec![ExtensionKey::authored(
                        "__host__/demo#compile-native-disabled",
                    )],
                },
            ),
            None,
        ))
        .expect("the interleaved registry collects")
    };

    let registry = build_registry();
    let compile_indices = registry.enabled_compile_indices();
    let compile_rows = compile_indices
        .iter()
        .map(|index| registry.row_at(index).expect("origin index projects"))
        .collect::<Vec<_>>();
    assert_eq!(
        keys(&compile_rows),
        [
            "__host__/demo#compile-builtin-host",
            "__host__/demo#compile-native-host",
            "__host__/demo#selector-native",
            "org.dep/tools#compile-native-activated",
        ],
        "compile rows retain global effective order, including selectors"
    );
    assert_eq!(
        keys(&registry.enabled_compile_rows()),
        keys(&compile_rows),
        "the borrowed compatibility view delegates to the index authority"
    );
    for (from_index, from_view) in compile_rows
        .iter()
        .copied()
        .zip(registry.enabled_compile_rows())
    {
        assert!(
            std::ptr::eq(from_index, from_view),
            "both projections borrow the same retained row allocation"
        );
    }

    let dense = compile_rows
        .iter()
        .enumerate()
        .map(|(order, row)| (order, row.key().as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        dense,
        [
            (0, "__host__/demo#compile-builtin-host"),
            (1, "__host__/demo#compile-native-host"),
            (2, "__host__/demo#selector-native"),
            (3, "org.dep/tools#compile-native-activated"),
        ],
        "manager dense order is enumeration of the complete compile subset"
    );
    let storage_positions = compile_rows
        .iter()
        .map(|candidate| {
            registry
                .rows()
                .iter()
                .position(|row| std::ptr::eq(row, *candidate))
                .expect("projected row belongs to storage")
        })
        .collect::<Vec<_>>();
    assert_ne!(
        storage_positions,
        (0..compile_rows.len()).collect::<Vec<_>>(),
        "dense compile order is not exported registry storage position"
    );

    let native_rows = registry
        .enabled_native_indices()
        .into_iter()
        .map(|index| registry.row_at(&index).expect("origin index projects"))
        .collect::<Vec<_>>();
    assert_eq!(
        keys(&native_rows),
        [
            "org.dep/tools#phase-native-dep",
            "org.dep/tools#slot-native-dep",
            "__host__/demo#phase-native-host",
            "__host__/demo#compile-native-host",
            "__host__/demo#selector-native",
            "org.dep/tools#compile-native-activated",
        ],
        "the native subset retains phase and compile rows in effective order"
    );
    assert!(
        native_rows.iter().all(|candidate| registry
            .rows()
            .iter()
            .any(|row| std::ptr::eq(row, *candidate))),
        "native indices project borrowed rows, never clones"
    );
    assert!(
        native_rows.iter().all(
            |row| !row.key().as_str().ends_with("#compile-native-inactive")
                && !row.key().as_str().ends_with("#compile-native-disabled")
        ),
        "inactive and disabled native rows stay absent"
    );
    let compile_native = compile_rows
        .iter()
        .find(|row| row.key().as_str().ends_with("#compile-native-host"))
        .expect("compile projection retains the host native row");
    let candidate_native = native_rows
        .iter()
        .find(|row| row.key().as_str().ends_with("#compile-native-host"))
        .expect("native projection retains the same host native row");
    assert!(
        std::ptr::eq(*compile_native, *candidate_native),
        "compile and native projections meet at the same row allocation"
    );

    let other = build_registry();
    assert!(
        other.row_at(&compile_indices[0]).is_none(),
        "an index from another live registry allocation is rejected safely"
    );
    assert!(registry.row_at(&compile_indices[0]).is_some());

    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<crate::RegistryRowIndex>();

    let moved_index = compile_indices[0].clone();
    let moved_registry = registry;
    assert!(
        moved_registry.row_at(&moved_index).is_some(),
        "moving the owner preserves its allocation identity"
    );
    let cloned_registry = moved_registry.clone();
    assert!(
        cloned_registry.row_at(&moved_index).is_none(),
        "a registry clone owns new rows and a fresh identity"
    );
    let clone_index = cloned_registry.enabled_compile_indices()[0].clone();
    assert!(
        moved_registry.row_at(&clone_index).is_none(),
        "clone indices are equally foreign to the origin"
    );

    let stale_index = {
        let origin = build_registry();
        origin.enabled_compile_indices().remove(0)
    };
    let replacement = build_registry();
    assert!(
        replacement.row_at(&stale_index).is_none(),
        "the old Arc token survives origin drop and defeats allocator ABA"
    );
}
