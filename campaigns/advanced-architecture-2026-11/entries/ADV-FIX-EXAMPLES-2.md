# ADV-FIX-EXAMPLES-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The example normaliser replaces the elapsed time on a `✻` trailer line with `<ELAPSED>`, so `… · 0s` and `… · 2m 14s` compare equal and a slow machine no longer turns an example red.

**Write.** `crates/vibe-doc/src/examples/normalize.rs`, `crates/vibe-doc/src/examples/fixture.rs`, `crates/vibe-doc/src/examples/normalize/tests.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-doc --lib elapsed`

**Test.** crates/vibe-doc/src/examples/normalize/tests.rs: (1) `a_trailer_elapsed_time_matches_whatever_it_took`: with `norm(NormalizeDecl::default())`, apply("✻ materialised 1 package into vibedeps/ · regenerated boot for 1 node(s) · 0s\n") equals apply("✻ materialised 1 package into vibedeps/ · regenerated boot for 1 node(s) · 2m 14s\n") and equals "✻ materialised 1 package into vibedeps/ · regenerated boot for 1 node(s) · <ELAPSED>"; apply("✻ installed org.example/tool@1.0.0 · 1 step · 1h 4m · log /x") equals "✻ installed org.example/tool@1.0.0 · 1 step · <ELAPSED> · log /x"; apply("copied · 3s") equals "copied · 3s" (not a trailer line). (2) `the_elapsed_class_can_be_switched_off`: with `NormalizeDecl { elapsed: false, ..NormalizeDecl::default() }`, apply("✻ done · 0s") equals "✻ done · 0s".

**Read first.**
- crates/vibe-doc/src/examples/fixture.rs:150-222
- crates/vibe-doc/src/examples/normalize.rs:55-145
- crates/vibe-doc/src/examples/normalize/tests.rs:1-80
- crates/vibe-cli/src/output/language/meta.rs:82-95

**Steps.**
1. Run the check: it runs zero tests today.
2. Add to `NormalizeDecl` in fixture.rs a field `pub elapsed: bool` with `#[serde(default = "yes")]` and a doc comment: "The elapsed time on a `✻` trailer line → `<ELAPSED>`: how long a command took is the machine's, not the product's."; set `elapsed: true` in `impl Default for NormalizeDecl`.
3. In normalize.rs compile a Regex `^(?:\d+h(?: \d+m)?|\d+m(?: \d+s)?|\d+s)$` into a new `Normalizer` field `elapsed: Regex` (refuse with `bad("<elapsed>", e)` like the others).
4. In `Normalizer::apply`, right after the `product_version` block and before the fixture `replace` loop: when `self.decl.elapsed`, rewrite every line that starts with "✻ " by splitting it on " · ", replacing each segment that fully matches the elapsed regex with "<ELAPSED>", and joining with " · "; other lines are untouched and line endings are preserved.
5. Add the two tests; run the check until it passes.

**Decisions already made.**
- Field name `elapsed`, placeholder text `<ELAPSED>`, default on.
- Only lines whose first two characters are `✻ ` are touched; the ASCII trailer glyph `*` is not handled.
- Segment grammar mirrors `elapsed()` in crates/vibe-cli/src/output/language/meta.rs: `Ns`, `Nm`, `Nm Ns`, `Nh`, `Nh Nm`.
- Observed today: the current binary prints a `✻ … · 0s` trailer on stdout for init/install/uninstall (e.g. `✻ initialized project w · 11 files created · 0 kept · 0s`). The manual's goldens do not carry it yet; they will after ADV-FIX-EXAMPLES-4 re-blesses them. Do not touch any golden in this entry.

**Not in this entry.** A general timestamp/duration scrubber for non-trailer lines, changing the CLI to suppress durations, or editing any manual page.

Entries that write the same file run one after another, never in parallel.
