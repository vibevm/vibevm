//! Unit tests for [`super`], out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path] mod tests;`, so the module-tree
//! position — and therefore `use super::*` — is unchanged from the
//! inline form. Non-`#[test]` helpers carry `#[cfg(test)]` so
//! file-grain scanners (the conform frontend) scope their `unwrap`s
//! as test code.

use super::*;

#[cfg(test)]
fn extract(src: &str) -> Vec<Fact> {
    RustFrontend.extract("crates/x/src/m.rs", "x", "x::m", src)
}

#[test]
fn extracts_items_with_cell_and_spec_attrs() {
    // The cell name is computed: seam `S`, variant `v` → `VS`
    // (Pascal("v") + seam), so the type is `VS`, the convention
    // cell-name-is-computed (B-038) checks.
    let facts = extract(
        r#"
        #[cell(seam = "S", variant = "v")]
        #[spec(implements = "spec://p/d#a")]
        pub struct VS;
        "#,
    );
    let Some(Fact::Item { symbol, attrs, .. }) =
        facts.iter().find(|f| matches!(f, Fact::Item { .. }))
    else {
        panic!("expected an item fact, got {facts:?}");
    };
    assert_eq!(symbol, "x::m::VS");
    assert!(attrs.iter().any(|a| a.starts_with("cell(")));
    assert!(attrs.iter().any(|a| a.starts_with("spec(")));
}

#[test]
fn extracts_imports_ctors_and_unsafe() {
    let facts = extract(
        r#"
        use crate::beta::Beta;
        pub fn build() {
            let _x = Widget::new(1);
            unsafe { core::hint::unreachable_unchecked() }
        }
        pub unsafe fn raw() {}
        "#,
    );
    assert!(
        facts
            .iter()
            .any(|f| matches!(f, Fact::Import { to_path, .. } if to_path == "crate::beta::Beta"))
    );
    assert!(
        facts
            .iter()
            .any(|f| matches!(f, Fact::Ctor { type_name, .. } if type_name == "Widget"))
    );
    let unsafes: Vec<_> = facts
        .iter()
        .filter(|f| matches!(f, Fact::UnsafeUse { .. }))
        .collect();
    assert_eq!(unsafes.len(), 2, "block + unsafe fn: {facts:?}");
}

#[test]
fn unparseable_source_yields_no_facts() {
    assert!(extract("pub fn broken( {").is_empty());
}

#[test]
fn emits_file_metrics_for_parsed_files() {
    let facts = extract("pub fn a() {}\npub fn b() {}\n");
    assert!(
        facts
            .iter()
            .any(|f| matches!(f, Fact::FileMetrics { lines: 2 })),
        "{facts:?}"
    );
}

#[test]
fn unwrap_in_domain_vs_test_scopes() {
    let facts = extract(
        r#"
        pub fn domain() { Some(1).unwrap(); }
        pub fn hinted() { std::fs::read("x").expect("io"); }
        #[test]
        fn in_test_fn() { Some(1).unwrap(); }
        #[cfg(test)]
        mod tests {
            fn helper() { Some(2).unwrap(); }
        }
        "#,
    );
    let unwraps: Vec<(&str, bool)> = facts
        .iter()
        .filter_map(|f| match f {
            Fact::UnwrapUse {
                method, in_test, ..
            } => Some((method.as_str(), *in_test)),
            _ => None,
        })
        .collect();
    assert_eq!(
        unwraps,
        vec![
            ("unwrap", false),
            ("expect", false),
            ("unwrap", true),
            ("unwrap", true),
        ],
        "{facts:?}"
    );
}

