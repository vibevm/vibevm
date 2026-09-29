# Bounded migration cross-target library checks

Both selected cross-target library checks eventually exited0 after root populated narrowly missing offline crates. This is compiler/type evidence on the named current workspace inputs, not native, linked or task acceptance. The exact named Cargo/lock/config/lib input hashes match before and after the attempts; no whole-checkout commit was queried.

Latest passing runs, sequential with CARGO_BUILD_JOBS=4/CARGO_NET_OFFLINE=true/existing shared target:

- aarch64-apple-darwin retry1 EXIT0: vibe-safefs and vibe-workspace checked, Finished dev profile29.12s.
- x86_64-pc-windows-gnu retry6 EXIT0: same two libraries checked, Finished dev profile23.99s.

Both used `cargo check --locked -p vibe-safefs -p vibe-workspace --lib --target <target>`. No target program was linked or executed; no platform directory/file sync, recovery, SDK/native filesystem behavior, full package/API coverage or user-path operation was tested. Existing Cargo/safefs warnings remain in raw logs. Actual rustc-vV1.98.0 commit88d9e12ae178fab0fb5cc050a94da85685d449ea, LLVM22.1.8; Cargo1.98.0(797e8a9bc2026-08-05).

Failure history is retained separately: initial Darwin101 missing core-foundation-sys0.8.7, followed by its passing retry1. Windows initial/retry1/retry2/retry3/retry4/retry5 each exited101 before source compilation, exposing winapi-util0.1.11, windows-core0.62.2, windows-implement0.60.2, windows-sys0.59.0, windows-sys0.60.2 and winx0.36.4 in sequence. Root acquired exact locked cache packages; the worker never fetched or changed source. Total9 attempts:2successes/7offline prerequisite failures. No earlier failed result is relabelled a compiler defect or erased.

results.json records per-attempt argv/env/exit/raw combined stdout-stderr digest, latest status, and input byte recheck. Version/input logs and all raw attempts remain here. Combined stream ordering is preserved; stdout/stderr were not separately captured. The historical d69a88b3 migration body identity is root-bound context, while this evidence compiles the current named byte snapshot without a Git/HEAD binding. No code, Git, test, SDK install or public operation by worker. Native macOS/Windows validation and D1 acceptance remain root-owned. Worker stopped.
