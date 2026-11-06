//! Shared policy uses real normalization; no domain wire or persistence owner.
#![cfg(test)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS");
use super::*;
fn input(text: &str) -> ConfigurationIntentDecl {
    toml::from_str(text).unwrap()
}
fn definition() -> ConfigurationDecl {
    toml::from_str("[[features]]\nid='a'\nname='A'\n[[groups]]\nmode='choice'\nid='g'\nname='Group'\ncardinality='zero-or-more'\nmembers=['a']\n").unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
fn absent_defaults_and_removed_choice_keep_reliance_until_review_or_explicit_takeover() {
    let mut d = definition();
    let original = input("policy='use-declared-defaults'\ndefault_features=false");
    let normalized = d.normalize_input(&original, "left").unwrap();
    let prior = consumer_reliance(
        &d,
        ConsumerInput {
            original: &original,
            normalized: &normalized,
        },
        &BTreeSet::new(),
        Authority::Reuse,
    );
    assert_eq!(
        prior,
        BTreeSet::from([ConsumerReliance::Choice("g".into())])
    );
    d.groups.clear();
    for authority in [
        Authority::Reuse,
        Authority::Reviewed,
        Authority::Replacement,
    ] {
        let normalized = d.normalize_input(&original, "left").unwrap();
        let actual = consumer_reliance(
            &d,
            ConsumerInput {
                original: &original,
                normalized: &normalized,
            },
            &prior,
            authority,
        );
        assert_eq!(actual.is_empty(), authority != Authority::Reuse);
    }
    let d = definition();
    let explicit = input(
        "policy='use-declared-defaults'\ndefault_features=false\nselections.g={intent='exact',members=[]}",
    );
    let normalized = d.normalize_input(&explicit, "left").unwrap();
    assert!(
        consumer_reliance(
            &d,
            ConsumerInput {
                original: &explicit,
                normalized: &normalized
            },
            &prior,
            Authority::Reuse
        )
        .is_empty()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
fn profile_and_consumer_reliance_do_not_become_merged_closure_ownership() {
    let d: ConfigurationDecl = toml::from_str(
        "[[features]]\nid='a'\nname='A'\n[[profiles]]\nid='p'\nname='Profile'\nfeatures=['a']",
    )
    .unwrap();
    let a = input("policy='require-selection'\nprofile='p'");
    let b = input("policy='require-selection'\ndefault_features=false");
    let an = d.normalize_input(&a, "left").unwrap();
    let bn = d.normalize_input(&b, "right").unwrap();
    let left = ConsumerInput {
        original: &a,
        normalized: &an,
    };
    let right = ConsumerInput {
        original: &b,
        normalized: &bn,
    };
    assert_eq!(
        consumer_reliance(&d, left, &BTreeSet::new(), Authority::Reuse),
        BTreeSet::from([ConsumerReliance::Profile])
    );
    assert!(consumer_reliance(&d, right, &BTreeSet::new(), Authority::Reuse).is_empty());
    assert!(
        instance_reliance(&d, &[left, right], None, &BTreeSet::new(), Authority::Reuse).is_empty()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
fn merged_instance_retains_dormant_closure_until_a_real_consumer_explicitly_takes_it_over() {
    let d: ConfigurationDecl = toml::from_str("[[features]]\nid='a'\nname='A'\n[[groups]]\nmode='closure'\nid='g'\nname='Group'\ncardinality='zero-or-one'\nmembers=['a']").unwrap();
    let dormant = input("policy='require-selection'\ndefault_features=false");
    let explicit = input("policy='require-selection'\ndefault_features=false\nfeatures=['a']");
    let dn = d.normalize_input(&dormant, "left").unwrap();
    let en = d.normalize_input(&explicit, "right").unwrap();
    let left = ConsumerInput {
        original: &dormant,
        normalized: &dn,
    };
    let right = ConsumerInput {
        original: &explicit,
        normalized: &en,
    };
    let prior = BTreeSet::from([InstanceReliance::Closure("g".into())]);
    assert_eq!(
        instance_reliance(&d, &[left], None, &prior, Authority::Reuse),
        prior
    );
    assert!(instance_reliance(&d, &[left, right], None, &prior, Authority::Reuse).is_empty());
    assert!(consumer_reliance(&d, left, &BTreeSet::new(), Authority::Reuse).is_empty());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
fn fresh_reuse_review_and_replacement_preserve_the_two_independent_review_causes() {
    let old = BTreeSet::from(["domain:parameter"]);
    let both = BTreeSet::from(["domain:parameter", "feature-defaults"]);
    let empty = BTreeSet::new();
    assert_eq!(
        review(
            Authority::Reuse,
            None,
            Basis {
                fingerprint: "b",
                reliance: &both
            }
        ),
        Review::NotRequired
    );
    assert_eq!(
        review(
            Authority::Reuse,
            Some(Basis {
                fingerprint: "a",
                reliance: &old
            }),
            Basis {
                fingerprint: "b",
                reliance: &both
            }
        ),
        Review::Required(BTreeSet::from([
            ReviewCause::ChangedMeaning,
            ReviewCause::AdditionalReliance
        ]))
    );
    assert_eq!(
        review(
            Authority::Reuse,
            Some(Basis {
                fingerprint: "a",
                reliance: &old
            }),
            Basis {
                fingerprint: "b",
                reliance: &empty
            }
        ),
        Review::NotRequired
    );
    for authority in [Authority::Reviewed, Authority::Replacement] {
        assert_eq!(
            review(
                authority,
                Some(Basis {
                    fingerprint: "a",
                    reliance: &old
                }),
                Basis {
                    fingerprint: "b",
                    reliance: &both
                }
            ),
            Review::NotRequired
        );
    }
}
