# ADV-FIX-EXAMPLES-5

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `tools/self-check.sh` runs `vibe doc check --examples` over the English manual as its own step, right after the `vibe check` step, so a changed command output turns the gate red with the diverging example named.

**Write.** `tools/self-check.sh`, `DEV-GUIDE.md`

**Check.** `grep -q 'doc check --examples' tools/self-check.sh && grep -q '^STEP_TOTAL=9$' tools/self-check.sh && cargo run --quiet -p vibe-cli -- doc check --examples --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0`

**Test.** No Rust test (shell script). The check fails today on the first grep; after the change both greps match and the step's own command exits 0. Red-on-divergence with the example named is already covered by crates/vibe-doc/src/examples/tests.rs `a_golden_that_lies_is_red_and_the_diff_says_where`.

**Read first.**
- tools/self-check.sh:1-35
- tools/self-check.sh:95-125
- DEV-GUIDE.md:373-383
- DEV-GUIDE.md:478-482

**Steps.**
1. Run the check: it fails on the first grep.
2. In tools/self-check.sh change `STEP_TOTAL=8` to `STEP_TOTAL=9`.
3. Directly after the `run_step "vibe check --path . --quiet"` block (its second line ends `|| OVERALL=$?`) insert three lines: `run_step "vibe doc check --examples (English manual)" \` / `  cargo run --quiet -p vibe-cli -- doc check --examples \` / `    --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0 || OVERALL=$?`.
4. Replace header lines 4-7 of tools/self-check.sh (from `# This gate intentionally covers` through `# commands directly when their inputs change.`) with exactly these seven lines: `# This gate intentionally covers the host product only. Independently shipped` / `# package workspaces, other documentation/browser suites, self-traces and` / `# historical policy ratchets are affected-area checks, not release blockers.` / `# Run those commands directly when their inputs change. The one documentation` / `# check it does run is the English manual's documented examples (PROP-057` / `# OBS-EXAMPLES-GOLDEN): a command whose output changed turns the gate red with` / `# the example named.` Keep line 3 (`#`) and the blank line after the header as they are.
5. DEV-GUIDE.md: in the §6 sentence "then the host spec, conform, codegen-freshness and wire-compatibility checks" insert ", the English manual's documented examples" after "wire-compatibility checks"; in the next sentence of that paragraph replace "documentation/browser suites" with "other documentation/browser suites"; in the "What each check asks" paragraph replace "Run it when commands or their documented output change;" with "`tools/self-check.sh` runs it over the English manual; run it directly when commands or their documented output change;".
6. Run the check until it passes.

**Decisions already made.**
- Gate location is tools/self-check.sh only; no CI workflow edit.
- Only `--examples`; `--derived`, `--citations`, `--coverage`, `--style` stay out of self-check.
- Same `cargo run` debug binary as the `vibe check` step; default sandbox location; no skip flag; `--keep-going` behaviour comes from run_step unchanged.
- The existing 'documentation/browser suites are affected-area checks' wording is kept but narrowed to 'other documentation/browser suites' in both files, so it no longer contradicts the new step.

**Not in this entry.** Adding --derived/--citations or the Russian manual to the gate, editing CI, or flipping spec status attributes in PROP-057/PROP-058.

**Depends on.** ADV-FIX-EXAMPLES-4

Entries that write the same file run one after another, never in parallel.
