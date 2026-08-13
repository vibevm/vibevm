//! Content-hash recipe + parity tests.
//!
//! Two jobs live here:
//!
//! 1. **Golden lock.** `golden-flow-wal-0.1.0` is a small, standalone
//!    synthetic package (`com.example/golden-pkg`) kept frozen purely as a
//!    cross-impl hash anchor — it deliberately reuses no real package's name
//!    (it was de-collided from the shipped `org.vibevm.world/wal` package,
//!    whose name it once mirrored). `.gitattributes` (`* text=auto eol=lf`,
//!    at the repo root) keeps every text byte identical on Windows / macOS /
//!    Linux, so the digest is stable cross-platform. [`GOLDEN`] is recipe 0's
//!    value for that fixture (the form every lockfile already carries), and
//!    [`flow_wal_v0_1_0_matches_canonical_algorithm`] asserts recipe 0
//!    explicitly so the constant stays verbatim even though `vibe-index`'s
//!    default is now recipe 1.
//!
//! 2. **Parity gate (PROP-005 §3.2).** `vibe-index`'s `content_hash` is a
//!    byte-identical port of `vibe-registry`'s, duplicated rather than
//!    imported for standalone redistribution — and §3.2 promises "a parity
//!    test gates divergence at CI time". Until this change that test called
//!    only ONE implementation and so could not detect divergence at all.
//!    [`both_implementations_agree_at_recipe_0`] /
//!    [`both_implementations_agree_at_recipe_1`] are the missing guard: they
//!    hash BOTH fixtures with BOTH implementations at EACH recipe and require
//!    byte-equality.
//!
//! 3. **Order trap (PROP-044 §4.7).** `golden-order-trap-0.1.0` is the tree on
//!    which recipes 0 and 1 diverge — and the shape that makes them diverge is
//!    NOT the one the plan predicted. The plan expected recipe 0 to follow the
//!    host separator (it sorts `Vec<PathBuf>` before normalising) and named a
//!    sibling byte strictly between `/` = 0x2F and `\` = 0x5C. Measurement
//!    refutes it: `Path`'s `Ord` compares **components**, so recipe 0 never
//!    sees a separator byte and is host-stable, and on such a sibling
//!    (`specX.md`, `X` = 0x58) the two recipes AGREE. They part company only
//!    when the sibling's byte is BELOW `/` — `spec-x.md`, `-` = 0x2D — where
//!    a normalised byte compare puts the sibling first and a component compare
//!    still puts the directory first. The fixture therefore carries both:
//!    `spec-x.md` is the trap, `specX.md` the control.
//!
//!    Only recipe 1 is frozen here ([`TRAP_RECIPE1`]). Recipe 0's value on
//!    this tree is deliberately unfrozen — not because it is host-dependent
//!    (it is not), but because freezing it would pin a value no shipped
//!    lockfile contains, and the guard that matters for recipe 0 is
//!    [`GOLDEN`], whose constant predates recipes entirely.
//!
//! If a golden drifts after a git operation that touches a fixture or the
//! algorithm: re-derive by running the failing test — the panic prints the
//! freshly-computed digest — and paste it below.

use std::path::PathBuf;

use specmark::verifies;
use vibe_index::content_hash::compute_content_hash_with;
use vibe_index::hash_recipe::RecipeId;

/// Recipe 0's frozen hash of `golden-flow-wal-0.1.0` — the form the registry
/// still emits and every lockfile already carries. Verbatim; do not edit
/// unless the fixture bytes or recipe 0 itself changed.
const GOLDEN: &str = "sha256:e10a49c0a8e1b35e3f0dc1e74d6ce26605052b2eead2225124051d67a2f76cb6";

/// Recipe 1's frozen hash of `golden-order-trap-0.1.0`. Recipe 1 normalises
/// separators before ordering, so this value is the same on every host. To
/// re-derive: `cargo test -p vibe-index recipe_1_freezes_the_order_trap_fixture`
/// (a drift prints the freshly-computed digest in the assertion panic) and
/// paste below.
const TRAP_RECIPE1: &str =
    "sha256-tree/1:883014931b57171ab81add3cd2183b295768a1e05c01b1290ee0512cbc59eb6e";

fn golden_flow_wal() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("golden-flow-wal-0.1.0")
}