#[test]
fn unwrap_in_deviation_scopes_fn_grain_only() {
    let facts = extract(
        r#"
        pub fn plain() { Some(1).unwrap(); }

        #[spec(deviates = "spec://p/d#a", reason = "recorded boundary")]
        pub fn testified() { Some(1).unwrap(); }

        #[spec(implements = "spec://p/d#a")]
        pub fn implementing() { Some(1).unwrap(); }

        pub struct S;
        impl S {
            #[spec(deviates = "spec://p/d#a", reason = "method-grain testimony")]
            fn method(&self) { Some(1).unwrap(); }
            fn bare(&self) { Some(1).unwrap(); }
        }

        #[spec(deviates = "spec://p/d#other", reason = "about the impl, not unwraps")]
        impl T for S {
            fn no_amnesty(&self) { Some(1).unwrap(); }
        }
        "#,
    );
    let unwraps: Vec<bool> = facts
        .iter()
        .filter_map(|f| match f {
            Fact::UnwrapUse { in_deviation, .. } => Some(*in_deviation),
            _ => None,
        })
        .collect();
    assert_eq!(
        unwraps,
        vec![false, true, false, true, false, false],
        "{facts:?}"
    );
}

#[test]
fn unsafe_scoping_sees_tests_testimony_and_impl_methods() {
    let facts = extract(
        r#"
        pub fn bare() { unsafe { std::hint::black_box(()) } }

        #[spec(deviates = "spec://p/d#a", reason = "recorded boundary")]
        pub fn testified() { unsafe { std::hint::black_box(()) } }

        pub struct S;
        impl S {
            pub unsafe fn raw_method(&self) {}
            #[spec(deviates = "spec://p/d#a", reason = "method testimony")]
            fn covered(&self) { unsafe { std::hint::black_box(()) } }
        }

        #[cfg(test)]
        mod tests {
            fn helper() { unsafe { std::hint::black_box(()) } }
        }
        "#,
    );
    let unsafes: Vec<(String, bool, bool)> = facts
        .iter()
        .filter_map(|f| match f {
            Fact::UnsafeUse {
                context,
                in_test,
                in_deviation,
                ..
            } => Some((context.clone(), *in_test, *in_deviation)),
            _ => None,
        })
        .collect();
    assert_eq!(
        unsafes,
        vec![
            ("block".into(), false, false),
            ("block".into(), false, true),
            ("fn raw_method".into(), false, false),
            ("block".into(), false, true),
            ("block".into(), true, false),
        ],
        "{facts:?}"
    );
}

