//! The trust mechanism's own laws: what an entry may be, what a subtree
//! covers, and the one rule that keeps the answer order-independent.

use specmark::verifies;

use super::*;

/// The shipped list is data, and `base()` builds its patterns by hand to
/// stay infallible — so the hand-built pattern must be exactly what
/// parsing the same entry produces, or the base list could mean something
/// the operator cannot write themselves.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
fn every_shipped_base_entry_is_an_entry_an_operator_could_have_written() {
    let base = TrustPolicy::base();
    assert_eq!(base.trusted().len(), BASE_TRUSTED_GROUPS.len());
    for (entry, shipped) in BASE_TRUSTED_GROUPS.iter().zip(base.trusted()) {
        assert_eq!(
            GroupPattern::parse(entry).unwrap(),
            *shipped.pattern(),
            "the shipped `{entry}` must parse to the pattern `base()` builds",
        );
        assert_eq!(shipped.layer(), TrustLayer::Base);
    }
    assert!(base.untrusted().is_empty(), "vibe ships no deny entry");
}

/// The owner's decision of 2026-11-10 in one assertion: everything under
/// `org.vibevm.*` is trusted out of the box, the prefix group included,
/// and a neighbouring organisation whose name merely starts the same way
/// is not.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
fn the_shipped_base_trusts_the_whole_vibevm_subtree_and_nothing_beside_it() {
    let policy = TrustPolicy::effective(&TrustLists::default(), &TrustLists::default()).unwrap();
    for trusted in [
        "org.vibevm",
        "org.vibevm.ai-native",
        "org.vibevm.node",
        "org.vibevm.world",
        "org.vibevm.a.deeply.nested.one",
    ] {
        assert!(policy.trusts(trusted), "{trusted} must be trusted by base");
    }
    for untrusted in [
        "org.vibevmx",
        "org.vibev",
        "org.example",
        "com.vibevm",
        "org",
    ] {
        assert!(
            !policy.trusts(untrusted),
            "{untrusted} must not be trusted by base",
        );
    }
}

/// Deny wins — over the base list, over the site's own trusted list and
/// over the user's own. That is the rule that makes the verdict
/// independent of the order the layers are read in, so it is asserted from
/// all three directions.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
fn a_deny_entry_wins_over_every_trusted_entry_of_every_layer() {
    // The owner's worked example: one subtree of the base refused, the
    // rest of it kept.
    let user = TrustLists::new(&[], &["org.vibevm.ai-native".to_string()]);
    let policy = TrustPolicy::effective(&TrustLists::default(), &user).unwrap();
    assert!(policy.trusts("org.vibevm.node"));
    assert!(!policy.trusts("org.vibevm.ai-native"));
    assert!(
        policy.trusts("org.vibevm.ai-native.extra"),
        "an exact deny entry denies exactly that group — the entry syntax is the \
         same in both lists, so denying a subtree is written `org.vibevm.ai-native.*`",
    );
    let subtree = TrustLists::new(&[], &["org.vibevm.ai-native.*".to_string()]);
    let policy = TrustPolicy::effective(&TrustLists::default(), &subtree).unwrap();
    assert!(!policy.trusts("org.vibevm.ai-native"));
    assert!(!policy.trusts("org.vibevm.ai-native.extra"));
    assert!(policy.trusts("org.vibevm.node"));

    // The whole base refused: the opt-out, and the only one there is.
    let opted_out = TrustLists::new(&[], &["org.vibevm.*".to_string()]);
    let policy = TrustPolicy::effective(&TrustLists::default(), &opted_out).unwrap();
    for group in ["org.vibevm", "org.vibevm.ai-native", "org.vibevm.node"] {
        assert!(!policy.trusts(group), "{group}");
    }

    // A group the user trusts explicitly and then denies: the deny still
    // wins, which is why the answer does not depend on the order.
    let contradicting = TrustLists::new(&["org.example".to_string()], &["org.example".to_string()]);
    let policy = TrustPolicy::effective(&TrustLists::default(), &contradicting).unwrap();
    assert!(!policy.trusts("org.example"));

    // And the site's deny covers the user's own trusted entry.
    let site = TrustLists::new(&[], &["org.example.*".to_string()]);
    let user = TrustLists::new(&["org.example.tools".to_string()], &[]);
    let policy = TrustPolicy::effective(&site, &user).unwrap();
    assert!(!policy.trusts("org.example.tools"));
}