fn golden_order_trap() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("golden-order-trap-0.1.0")
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#trust",
    r = 1
)]
fn flow_wal_v0_1_0_matches_canonical_algorithm() {
    let fixture = golden_flow_wal();
    assert!(
        fixture.is_dir(),
        "fixture not present at `{}`",
        fixture.display()
    );
    // GOLDEN is recipe 0's value; assert recipe 0 explicitly so the constant
    // stays verbatim even though this crate's default is now recipe 1.
    let hash = compute_content_hash_with(RecipeId::Legacy0, &fixture).expect("hash computes");
    assert_eq!(
        hash, GOLDEN,
        "vibe-index content_hash (recipe 0) diverged from the canonical algorithm — \
         either the fixture bytes changed or the algorithm did. See \
         spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#types"
    );
}

/// The parity guard PROP-005 §3.2 promises but, until this change, did not
/// have: the two duplicated hashers must agree byte-for-byte for a given
/// recipe. This tests recipe 0 (the registry's default) on BOTH fixtures.
#[test]
fn both_implementations_agree_at_recipe_0() {
    for fixture in [golden_flow_wal(), golden_order_trap()] {
        let index = compute_content_hash_with(RecipeId::Legacy0, &fixture).unwrap();
        let registry =
            vibe_registry::compute_content_hash_with(vibe_registry::RecipeId::Legacy0, &fixture)
                .unwrap();
        assert_eq!(
            index,
            registry,
            "recipe 0 diverged between vibe-index and vibe-registry on `{}`",
            fixture.display()
        );
    }
}

/// The same parity guard for recipe 1 (the index's default) on both fixtures.
#[test]
fn both_implementations_agree_at_recipe_1() {
    for fixture in [golden_flow_wal(), golden_order_trap()] {
        let index = compute_content_hash_with(RecipeId::Tree1, &fixture).unwrap();
        let registry =
            vibe_registry::compute_content_hash_with(vibe_registry::RecipeId::Tree1, &fixture)
                .unwrap();
        assert_eq!(
            index,
            registry,
            "recipe 1 diverged between vibe-index and vibe-registry on `{}`",
            fixture.display()
        );
    }
}

#[test]
fn recipe_1_freezes_the_order_trap_fixture() {
    let fixture = golden_order_trap();
    assert!(
        fixture.is_dir(),
        "fixture not present at `{}`",
        fixture.display()
    );
    let hash = compute_content_hash_with(RecipeId::Tree1, &fixture).unwrap();
    assert_eq!(
        hash, TRAP_RECIPE1,
        "recipe 1 on the order-trap fixture diverged — see TRAP_RECIPE1 for how to re-derive"
    );
}

/// On the order-trap fixture recipes 0 and 1 produce DIFFERENT digests, on
/// every host.
///
/// This test was written Windows-only on the assumption that recipe 0's
/// ordering follows the host separator. It does not: recipe 0 sorts
/// `Vec<PathBuf>`, and `Path`'s `Ord` compares **components**, so it never
/// sees a separator byte at all — measured, and recorded in
/// `vibe-registry`'s `neither_recipe_depends_on_the_input_separator`. The real
/// difference is component order versus normalised-byte order, and the fixture
/// carries the shape that separates them: `spec-x.md` beside the `spec/`
/// directory, where `-` (0x2D) sorts below `/` (0x2F). The sibling `specX.md`
/// is kept as the control — `X` (0x58) sorts above `/`, so it orders the same
/// under both recipes and proves nothing on its own.
#[test]
fn recipe_0_and_recipe_1_diverge_on_the_order_trap() {
    let fixture = golden_order_trap();
    let legacy = compute_content_hash_with(RecipeId::Legacy0, &fixture).unwrap();
    let tree = compute_content_hash_with(RecipeId::Tree1, &fixture).unwrap();
    assert_ne!(
        legacy.trim_start_matches("sha256:"),
        tree.trim_start_matches("sha256-tree/1:"),
        "the order-trap fixture must make recipes 0 and 1 disagree on the DIGEST, \
         not merely on the label"
    );
}

