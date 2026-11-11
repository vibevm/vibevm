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

fn offered(groups: &[&str], trust: &TrustPolicy) -> Lookup {
    Lookup {
        candidates: Lookup::sorted(
            groups
                .iter()
                .map(|group| Candidate::offered(g(group), trust))
                .collect(),
        ),
        consulted: vec!["corp".into()],
        skipped: Vec::new(),
        offline: false,
        local: false,
    }
}

#[test]
fn one_eligible_candidate_resolves() {
    let lookup = offered(&["org.vibevm.node"], &base());
    assert_eq!(decide(&lookup), Outcome::Resolved(g("org.vibevm.node")));
}

#[test]
fn the_provenance_line_is_the_typed_name_and_the_qualified_one() {
    assert_eq!(
        provenance("node", &g("org.vibevm.node")),
        "node → org.vibevm.node/node"
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
    assert!(message.contains("  1. com.acme/node"), "{message}");
    assert!(message.contains("  2. org.vibevm.node/node"), "{message}");
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
    assert_eq!(decide(&lookup), Outcome::Resolved(g("org.vibevm.node")));
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
    assert_eq!(decide(&lookup), Outcome::Resolved(g("com.acme")));
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
        consulted: vec!["corp".into()],
        skipped: vec!["github".into(), "gitverse".into()],
        offline: false,
        local: false,
    };
    let message = render_not_found("vibe app describe", "node", &lookup);
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
