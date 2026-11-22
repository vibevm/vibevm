# ADV-RELEASE-AARCH64-5

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The release workflow builds `aarch64-unknown-linux-musl` natively on an ARM64 Linux runner, uploads it, and finalizes only a draft that holds every supported target. `xtask dist build` accepts that target on a Linux aarch64 host.

**Write.** `.github/workflows/release-distributions.yml`, `distribution/release/build-linux-x86_64.sh`, `distribution/release/build-linux-aarch64.sh`, `xtask/src/dist/build.rs`, `xtask/src/dist/build/target.rs`, `xtask/src/dist/build/tests.rs`, `xtask/src/dist/mod.rs`, `vibevm/vibespecs/common/PROP-019-version-manager.xml`, `specmap.json`

**Check.** `cargo test -p xtask dist::build::tests::release_workflow_builds_and_uploads_every_supported_target -- --exact 2>&1 | grep -F 'test result: ok. 1 passed'`

**Test.** New `#[test] fn release_workflow_builds_and_uploads_every_supported_target` in xtask/src/dist/build/tests.rs. It reads `Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/release-distributions.yml")` and turns `\r\n` into `\n`. For every target in SUPPORTED_DISTRIBUTION_TARGETS it asserts that some line has `line.trim() == format!("target: {target}")`, which is the build matrix, and that some line has `line.trim() == format!("- {target}")`, which is the upload matrix. It collects every line whose trimmed text starts with `script: ` and asserts there are exactly `SUPPORTED_DISTRIBUTION_TARGETS.len()` of them. It also asserts that each named path is a file under `Path::new(env!("CARGO_MANIFEST_DIR")).join("..")`.

**Read first.**
- `fn resolve_target(requested: Option<&str>) -> Result<String>` and `fn native_target() -> Result<&'static str>` in xtask/src/dist/build.rs
- jobs.build matrix, the 'Install the Linux static-linker toolchain' step, the 'Provision the pinned JTD generator for the full gate' step, the jobs.upload matrix and jobs.finalize in .github/workflows/release-distributions.yml
- distribution/release/build-linux-x86_64.sh and distribution/release/build-linux-x86_64-gnu.sh
- native_distribution_target in crates/vibe-wire/src/behaviour/release_manifest/native.rs (added by entry -3, re-exported as vibe_publish::release_manifest::native_distribution_target)
- RELEASE-HOST-SCRIPTS in vibevm/vibespecs/common/PROP-019-version-manager.xml

