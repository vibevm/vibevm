# ADV-RELEASE-SMOKE-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A maintainer can run `sh distribution/release/smoke-bootstrap.sh <bootstrap> <registry>` (or `smoke-bootstrap.ps1` under pwsh on Windows) against any built vibe: it exits 0 and prints the tree row `org.vibevm.world/wal  dynamic  .  .  .` then `smoke-bootstrap: ok` after init, install of flow:org.vibevm.world/wal and tree, and exits 1 with `smoke-bootstrap: install failed` when the registry does not hold the package.

**Write.** `distribution/release/smoke-bootstrap.sh`, `distribution/release/smoke-bootstrap.ps1`, `crates/vibe-cli/tests/release_smoke_script.rs`

**Check.** `cargo test -p vibe-cli --test release_smoke_script 2>&1 | grep -F 'test result: ok. 2 passed'`

**Test.** crates/vibe-cli/tests/release_smoke_script.rs: smoke_script_installs_wal_with_the_built_vibe and smoke_script_fails_at_install_with_an_empty_registry (new file; today `cargo test -p vibe-cli --test release_smoke_script` exits 101 with no such test target, so the grep finds nothing and the check fails).

**Read first.**
- .github/workflows/release-distributions.yml step `Smoke Intel macOS portable-flow materialization` (the commands being moved into a script)
- distribution/release/build-linux-x86_64.sh (house style of the release shell scripts: `#!/usr/bin/env sh`, `set -eu`, printf to stderr)
- xtask/src/dist/build.rs fn bootstrap_asset_name (the bootstrap is the built `vibe` binary, named `vibe-bootstrap-<target>` plus `.exe` only for x86_64-pc-windows-msvc)
- crates/vibe-cli/tests/cli_self_release_sources.rs (integration-test file conventions in this crate)

**Steps.**
1. Create distribution/release/smoke-bootstrap.sh with exactly these bytes (file mode 100644 like the other scripts in distribution/release; it is always run as `sh <script>`):
```
#!/usr/bin/env sh
# Smoke one built vibe bootstrap: init a scratch project, install the wal flow
# from a local registry, and require the tree to list it.
# usage: smoke-bootstrap.sh <bootstrap-path> <registry-dir>
set -eu

if [ "$#" -ne 2 ]; then
    printf '%s\n' 'usage: smoke-bootstrap.sh <bootstrap-path> <registry-dir>' >&2
    exit 2
fi
vibe=$1
registry=$2
if [ ! -f "$vibe" ]; then
    printf 'smoke-bootstrap: bootstrap not found: %s\n' "$vibe" >&2
    exit 2
fi
case $vibe in
    /*) ;;
    *) vibe=$PWD/$vibe ;;
esac
chmod 0755 "$vibe"

work=$(mktemp -d "${TMPDIR:-/tmp}/vibevm-smoke.XXXXXX")
cleanup() {
    rm -rf -- "$work"
}
trap cleanup 0 HUP INT TERM
project=$work/project
settings=$work/settings
mkdir -- "$project" "$settings"

VIBE_SETTINGS=$settings "$vibe" init --path "$project" --no-registry --no-progress || {
    status=$?
    printf 'smoke-bootstrap: init failed (exit %s)\n' "$status" >&2
    exit 1
}
VIBE_SETTINGS=$settings "$vibe" install flow:org.vibevm.world/wal --path "$project" \
    --registry "$registry" --assume-yes --no-progress || {
    status=$?
    printf 'smoke-bootstrap: install failed (exit %s)\n' "$status" >&2
    exit 1
}
VIBE_SETTINGS=$settings "$vibe" tree --path "$project" --plain > "$work/tree.txt" || {
    status=$?
    printf 'smoke-bootstrap: tree failed (exit %s)\n' "$status" >&2
    exit 1
}
if ! grep -F 'org.vibevm.world/wal' "$work/tree.txt"; then
    printf '%s\n' 'smoke-bootstrap: tree does not list org.vibevm.world/wal' >&2
    exit 1
fi
printf '%s\n' 'smoke-bootstrap: ok'
```
2. Create distribution/release/smoke-bootstrap.ps1 with exactly these bytes (written for and run under pwsh; no `&&`, no ternary):
```
# Smoke one built vibe bootstrap: init a scratch project, install the wal flow
# from a local registry, and require the tree to list it.
# usage: smoke-bootstrap.ps1 <bootstrap-path> <registry-dir>
param(
    [Parameter(Mandatory = $true, Position = 0)][string]$Bootstrap,
    [Parameter(Mandatory = $true, Position = 1)][string]$Registry
)
$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $Bootstrap -PathType Leaf)) {
    [Console]::Error.WriteLine("smoke-bootstrap: bootstrap not found: $Bootstrap")
    exit 2
}
$vibe = (Resolve-Path -LiteralPath $Bootstrap).Path
$work = Join-Path ([IO.Path]::GetTempPath()) ('vibevm-smoke-' + [Guid]::NewGuid().ToString('N'))
$project = Join-Path $work 'project'
$settings = Join-Path $work 'settings'
New-Item -ItemType Directory -Path $project -Force | Out-Null
New-Item -ItemType Directory -Path $settings -Force | Out-Null
$env:VIBE_SETTINGS = $settings
$code = 0
try {
    & $vibe init --path $project --no-registry --no-progress
    if ($LASTEXITCODE -ne 0) { throw "init failed (exit $LASTEXITCODE)" }
    & $vibe install flow:org.vibevm.world/wal --path $project --registry $Registry --assume-yes --no-progress
    if ($LASTEXITCODE -ne 0) { throw "install failed (exit $LASTEXITCODE)" }
    $tree = & $vibe tree --path $project --plain
    if ($LASTEXITCODE -ne 0) { throw "tree failed (exit $LASTEXITCODE)" }
    $listed = @($tree | Where-Object { $_.Contains('org.vibevm.world/wal') })
    if ($listed.Count -eq 0) { throw 'tree does not list org.vibevm.world/wal' }
    $listed | ForEach-Object { Write-Output $_ }
    Write-Output 'smoke-bootstrap: ok'
} catch {
    [Console]::Error.WriteLine("smoke-bootstrap: $($_.Exception.Message)")
    $code = 1
} finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
exit $code
```
3. Create crates/vibe-cli/tests/release_smoke_script.rs with exactly these bytes (std plus the existing dev-dependency `tempfile`; no `mod common`):
```
//! The release smoke script (`distribution/release/smoke-bootstrap.sh`, and
//! `smoke-bootstrap.ps1` on Windows) run against the freshly built `vibe`:
//! it passes with the repository registry and stops at install with an empty
//! one. No test reaches the network.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_smoke(registry: &Path) -> Output {
    let scripts = repository_root().join("distribution/release");
    let mut command = if cfg!(windows) {
        let mut command = Command::new("pwsh");
        command
            .args(["-NoLogo", "-NoProfile", "-NonInteractive"])
            .args(["-ExecutionPolicy", "Bypass", "-File"])
            .arg(scripts.join("smoke-bootstrap.ps1"));
        command
    } else {
        let mut command = Command::new("sh");
        command.arg(scripts.join("smoke-bootstrap.sh"));
        command
    };
    command
        .arg(env!("CARGO_BIN_EXE_vibe"))
        .arg(registry)
        .output()
        .unwrap()
}

#[test]
fn smoke_script_installs_wal_with_the_built_vibe() {
    let output = run_smoke(&repository_root().join("vibevm/vibepacks"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("org.vibevm.world/wal  dynamic"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("smoke-bootstrap: ok"), "stdout={stdout}");
}

#[test]
fn smoke_script_fails_at_install_with_an_empty_registry() {
    let registry = tempfile::tempdir().unwrap();
    let output = run_smoke(registry.path());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr={stderr}");
    assert!(
        stderr.contains("smoke-bootstrap: install failed"),
        "stderr={stderr}"
    );
}
```
4. Run `cargo fmt --all` (if it reflows the test file, keep its formatting), then the check; also run `cargo clippy -p vibe-cli --tests -- -D warnings` (the file uses `cfg!` not `#[cfg]`, so both branches compile everywhere and nothing is dead).

