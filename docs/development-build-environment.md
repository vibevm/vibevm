# Development build environment

This records the build paths verified during the Zap + Lean preparation on
2026-10-08. Machine paths below identify evidence, not required install
locations. VibeVM feature development is awaiting owner review of Zap + Lean.

## Rust builds in this checkout

Each Git worktree uses its own Cargo `target/`; do not share a target across
worktrees. This machine has an ancestor Cargo config at
`/fast/git/v/.cargo/config.toml` with nightly-only `-Zthreads=8` and an
unstable `codegen-backend = "cranelift"` dev profile. The correct name is
**Cranelift**. Ordinary Rust builds using that config need a compatible
nightly toolchain. The config also compiles `sha2` with LLVM at opt-level 3:
the machine probe found SHA-heavy PodBay setup far slower under Cranelift.
These backend overrides are machine-local, not a portable project setting.

A stable Cargo command launched inside `/fast/git/v` can fail before testing
the product because it inherits the ancestor config; `-Zthreads=8` gives an
exit-101 stable compiler failure, while one Cranelift run aborted during panic
initiation. To run a stable-only isolated check, launch from outside that
ancestor directory (for example `/` or `/tmp`), pass absolute
`--manifest-path` and a private `--target-dir`, set the intended stable
`RUSTC` and `RUSTDOC` explicitly, and clear inherited `RUSTFLAGS`,
`CARGO_ENCODED_RUSTFLAGS`, `RUSTDOCFLAGS`, and `RUSTC_WRAPPER`. Record the
actual command, toolchain and exit code. A no-feature zero-test invocation
does not establish a test pass. Unwind-sensitive failures under Cranelift
need an LLVM comparison before classification as a source regression. Evidence:
`/fast/git/v/research/2026-10-08-zap-age-source-bridge/REPORT.md` and
`/fast/git/v/research/2026-10-08-zap-replay-index/REPORT.md`.

## Aeneas and Charon for Zap proof work

The Zap translation backend is built from **Aeneas and matching Charon source**
with ordinary Docker, OPAM and Dune, rather than depending on Nix or a live
GitHub fetch during the build. The reproducible, pinned, unmodified baseline
recipe and limits live in Zap's
`vibevm/vibepacks/org.vibevm.zap/zap/v1.1.0/tooling/offline-aeneas/README.md`.
The verified baseline uses Aeneas `183e4f0`, Charon `fea3fc68`, OCaml 5.3.0,
Dune 3.20.2, 131 OPAM packages and four LLBC controls. Expected translation
results are `0/0/2/2`: the two exit-2 cases reproduce the owned-transfer
bottom-value bug. This is a baseline reproduction, not a repaired translator.

The persistent machine cache is `/fast/opam/`; its `README.md` and
`verify_cache.py` describe and check the bundle. It holds the cached switch,
archives, source snapshots and image inputs so Docker rebuilds do not fetch or
compile everything again. Do not clean it as Cargo output. A separate genuine
upstream Aeneas PR checkout is being prepared at
`/fast/git/pr/aeneas-bug-1/aeneas`, paired with its pinned Charon checkout
under `deps/charon/` and a task-local Rust nightly under `deps/rustup/`.
Its current candidate is uncommitted and unverified; see that workspace's
`HANDOFF.md` before using it. Do not treat the rejected earlier experiment as
the fix, or the baseline source build as proof that the PR candidate works.