/// Recipe 1 orders the same on every host — proved against the ordering
/// function directly, not by hashing a directory twice.
///
/// The plan expected this to be the whole story: recipe 0 host-dependent,
/// recipe 1 host-stable. Measurement says otherwise — recipe 0 is host-stable
/// too, because `Path`'s `Ord` is component-wise. So this test proves recipe
/// 1's half of the property, and `the_recipes_diverge_on_a_sibling_below_slash`
/// (in `vibe-registry`, over its own copy) carries what actually distinguishes
/// them. Kept as its own test because a property of the ordering function is
/// the only thing a hash of a real directory cannot demonstrate on one host.
#[test]
fn normalisation_precedes_ordering() {
    use vibe_index::hash_recipe::order_paths;
    let with_backslashes = vec![
        "spec\\inner\\a.md".to_string(),
        "spec-x.md".to_string(),
        "specX.md".to_string(),
        "README.md".to_string(),
        "vibe.toml".to_string(),
    ];
    let with_slashes = vec![
        "spec/inner/a.md".to_string(),
        "spec-x.md".to_string(),
        "specX.md".to_string(),
        "README.md".to_string(),
        "vibe.toml".to_string(),
    ];
    assert_eq!(
        order_paths(RecipeId::Tree1, &with_backslashes),
        order_paths(RecipeId::Tree1, &with_slashes),
        "recipe 1 must order identically regardless of the input separator"
    );
    // The settled order is the normalised-bytewise one: `-` (0x2D) below
    // `/` (0x2F) below `X` (0x58).
    assert_eq!(
        order_paths(RecipeId::Tree1, &with_slashes),
        vec![
            "README.md".to_string(),
            "spec-x.md".to_string(),
            "spec/inner/a.md".to_string(),
            "specX.md".to_string(),
            "vibe.toml".to_string(),
        ],
    );
}

/// On a non-pathological tree (no `dir` next to `dirX`) recipes 0 and 1
/// produce the SAME hex, differing only in their wire label. This fixes that
/// the change moves no existing value on any ordinary package — it only
/// names how the value was computed.
#[test]
fn recipes_share_the_digest_on_a_normal_tree() {
    let fixture = golden_flow_wal();
    let legacy = compute_content_hash_with(RecipeId::Legacy0, &fixture).unwrap();
    let tree = compute_content_hash_with(RecipeId::Tree1, &fixture).unwrap();
    assert!(legacy.starts_with("sha256:"));
    assert!(tree.starts_with("sha256-tree/1:"));
    assert_eq!(
        &legacy["sha256:".len()..],
        &tree["sha256-tree/1:".len()..],
        "on a normal tree the two recipes share the digest; only the label differs"
    );
}

// Recipe 1 makes a non-UTF-8 path a hard error: two distinct invalid names
// would otherwise lossy-collapse to one hash, so an unverifiable answer would
// look valid (PROP-044 §4.7). Constructing such a path needs raw `OsString`
// bytes, which `std::os::unix::ffi::OsStringExt` provides — so the real
// assertion runs on Unix; on every other host the test is `#[ignore]`'d with
// this explanation rather than silently absent.
#[cfg(unix)]
#[test]
fn non_utf8_path_is_a_hard_error_under_recipe_1() {
    use std::os::unix::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    // 0xFF is not a valid UTF-8 lead byte.
    let bad = dir.path().join(std::ffi::OsString::from_vec(vec![0xFF]));
    std::fs::write(&bad, b"x").unwrap();

    let err = compute_content_hash_with(RecipeId::Tree1, dir.path());
    assert!(
        err.is_err(),
        "recipe 1 must reject a non-UTF-8 path as a hard error"
    );
    // Recipe 0 is frozen-lossy: it still hashes the tree (the pre-recipe
    // behaviour), for contrast.
    assert!(
        compute_content_hash_with(RecipeId::Legacy0, dir.path()).is_ok(),
        "recipe 0 stays lossy on non-UTF-8 (frozen behaviour)"
    );
}

#[cfg(not(unix))]
#[test]
#[ignore = "constructing a non-UTF-8 path needs raw OsString bytes (std::os::unix::ffi); \
            run `cargo test -- --ignored non_utf8_path_is_a_hard_error_under_recipe_1` on Unix"]
fn non_utf8_path_is_a_hard_error_under_recipe_1() {
    // The Unix cfg twin carries the assertion; this stub exists only so the
    // test is visible (ignored, not absent) on non-Unix hosts.
}
