//! The pure half of application short-name resolution: the three-way
//! verdict, the trust narrowing, the textual splice and the two refusals.
//! Nothing here opens a registry — the decision is tested over values.

use super::*;

fn g(text: &str) -> Group {
    Group::parse(text).unwrap()
}

/// The trust every machine ships with: `org.vibevm.*` and nothing else.
fn base() -> TrustPolicy {
    TrustPolicy::base()
}

/// The group a lookup resolves to, as a string — the verdict's own
/// shape carries the whole candidate now, and these cases only ever
/// cared which group won.
fn resolved(lookup: &Lookup) -> String {
    match decide(lookup) {
        Outcome::Resolved(candidate) => candidate.group.as_str().to_owned(),
        other => panic!("expected a resolution, got {other:?}"),
    }
}

/// The candidate the registry tier now produces: a spelling registry
/// `corp` ISSUED for `<group>/<name>` (PROP-008 §2.6).
fn issued_by_corp(group: &str, name: &str) -> AliasCandidate {
    AliasCandidate {
        target: vibe_registry::short_name::AliasTarget {
            group: g(group),
            name: name.into(),
        },
        registries: vec!["corp".into()],
    }
}

/// A lookup whose registry candidates are the spellings `corp` issued —
/// one per group, each pointing at a package of the bare name, which is
/// the `node → org.vibevm.node/node` shape of the live tables.
fn offered(groups: &[&str], trust: &TrustPolicy) -> Lookup {
    Lookup {
        candidates: Lookup::sorted(
            groups
                .iter()
                .map(|group| Candidate::aliased(issued_by_corp(group, "node"), trust))
                .collect(),
        ),
        named: Vec::new(),
        consulted: vec!["corp".into()],
        skipped: Vec::new(),
        offline: false,
        local: false,
    }
}

#[test]
fn one_eligible_candidate_resolves() {
    let lookup = offered(&["org.vibevm.node"], &base());
    assert_eq!(resolved(&lookup), "org.vibevm.node");
}

/// An issued spelling equal to the package's own name is the primary
/// case since the owner's 2026-11-12 ruling, and it resolves — the rule
/// that used to refuse such a row is gone.
#[test]
fn an_issued_name_equal_to_the_package_name_resolves() {
    let lookup = offered(&["org.vibevm.node"], &base());
    let Outcome::Resolved(candidate) = decide(&lookup) else {
        panic!("an issued spelling must resolve");
    };
    assert_eq!(candidate.package_name("node"), "node");
    assert_eq!(
        provenance("node", &candidate),
        "node → org.vibevm.node/node (alias in registry corp)"
    );
}

#[test]
fn the_provenance_line_names_the_registry_that_issued_the_spelling() {
    assert_eq!(
        provenance(
            "node",
            &Candidate::aliased(issued_by_corp("org.vibevm.node", "node"), &base())
        ),
        "node → org.vibevm.node/node (alias in registry corp)"
    );
}

#[test]
fn two_eligible_candidates_are_the_collision() {
    let trust = TrustPolicy::effective(
        &vibe_core::trust::TrustLists::new(&["com.acme".to_string()], &[]),
        &vibe_core::trust::TrustLists::default(),
    )
    .unwrap();
    let lookup = offered(&["org.vibevm.node", "com.acme"], &trust);
    assert_eq!(decide(&lookup), Outcome::Ambiguous);
    let message = render_collision("vibe install -g", "node", &lookup);
    assert!(
        message.contains("`node` is ambiguous — 2 packages match"),
        "{message}"
    );
    assert!(
        message.contains("  1. com.acme/node (alias in registry corp)"),
        "{message}"
    );
    assert!(
        message.contains("  2. org.vibevm.node/node (alias in registry corp)"),
        "{message}"
    );
    assert!(
        message.contains("`vibe install -g com.acme/node`"),
        "the re-run hint names the command the operator typed: {message}"
    );
}

#[test]
fn an_untrusted_group_is_never_the_one_candidate() {
    // `com.acme` is in no trusted list, so it is a candidate that exists
    // and is never picked for an operator who did not name it.
    let lookup = offered(&["com.acme"], &base());
    assert_eq!(decide(&lookup), Outcome::NotFound);
    let message = render_not_found("vibe install -g", "node", &lookup);
    assert!(
        message.contains("never picked for you, their group is untrusted: com.acme/node"),
        "{message}"
    );
    assert!(
        message.contains("trusted_groups"),
        "the remedy names the list to edit: {message}"
    );
}

#[test]
fn an_untrusted_group_cannot_turn_a_single_trusted_match_into_a_collision() {
    // Two candidates, one of them untrusted: exactly one is eligible, so
    // the answer is that one — and the untrusted sibling never silently
    // disappears from the record, it simply is not picked.
    let lookup = offered(&["org.vibevm.node", "com.acme"], &base());
    assert_eq!(resolved(&lookup), "org.vibevm.node");
    assert_eq!(lookup.candidates.len(), 2);
}

#[test]
fn untrusted_takes_the_base_trust_back() {
    let user = vibe_core::trust::TrustLists::new(&[], &["org.vibevm.*".to_string()]);
    let trust = TrustPolicy::effective(&vibe_core::trust::TrustLists::default(), &user).unwrap();
    let lookup = offered(&["org.vibevm.node"], &trust);
    assert_eq!(decide(&lookup), Outcome::NotFound);
}

#[test]
fn an_installed_candidate_is_eligible_whatever_the_trust_lists_say() {
    // The operator installed it; a verb acting on installed state must be
    // able to name it.
    let lookup = Lookup::of_installed(vec![g("com.acme")]);
    assert_eq!(resolved(&lookup), "com.acme");
}

