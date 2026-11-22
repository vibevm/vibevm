# ADV-RELEASE-AARCH64-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A vibe running on Linux aarch64 names `aarch64-unknown-linux-musl` as its own release target. `vibe self update`, `self install` and the native installer then select that bundle instead of failing on an unsupported platform.

**Write.** `crates/vibe-wire/src/behaviour/release_manifest/native.rs`, `crates/vibe-wire/src/behaviour/release_manifest.rs`, `crates/vibe-publish/src/release_manifest.rs`, `crates/vibe-bootstrap/src/lib.rs`, `crates/vibe-cli/src/commands/vvm/bundle/selection.rs`, `specmap.json`

**Check.** `cargo test -p vibe-wire --lib behaviour::release_manifest::native::tests::linux_aarch64_resolves_to_the_musl_release_target -- --exact 2>&1 | grep -F 'test result: ok. 1 passed'`

**Test.** New `#[test] fn linux_aarch64_resolves_to_the_musl_release_target` in a `#[cfg(test)] mod tests` inside the new file crates/vibe-wire/src/behaviour/release_manifest/native.rs. It asserts the following. For musl in [false, true], `native_distribution_target("linux", "aarch64", musl) == Some("aarch64-unknown-linux-musl")`. The x86_64 Linux pair gives gnu for false and musl for true. ("windows","x86_64",false) gives Some("x86_64-pc-windows-msvc"). ("macos","x86_64",false) gives Some("x86_64-apple-darwin"). ("macos","aarch64",false) gives Some("aarch64-apple-darwin"). ("linux","riscv64",false) and ("windows","aarch64",false) give None. For every os in [windows, linux, macos], arch in [x86_64, aarch64] and musl in [false, true], every Some value is in SUPPORTED_DISTRIBUTION_TARGETS.

**Read first.**
- `pub fn target() -> Result<&'static str>` in crates/vibe-bootstrap/src/lib.rs
- `pub(in crate::commands::vvm) fn current_target() -> Result<&'static str>` in crates/vibe-cli/src/commands/vvm/bundle/selection.rs
- the `pub use vibe_wire::behaviour::release_manifest::{...}` list in crates/vibe-publish/src/release_manifest.rs
- module header (`specmark::scope!`, `use specmark::spec;`, `mod error;`, `mod validation;`) of crates/vibe-wire/src/behaviour/release_manifest.rs
- `#[spec(...)]` use in crates/vibe-wire/src/behaviour/release_manifest/error.rs (a child module using `use specmark::spec;`)
- `pub fn running_target() -> Option<&'static str>` in crates/vibe-net/src/px_store.rs (read only, do not change)

**Steps.**
1. Run this after ADV-RELEASE-AARCH64-2, not at the same time. It needs the sixth target, and it edits release_manifest.rs, vibe-publish/src/release_manifest.rs, selection.rs and specmap.json after -1 and -2.
2. Create crates/vibe-wire/src/behaviour/release_manifest/native.rs with `use specmark::spec;` and a doc-commented `#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#RELEASE-TARGETS")] pub fn native_distribution_target(os: &str, arch: &str, musl: bool) -> Option<&'static str>`. The body is a `match (os, arch)` with these arms: ("windows","x86_64") gives x86_64-pc-windows-msvc. ("linux","x86_64") gives x86_64-unknown-linux-musl if musl, else x86_64-unknown-linux-gnu. ("linux","aarch64") gives aarch64-unknown-linux-musl. ("macos","x86_64") gives x86_64-apple-darwin. ("macos","aarch64") gives aarch64-apple-darwin. Anything else gives None. Import SUPPORTED_DISTRIBUTION_TARGETS only inside the test module, as `use super::super::SUPPORTED_DISTRIBUTION_TARGETS;`, so that a non-test build has no unused import.
3. In release_manifest.rs, add `mod native;` and `pub use native::native_distribution_target;` right after `mod validation;`.
4. In crates/vibe-publish/src/release_manifest.rs, add `native_distribution_target` to the `pub use vibe_wire::behaviour::release_manifest::{...}` list.
5. In vibe_bootstrap::target() keep the doc test and replace the body with
```
    match native_distribution_target(
        std::env::consts::OS,
        std::env::consts::ARCH,
        cfg!(target_env = "musl"),
    ) {
        Some(target) => Ok(target),
        None => bail!("unsupported native installer platform"),
    }
```
Add `native_distribution_target` to the existing `use vibe_wire::behaviour::release_manifest::{...}` import in that file.
6. In selection.rs, replace the body of current_target() with
```
    let (arch, os) = (std::env::consts::ARCH, std::env::consts::OS);
    match native_distribution_target(os, arch, cfg!(target_env = "musl")) {
        Some(target) => Ok(target),
        None => bail!("no vibevm binary distribution target for {arch}-{os}"),
    }
```
and add `native_distribution_target` to the existing `use vibe_publish::release_manifest::{...}` import.
7. Run `cargo xtask specmap`. If it fails with `Vibe specification authority is missing`, leave specmap.json unchanged and report it.
8. Run `cargo test -p vibe-bootstrap`, `cargo test -p vibe-cli --lib commands::vvm::bundle` and `cargo test -p vibe-publish --lib` and check they pass. Then run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`. Both edited caller files still call `bail!` elsewhere (vibe-bootstrap lib.rs in its bundle checks, selection.rs in read_aggregate), so no import becomes unused.

**Decisions already made.**
- Linux aarch64 maps to the musl target whatever libc the running binary was built against. Only musl ships for aarch64, so a glibc vibe built locally on ARM also updates to the released musl build.
- The table lives in vibe-wire next to SUPPORTED_DISTRIBUTION_TARGETS. It is a pure function of (os, arch, musl), and callers pass std::env::consts::OS, ARCH and cfg!(target_env = "musl"). That makes it testable on any host.
- No new crate dependencies. vibe-net does not depend on vibe-wire, so px_store::running_target stays as it is. On aarch64 Linux it returns None, which already produces the 'px has no default build' hint.

**Not in this entry.** Do not change xtask native_target/resolve_target (entry -5) or vibe-net px_store::running_target. Do not add px for aarch64. Do not touch install.sh (entry -4). Do not touch kit_layout in vibe-bootstrap/src/kit.rs, which already maps any target that is neither Windows nor darwin to linux. Do not change read_aggregate or its doc (entry -1 already reworded the doc).

**Depends on.** ADV-RELEASE-AARCH64-2

Entries that write the same file run one after another, never in parallel.
