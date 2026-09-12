# Developer Guide

Contributor-facing setup: what to install on a fresh machine to clone the repo, build the CLI, run the tests, run manual smokes, and publish packages (if authorized).

For end-user setup (how to *use* the shipped `vibe` CLI), see [`RUNTIME-GUIDE.md`](RUNTIME-GUIDE.md).

**Update policy.** Every change touching toolchain, prerequisites, env vars, or bootstrap steps MUST update this file in the same commit. Never ship a dev-env change and a doc update separately. Policy pinned in [PROP-000](vibevm/vibespecs/common/PROP-000.xml) — the obligation is load-bearing.

---

## 1. Supported platforms

- **Primary dev:** Windows 11 + Git Bash (the machine of record for this project).
- **Also supported:** macOS 12+, Linux (any recent glibc distro).

## 2. Prerequisites

### 2.1 Rust toolchain

Pinned in [`rust-toolchain.toml`](rust-toolchain.toml). Install rustup from <https://rustup.rs>, clone the repo, and the first `cargo` invocation picks up the pinned toolchain automatically.

### 2.2 git

System `git` must be in `PATH`. `vibe-registry` shells out to `git` for all registry operations — see [PROP-001 §2.1](vibevm/vibespecs/modules/vibe-registry/PROP-001-git-backend.xml#backend).

- Windows: [Git for Windows](https://git-scm.com/download/win). Bundled OpenSSH works with GitVerse out of the box once your key is in `ssh-agent`.
- macOS: `brew install git` or Xcode command-line tools.
- Linux: your distro's `git` package.

Verify with `git --version`.

### 2.3 SSH key for GitVerse (required to push vibevm itself)

Needed to push to `git@gitverse.ru:vibevm/vibevm.git` — the project source-of-truth repo lives on GitVerse. Load the key into `ssh-agent`, verify with `ssh -T git@gitverse.ru` — it should confirm auth and exit without a shell.

### 2.4 Publish token (required for `vibe registry publish`)

The package registry organization (`vibespecs`) lives on GitHub at <https://github.com/vibespecs>. Publish-side ops require a GitHub personal access token (PAT) with `repo` scope on the org, stored at:

- POSIX: `~/.vibe/github.publish.token`
- Windows: `%USERPROFILE%\.vibe\github.publish.token`

The publish-token loader also accepts the legacy `~/.vibe/git.publish.token` (host-agnostic fallback) and the env-var `VIBEVM_PUBLISH_TOKEN` (wins over both). Per-host file precedence — `~/.vibe/<host-prefix>.publish.token` — exists so you can hold tokens for several hosts without juggling env vars. The pre-consolidation `~/.vibevm/` is no longer read (removed 2026-07-26): if a token still lives there, move it into `~/.vibe/` yourself — `vibe` will not find it, and never reads or moves anything in that directory.

**Token files are surface secrets per [PROP-000 §20](vibevm/vibespecs/common/PROP-000.xml#token-secrecy):**

- chmod 600 / Windows ACL-restricted to your user.
- Never committed to git, never pasted into chat, never echoed in shell snippets, never quoted in screenshots or recordings.
- `vibe` itself redacts the token at every output surface — CLI step lines, `--json` events, error messages, debug logs. The CLI prints the *source* of the token (env-var name or file path) but never the value. Maintain that discipline at the operator level too.

Needed only for the publish subcommand; ordinary install/update never touches a token.

### 2.5 Schema codegen (JTD)

JTD (JSON Type Definition, RFC 8927) is the source of truth for every wire contract in the project ([PROP-000 §16](vibevm/vibespecs/common/PROP-000.xml#jtd)). `jtd-codegen` generates Rust types (and, eventually, other-language client types) from the `*.jtd.json` schemas at the repo root under [`schemas/`](schemas/) into [`crates/vibe-wire/src/generated/`](crates/vibe-wire/src/generated/).

**Install** the generator binary into the project-local `tools/jtd-codegen/` per the procedure in [`tools/jtd-codegen/README.md`](tools/jtd-codegen/README.md). The binary itself is gitignored; only the README travels with the repo.

**Regenerate** types after editing schemas:

```sh
cargo xtask codegen
```

**Drift check** (CI runs this):

```sh
cargo xtask check-codegen
```

The xtask reports an actionable error if `jtd-codegen` is not on PATH or in `tools/jtd-codegen/`.

## 3. Build / test / lint

From repo root:

```
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

81 tests green on `main` as of the last checkpoint; clippy clean with `-D warnings`.

### 3.1 Quick-reinstall shortcut (optional)

While iterating on `vibe-cli` you'll typically rebuild + reinstall many times a session. There are two paths, with very different cost / coverage trade-offs.

#### Two modes, when to use which

**Fast (default — for the iteration loop):**

```
cargo build -p vibe-cli
cp target/debug/vibe(.exe) ~/.cargo/bin/
```

`cargo build -p vibe-cli` uses the regular `target/debug/` cache with **incremental compilation** — only what you actually changed gets recompiled. Then we just copy the resulting binary over `~/.cargo/bin/vibe(.exe)` (the path `cargo install` would put it). Total: 5–30 seconds depending on the change. The binary is bigger and microseconds slower at startup, but for a CLI that runs in fractions of a second that's invisible.

Use this in your edit-build-test loop. It is **always safe** for behaviour — the binary is the same logic, just compiled with `-O0` and without LTO.

**Release (occasional — before commits / pushes):**

```
cargo install --path crates/vibe-cli --locked
```

Goes through `[profile.release]`: `lto = "thin"` + `codegen-units = 1` + `strip = "symbols"`, all single-threaded final stages. 1–3 minutes on a clean cache (and `cargo install` keeps a separate cache from `cargo build`, so it usually IS clean).

Use this:
- Before pushing — to confirm the release-build still compiles cleanly.
- Before tagging a release — same reason, plus to install the actual release-shaped binary you'd ship.
- When debugging a release-only issue — sometimes optimizations expose UB or expose a different code path.

#### PowerShell (Windows)

PowerShell aliases (`Set-Alias`) can't accept arguments or chain commands — use a function plus a short alias.

If `$PROFILE` doesn't yet exist (visible as `notepad $PROFILE` opening nothing, or `Test-Path $PROFILE` returning `False`), create it once:

```powershell
New-Item -Path $PROFILE -ItemType File -Force
```

`-Force` also creates the parent directory (`Documents\PowerShell\` for PS 7+, `Documents\WindowsPowerShell\` for PS 5.1) if it's absent.

Then add to `$PROFILE`:

```powershell
# Default assumes the repo is at ~/gits/vibevm. If it's elsewhere,
# replace the right-hand side with an absolute path like
# 'D:\src\vibevm'. Do NOT leave a placeholder like <you> in the
# string — Windows treats `<>` as illegal path characters and
# Test-Path will throw before reaching the existence check.
$env:VIBEVM_REPO = "$HOME\gits\vibevm"

function Update-Vibe {
    [CmdletBinding()]
    param(
        # -Release: `cargo install --path` (release profile, LTO, 1-3 min).
        # Default is fast: `cargo build` + copy of the debug binary.
        [switch]$Release,
        # -Refresh: after a successful build, run `vibe mcp upgrade --yes`
        # so SKILL.md / MCP-config in every wired agent get resynced to
        # the freshly-built binary.
        [switch]$Refresh
    )
    if (-not (Test-Path $env:VIBEVM_REPO)) {
        Write-Error "VIBEVM_REPO ($env:VIBEVM_REPO) does not exist"
        return
    }
    Push-Location $env:VIBEVM_REPO
    try {
        $ok = $false
        if ($Release) {
            cargo install --path crates/vibe-cli --locked
            $ok = ($LASTEXITCODE -eq 0)
        } else {
            cargo build -p vibe-cli
            if ($LASTEXITCODE -eq 0) {
                $src = Join-Path $env:VIBEVM_REPO 'target\debug\vibe.exe'
                $dst = Join-Path $env:USERPROFILE '.cargo\bin\vibe.exe'
                # Copy-Item is a PowerShell cmdlet — it does NOT update
                # $LASTEXITCODE on failure (that's nativ-command-only).
                # Wrap in try/catch so a locked destination (vibe.exe
                # held open by a running `vibe mcp serve` from your
                # opencode / Claude session, etc.) surfaces loudly
                # instead of silently leaving a stale binary in place.
                try {
                    Copy-Item -LiteralPath $src -Destination $dst -Force -ErrorAction Stop
                    Write-Host "[OK] vibe.exe -> $dst" -ForegroundColor Green
                    $ok = $true
                } catch {
                    Write-Error "Copy-Item failed: $_"
                    Write-Error "Hint: another process may hold vibe.exe -- close opencode / Claude / any shell currently running vibe and retry."
                }
            }
        }
        if ($ok -and $Refresh) {
            vibe mcp upgrade --yes --invoked-by powershell-update-vibe
        }
    } finally {
        Pop-Location
    }
}
Set-Alias vu Update-Vibe
```

Reload the profile in the current session (or open a new window):

```powershell
. $PROFILE
```

Usage:

- `vu` — fast (debug build + copy). Default for the iteration loop.
- `vu -Release` (or `vu -R`) — full release-mode `cargo install`. Use before pushing / tagging.
- `vu -Refresh` (or `vu -r`) — fast build + `vibe mcp upgrade` to resync agent integrations.
- `vu -R -r` — release build + agent resync.

PowerShell accepts unambiguous parameter-name prefixes, so `vu -Re` would be ambiguous (matches both `-Release` and `-Refresh`) — use `-R` / `-r` (single-letter) or the full names.

If PowerShell refuses to load the profile with `running scripts is disabled on this system`, allow user-scope scripts once: `Set-ExecutionPolicy -Scope CurrentUser RemoteSigned`.

**Encoding gotcha.** Windows PowerShell 5.1 reads `.ps1` files as ANSI codepage when they have no BOM. Notepad (default save) writes ANSI / UTF-8-no-BOM, so any non-ASCII character pasted into `$PROFILE` (smart quotes, em-dashes, check marks) becomes mojibake and breaks the parser ("string is missing the terminator"). The snippet above is intentionally ASCII-only to avoid this. If you must include non-ASCII (e.g. Cyrillic comments), save the profile as **UTF-8 with BOM** — in notepad: File → Save As → Encoding → "UTF-8 with BOM".

#### Bash / zsh (macOS, Linux, Git Bash)

Add to `~/.bashrc` / `~/.zshrc`:

```bash
# Default assumes the repo is at ~/gits/vibevm. If it's elsewhere,
# replace the right-hand side with an absolute path.
export VIBEVM_REPO="$HOME/gits/vibevm"

vu() {
    [ -d "$VIBEVM_REPO" ] || { echo "VIBEVM_REPO ($VIBEVM_REPO) does not exist" >&2; return 1; }
    local mode="fast" refresh=false arg
    for arg in "$@"; do
        case "$arg" in
            --release|-R) mode="release" ;;
            --refresh|-r) refresh=true ;;
            *) echo "vu: unknown arg '$arg'" >&2; return 1 ;;
        esac
    done
    local ext=""
    case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) ext=".exe" ;; esac
    if [ "$mode" = "release" ]; then
        ( cd "$VIBEVM_REPO" && cargo install --path crates/vibe-cli --locked ) || return $?
    else
        ( cd "$VIBEVM_REPO" && cargo build -p vibe-cli ) || return $?
        # cp's exit status surfaces destination-locked errors
        # (busy on Windows when a process holds vibe.exe).
        cp "$VIBEVM_REPO/target/debug/vibe$ext" "$HOME/.cargo/bin/vibe$ext" \
            && echo "[OK] vibe$ext -> $HOME/.cargo/bin/vibe$ext" \
            || { echo "vu: cp failed -- close any running vibe / vibe mcp serve and retry" >&2; return 1; }
    fi
    if [ "$refresh" = true ]; then
        vibe mcp upgrade --yes --invoked-by shell-update-vibe
    fi
}
```

Reload: `source ~/.bashrc` (or `~/.zshrc`).

Usage:

- `vu` — fast (default).
- `vu --release` / `vu -R` — full release-mode install.
- `vu --refresh` / `vu -r` — fast build + agent resync.
- `vu -R -r` — release build + agent resync.

The `( … )` subshell handles CWD restoration automatically — exit the subshell, you're back where you started. The `|| return $?` propagates a build failure so the optional refresh step never runs against a stale binary.

### 3.2 IDE setup — rust-analyzer (the Rust LSP)

[rust-analyzer](https://rust-analyzer.github.io/) is the recommended language server for Rust: inline types, go-to-definition, inlay hints, completion, and clippy-on-save. It runs the same `cargo` / `clippy` pipeline §3 documents, just inside your editor.

**Install.** The server ships with the toolchain: `rustup component add rust-analyzer`. Editor bindings: VS Code / VSCodium — the official *rust-analyzer* extension; Neovim / Emacs / Helix / JetBrains (via the Rust plugin) all drive the same `rust-analyzer` binary.

**Multi-workspace layout — the one gotcha.** This repo is a monorepo of **several Cargo workspaces** (PROP-024). The root workspace ([`Cargo.toml`](Cargo.toml)) excludes `vibevm/vibepacks/` and `vibevm/vibedeps/`, and those carry their *own* workspaces — `vibevm/vibepacks/org.vibevm.fractality/fractality/v1.0.0/`, the AI-native stacks under `vibevm/vibepacks/org.vibevm.ai-native/…`, and the materialized copies under `vibevm/vibedeps/`. So:

- Opening the **repo root** indexes the host workspace (`crates/`, `xtask`, `apps/`). That is what you want for `vibe-cli` work.
- To edit a **nested workspace** (fractality, the discipline stacks, an mcp package), open that subdirectory in a second editor window, or add its `Cargo.toml` to your editor's linked-projects list (VS Code: `rust-analyzer.linkedProjects`). Pointing the root workspace at a nested one slows indexing and confuses `cargo metadata`.

**Recommended settings** — match the project gate (§6: clippy with `-D warnings`, all targets). VS Code / VSCodium `settings.json`:

```jsonc
{
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.check.extraArgs": ["--all-targets"],
  "rust-analyzer.cargo.allTargets": true,
  "rust-analyzer.linkedProjects": []
  // add nested-workspace Cargo.toml paths here when you edit them
}
```

Or a project-local `rust-analyzer.toml` at the repo root (read by recent rust-analyzer):

```toml
[check]
command = "clippy"
extraArgs = ["--all-targets"]

[cargo]
allTargets = true
```

**Spec anchors.** Every code cell carries `specmark::scope!` / `#[spec(...)]` / `#[verifies(...)]` citing `spec://…` URIs. rust-analyzer's go-to-definition and rename treat these like any other macro; the URI strings are the source of truth — edit the spec, not the strings (see the Addressable Specs flow in [`vibevm/vibespecs/boot/STATIC.xml`](vibevm/vibespecs/boot/STATIC.xml)). The `vibe trace` command (a delegating alias to `rust-ai-native trace`) resolves a URI to its definition.

**Toolchain note.** Pinned to edition 2024 / stable ≥ 1.93 ([`rust-toolchain.toml`](rust-toolchain.toml)). A rust-analyzer release from the last few months is current enough; if inlay hints misrender or macros error, `rustup update` and update the editor extension.

## 4. Manual smoke-tests

Live integration scripts live under [`manual-tests/`](manual-tests/). One file per scenario, self-contained walkthrough with clean-slate setup and teardown. Read [`manual-tests/README.md`](manual-tests/README.md) for the authoring conventions. Run the relevant script before tagging any milestone and after any change to an integration surface (git backend, CLI args, lockfile schema).

## 5. Publishing packages (maintainers only)

`vibe registry publish <path>` is the maintainer tool for creating a package repo in the configured registry's organization and pushing a tagged release. The CLI dispatches to a host-specific `RepoCreator` adapter chosen from the registry URL's hostname:

- `github.com` (or any subdomain) → `GitHubCreator`. `POST /orgs/{org}/repos` works natively; HTTPS push uses the token embedded in the URL for one push (modern git ≥ 2.31 redacts URL passwords in its own logs).
- `gitverse.ru` → `GitVerseCreator`. `GET /repos/{owner}/{repo}` works for presence; `POST /orgs/{org}/repos` is not exposed by the live host (verified 2026-04-26), so create-leg requires manual web-UI pre-creation. The adapter remains in tree for any future Gitea-shape host that fully supports the org-scoped POST.

Full design: [PROP-002 §2.10](vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml#publish). User-facing reference: [`docs/commands/registry-publish.md`](docs/commands/registry-publish.md).

**Routine usage:**

```sh
# Dry-run first (read-only — only hits GET /repos/...).
cargo run --release -p vibe-cli -- registry publish vibevm/vibepacks/org.vibevm.world/wal/v1.0.0 --dry-run

# Apply.
cargo run --release -p vibe-cli -- registry publish vibevm/vibepacks/org.vibevm.world/wal/v1.0.0
```

The dry-run output shows the synthetic clone URL, the action verb (`Would create` or `Would reuse existing`), and the tag that would be pushed. No token value appears anywhere in output — `vibe` reads the token in-process, redacts on `Display`/`Debug`, and never logs the value. The video-recording-safe defaults are baked in.

## 6. Affected checks and the full integration panel

For an ordinary edit, identify changed behavior and its actual contract consumers. Select the Cargo build/test target first, then the relevant regression cases. A name filter without `--lib`, `--test <target>` or another explicit target can still compile unrelated test executables. Inspect source and Cargo metadata before invoking a target-specific test list; confirm that the intended nonzero set actually runs. Do not use a whole-crate test/list invocation as a discovery shortcut.

For example, after replacing the placeholders with existing target and case names:

```sh
cargo test -p <package> --test <integration-target> <case-name> -- --exact
cargo test -p <package> --lib <affected-module-prefix>
```

Include affected negative cases, invariants and consumer integration checks. Schema/codegen, wire-corpus, spec and exhaustive-facts oracles apply when their actual inputs/contracts changed; a broad allowed write perimeter does not trigger them. Keep compatible incremental build artifacts. Build-cache reuse does not reuse a test verdict automatically.

The full integration/release entry point remains:

```sh
bash tools/self-check.sh
```

It runs the complete stage list declared in `tools/self-check.sh`: host workspace tests/lints, spec/conformance/codegen/wire checks and the independently shipped package workspaces. It exits on the first failure unless `--keep-going` is passed. This is an expensive panel, not a per-commit, per-worker or per-milestone requirement.

Use affected checks during development and integration. Run the whole panel for the designated final campaign/release gate, an explicit full-check request, or demonstrated cross-cutting impact for which narrower proof is inadequate. Completing an atom, touching two crates, preparing a push or ending a milestone is not by itself that reason.

After a failure, rerun the failed and affected steps and execute the never-run tail. Reuse a successful step only with evidence for the same relevant inputs, dependencies, command, toolchain and environment. Account for the complete required stage set. A stitched coverage report must not be called a successful full-script invocation. The script itself has no resume option; direct constituent commands or a justified full invocation are the available mechanisms.

For the spec linter alone, invoke `vibe check --path <affected-project-or-package-root>` using a binary whose source provenance is appropriate for the check. A checked-in or prebuilt executable is not automatically current. The root project uses installed dependencies; keep its authored manifest/lock and regenerate derived slots/boot through supported scoped operations.

The installed multi-user-planning flow's `verification-selection` contract specifies target selection, escalation and evidence reuse. A campaign binds its task recipes and designated final gate to that permanent contract.

## 7. Troubleshooting

(Populated as real issues arise. Empty today.)
