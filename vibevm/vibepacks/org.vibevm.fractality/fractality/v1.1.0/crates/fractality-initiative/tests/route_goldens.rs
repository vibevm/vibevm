//! The route calculus goldens (Campaign 2 D6/P5): the §worked table of
//! the delegation decision matrix, one assert per row, plus the FromStr
//! surface and the matrix.toml drift tripwire — exercised through the
//! crate's public API. Split from route.rs along the file budget.

use fractality_initiative::route::{Axes, Context, ErrorCost, Size, Verdict, Verify, route};

/// Every rule name the matrix knows — the three KEEP gates plus the
/// three distinct delegate routes; each appears exactly once in
/// matrix.toml (its `name` field). Tripwire input only.
const RULE_NAMES: [&str; 6] = [
    "never-delegate",
    "untransferable-context",
    "judgment-small",
    "route-small-mechanical",
    "route-big-mechanical",
    "route-big-judgment-draft",
];

/// Legal axis values (§axes), for the tripwire presence checks.
const AXIS_VALUES: [&str; 10] = [
    "reversible",
    "irreversible",
    "compilable",
    "boot-loadable",
    "untransferable",
    "mechanical",
    "judgment",
    "S",
    "M",
    "L",
];

fn axes(error_cost: ErrorCost, context: Context, verify: Verify, size: Size) -> Axes {
    Axes {
        error_cost,
        context,
        verify,
        size,
    }
}

/// Unpack a Keep verdict to its rule name.
fn keep_rule(v: Verdict) -> &'static str {
    match v {
        Verdict::Keep { rule, .. } => rule,
        _ => panic!("expected Keep, got {v:?}"),
    }
}

/// Unpack a Delegate verdict to (slot, scenario, discretionary, rule).
fn delegate_tuple(v: Verdict) -> (&'static str, &'static str, bool, &'static str) {
    match v {
        Verdict::Delegate {
            slot,
            scenario,
            discretionary,
            rule,
            ..
        } => (slot, scenario, discretionary, rule),
        _ => panic!("expected Delegate, got {v:?}"),
    }
}

// ---- the §worked verdicts as goldens (plan P5: 10/10) --------------------
// One assert per golden; each cites the §worked row it reproduces, or the
// §verdict routing cell for the three completions the worked rows point at.

#[test]
fn golden_unit_tests_for_a_runner_with_a_stated_api() {
    // reversible / compilable / mechanical / S → delegate small.
    let v = route(&axes(
        ErrorCost::Reversible,
        Context::Compilable,
        Verify::Mechanical,
        Size::S,
    ));
    assert_eq!(
        delegate_tuple(v),
        ("small", "1", false, "route-small-mechanical"),
        "§worked: Write unit tests for a runner with a stated API"
    );
}

#[test]
fn golden_parser_plus_goldens_from_a_live_fixture() {
    // reversible / compilable / mechanical / M → delegate big.
    let v = route(&axes(
        ErrorCost::Reversible,
        Context::Compilable,
        Verify::Mechanical,
        Size::M,
    ));
    assert_eq!(
        delegate_tuple(v),
        ("big", "1", false, "route-big-mechanical"),
        "§worked: Implement a parser + goldens from a live fixture"
    );
}

#[test]
fn golden_uri_scheme_sweep_size_s() {
    // reversible / compilable / mechanical / S–M: the S end → small.
    let v = route(&axes(
        ErrorCost::Reversible,
        Context::Compilable,
        Verify::Mechanical,
        Size::S,
    ));
    assert_eq!(
        delegate_tuple(v),
        ("small", "1", false, "route-small-mechanical"),
        "§worked: Sweep 27 files swapping a URI scheme (S end of S–M)"
    );
}

#[test]
fn golden_uri_scheme_sweep_size_m() {
    // reversible / compilable / mechanical / S–M: the M end (cross-file
    // coupling) → big, per §verdict's M × mechanical → big slot.
    let v = route(&axes(
        ErrorCost::Reversible,
        Context::Compilable,
        Verify::Mechanical,
        Size::M,
    ));
    assert_eq!(
        delegate_tuple(v),
        ("big", "1", false, "route-big-mechanical"),
        "§worked: Sweep 27 files swapping a URI scheme (M end / cross-file coupling)"
    );
}

