//! The T6a stop-on-failure RED matrix
//! (`R4-TRANSFORM-PLAN-ABI-v0.1.md` §6.2): one typed callback error returns
//! unchanged through every parse genre — simple input, `#use` recursion,
//! `#source` expansion and `#embed` recursion — never conflated with a
//! [`SectionSource`] lookup failure, with deterministic first-failure
//! precedence. The shared fixture lives in the parent [`super`] cell.

use super::*;

/// The origin a normal input must carry for the permutation roots: the
/// package coordinate of its seed, per the plan's identity law.
fn first_origin(address: &str) -> String {
    format!(
        "org.demo/{}",
        address
            .split('/')
            .nth(3)
            .unwrap_or_else(|| panic!("test address has a package name: {address}"))
    )
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn one_sentinel_callback_error_returns_unchanged() {
    let source = standard_source();
    let calls = Calls::default();
    let result = discover(
        &standard_plan(),
        &source,
        |input| {
            calls.record(&input);
            if input.address() == &DocumentAddress::Spec(spec(ALPHA)) {
                Err(Sentinel {
                    root: "alpha",
                    ordinal: 7,
                })
            } else {
                Ok(parse(input))
            }
        },
        // A REAL recorder capturing into the same `calls`: the emptiness
        // assertion below is then load-bearing — a mutation that relabels
        // the callback's `E` as a SectionSource/use failure routes it
        // through this closure and turns this test red.
        |address, reason| {
            calls
                .failures
                .borrow_mut()
                .push((address.to_string(), reason));
        },
    );
    // The exact typed payload crosses untouched.
    assert_eq!(
        result.unwrap_err(),
        Sentinel {
            root: "alpha",
            ordinal: 7,
        }
    );
    // Nothing was relabelled as a use failure: the recorder captured no
    // SectionSource-style failure at all, and no later document was parsed.
    assert!(calls.failures.borrow().is_empty());
    assert_eq!(calls.keys(), [spec(ALPHA).without_pin()]);
}

/// An error type with NO derived traits: discovery must not require Debug,
/// Clone, Display, 'static, Send or Sync beyond what propagation needs.
struct OpaqueError {
    secret: usize,
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn the_callback_error_needs_no_traits() {
    let source = standard_source();
    let result = discover(
        &standard_plan(),
        &source,
        |input| {
            if input.address() == &DocumentAddress::Spec(spec(SHARED)) {
                Err(OpaqueError { secret: 42 })
            } else {
                Ok(parse(input))
            }
        },
        no_failure,
    );
    match result {
        Err(OpaqueError { secret }) => assert_eq!(secret, 42),
        Ok(_) => panic!("the shared parse failed, so discovery cannot succeed"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn simple_input_parse_failure_stops_discovery() {
    let source = standard_source();
    let calls = Calls::default();
    let result = discover(
        &standard_plan(),
        &source,
        |input| {
            calls.record(&input);
            if matches!(input.address(), DocumentAddress::StaticEntry { .. }) {
                Err(Sentinel {
                    root: "simple",
                    ordinal: 3,
                })
            } else {
                Ok(parse(input))
            }
        },
        no_failure,
    );
    assert_eq!(
        result.unwrap_err(),
        Sentinel {
            root: "simple",
            ordinal: 3,
        }
    );
    // The two normal-root documents of the first subtree parsed first; the
    // simple input failed; the embed phase never ran, so the piece was
    // never even read from the source.
    assert_eq!(
        calls.keys(),
        [
            spec(ALPHA).without_pin(),
            spec(SHARED).without_pin(),
            "static entry (origin \"host\", path \"boot/local.md\")".to_string(),
        ]
    );
    assert_eq!(source.load_count(PIECE), 0);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn use_recursion_failure_stops_and_skips_unrelated_roots() {
    let source = standard_source();
    let calls = Calls::default();
    let result = discover(
        &standard_plan(),
        &source,
        |input| {
            calls.record(&input);
            if input.address() == &DocumentAddress::Spec(spec(SHARED)) {
                Err(Sentinel {
                    root: "shared",
                    ordinal: 2,
                })
            } else {
                Ok(parse(input))
            }
        },
        no_failure,
    );
    assert_eq!(
        result.unwrap_err(),
        Sentinel {
            root: "shared",
            ordinal: 2
        }
    );
    // The failure is inside the first root's #use recursion: the later
    // omega root and the simple input were never read from the source.
    assert_eq!(
        calls.keys(),
        [spec(ALPHA).without_pin(), spec(SHARED).without_pin()]
    );
    assert_eq!(source.load_count(OMEGA), 0);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn source_recursion_failure_returns_after_the_expanded_targets() {
    let source = Source::expanding(|_| vec![spec(IMPL_A), spec(IMPL_B)]);
    let plan = ArtifactPlan::new(
        static_context(),
        vec![ArtifactInput::normal("org.demo/holder", "boot/holder.md", spec(SOURCE_DOC)).unwrap()],
    )
    .unwrap();
    let calls = Calls::default();
    let result = discover(
        &plan,
        &source,
        |input| {
            calls.record(&input);
            if input.address() == &DocumentAddress::Spec(spec(IMPL_B)) {
                Err(Sentinel {
                    root: "impl-b",
                    ordinal: 3,
                })
            } else {
                Ok(parse(input))
            }
        },
        no_failure,
    );
    assert_eq!(
        result.unwrap_err(),
        Sentinel {
            root: "impl-b",
            ordinal: 3
        }
    );
    // The holder parsed in the use phase, the pattern expanded to both
    // targets, the first target parsed and the second failed.
    assert_eq!(
        calls.keys(),
        [
            spec(SOURCE_DOC).without_pin(),
            spec(IMPL_A).without_pin(),
            spec(IMPL_B).without_pin(),
        ]
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn embed_recursion_failure_returns_after_earlier_phases() {
    let source = standard_source();
    let calls = Calls::default();
    let result = discover(
        &standard_plan(),
        &source,
        |input| {
            calls.record(&input);
            if input.address() == &DocumentAddress::Spec(spec(PIECE)) {
                Err(Sentinel {
                    root: "piece",
                    ordinal: 5,
                })
            } else {
                Ok(parse(input))
            }
        },
        no_failure,
    );
    assert_eq!(
        result.unwrap_err(),
        Sentinel {
            root: "piece",
            ordinal: 5
        }
    );
    // The embed phase runs last: every use/simple parse happened before the
    // embedded target failed.
    assert_eq!(
        calls.keys(),
        [
            spec(ALPHA).without_pin(),
            spec(SHARED).without_pin(),
            "static entry (origin \"host\", path \"boot/local.md\")".to_string(),
            spec(OMEGA).without_pin(),
            spec(PIECE).without_pin(),
        ]
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn section_source_failure_keeps_its_observation_and_returns_ok() {
    let alpha_uses_missing = "# Alpha {#root}\n#use spec://org.demo/missing/boot/base\nA\n";
    let source = Source::with(&[
        (ALPHA, alpha_uses_missing),
        (OMEGA, "# Omega {#root}\nOMEGA\n"),
    ]);
    let calls = Calls::default();
    let recorded: std::cell::RefCell<Vec<(String, String)>> = std::cell::RefCell::new(Vec::new());
    let worklist = discover(
        &standard_plan(),
        &source,
        |input| {
            calls.record(&input);
            infallible(input)
        },
        |address, reason| {
            recorded.borrow_mut().push((address.to_string(), reason));
        },
    )
    .unwrap();
    // The lookup failure keeps its historical semantics: recorded once with
    // the requested (authored pinless) address, observed as Failed in the
    // snapshot, not parsed, and the worklist is still Ok — the
    // callback-failure channel did not swallow it.
    let missing = "spec://org.demo/missing/boot/base".to_string();
    assert_eq!(
        recorded.borrow().as_slice(),
        [(missing.clone(), format!("missing {missing}"))]
    );
    assert!(matches!(
        worklist.sources.documents.get(&missing),
        Some(DocumentObservation::Failed { .. })
    ));
    assert_eq!(
        calls.keys(),
        [
            spec(ALPHA).without_pin(),
            "static entry (origin \"host\", path \"boot/local.md\")".to_string(),
            spec(OMEGA).without_pin(),
        ]
    );
    // The failing target still owns its error attribution.
    assert_eq!(worklist.owners.owner(&missing), Some(0));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn first_failure_precedence_follows_plan_order_deterministically() {
    let source = standard_source();
    let two_roots = |first: &str, second: &str| {
        ArtifactPlan::new(
            static_context(),
            vec![
                ArtifactInput::normal(first_origin(first), "boot/first.md", spec(first)).unwrap(),
                ArtifactInput::normal(first_origin(second), "boot/second.md", spec(second))
                    .unwrap(),
            ],
        )
        .unwrap()
    };
    let failing = |calls: &Calls, input: SourceIr| {
        calls.record(&input);
        let DocumentAddress::Spec(address) = input.address() else {
            return Ok(parse(input));
        };
        let root = if *address == spec(ALPHA) {
            "alpha"
        } else if *address == spec(OMEGA) {
            "omega"
        } else {
            "other"
        };
        Err(Sentinel {
            root,
            ordinal: calls.addresses.borrow().len(),
        })
    };
    let calls = Calls::default();
    let first = discover(
        &two_roots(ALPHA, OMEGA),
        &source,
        |input| failing(&calls, input),
        no_failure,
    )
    .unwrap_err()
    .root;
    assert_eq!(first, "alpha");
    let calls = Calls::default();
    let second = discover(
        &two_roots(OMEGA, ALPHA),
        &source,
        |input| failing(&calls, input),
        no_failure,
    )
    .unwrap_err()
    .root;
    assert_eq!(second, "omega");
    // Fixed order, repeated runs: the failing call ordinal never moves.
    for _ in 0..3 {
        let calls = Calls::default();
        let error = discover(
            &two_roots(ALPHA, OMEGA),
            &source,
            |input| failing(&calls, input),
            no_failure,
        )
        .unwrap_err();
        assert_eq!(error.ordinal, 1);
    }
}