**Steps.**
1. Run this last, after -1, -2 and -3. It edits xtask/src/dist/build/tests.rs after -2, and PROP-019 and specmap.json after -1 and -2.
2. In xtask/src/dist/build.rs, replace the whole native_target function with
```
fn native_target() -> Result<&'static str> {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    match native_distribution_target(os, arch, cfg!(target_env = "musl")) {
        Some(target) => Ok(target),
        None => bail!(
            "unsupported distribution host OS/architecture `{os}/{arch}`; supported native \
             targets are: {}",
            SUPPORTED_DISTRIBUTION_TARGETS.join(", ")
        ),
    }
}
```
In resolve_target, replace the `let compatible = matches!(...);` statement with
```
    let compatible = [false, true].into_iter().any(|musl| {
        native_distribution_target(std::env::consts::OS, std::env::consts::ARCH, musl)
            == Some(selected)
    });
```
Add `use vibe_publish::release_manifest::native_distribution_target;` after the existing `use vibe_publish::{...};` block. Leave everything else in both functions as it is.
3. Run `cargo fmt --all`, then `wc -l xtask/src/dist/build.rs`. It has 593 lines today, and these edits should leave it near 588. If it is above 600, move resolve_target and native_target into a new file xtask/src/dist/build/target.rs. Declare it as `mod target;` and `use target::resolve_target;` in build.rs, make resolve_target `pub(super)`, and give target.rs its own `use anyhow::{Result, bail};` and `use vibe_publish::{SUPPORTED_DISTRIBUTION_TARGETS, release_manifest::native_distribution_target};`. Then remove any import in build.rs that has become unused. If the file is 600 or less, do not create target.rs.
4. In distribution/release/build-linux-x86_64.sh, replace
```
linux_abi=${VIBEVM_LINUX_ABI-musl}
case "$linux_abi" in
    musl) rust_target=x86_64-unknown-linux-musl ;;
    gnu) rust_target=x86_64-unknown-linux-gnu ;;
    *) printf 'build-linux-x86_64.sh: unsupported VIBEVM_LINUX_ABI=%s\n' "$linux_abi" >&2; exit 2 ;;
esac
```
with
```
linux_abi=${VIBEVM_LINUX_ABI-musl}
linux_arch=${VIBEVM_LINUX_ARCH-x86_64}
case "$linux_arch/$linux_abi" in
    x86_64/musl) rust_target=x86_64-unknown-linux-musl ;;
    x86_64/gnu) rust_target=x86_64-unknown-linux-gnu ;;
    aarch64/musl) rust_target=aarch64-unknown-linux-musl ;;
    *) printf 'build-linux-x86_64.sh: unsupported VIBEVM_LINUX_ARCH/VIBEVM_LINUX_ABI=%s/%s\n' "$linux_arch" "$linux_abi" >&2; exit 2 ;;
esac
```
5. In the same script, replace
```
if [ "$detected_os" != Linux ] || { [ "$detected_arch" != x86_64 ] && [ "$detected_arch" != amd64 ]; }; then
    printf 'build-linux-x86_64.sh: requires Linux x86_64; detected OS=%s architecture=%s\n' \
        "$detected_os" "$detected_arch" >&2
```
with
```
case "$detected_arch" in
    amd64) detected_arch=x86_64 ;;
    arm64) detected_arch=aarch64 ;;
esac
if [ "$detected_os" != Linux ] || [ "$detected_arch" != "$linux_arch" ]; then
    printf 'build-linux-x86_64.sh: requires Linux %s; detected OS=%s architecture=%s\n' \
        "$linux_arch" "$detected_os" "$detected_arch" >&2
```
The `    exit 2` and `fi` lines that follow stay as they are.
6. In the same script, right after the `fi` that closes the `if [ "$linux_abi" = musl ] && ! command -v musl-gcc` block, add
```
if [ "$rust_target" = aarch64-unknown-linux-musl ]; then
    CC_aarch64_unknown_linux_musl=${CC_aarch64_unknown_linux_musl-musl-gcc}
    export CC_aarch64_unknown_linux_musl
fi
```
7. Create distribution/release/build-linux-aarch64.sh with exactly these lines:
```
#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
VIBEVM_LINUX_ARCH=aarch64
VIBEVM_LINUX_ABI=musl
export VIBEVM_LINUX_ARCH VIBEVM_LINUX_ABI
exec sh "$script_dir/build-linux-x86_64.sh" "$@"
```
Leave it non-executable (mode 100644), like every sibling in `git ls-files -s distribution/release`. The workflow runs it through `sh`.
8. In the workflow build matrix, after the `macOS ARM64` row (whose last line is `            script: distribution/release/build-macos-aarch64.sh`), add
```
          - label: Linux ARM64 musl
            runner: ubuntu-24.04-arm
            target: aarch64-unknown-linux-musl
            script: distribution/release/build-linux-aarch64.sh
```
9. In the workflow, change `if: ${{ matrix.target == 'x86_64-unknown-linux-musl' }}` on the static-linker step to `if: ${{ endsWith(matrix.target, '-unknown-linux-musl') }}`.
10. In the JTD provisioning step, change `if [ '${{ runner.os }}' = macOS ] && [ '${{ runner.arch }}' = ARM64 ]; then` to `if [ '${{ runner.arch }}' = ARM64 ]; then`, so that every ARM64 runner builds jtd-codegen from source. There is no prebuilt Linux ARM64 asset.
11. In the upload matrix, after `          - aarch64-apple-darwin`, add `          - aarch64-unknown-linux-musl`.
12. Reword the texts that count five targets and keep their meaning. The `upload` input description `Replace the version's GitHub draft, upload all five bundles, and publish it` becomes `Replace the version's GitHub draft, upload every target's bundle, and publish it`. The finalize job name `Verify five bundles and finalize the release` becomes `Verify every bundle and finalize the release`. The step name `Finalize only the complete five-target draft` becomes `Finalize only a draft holding every target`. In xtask/src/dist/mod.rs, the doc line `/// Show completeness of the five-target GitHub release.` becomes `/// Show completeness of the every-target GitHub release.`
13. Add the new test, apply the spec_edit with Edit, then run `cargo xtask specmap`. If it fails with `Vibe specification authority is missing`, leave specmap.json unchanged and report it.
14. Run `cargo test -p xtask dist::`, `sh -n distribution/release/build-linux-x86_64.sh`, `sh -n distribution/release/build-linux-aarch64.sh` and `sh distribution/release/build-linux-aarch64.sh --help` and check they pass. The last one must print the usage line, because the script parses arguments before it checks the host. Then run `cargo fmt --all` and `cargo clippy -p xtask --all-targets -- -D warnings`.
15. Do not dispatch the workflow and do not push. In the report, say that the workflow change has not run and that its first build-only dispatch is still to come.