#[test]
fn golden_draft_the_swarm_phase_architecture() {
    // reversible / untransferable / judgment / L → keep (gate 2).
    let v = route(&axes(
        ErrorCost::Reversible,
        Context::Untransferable,
        Verify::Judgment,
        Size::L,
    ));
    assert_eq!(
        keep_rule(v),
        "untransferable-context",
        "§worked: Draft the swarm-phase architecture (context untransferable)"
    );
}

#[test]
fn golden_rotate_the_zai_token() {
    // irreversible / — / — / S → keep (gate 1). Context and verify are
    // don't-cares: the irreversible gate fires first.
    let v = route(&axes(
        ErrorCost::Irreversible,
        Context::Compilable,
        Verify::Mechanical,
        Size::S,
    ));
    assert_eq!(
        keep_rule(v),
        "never-delegate",
        "§worked: Rotate the z.ai token (irreversible beats all)"
    );
}

#[test]
fn golden_summarize_a_200_page_vendor_doc() {
    // reversible / boot-loadable / mechanical / M → delegate big, scenario 2.
    let v = route(&axes(
        ErrorCost::Reversible,
        Context::BootLoadable,
        Verify::Mechanical,
        Size::M,
    ));
    assert_eq!(
        delegate_tuple(v),
        ("big", "2", false, "route-big-mechanical"),
        "§worked: Summarize a 200-page vendor doc into a fact sheet (scenario 2)"
    );
}

#[test]
fn golden_l_mechanical_compilable() {
    // §verdict step 4 completion: L × mechanical + compilable → big, scenario 1.
    let v = route(&axes(
        ErrorCost::Reversible,
        Context::Compilable,
        Verify::Mechanical,
        Size::L,
    ));
    assert_eq!(
        delegate_tuple(v),
        ("big", "1", false, "route-big-mechanical"),
        "§verdict: M|L × mechanical → big slot (L, compilable → scenario 1)"
    );
}

#[test]
fn golden_l_mechanical_boot_loadable() {
    // §verdict step 4 completion: L × mechanical + boot-loadable → big, scenario 2.
    let v = route(&axes(
        ErrorCost::Reversible,
        Context::BootLoadable,
        Verify::Mechanical,
        Size::L,
    ));
    assert_eq!(
        delegate_tuple(v),
        ("big", "2", false, "route-big-mechanical"),
        "§verdict: M|L × mechanical → big slot (L, boot-loadable → scenario 2)"
    );
}

#[test]
fn golden_l_judgment_discretionary_draft() {
    // §verdict step 4 completion: L × judgment is the one discretionary
    // cell — gate 3 keeps only S|M, so L reaches the delegate table.
    let v = route(&axes(
        ErrorCost::Reversible,
        Context::Compilable,
        Verify::Judgment,
        Size::L,
    ));
    assert_eq!(
        delegate_tuple(v),
        ("big", "1", true, "route-big-judgment-draft"),
        "§verdict: L × judgment + compilable → big draft, scenario 1, discretionary"
    );
    let v = route(&axes(
        ErrorCost::Reversible,
        Context::BootLoadable,
        Verify::Judgment,
        Size::L,
    ));
    assert_eq!(
        delegate_tuple(v),
        ("big", "2", true, "route-big-judgment-draft"),
        "§verdict: L × judgment + boot-loadable → big draft, scenario 2, discretionary"
    );
}

// ---- FromStr round-trips + error messages list legal values --------------

#[test]
fn fromstr_round_trips_every_axis_value() {
    assert_eq!(
        "reversible".parse::<ErrorCost>().unwrap(),
        ErrorCost::Reversible
    );
    assert_eq!(
        "irreversible".parse::<ErrorCost>().unwrap(),
        ErrorCost::Irreversible
    );
    assert_eq!(
        "compilable".parse::<Context>().unwrap(),
        Context::Compilable
    );
    assert_eq!(
        "boot-loadable".parse::<Context>().unwrap(),
        Context::BootLoadable
    );
    assert_eq!(
        "untransferable".parse::<Context>().unwrap(),
        Context::Untransferable
    );
    assert_eq!("mechanical".parse::<Verify>().unwrap(), Verify::Mechanical);
    assert_eq!("judgment".parse::<Verify>().unwrap(), Verify::Judgment);
    for (s, want) in [("S", Size::S), ("M", Size::M), ("L", Size::L)] {
        assert_eq!(s.parse::<Size>().unwrap(), want);
        assert_eq!(
            s.to_ascii_lowercase().parse::<Size>().unwrap(),
            want,
            "size accepts lowercase {s}"
        );
    }
}

