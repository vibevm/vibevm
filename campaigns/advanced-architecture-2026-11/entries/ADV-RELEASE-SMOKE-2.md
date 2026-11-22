# ADV-RELEASE-SMOKE-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Every native job of the release-distributions workflow runs the repository smoke script on its own freshly built bootstrap after the build step and before the bundle is retained, so a bootstrap broken on any target fails its own job and is never uploaded.

**Write.** `.github/workflows/release-distributions.yml`, `xtask/tests/release_workflow.rs`

**Check.** `cargo test -p xtask --test release_workflow 2>&1 | grep -F 'test result: ok. 1 passed'`

**Test.** xtask/tests/release_workflow.rs: every_build_target_smokes_its_bootstrap_before_retention (new file; today `cargo test -p xtask --test release_workflow` exits 101 with `no test target named `release_workflow` in `xtask` package`, so the grep finds nothing and the check fails).

**Read first.**
- .github/workflows/release-distributions.yml job `build`: steps `Build the Windows bundle`, `Build the POSIX bundle`, `Smoke Intel macOS portable-flow materialization`, `Retain the native bundle as a workflow artifact`
- distribution/release/smoke-bootstrap.sh (from ADV-RELEASE-SMOKE-1; its arguments are bootstrap path then registry dir)
- distribution/release/smoke-bootstrap.ps1 (from ADV-RELEASE-SMOKE-1)

**Steps.**
1. In .github/workflows/release-distributions.yml, replace the lines from `      - name: Smoke Intel macOS portable-flow materialization` through the blank line that follows `            | grep -F 'org.vibevm.world/wal'` with exactly the literal below. The literal already ends with the single blank line that sits before `      - name: Retain the native bundle as a workflow artifact`, so after the edit exactly one blank line separates the Windows smoke step from the Retain step:
```
      - name: Smoke the POSIX bootstrap
        if: ${{ runner.os != 'Windows' }}
        shell: bash
        run: sh distribution/release/smoke-bootstrap.sh "$GITHUB_WORKSPACE/dist/vibe-bootstrap-${{ matrix.target }}" "$GITHUB_WORKSPACE/vibevm/vibepacks"

      - name: Smoke the Windows bootstrap
        if: ${{ runner.os == 'Windows' }}
        shell: pwsh
        run: ./distribution/release/smoke-bootstrap.ps1 "$env:GITHUB_WORKSPACE/dist/vibe-bootstrap-${{ matrix.target }}.exe" "$env:GITHUB_WORKSPACE/vibevm/vibepacks"

```
2. Create xtask/tests/release_workflow.rs with exactly these bytes (std only):
```
//! The release workflow smokes the bootstrap of every build target with the
//! repository smoke scripts, after the native build and before the bundle is
//! retained for upload.

use std::fs;
use std::path::{Path, PathBuf};

const POSIX_SMOKE: &str = "      - name: Smoke the POSIX bootstrap
        if: ${{ runner.os != 'Windows' }}
        shell: bash
        run: sh distribution/release/smoke-bootstrap.sh \"$GITHUB_WORKSPACE/dist/vibe-bootstrap-${{ matrix.target }}\" \"$GITHUB_WORKSPACE/vibevm/vibepacks\"
";

const WINDOWS_SMOKE: &str = "      - name: Smoke the Windows bootstrap
        if: ${{ runner.os == 'Windows' }}
        shell: pwsh
        run: ./distribution/release/smoke-bootstrap.ps1 \"$env:GITHUB_WORKSPACE/dist/vibe-bootstrap-${{ matrix.target }}.exe\" \"$env:GITHUB_WORKSPACE/vibevm/vibepacks\"
";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn every_build_target_smokes_its_bootstrap_before_retention() {
    let root = repository_root();
    let workflow =
        fs::read_to_string(root.join(".github/workflows/release-distributions.yml")).unwrap();
    let build = workflow
        .find("      - name: Build the POSIX bundle\n")
        .expect("POSIX build step");
    let posix = workflow.find(POSIX_SMOKE).expect("POSIX smoke step");
    let windows = workflow.find(WINDOWS_SMOKE).expect("Windows smoke step");
    let retain = workflow
        .find("      - name: Retain the native bundle as a workflow artifact\n")
        .expect("retention step");
    assert!(
        build < posix && posix < windows && windows < retain,
        "smoke steps must run after the build and before retention"
    );
    assert!(
        !workflow.contains("matrix.target == 'x86_64-apple-darwin'"),
        "a step is still limited to Intel macOS"
    );
    for script in ["smoke-bootstrap.sh", "smoke-bootstrap.ps1"] {
        let path = root.join("distribution/release").join(script);
        assert!(path.is_file(), "missing {}", path.display());
    }
}
```
3. Run `cargo fmt --all`, then the check, then `cargo clippy -p xtask --tests -- -D warnings`.

**Decisions already made.**
- Smoke selection is by `runner.os`, not by `matrix.target`, so all five current targets (and the Linux aarch64 target a later ADV-RELEASE capability adds) are smoked with no further edit.
- Bootstrap path: `$GITHUB_WORKSPACE/dist/vibe-bootstrap-<target>` on POSIX and the same name plus `.exe` on Windows, matching xtask fn bootstrap_asset_name (`.exe` only for x86_64-pc-windows-msvc, the only Windows target).
- The musl and GNU Linux bootstraps are smoked directly on the ubuntu-24.04 runner; no container.
- Placement: both smoke steps go where the Intel macOS step is now, so a failing smoke fails the job before `Retain the native bundle as a workflow artifact` and the upload job never sees that target.
- Exactly one blank line separates the Windows smoke step from the Retain step; the literal carries it, and the old step's trailing blank line is part of the replaced range.
- No new workflow input; `checks`, `tests`, `self_check`, `upload` and `version` stay unchanged.
- The workflow file is stored LF everywhere (.gitattributes `* text=auto eol=lf`), so the byte-exact step literals in the test match on every OS.
- This is one atomic commit, landed before the aarch64, published-release-guard and attestation workflow edits (owner decision 4); those later entries edit the same workflow file and run after this one.

**Not in this entry.** Not in this entry: adding the Linux aarch64 matrix row, attestation steps, a published-release guard, workflow_dispatch runs, or any change to the build-*.sh/.ps1 scripts or the upload/finalize jobs.

**Depends on.** ADV-RELEASE-SMOKE-1

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