**Decisions already made.**
- Build natively on GitHub's `ubuntu-24.04-arm` runner without cross-compiling, because the spec states that each host builds only its own native target.
- Reuse build-linux-x86_64.sh and select the architecture with VIBEVM_LINUX_ARCH, the same way the gnu wrapper selects the ABI with VIBEVM_LINUX_ABI. The script keeps its file name and its x86_64 default, so existing callers are unchanged.
- Set CC_aarch64_unknown_linux_musl=musl-gcc explicitly, unless the caller already set it, so that C build scripts such as ring's use the musl wrapper on ARM.
- This entry adds no smoke step for the new target, because smoke on all targets is its own ADV-RELEASE capability. The only proof that ubuntu-24.04-arm has rustup and a working musl-gcc is the first green build-only dispatch (upload=false), which the owner runs. Until that run is green the capability is not complete (see owner_questions).

**Not in this entry.** The bootstrap smoke on all targets, the published-release guard and provenance attestations are separate ADV-RELEASE capabilities. Also out of scope: docker-images.yml or multi-arch images, px for aarch64, a gnu aarch64 target, and any change to the prepare/upload/finalize logic in xtask/src/dist/release.rs, which already iterates SUPPORTED_DISTRIBUTION_TARGETS. The executor must not dispatch the workflow or push.

**Depends on.** ADV-RELEASE-AARCH64-1, ADV-RELEASE-AARCH64-2, ADV-RELEASE-AARCH64-3

**Spec edit (apply as a step of this entry).**

In vibevm/vibespecs/common/PROP-019-version-manager.xml replace
```
<RELEASE-HOST-SCRIPTS fact="true" status="impl/done">Five dependency-light wrappers under `distribution/release/` drive
```
with
```
<RELEASE-HOST-SCRIPTS fact="true" status="impl/done">Six dependency-light wrappers under `distribution/release/` drive
```
and replace
```
  `--upload` remains the manual build-plus-upload convenience. The Linux
  wrapper installs the Rust musl target through rustup and fails early with an
  actionable message when the required `musl-gcc` is absent; it never invokes
```
with
```
  `--upload` remains the manual build-plus-upload convenience. The Linux
  wrappers (x86_64 musl and GNU, aarch64 musl) install their Rust target
  through rustup; the musl ones fail early with an actionable message when
  `musl-gcc` is absent. None of them invokes
```
and right after that, replace
```
  a package manager or sudo.</RELEASE-HOST-SCRIPTS>
```
with
```
  a package manager or sudo.</RELEASE-HOST-SCRIPTS>
```
(unchanged; it is listed only to show where the sentence ends). Keep status="impl/done".

Entries that write the same file run one after another, never in parallel.