#[test]
fn fromstr_errors_list_the_legal_values() {
    let e = "nope".parse::<ErrorCost>().unwrap_err();
    assert!(
        e.contains("reversible") && e.contains("irreversible"),
        "{e}"
    );
    let e = "nope".parse::<Context>().unwrap_err();
    assert!(
        e.contains("compilable") && e.contains("boot-loadable") && e.contains("untransferable"),
        "{e}"
    );
    let e = "nope".parse::<Verify>().unwrap_err();
    assert!(e.contains("mechanical") && e.contains("judgment"), "{e}");
    let e = "XL".parse::<Size>().unwrap_err();
    assert!(e.contains('S') && e.contains('M') && e.contains('L'), "{e}");
}

// ---- the drift tripwire: matrix.toml ↔ the executable form --------------
// Axis values legitimately recur across [axes], gate `when` conditions,
// and route keys, so they are presence-checked. Rule names are unique
// identifiers (each lives only in its `name` field), so they must appear
// exactly once. The structural counts pin the cardinality of the table.

#[test]
fn matrix_toml_mentions_every_axis_value_and_rule_name_once() {
    let toml = include_str!("../src/matrix.toml");

    // Every axis value is present somewhere in the TOML.
    for value in AXIS_VALUES {
        assert!(
            toml.contains(value),
            "matrix.toml must mention axis value {value:?}"
        );
    }

    // Every rule name appears exactly once (its `name = "…"` line).
    for name in RULE_NAMES {
        let count = toml.matches(name).count();
        assert_eq!(
            count, 1,
            "matrix.toml must mention rule name {name:?} exactly once, found {count}"
        );
    }

    // Structural cardinality: three gates, three routes (M|L grouped),
    // three keep verdicts, one discretionary flag.
    assert_eq!(toml.matches("[[gate]]").count(), 3, "three KEEP gates");
    assert_eq!(
        toml.matches("[[route]]").count(),
        3,
        "three delegate routes (M|L grouped)"
    );
    assert_eq!(
        toml.matches("verdict = \"keep\"").count(),
        3,
        "three keep verdicts"
    );
    assert_eq!(
        toml.matches("discretionary = true").count(),
        1,
        "one discretionary cell"
    );
}

// ---- gate order: irreversible beats untransferable beats judgment-small --

#[test]
fn gate_order_irreversible_beats_untransferable_beats_judgment_small() {
    // Gate 1 fires before gate 2: irreversible + untransferable is kept
    // by never-delegate, not untransferable-context.
    let v = route(&axes(
        ErrorCost::Irreversible,
        Context::Untransferable,
        Verify::Judgment,
        Size::S,
    ));
    assert_eq!(
        keep_rule(v),
        "never-delegate",
        "gate 1 (irreversible) fires before gate 2 (untransferable)"
    );
    // Gate 2 fires before gate 3: untransferable + judgment + S is kept
    // by untransferable-context, not judgment-small.
    let v = route(&axes(
        ErrorCost::Reversible,
        Context::Untransferable,
        Verify::Judgment,
        Size::S,
    ));
    assert_eq!(
        keep_rule(v),
        "untransferable-context",
        "gate 2 (untransferable) fires before gate 3 (judgment-small)"
    );
    // Gate 3 itself fires for a transferable, judgment, S|M task — no
    // §worked row exercises this cell, so it is pinned here.
    let v = route(&axes(
        ErrorCost::Reversible,
        Context::Compilable,
        Verify::Judgment,
        Size::M,
    ));
    assert_eq!(
        keep_rule(v),
        "judgment-small",
        "gate 3 (judgment-small) fires for transferable × judgment × S|M"
    );
}