**Decisions already made.**
- Package reference is the full name `flow:org.vibevm.world/wal`, not today's `flow:wal`: on the current debug build `vibe install flow:wal` exits 1 with `the short name `wal` is not issued by any consulted registry; give the full name`, while `flow:org.vibevm.world/wal` installs and `tree --plain` ends with the row `org.vibevm.world/wal  dynamic  .  .  .`. The existing Intel macOS CI smoke therefore fails at HEAD; this script is its corrected form.
- The success test asserts the tree row text `org.vibevm.world/wal  dynamic` (two spaces), not the bare package name: init's hint, the install line and the closure diff already print `org.vibevm.world/wal` to stdout before tree runs, so only the `dynamic` column proves the script's tree grep printed the row.
- Exit codes: the .sh exits 2 on wrong arity; both scripts exit 2 when the bootstrap file is missing; a .ps1 called without both arguments fails pwsh parameter binding (exit 1, not 2) and that is left as is. Any failed vibe step or a tree without the package exits 1 with `smoke-bootstrap: <step> failed (exit N)` or `smoke-bootstrap: tree does not list org.vibevm.world/wal` on stderr; success prints the matching tree line(s) then `smoke-bootstrap: ok` on stdout.
- The tree output goes to a file and is grepped from there, never `tree | grep`, because POSIX sh has no pipefail and a failing tree would be masked.
- Isolation: a fresh temp work dir holds the project and the VIBE_SETTINGS root, removed on exit; no settings home outside it is touched.
- No skip switch, no evidence file, no Docker/container: the output stays on stdout/stderr.
- The broken case is an empty temporary registry directory (install fails fast with `package `org.vibevm.world/wal` is not available in any configured registry`, no network).
- The test lives in vibe-cli because only that crate gets `CARGO_BIN_EXE_vibe`; it picks `pwsh` + the .ps1 on Windows and `sh` + the .sh elsewhere, so the same check proves the native script on each OS. On Windows the check needs `pwsh` on PATH; Windows PowerShell 5.1 is not a target.

**Not in this entry.** Not in this entry: editing .github/workflows/release-distributions.yml (that is ADV-RELEASE-SMOKE-2), adding a `--skip-smoke` flag or evidence output, calling the smoke from the build-*.sh/.ps1 scripts or xtask dist build, or making the .ps1 exit 2 on missing arguments.

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
