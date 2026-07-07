//! End-to-end tests for the `mcp` package kind (PROP-027; VIBEVM-SPEC
//! §4.1): the kind installs like any other, its slot carries the
//! `mcp-` prefix, and — the load-bearing law — its exact `=X.Y.Z` pin
//! selects EXACTLY the pinned version of the package it serves, even
//! when the registry offers a newer one. The fixture pair
//! `org.vibevm/pin-server` (kind = mcp, pins `=0.1.0`) and
//! `org.vibevm/pin-stack` (v0.1.0 AND v0.2.0 published) exists for
//! exactly this test.

mod common;

use std::fs;

use common::{fixture_registry, init_project, vibe};
use specmark::verifies;

#[test]
#[verifies("spec://vibevm/modules/vibe-mcp/PROP-027#kind")]
#[verifies("spec://vibevm/modules/vibe-mcp/PROP-027#exact-pin")]
fn mcp_kind_installs_and_its_exact_pin_selects_the_pinned_stack() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("mcp:org.vibevm/pin-server")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--assume-yes")
        .assert()
        .success();

    // The mcp slot materialises under the kind-prefixed dir, tree verbatim
    // (the [[mcp_server]]-referenced binary crate included).
    let mcp_slot = project.path().join("vibedeps/mcp-pin-server/0.1.0");
    for rel in ["vibe.toml", "crates/pin-server-mcp/src/main.rs"] {
        assert!(
            mcp_slot.join(rel).is_file(),
            "expected `vibedeps/mcp-pin-server/0.1.0/{rel}` after install"
        );
    }

    // The exact pin resolved pin-stack at 0.1.0 — NOT the newer 0.2.0 the
    // fixture registry deliberately offers (PROP-027 §2.3: one version
    // set, held by the resolver).
    assert!(
        project
            .path()
            .join("vibedeps/stack-pin-stack/0.1.0/vibe.toml")
            .is_file(),
        "the pinned stack version must be materialised"
    );
    assert!(
        !project
            .path()
            .join("vibedeps/stack-pin-stack/0.2.0")
            .exists(),
        "the newer stack version must NOT be selected over the exact pin"
    );

    // The lockfile records the mcp kind.
    let lock = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    assert!(lock.contains("kind = \"mcp\""), "{lock}");
    assert!(lock.contains("name = \"pin-server\""), "{lock}");

    // The check gate accepts the resulting project.
    vibe()
        .arg("check")
        .arg("--path")
        .arg(project.path())
        .arg("--quiet")
        .assert()
        .success();
}
