//! `prepare` returns bands, never a request — and never guesses a gate.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-CONTINUATION-CACHEABLE",
    r = 1
);

use std::fs;

use vibe_core::layout;
use vibe_core::manifest::TargetOs;

use crate::system_prompt::{check_node_prompt, prepare_request};

use super::super::check::tests::Node;

#[test]
fn the_bands_come_back_separately_with_their_own_observed_digests() {
    let node = Node::published(true);
    let request = node.request();
    let answer = check_node_prompt(&request).unwrap();
    let prepared = prepare_request(&request, &answer, None).unwrap();
    assert_eq!(prepared.prefix.path, layout::SYSTEM_MD);
    assert_eq!(prepared.prefix.len, answer.system.len.unwrap());
    assert_eq!(
        prepared.prefix.sha256,
        answer.system.sha256.clone().unwrap()
    );
    // Band U is the plan's own unconditional rows, each observed now.
    assert_eq!(prepared.unconditional.len(), 1);
    assert!(prepared.unconditional[0].path.ends_with("90-user.xml"));
    assert!(prepared.gated.is_empty());
    assert!(prepared.unresolved.is_empty());
    // The breakpoints are offsets, and the last one ends band U — which is
    // the whole reason the bands are not concatenated.
    assert_eq!(
        prepared.cache_boundaries.last().copied(),
        Some(prepared.prefix.len + prepared.unconditional[0].len)
    );
}

#[test]
fn a_band_u_edit_changes_only_the_observed_digest_and_not_the_prefix() {
    let node = Node::published(true);
    let request = node.request();
    let before = prepare_request(&request, &check_node_prompt(&request).unwrap(), None).unwrap();
    fs::write(
        node.boot().join("90-user.xml"),
        "# a much longer user snippet than before\n",
    )
    .unwrap();
    let after = prepare_request(&request, &check_node_prompt(&request).unwrap(), None).unwrap();
    assert_eq!(before.prefix, after.prefix);
    assert_ne!(
        before.unconditional[0].sha256,
        after.unconditional[0].sha256
    );
}

#[test]
fn a_stale_prefix_is_refused_with_the_build_that_fixes_it_and_never_rebuilt() {
    let node = Node::published(true);
    fs::write(node.boot().join("SYSTEM-TEMPLATE.md"), "# other\n").unwrap();
    let request = node.request();
    let answer = check_node_prompt(&request).unwrap();
    let error = prepare_request(&request, &answer, None).unwrap_err();
    let text = error.to_string();
    assert!(text.contains("vibe prompt build"), "{text}");
    // Refused, not repaired: the artifact is still the stale one.
    assert_eq!(
        check_node_prompt(&request).unwrap().system.sha256,
        answer.system.sha256
    );
}

#[test]
fn a_node_that_never_opted_in_has_nothing_to_prepare() {
    let node = Node::published(false);
    let request = node.request();
    let answer = check_node_prompt(&request).unwrap();
    let text = prepare_request(&request, &answer, None)
        .unwrap_err()
        .to_string();
    assert!(text.contains("has not opted in"), "{text}");
}

#[test]
fn a_gated_row_with_no_supplied_target_is_unresolved_and_never_the_host() {
    let node = Node::published(true);
    // Rewrite the sidecar's one row as gated: the generator's own operating
    // system must never decide it, whichever machine runs the test.
    let path = node.boot().join(layout::SYSTEM_META_JSON);
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace(
            "\"band\": \"unconditional\"",
            "\"band\": \"gated\",\n      \"when\": \"os:windows\"",
        ),
    )
    .unwrap();
    let request = node.request();
    let answer = check_node_prompt(&request).unwrap();

    let unresolved = prepare_request(&request, &answer, None).unwrap();
    assert_eq!(unresolved.unresolved.len(), 1);
    assert_eq!(unresolved.unresolved[0].when, "os:windows");
    assert!(unresolved.gated.is_empty());
    assert!(
        unresolved.unresolved[0].reason.contains("--session-target"),
        "{:?}",
        unresolved.unresolved[0]
    );

    // An explicitly supplied target selects it; another one drops it.
    let selected = prepare_request(&request, &answer, Some(TargetOs::Windows)).unwrap();
    assert_eq!(selected.gated.len(), 1);
    assert!(selected.unresolved.is_empty());
    let dropped = prepare_request(&request, &answer, Some(TargetOs::Linux)).unwrap();
    assert!(dropped.gated.is_empty() && dropped.unresolved.is_empty());
}

/// A host mismatch is one of `prepare`'s observations, never a refusal: the
/// bands come back whole beside it (`#SYS-HOST-AND-TARGET`). The session OS
/// is stated rather than detected, so the three legs mean the same on every
/// machine.
#[test]
fn a_host_mismatch_is_an_observation_and_the_bands_come_back_whole() {
    let fixed = Node::published_declaring(true, "[system_prompt]\nhost = \"macos\"\n");
    let request = fixed.request_from(Some(TargetOs::Linux));
    let answer = check_node_prompt(&request).unwrap();
    let prepared = prepare_request(&request, &answer, None).unwrap();
    assert_eq!(prepared.warnings.len(), 1, "{:?}", prepared.warnings);
    assert!(
        prepared.warnings[0].contains("declares `macos`")
            && prepared.warnings[0].contains("runs on `linux`"),
        "{:?}",
        prepared.warnings
    );
    // The materials are complete: a warning changes no band.
    let silent =
        prepare_request(&fixed.request_from(Some(TargetOs::Macos)), &answer, None).unwrap();
    assert!(silent.warnings.is_empty(), "{:?}", silent.warnings);
    assert_eq!(prepared.prefix, silent.prefix);
    assert_eq!(prepared.unconditional, silent.unconditional);

    // And a portable node is silent whatever the session is.
    let portable = Node::published(true);
    let request = portable.request_from(Some(TargetOs::Windows));
    let answer = check_node_prompt(&request).unwrap();
    assert!(
        prepare_request(&request, &answer, None)
            .unwrap()
            .warnings
            .is_empty()
    );
}
