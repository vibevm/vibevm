# Preset table doctest — exact native inventory alignment

2026-11-19; accepted commit base16cf7329421c4f679845c2364ca5d130c4359a99 plus separately frozen review candidates. Last coherent self-check bze3v32c5 ended101 at workspace-test step2 after1115s: vibe-app doctests87 PASS/1 FAIL. preset_table still asserted6 while accepted Hermes/Pi presets made8. Prior failed log remains control/integration-coherent-receipt-aligned-self-check.log; no retroactive gate success.

Narrow repair replaces only the stale count assertion with the exact ordered eight preset names already specified by agents/presets.rs. The placeholders assertion remains. No product behavior, filtering, schema, generated code or test suppression changes. Independent read-only workflow verified the actual preset source and failure, and all19 files in existing quiet/projection/CLI/lifecycle manifests retained exact hashes; its results are review evidence, not acceptance.

Target gate: cargo test -p vibe-app --doc with process-local CARGO_BUILD_JOBS=2 and existing target/incremental; result retained control/integration-preset-doctest-aligned.log. Formatting/diff and independent architect acceptance apply. Coherent tools/self-check remains owed after this alignment. No new Hermes experiment, upstream change, scope waiver or SYS/NST completion follows from this test correction.
