//! The reviewer's pin for the one guard no solver-written world can exercise:
//! a lane owner never sits in its own installed vector, even when a
//! hand-edited lock carries a dependency cycle.

use tempfile::TempDir;

use specmark::verifies;

use super::DurableExtensionWorld;
use super::collect_owner_view;
use super::test_support::{found, id, lock, locked, node, slot};

/// One coordinate cannot occupy both seats of its own lane — pinned through
/// the one world shape that can reach the guard.
///
/// P is never reachable from its own edges in a solver-written lock, so the
/// exclusion in `package_owner_view` fires only when a hand-edited or
/// corrupted lock carries a dependency cycle — a shape the closure walk
/// deliberately survives (the reached-set makes the BFS terminate). Without
/// the exclusion, P would then enter its own installed vector while also
/// holding the host seat, and its every `ExtensionKey` would collide with
/// itself at collection. The cycle is authored on purpose here; the test is
/// the reviewer's pin for a guard the shared fixture cannot reach.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn a_lane_owner_never_sits_in_its_own_installed_vector_even_under_a_cyclic_lock() {
    let workspace = TempDir::new().unwrap();
    let root = workspace.path();
    slot(
        root,
        "org.cyc",
        "p-tools",
        r#"
[package]
group = "org.cyc"
name = "p-tools"
kind = "tool"
version = "1.0.0"

[requires.packages]
"org.cyc/q-tools" = "=1.0.0"

[[extension]]
id = "p-src"
point = "compile:source"
handler = { kind = "builtin", name = "log" }
"#,
    );
    slot(
        root,
        "org.cyc",
        "q-tools",
        r#"
[package]
group = "org.cyc"
name = "q-tools"
kind = "tool"
version = "1.0.0"

[requires.packages]
"org.cyc/p-tools" = "=1.0.0"

[[extension]]
id = "q-src"
point = "compile:source"
handler = { kind = "builtin", name = "log" }
"#,
    );
    let manifest = node(root, "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n");
    let lockfile = lock(vec![
        locked("org.cyc", "p-tools", &["org.cyc/q-tools@=1.0.0"]),
        locked("org.cyc", "q-tools", &["org.cyc/p-tools@=1.0.0"]),
    ]);
    let world = DurableExtensionWorld::from_lock(root, root, &manifest, &lockfile)
        .expect("a cyclic lock still snapshots — the walk terminates on the reached set");

    let owner = id("org.cyc", "p-tools");
    let view = world
        .package_owner_view(&owner)
        .expect("the cycle does not cost P its lane");
    assert!(
        !view
            .installed
            .iter()
            .any(|source| source.provider.id == owner),
        "P holds the host seat, so P is out of its own installed vector"
    );
    assert!(
        view.installed
            .iter()
            .any(|source| source.provider.id == id("org.cyc", "q-tools")),
        "the cycle does not cost P its real dependency either"
    );
    let registry = collect_owner_view(view, Vec::new())
        .expect("one seat per coordinate, so P's keys collide with nothing");
    assert!(
        found(&registry, "#p-src"),
        "P declares through the host seat"
    );
    assert!(
        found(&registry, "#q-src"),
        "Q declares through the dependency tier"
    );
}
