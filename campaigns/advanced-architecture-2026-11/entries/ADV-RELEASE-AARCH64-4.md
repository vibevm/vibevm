# ADV-RELEASE-AARCH64-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** On Linux aarch64 (or arm64), install.sh selects `aarch64-unknown-linux-musl` instead of refusing with 'this release supports x86_64 only'.

**Write.** `distribution/install/install.sh`, `distribution/install/test-install.sh`

**Check.** `sh -c 'VIBEVM_INSTALL_TEST_MODE=1 . ./distribution/install/install.sh; uname() { case "$1" in -s) echo Linux;; -m) echo aarch64;; esac; }; [ "$(vibe_install_detect_target)" = aarch64-unknown-linux-musl ]'`

**Test.** A new block in distribution/install/test-install.sh, inserted immediately before the final line `printf '%s\n' 'install.sh offline tests passed'`. It starts with the comment `# Linux aarch64 and arm64 select the static musl build` and is wrapped in a subshell `( ... )` so that the uname override does not leak. Inside it, `uname() { case "$1" in -s) echo Linux;; -m) echo "$fake_arch";; esac; }`. For fake_arch in aarch64 and arm64 it runs `if [ "$(vibe_install_detect_target)" != aarch64-unknown-linux-musl ]; then printf '%s\n' "$fake_arch was not mapped to aarch64-unknown-linux-musl" >&2; exit 1; fi`. With fake_arch=riscv64 it runs `if ( vibe_install_detect_target ) >/dev/null 2>&1; then printf '%s\n' 'riscv64 unexpectedly accepted' >&2; exit 1; fi`, the same pattern as the file's existing negative checks, because the file runs under `set -eu`.

**Read first.**
- vibe_install_detect_target in distribution/install/install.sh
- the VIBEVM_INSTALL_TEST_MODE guard at the end of distribution/install/install.sh
- distribution/install/test-install.sh (how it sources install.sh, and its `if ...; then printf ... >&2; exit 1; fi` negative checks)

**Steps.**
1. In vibe_install_detect_target, in the `Linux)` branch, add the arm `aarch64|arm64) printf '%s\n' 'aarch64-unknown-linux-musl' ;;` before the `*)` arm. Do not probe for a musl loader, because only the musl build exists for aarch64.
2. Change the Linux `*)` failure text from `this release supports x86_64 only` to `this release supports x86_64 and aarch64`.
3. Add the test block, then run `sh distribution/install/test-install.sh`. It must print `install.sh offline tests passed`.
4. The installer stays POSIX sh: no bashisms and no `local`.

**Decisions already made.**
- arm64 is accepted as an alias of aarch64, as the Darwin branch already does.
- Releases published before aarch64 shipped have no aarch64 entry. For those, the existing code already refuses with release manifest has no bootstrap for aarch64-unknown-linux-musl, which is the correct result.

**Not in this entry.** install.ps1/install.cmd, the x86_64 musl/gnu selection, and any change to how the manifest is read. Do not wire test-install.sh into self-check or CI.

Entries that write the same file run one after another, never in parallel.