/// A verb acting on installed state refuses locally and never reaches for
/// a registry it did not ask — the same law `qualify_locked` obeys for
/// `vibe uninstall`.
#[test]
fn an_installed_state_refusal_stays_local() {
    let lookup = Lookup::of_installed(Vec::new());
    assert_eq!(decide(&lookup), Outcome::NotFound);
    let message = render_not_found("vibe app hold", "node", &lookup);
    assert!(message.contains("`node` is not installed"), "{message}");
    assert!(message.contains("application index"), "{message}");
    assert!(
        message.contains("`vibe app hold <group>/node`"),
        "{message}"
    );
    assert!(
        !message.to_lowercase().contains("registry") && !message.contains("consulted"),
        "it must not reach for a registry: {message}"
    );
}

#[test]
fn a_not_found_names_the_consulted_and_the_excluded() {
    let lookup = Lookup {
        candidates: Vec::new(),
        named: Vec::new(),
        consulted: vec!["corp".into()],
        skipped: vec!["github".into(), "gitverse".into()],
        offline: false,
        local: false,
    };
    let message = render_not_found("vibe app describe", "node", &lookup);
    assert!(
        message.contains("the short name `node` is not issued by any consulted registry"),
        "{message}"
    );
    assert!(message.contains("consulted: corp"), "{message}");
    assert!(
        message.contains("excluded by the effective source policy: github, gitverse"),
        "{message}"
    );
    assert!(
        message.contains("`vibe app describe <group>/node`"),
        "{message}"
    );
    assert!(
        !message.contains("--offline"),
        "an online run says nothing about the cache: {message}"
    );
}

#[test]
fn an_offline_not_found_says_the_cached_index_is_the_whole_source() {
    let lookup = Lookup {
        offline: true,
        ..Lookup::default()
    };
    let message = render_not_found("vibe install -g", "node", &lookup);
    assert!(
        message.contains("`--offline` reads the cached index alone"),
        "{message}"
    );
    assert!(
        message.contains("consulted: (no registry answered)"),
        "{message}"
    );
    assert!(
        message.contains("Give the qualified form instead"),
        "{message}"
    );
}

/// The inverted law. A package BEARING the spelling is no candidate at
/// any rank: the verdict is a refusal, and the package is named only as
/// something to copy.
#[test]
fn a_published_name_no_registry_issued_is_refused_with_a_hint() {
    let lookup = Lookup {
        candidates: Vec::new(),
        named: vec![g("org.vibevm.opencode")],
        consulted: vec!["vibespecs".into()],
        skipped: Vec::new(),
        offline: false,
        local: false,
    };
    assert_eq!(decide(&lookup), Outcome::NotFound);
    let message = render_not_found("vibe install -g", "opencode", &lookup);
    assert!(
        message.contains("the short name `opencode` is not issued by any consulted registry"),
        "{message}"
    );
    assert!(
        message.contains("none of them chosen for you"),
        "the hint must not read as a choice: {message}"
    );
    assert!(
        message.contains("org.vibevm.opencode/opencode"),
        "the qualified name must be copyable: {message}"
    );
    assert!(
        message.contains("`vibe install -g org.vibevm.opencode/opencode`"),
        "{message}"
    );
}

/// And an issued spelling is not weakened by a package that bears it:
/// the two lists coexist, the candidate resolves, the hint is unused.
#[test]
fn a_published_name_beside_an_issued_one_does_not_displace_it() {
    let lookup = Lookup {
        named: vec![g("com.acme")],
        ..offered(&["org.vibevm.node"], &base())
    };
    assert_eq!(resolved(&lookup), "org.vibevm.node");
}

#[test]
fn splicing_keeps_the_requirement_byte_for_byte() {
    let group = g("org.vibevm.node");
    assert_eq!(splice("node", &group), "org.vibevm.node/node");
    // `22` must not come back as `^22`.
    assert_eq!(splice("node@22", &group), "org.vibevm.node/node@22");
    assert_eq!(
        splice("node@>=20, <23", &group),
        "org.vibevm.node/node@>=20, <23"
    );
    assert_eq!(
        splice("node@lts/jod", &group),
        "org.vibevm.node/node@lts/jod"
    );
    assert_eq!(splice("app:node@22", &group), "app:org.vibevm.node/node@22");
}

#[test]
fn a_qualified_spelling_is_not_a_short_name() {
    assert_eq!(bare_name("org.vibevm.node/node"), None);
    assert_eq!(bare_name("app:org.vibevm.node/node@22"), None);
    assert_eq!(bare_name("node").as_deref(), Some("node"));
    assert_eq!(bare_name("node@22").as_deref(), Some("node"));
    assert_eq!(bare_name("app:node").as_deref(), Some("node"));
    // Not a pkgref at all: the command keeps its own error.
    assert_eq!(bare_name("not a pkgref"), None);
}

#[test]
fn candidates_are_sorted_and_deduplicated_by_group() {
    let trust = base();
    let lookup = offered(&["org.vibevm.node", "com.acme", "org.vibevm.node"], &trust);
    assert_eq!(
        lookup
            .candidates
            .iter()
            .map(|candidate| candidate.group.as_str())
            .collect::<Vec<_>>(),
        ["com.acme", "org.vibevm.node"]
    );
}

/// A home with nothing installed offers no candidate of its own, which is
/// what makes a registry-backed lookup fall through to the index.
#[test]
fn an_empty_home_contributes_no_installed_candidate() {
    assert!(installed_groups(&Index::default(), "node").is_empty());
}
