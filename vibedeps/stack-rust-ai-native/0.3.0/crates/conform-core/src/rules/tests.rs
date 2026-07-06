specmark::scope!("spec://vibevm/discipline/ENGINE-CONFORM-v0.1#rules");

use super::*;
use crate::Rule;
use crate::rules;

fn sf(file: &str, crate_name: &str, facts: Vec<Fact>) -> SourceFacts {
    SourceFacts {
        file: file.to_string(),
        crate_name: crate_name.to_string(),
        facts,
    }
}

fn cell_item(symbol: &str) -> Fact {
    Fact::Item {
        kind: "struct".into(),
        symbol: symbol.into(),
        line: 1,
        attrs: vec!["cell(seam = \"S\", variant = \"v\")".into()],
        is_pub: true,
        has_doctest: false,
    }
}

#[test]
fn r001_flags_ctor_outside_registry() {
    let facts = vec![
        sf(
            "crates/vibe-resolver/src/naive.rs",
            "vibe-resolver",
            vec![cell_item("vibe_resolver::naive::NaiveDepSolver")],
        ),
        sf(
            "crates/vibe-cli/src/commands/install.rs",
            "vibe-cli",
            vec![Fact::Ctor {
                type_name: "NaiveDepSolver".into(),
                line: 7,
            }],
        ),
        sf(
            "crates/vibe-cli/src/registry.rs",
            "vibe-cli",
            vec![Fact::Ctor {
                type_name: "NaiveDepSolver".into(),
                line: 9,
            }],
        ),
    ];
    let rule = rules::FlagSites {
        registry_file: "crates/vibe-cli/src/registry.rs".into(),
        gated_crate: "vibe-cli".into(),
    };
    let found = rule.check(&facts);
    assert_eq!(found.len(), 1);
    assert!(found[0].file.ends_with("install.rs"));
}

#[test]
fn r002_flags_sibling_cell_import() {
    let facts = vec![
        sf(
            "crates/x/src/alpha.rs",
            "x",
            vec![
                cell_item("x::alpha::Alpha"),
                Fact::Import {
                    from_module: "x::alpha".into(),
                    to_path: "crate::beta::Beta".into(),
                    line: 3,
                },
            ],
        ),
        sf(
            "crates/x/src/beta.rs",
            "x",
            vec![cell_item("x::beta::Beta")],
        ),
    ];
    let found = rules::CellIsolation.check(&facts);
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("beta"));
}

#[test]
fn unsafe_gate_respects_audit_crates() {
    let facts = vec![
        sf(
            "crates/a/src/lib.rs",
            "a",
            vec![Fact::UnsafeUse {
                context: "block".into(),
                line: 5,
                in_test: false,
                in_deviation: false,
            }],
        ),
        sf(
            "crates/audited/src/lib.rs",
            "audited",
            vec![Fact::UnsafeUse {
                context: "fn".into(),
                line: 6,
                in_test: false,
                in_deviation: false,
            }],
        ),
    ];
    let rule = rules::UnsafeGate {
        audit_crates: vec!["audited".into()],
    };
    let found = rule.check(&facts);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].file, "crates/a/src/lib.rs");
}

#[test]
fn unsafe_gate_honors_testimony_but_not_test_context() {
    // Three uses in one file: a bare one, a testified one, a
    // test-context one. The testimony is honored; the test context
    // is not (unsoundness in tests is still unsoundness).
    let facts = vec![sf(
        "crates/a/src/lib.rs",
        "a",
        vec![
            Fact::UnsafeUse {
                context: "block".into(),
                line: 5,
                in_test: false,
                in_deviation: false,
            },
            Fact::UnsafeUse {
                context: "block".into(),
                line: 9,
                in_test: false,
                in_deviation: true,
            },
            Fact::UnsafeUse {
                context: "block".into(),
                line: 40,
                in_test: true,
                in_deviation: false,
            },
        ],
    )];
    let rule = rules::UnsafeGate {
        audit_crates: vec![],
    };
    let found = rule.check(&facts);
    assert_eq!(found.len(), 2, "{found:?}");
    // The testified block still advances the ordinal: the bare block
    // keys #0, the test-context block keys #2 — gaining or losing a
    // neighbour's testimony never re-keys an existing fingerprint.
    assert_eq!(
        found[0].fingerprint,
        "unsafe-gate|crates/a/src/lib.rs|block#0"
    );
    assert_eq!(
        found[1].fingerprint,
        "unsafe-gate|crates/a/src/lib.rs|block#2"
    );
}