#[test]
fn extracts_visibility_and_doctest_presence() {
    let facts = extract(
        r#"
        /// Canonical use:
        ///
        /// ```
        /// assert_eq!(1, 1);
        /// ```
        pub fn documented() {}

        /// Prose only.
        pub fn bare() {}

        fn private() {}
        "#,
    );
    let item = |name: &str| {
        facts
            .iter()
            .find_map(|f| match f {
                Fact::Item {
                    symbol,
                    is_pub,
                    has_doctest,
                    ..
                } if symbol.ends_with(name) => Some((*is_pub, *has_doctest)),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(item("documented"), (true, true));
    assert_eq!(item("bare"), (true, false));
    assert_eq!(item("private"), (false, false));
}

#[test]
fn extracts_thiserror_variants_with_enum_attrs() {
    let facts = extract(
        r#"
        #[spec(implements = "spec://p/d#err")]
        #[derive(Debug)]
        pub enum Error {
            #[error("file `{0}` missing")]
            Missing(String),
            #[error(transparent)]
            Io(std::io::Error),
        }
        "#,
    );
    let variants: Vec<_> = facts
        .iter()
        .filter_map(|f| match f {
            Fact::ErrorVariant {
                variant,
                message,
                enum_attrs,
                ..
            } => Some((variant.clone(), message.clone(), enum_attrs.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(variants.len(), 2, "{facts:?}");
    assert_eq!(variants[0].0, "Missing");
    assert!(variants[0].1.contains("missing"));
    assert!(variants[0].2.iter().any(|a| a.starts_with("spec(")));
    // transparent carries no display template
    assert_eq!(variants[1].1, "");
}

#[test]
fn emits_invariant_comments_with_canonical_markers() {
    // A marker leads the comment → emitted, normalised to the config's
    // canonical spelling (the colon-bearing labeled tag verbatim). The
    // line is the comment's source line.
    let facts = extract(
        "// INVARIANT: relies on the lock being held.\n\
         // PANICS: when n < 0.\n\
         // You must never call this mid-sentence.\n",
    );
    let comments: Vec<(String, u32, bool)> = facts
        .iter()
        .filter_map(|f| match f {
            Fact::InvariantComment {
                marker,
                line,
                in_test,
            } => Some((marker.clone(), *line, *in_test)),
            _ => None,
        })
        .collect();
    // Line 3 is prose — it leads with `You`, not a labeled tag, and the
    // bare mid-sentence `must`/`never` carry no colon, so neither fires.
    // Only the two leading labeled markers fire.
    assert_eq!(
        comments,
        vec![
            ("INVARIANT:".to_string(), 1, false),
            ("PANICS:".to_string(), 2, false),
        ],
        "{comments:?}"
    );
}

#[test]
fn stamps_in_test_on_a_comment_inside_a_cfg_test_mod() {
    // An invariant marker inside a `#[cfg(test)]` module carries
    // `in_test` — the line-grain twin of the item-level `test_depth`
    // predicate.
    let facts = extract(
        "pub fn answer() -> u32 { 42 }\n\
         #[cfg(test)]\n\
         mod tests {\n\
         \x20   // INVARIANT: only safe under the test harness.\n\
         \x20   #[test]\n\
         \x20   fn checks() {}\n\
         }\n",
    );
    let in_test = facts.iter().find_map(|f| match f {
        Fact::InvariantComment { in_test, .. } => Some(*in_test),
        _ => None,
    });
    assert_eq!(in_test, Some(true), "{facts:?}");
}

// --- R-060 narrowed predicate: range-axis nesting ---------------------

/// The depth of nested loops carrying the `nested-loops` swept-matrix
/// signal, or `None` when no such fact fired. [`extract`] is the test
/// harness; the helper keeps the assertions below one-liners.
#[cfg(test)]
fn nested_loop_depth(facts: &[Fact]) -> Option<&str> {
    facts.iter().find_map(|f| match f {
        Fact::TestSweep { kind, detail, .. } if kind == "nested-loops" => Some(detail.as_str()),
        _ => None,
    })
}

#[cfg(test)]
fn has_bitmask(facts: &[Fact]) -> bool {
    facts
        .iter()
        .any(|f| matches!(f, Fact::TestSweep { kind, .. } if kind == "bitmask"))
}

/// Three nested RANGE for-loops (generated axes) in a test fire the
/// `nested-loops` signal once, at depth 3 — the Cartesian half of R-060.
#[test]
fn a_three_deep_range_nest_in_a_test_emits_nested_loops() {
    let facts = extract(
        r#"#[test]
fn swept() {
    for a in 0..2 {
        for b in 0..2 {
            for c in 0..2 {
                let _ = c;
            }
        }
    }
}
"#,
    );
    assert_eq!(nested_loop_depth(&facts), Some("3"), "{facts:?}");
}

/// The host `vibe-workspace` shape — three nested `for x in [literal]`
/// loops (declared axes). The narrowing keeps it GREEN: a collection
/// iterable is data someone wrote, so exhausting a closed set by nesting
/// is compliant and no sweep fires.
#[test]
fn a_three_deep_collection_nest_in_a_test_is_silent() {
    let facts = extract(
        r#"#[test]
fn exhausted() {
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                let _ = (a, b, c);
            }
        }
    }
}
"#,
    );
    assert!(
        facts.iter().all(|f| !matches!(f, Fact::TestSweep { .. })),
        "a declared-axis collection nest never fires: {facts:?}"
    );
}

/// The host `progress-core` shape — three nested `for x in CONST_PATH`
/// loops (declared axes). A path iterable is data too, so this stays GREEN.
#[test]
fn a_three_deep_path_iterable_nest_in_a_test_is_silent() {
    let facts = extract(
        r#"const STAGES: [u8; 2] = [0, 1];
#[test]
fn exhausted() {
    for a in STAGES {
        for b in STAGES {
            for c in STAGES {
                let _ = (a, b, c);
            }
        }
    }
}
"#,
    );
    assert!(
        facts.iter().all(|f| !matches!(f, Fact::TestSweep { .. })),
        "a path-iterable collection nest never fires: {facts:?}"
    );
}

/// The refinement's mixed case: count only range axes. A collection-loop
/// wrapper does not count, but three range loops nested under it still
/// reach depth 3 and fire.
#[test]
fn a_range_nest_under_a_collection_loop_still_fires() {
    let facts = extract(
        r#"#[test]
fn mixed() {
    for _t in [0, 1] {
        for a in 0..2 {
            for b in 0..2 {
                for c in 0..2 {
                    let _ = c;
                }
            }
        }
    }
}
"#,
    );
    assert_eq!(nested_loop_depth(&facts), Some("3"), "{facts:?}");
}

/// The `2^n` bit-mask bound is unchanged by the narrowing: a `for mask in
/// 0..(1 << n)` in a test fires the `bitmask` signal (its iterable IS a
/// range, but the signal is the bound, checked for every for-loop).
#[test]
fn a_bitmask_for_loop_in_a_test_emits_bitmask() {
    let facts = extract(
        r#"#[test]
fn swept() {
    for mask in 0..(1 << 3) {
        let _ = mask;
    }
}
"#,
    );
    assert!(has_bitmask(&facts), "{facts:?}");
    // A single 1-deep loop is below the ≥ 3 nesting threshold.
    assert_eq!(nested_loop_depth(&facts), None, "{facts:?}");
}

/// Outside test context no sweep fires, even for a 3-deep range nest —
/// the rule is about tests, not production loops.
#[test]
fn a_range_nest_outside_a_test_is_silent() {
    let facts = extract(
        r#"pub fn swept() {
    for a in 0..2 {
        for b in 0..2 {
            for c in 0..2 {
                let _ = c;
            }
        }
    }
}
"#,
    );
    assert!(
        facts.iter().all(|f| !matches!(f, Fact::TestSweep { .. })),
        "outside test context no sweep fires: {facts:?}"
    );
}

// --- V8-OUTOFLINE-TESTS: out-of-line #[cfg(test)] mod bodies ------------

use super::out_of_line::{mod_dir_of, out_line_test_body_paths};

/// `mod_dir_of` is Rust 2018 module resolution: a crate root (`lib.rs`/
/// `main.rs`) or `mod.rs` resolves in its OWN directory; any other
/// `<stem>.rs` resolves in a sibling directory named after the stem.
#[test]
fn mod_dir_of_follows_rusts_module_rules() {
    assert_eq!(mod_dir_of("crates/x/src/lib.rs"), "crates/x/src");
    assert_eq!(
        mod_dir_of("crates/x/src/fragment/mod.rs"),
        "crates/x/src/fragment"
    );
    assert_eq!(
        mod_dir_of("crates/x/src/fragment.rs"),
        "crates/x/src/fragment"
    );
}

/// A body-less `#[cfg(test)] mod` resolves its body by Rust's rules — both
/// forms at a crate root (the sibling file AND the dir's mod.rs), the
/// submodule's OWN dir when declared there (acceptance #4), a `#[path]`
/// pinning one file outright, and an INLINE mod emitting nothing (the
/// per-file depth logic already scopes it — acceptance #2's regression).
#[test]
fn out_of_line_test_mod_body_paths() {
    let both = out_line_test_body_paths("crates/x/src/lib.rs", "#[cfg(test)]\nmod tests;\n");
    assert_eq!(both.len(), 2);
    assert!(
        both.iter()
            .all(|p| p == "crates/x/src/tests.rs" || p == "crates/x/src/tests/mod.rs")
    );
    let sub = out_line_test_body_paths("crates/x/src/fragment.rs", "#[cfg(test)]\nmod t;\n");
    assert!(sub.contains(&"crates/x/src/fragment/t.rs".to_string()));
    let pinned = out_line_test_body_paths(
        "crates/x/src/lib.rs",
        "#[cfg(test)]\n#[path = \"lib/tests.rs\"]\nmod tests;\n",
    );
    assert_eq!(pinned, vec!["crates/x/src/lib/tests.rs".to_string()]);
    let inline = out_line_test_body_paths("crates/x/src/lib.rs", "#[cfg(test)]\nmod t {\n}\n");
    assert!(inline.is_empty());
}

/// The fix, in memory (acceptance #1 + #3): a `cfg(test)` body's `unwrap`
/// is stamped `in_test` (it reads as domain before — the defect), but a
/// plain `mod foo;` body's stays a finding. The real store path is pinned
/// in `tests/engine.rs`.
#[test]
fn out_of_line_body_stamped_test_production_stays_finding() {
    use conform_core::SourceFacts;
    let body = "fn h() { Some(1).unwrap(); }\n";
    let prod = "pub fn f() { Some(2).unwrap(); }\n";
    let files = vec![
        (
            "crates/x/src/lib.rs".into(),
            "#[cfg(test)]\nmod tests;\n".into(),
        ),
        ("crates/x/src/tests.rs".into(), body.to_string()),
        ("crates/x/src/foo.rs".into(), "mod foo;\n".into()),
        ("crates/x/src/foo/tests.rs".into(), prod.to_string()),
    ];
    let bodies = out_of_line_test_bodies(&files);
    assert!(bodies.contains("crates/x/src/tests.rs"));
    assert!(!bodies.contains("crates/x/src/foo/tests.rs"));
    let mut sfs = vec![
        SourceFacts {
            file: "crates/x/src/tests.rs".into(),
            crate_name: "x".into(),
            facts: RustFrontend.extract("crates/x/src/tests.rs", "x", "x::tests", body),
        },
        SourceFacts {
            file: "crates/x/src/foo/tests.rs".into(),
            crate_name: "x".into(),
            facts: RustFrontend.extract("crates/x/src/foo/tests.rs", "x", "x::foo::tests", prod),
        },
    ];
    assert!(
        sfs[0]
            .facts
            .iter()
            .all(|f| !matches!(f, Fact::UnwrapUse { in_test: true, .. }))
    );
    stamp_test_context(&mut sfs, &bodies);
    let stamped = |i: usize| {
        sfs[i]
            .facts
            .iter()
            .any(|f| matches!(f, Fact::UnwrapUse { in_test: true, .. }))
    };
    assert!(
        stamped(0),
        "cfg(test) body stamped test: {:?}",
        sfs[0].facts
    );
    assert!(
        !stamped(1),
        "production body not silenced: {:?}",
        sfs[1].facts
    );
}

/// Cross-file transitivity: a test body that declares another `#[cfg(test)]`
/// out-of-line mod is scanned too, so the nested body is detected (one
/// declaration per file, no fixpoint). NOT covered (recorded limit): a plain
/// `mod sub;` inside an already-test body carries no `cfg(test)` attr.
#[test]
fn a_test_body_declaring_another_cfg_test_mod_is_detected() {
    let files = vec![
        (
            "crates/x/src/lib.rs".into(),
            "#[cfg(test)]\nmod a;\n".into(),
        ),
        ("crates/x/src/a.rs".into(), "#[cfg(test)]\nmod b;\n".into()),
    ];
    let bodies = out_of_line_test_bodies(&files);
    assert!(bodies.contains("crates/x/src/a.rs"));
    assert!(bodies.contains("crates/x/src/a/b.rs"), "{bodies:?}");
}
