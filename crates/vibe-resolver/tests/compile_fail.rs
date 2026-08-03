//! The Class-B seam checker, second half (card
//! scaffold-b-typed-builders, routine step 5): confirm the
//! previously-wrong call no longer compiles. Each `tests/ui/*.rs`
//! file is a call shape the typed seam must reject at `cargo check`
//! time — the compile error IS the assertion.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#subskill-activation"
);

#[test]
fn wrong_seam_calls_do_not_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