#[test]
fn req_grammar_renderer_and_acceptor_agree() {
    let msg = rules::req_message(
        "discipline://rust-ai-native/cards/scaffold-g-doctests#ops",
        "public seam fn `solve` has no compiled doctest",
        "add one doctest on `solve`",
    );
    assert!(rules::matches_req_grammar(&msg), "{msg}");
    assert!(!rules::matches_req_grammar("free text error"));
    assert!(!rules::matches_req_grammar(
        "violates REQ http://nope: x; fix surface: y"
    ));
    assert!(!rules::matches_req_grammar(
        "violates REQ spec://p/d#a: missing the fix surface"
    ));
}

#[test]
fn every_rule_message_speaks_the_req_grammar() {
    // Class F applied to conform itself: each rule's findings on a
    // synthetic violating corpus must match the grammar.
    let corpus = vec![
        sf(
            "crates/x/src/alpha.rs",
            "x",
            vec![
                cell_item("x::alpha::Alpha"),
                Fact::Import {
                    from_module: "x::alpha".into(),
                    to_path: "crate::beta::Beta".into(),
                    line: 3,
                },
            ],
        ),
        sf(
            "crates/x/src/beta.rs",
            "x",
            vec![
                cell_item("x::beta::Beta"),
                Fact::Ctor {
                    type_name: "Alpha".into(),
                    line: 9,
                },
                Fact::UnsafeUse {
                    context: "block".into(),
                    line: 12,
                    in_test: false,
                    in_deviation: false,
                },
            ],
        ),
        sf(
            "crates/x/src/lib.rs",
            "x",
            vec![
                Fact::Item {
                    kind: "fn".into(),
                    symbol: "x::solve".into(),
                    line: 4,
                    attrs: vec![],
                    is_pub: true,
                    has_doctest: false,
                },
                Fact::ErrorVariant {
                    enum_symbol: "x::Error".into(),
                    variant: "Bad".into(),
                    message: "bad thing".into(),
                    line: 8,
                    enum_attrs: vec![],
                },
            ],
        ),
    ];
    let flag_sites = rules::FlagSites {
        registry_file: "crates/x/src/registry.rs".into(),
        gated_crate: "x".into(),
    };
    let isolation = rules::CellIsolation;
    let unsafe_gate = rules::UnsafeGate {
        audit_crates: vec![],
    };
    let doctests = rules::SeamHasDoctest {
        gated_crates: vec!["x".into()],
    };
    let err_req = rules::ErrorEnumCitesReq {
        gated_crates: vec!["x".into()],
    };
    let all: Vec<&dyn Rule> = vec![&flag_sites, &isolation, &unsafe_gate, &doctests, &err_req];
    for rule in &all {
        let found = rule.check(&corpus);
        assert!(!found.is_empty(), "rule {} found nothing", rule.id());
        for f in found {
            assert!(
                rules::matches_req_grammar(&f.message),
                "rule {} message off-grammar: {}",
                rule.id(),
                f.message
            );
        }
    }
}