/// The verdict names the entry that decided it, and a deny entry says
/// which layers' trust it overrides — the clarity a listing prints.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
fn the_verdict_names_the_deciding_entry_and_what_a_deny_overrides() {
    let site = TrustLists::new(&["org.corp.*".to_string()], &[]);
    let user = TrustLists::new(
        &["org.example".to_string()],
        &[
            "org.vibevm.ai-native".to_string(),
            "org.corp.legacy".to_string(),
        ],
    );
    let policy = TrustPolicy::effective(&site, &user).unwrap();

    match policy.verdict("org.corp.tools") {
        TrustVerdict::Trusted { by } => {
            assert_eq!((by.written(), by.layer()), ("org.corp.*", TrustLayer::Site));
        }
        other => panic!("{other:?}"),
    }
    match policy.verdict("org.vibevm.ai-native") {
        TrustVerdict::Untrusted { by } => {
            assert_eq!(
                (by.written(), by.layer()),
                ("org.vibevm.ai-native", TrustLayer::User),
            );
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        policy.verdict("org.nobody.mentioned"),
        TrustVerdict::Unlisted,
    ));

    let overrides: Vec<Vec<TrustLayer>> = policy
        .untrusted()
        .iter()
        .map(|deny| policy.overridden_by(deny))
        .collect();
    assert_eq!(
        overrides,
        [vec![TrustLayer::Base], vec![TrustLayer::Site]],
        "each deny entry names the layer whose trust it takes away",
    );
}

/// Every refused spelling, each with the entry in its message. A silently
/// ignored entry would either grant trust the operator did not write or
/// withhold the trust they did.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
fn an_entry_that_is_not_a_group_or_a_subtree_is_refused_by_name() {
    for (entry, expected) in [
        ("", "it is empty"),
        ("   ", "it is empty"),
        ("*", "it trusts every group there is"),
        (".*", "it trusts every group there is"),
        ("**", "`**` is not a pattern"),
        ("org.**", "`**` is not a pattern"),
        (
            "org.*.tools",
            "a wildcard is only ever the whole last label",
        ),
        ("org.vibe*", "a wildcard is only ever the whole last label"),
        ("*.vibevm", "a wildcard is only ever the whole last label"),
        ("Org.Vibevm", "is not a group"),
        ("org..vibevm", "is not a group"),
        ("org.vibevm.", "is not a group"),
    ] {
        let error = GroupPattern::parse(entry).unwrap_err().to_string();
        assert!(
            error.contains(expected),
            "`{entry}` must be refused with `{expected}`: {error}",
        );
        assert!(
            error.contains("PROP-065#ordinary-trust") && error.contains("fix:"),
            "`{entry}`: every refusal cites its law and a fix: {error}",
        );
    }
    // And a bad entry in either list of either layer refuses the whole
    // resolution rather than being dropped.
    let bad = TrustLists::new(&["org.*.tools".to_string()], &[]);
    assert!(TrustPolicy::effective(&TrustLists::default(), &bad).is_err());
    let bad = TrustLists::new(&[], &["**".to_string()]);
    assert!(TrustPolicy::effective(&bad, &TrustLists::default()).is_err());
}

/// The two doors a caller that supplies its own trust comes through: an
/// empty policy trusts nothing, and a run-granted group is an exact entry.
#[test]
fn an_empty_policy_trusts_nothing_and_a_run_grant_is_exact() {
    assert!(!TrustPolicy::EMPTY.trusts("org.vibevm"));
    assert_eq!(TrustPolicy::default(), TrustPolicy::EMPTY);

    let mut policy = TrustPolicy::EMPTY;
    policy.allow_for_this_run(&Group::parse("org.example").unwrap());
    assert!(policy.trusts("org.example"));
    assert!(
        !policy.trusts("org.example.sub"),
        "a run grant is the group the operator was asked about, not its subtree",
    );
}