#[test]
fn seam_has_doctest_gates_pub_root_items_only() {
    let facts = vec![sf(
        "crates/x/src/lib.rs",
        "x",
        vec![
            Fact::Item {
                kind: "fn".into(),
                symbol: "x::documented".into(),
                line: 1,
                attrs: vec![],
                is_pub: true,
                has_doctest: true,
            },
            Fact::Item {
                kind: "fn".into(),
                symbol: "x::bare".into(),
                line: 5,
                attrs: vec![],
                is_pub: true,
                has_doctest: false,
            },
            Fact::Item {
                kind: "fn".into(),
                symbol: "x::private".into(),
                line: 9,
                attrs: vec![],
                is_pub: false,
                has_doctest: false,
            },
        ],
    )];
    let rule = rules::SeamHasDoctest {
        gated_crates: vec!["x".into()],
    };
    let found = rule.check(&facts);
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("`bare`"));
    // Non-root files are not seams for this rule.
    let nested = vec![sf(
        "crates/x/src/inner.rs",
        "x",
        vec![Fact::Item {
            kind: "fn".into(),
            symbol: "x::inner::bare".into(),
            line: 5,
            attrs: vec![],
            is_pub: true,
            has_doctest: false,
        }],
    )];
    assert!(rule.check(&nested).is_empty());
}

#[test]
fn error_enum_cites_req_flags_once_per_enum() {
    let facts = vec![sf(
        "crates/x/src/error.rs",
        "x",
        vec![
            Fact::ErrorVariant {
                enum_symbol: "x::error::Error".into(),
                variant: "A".into(),
                message: "a".into(),
                line: 4,
                enum_attrs: vec![],
            },
            Fact::ErrorVariant {
                enum_symbol: "x::error::Error".into(),
                variant: "B".into(),
                message: "b".into(),
                line: 6,
                enum_attrs: vec![],
            },
            Fact::ErrorVariant {
                enum_symbol: "x::error::Tagged".into(),
                variant: "C".into(),
                message: "c".into(),
                line: 14,
                enum_attrs: vec!["spec(implements = \"spec://p/d#err\")".into()],
            },
        ],
    )];
    let rule = rules::ErrorEnumCitesReq {
        gated_crates: vec!["x".into()],
    };
    let found = rule.check(&facts);
    assert_eq!(found.len(), 1, "one finding per untagged enum: {found:?}");
    assert!(found[0].message.contains("`Error`"));
}

#[test]
fn cell_has_oracle_satisfied_by_test_reference() {
    let cell = sf(
        "crates/x/src/naive.rs",
        "x",
        vec![cell_item("x::naive::NaiveSolver")],
    );
    let rule = rules::CellHasOracle;
    // No tests at all → finding.
    assert_eq!(rule.check(std::slice::from_ref(&cell)).len(), 1);
    // A test importing the cell type satisfies the rule.
    let test_import = sf(
        "crates/x/tests/oracle.rs",
        "x",
        vec![Fact::Import {
            from_module: "x::oracle".into(),
            to_path: "x::{DepSolver,NaiveSolver}".into(),
            line: 5,
        }],
    );
    assert!(rule.check(&[cell.clone(), test_import]).is_empty());
    // A test constructing the cell also satisfies it.
    let test_ctor = sf(
        "crates/x/tests/props.rs",
        "x",
        vec![Fact::Ctor {
            type_name: "NaiveSolver".into(),
            line: 9,
        }],
    );
    assert!(rule.check(&[cell, test_ctor]).is_empty());
}

#[test]
fn unsafe_gate_fingerprint_survives_line_shifts() {
    let before = vec![sf(
        "crates/a/src/lib.rs",
        "a",
        vec![Fact::UnsafeUse {
            context: "block".into(),
            line: 33,
            in_test: false,
            in_deviation: false,
        }],
    )];
    let after = vec![sf(
        "crates/a/src/lib.rs",
        "a",
        vec![Fact::UnsafeUse {
            context: "block".into(),
            line: 35,
            in_test: false,
            in_deviation: false,
        }],
    )];
    let rule = rules::UnsafeGate {
        audit_crates: vec![],
    };
    let fp_before = rule.check(&before)[0].fingerprint.clone();
    let fp_after = rule.check(&after)[0].fingerprint.clone();
    assert_eq!(
        fp_before, fp_after,
        "a pure line shift must not change the fingerprint"
    );
}
