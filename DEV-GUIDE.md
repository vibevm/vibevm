# Developer Guide

Contributor-facing setup: what to install on a fresh machine to clone the repo, build the CLI, run the tests, run manual smokes, and publish packages (if authorized).

For end-user setup (how to *use* the shipped `vibe` CLI), see [`RUNTIME-GUIDE.md`](RUNTIME-GUIDE.md).

**Update policy.** Every change touching toolchain, prerequisites, env vars, or bootstrap steps MUST update this file in the same commit. Never ship a dev-env change and a doc update separately. Policy pinned in [PROP-000](spec/common/PROP-000.md) — the obligation is load-bearing.

---

## 1. Supported platforms

- **Primary dev:** Windows 11 + Git Bash (the machine of record for this project).
- **Also supported:** macOS 12+, Linux (any recent glibc distro).

## 2. Prerequisites

### 2.1 Rust toolchain

Pinned in [`rust-toolchain.toml`](rust-toolchain.toml). Install rustup from <https://rustup.rs>, clone the repo, and the first `cargo` invocation picks up the pinned toolchain automatically.

### 2.2 git

System `git` must be in `PATH`. `vibe-registry` shells out to `git` for all registry operations — see [PROP-001 §2.1](spec/modules/vibe-registry/PROP-001-git-backend.md#backend).

- Windows: [Git for Windows](https://git-scm.com/download/win). Bundled OpenSSH works with GitVerse out of the box once your key is in `ssh-agent`.
- macOS: `brew install git` or Xcode command-line tools.
- Linux: your distro's `git` package.

Verify with `git --version`.

### 2.3 SSH key for GitVerse (required to push vibevm itself)

Needed to push to `git@gitverse.ru:anarchic/vibevm.git` — the project source-of-truth repo lives on GitVerse. Load the key into `ssh-agent`, verify with `ssh -T git@gitverse.ru` — it should confirm auth and exit without a shell.

### 2.4 Publish token (required for `vibe registry publish`)

The package registry organization (`vibespecs`) lives on GitHub at <https://github.com/vibespecs>. Publish-side ops require a GitHub personal access token (PAT) with `repo` scope on the org, stored at:

- POSIX: `~/.vibevm/github.publish.token`
- Windows: `%USERPROFILE%\.vibevm\github.publish.token`

The publish-token loader also accepts the legacy `~/.vibevm/git.publish.token` (host-agnostic fallback) and the env-var `VIBEVM_PUBLISH_TOKEN` (wins over both). Per-host file precedence — `~/.vibevm/<host-prefix>.publish.token` — exists so you can hold tokens for several hosts without juggling env vars.

**Token files are surface secrets per [PROP-000 §20](spec/common/PROP-000.md#token-secrecy):**

- chmod 600 / Windows ACL-restricted to your user.
- Never committed to git, never pasted into chat, never echoed in shell snippets, never quoted in screenshots or recordings.
- `vibe` itself redacts the token at every output surface — CLI step lines, `--json` events, error messages, debug logs. The CLI prints the *source* of the token (env-var name or file path) but never the value. Maintain that discipline at the operator level too.

Needed only for the publish subcommand; ordinary install/update never touches a token.

### 2.5 Schema codegen (JTD)

JTD (JSON Type Definition, RFC 8927) is the source of truth for every wire contract in the project ([PROP-000 §16](spec/common/PROP-000.md#jtd)). `jtd-codegen` generates Rust types (and, eventually, other-language client types) from the `*.jtd.json` schemas at the repo root under [`schemas/`](schemas/) into [`crates/vibe-wire/src/generated/`](crates/vibe-wire/src/generated/).

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
$env:VIBEVM_REPO = 'C:\Users\<you>\gits\vibevm'   # adjust path

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
        if ($Release) {
            cargo install --path crates/vibe-cli --locked
        } else {
            cargo build -p vibe-cli
            if ($LASTEXITCODE -eq 0) {
                Copy-Item target\debug\vibe.exe `
                    "$env:USERPROFILE\.cargo\bin\vibe.exe" -Force
            }
        }
        if ($LASTEXITCODE -eq 0 -and $Refresh) {
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

#### Bash / zsh (macOS, Linux, Git Bash)

Add to `~/.bashrc` / `~/.zshrc`:

```bash
export VIBEVM_REPO="$HOME/gits/vibevm"   # adjust path

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
    if [ "$mode" = "release" ]; then
        ( cd "$VIBEVM_REPO" && cargo install --path crates/vibe-cli --locked ) || return $?
    else
        ( cd "$VIBEVM_REPO" \
            && cargo build -p vibe-cli \
            && cp "target/debug/vibe$([ "$(uname -s)" = "MINGW"* ] && echo .exe)" \
                  "$HOME/.cargo/bin/vibe$([ "$(uname -s)" = "MINGW"* ] && echo .exe)" \
        ) || return $?
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

## 4. Manual smoke-tests

Live integration scripts live under [`manual-tests/`](manual-tests/). One file per scenario, self-contained walkthrough with clean-slate setup and teardown. Read [`manual-tests/README.md`](manual-tests/README.md) for the authoring conventions. Run the relevant script before tagging any milestone and after any change to an integration surface (git backend, CLI args, lockfile schema).

## 5. Publishing packages (maintainers only)

`vibe registry publish <path>` is the maintainer tool for creating a package repo in the configured registry's organization and pushing a tagged release. The CLI dispatches to a host-specific `RepoCreator` adapter chosen from the registry URL's hostname:

- `github.com` (or any subdomain) → `GitHubCreator`. `POST /orgs/{org}/repos` works natively; HTTPS push uses the token embedded in the URL for one push (modern git ≥ 2.31 redacts URL passwords in its own logs).
- `gitverse.ru` → `GitVerseCreator`. `GET /repos/{owner}/{repo}` works for presence; `POST /orgs/{org}/repos` is not exposed by the live host (verified 2026-04-26), so create-leg requires manual web-UI pre-creation. The adapter remains in tree for any future Gitea-shape host that fully supports the org-scoped POST.

Full design: [PROP-002 §2.10](spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish). User-facing reference: [`docs/commands/registry-publish.md`](docs/commands/registry-publish.md).

**Routine usage:**

```sh
# Dry-run first (read-only — only hits GET /repos/...).
cargo run --release -p vibe-cli -- registry publish fixtures/registry/flow/wal/v0.1.0 --dry-run

# Apply.
cargo run --release -p vibe-cli -- registry publish fixtures/registry/flow/wal/v0.1.0
```

The dry-run output shows the synthetic clone URL, the action verb (`Would create` or `Would reuse existing`), and the tag that would be pushed. No token value appears anywhere in output — `vibe` reads the token in-process, redacts on `Display`/`Debug`, and never logs the value. The video-recording-safe defaults are baked in.

## 6. Self-check (`vibe check` against the vibevm tree)

vibevm is its own bootstrap project: the repo root carries a minimal `vibe.toml` plus an empty-`[[package]]` `vibe.lock` so the shipped `vibe check` linter can run against the same `spec/` corpus the tool itself produces. The manifest does not declare any installed packages — vibevm is the tool, not a consumer of itself today; full self-hosting under `packages/` lands post-M1.

The canonical entry point is the bundled script:

```sh
bash tools/self-check.sh
```

It runs three invariants in order, exiting non-zero on the first failure (pass `--keep-going` to run all three regardless):

1. `cargo test --workspace` — every test green.
2. `cargo clippy --workspace --all-targets -- -D warnings` — zero warnings, treated as errors.
3. `cargo run -p vibe-cli -- check --path . --quiet` — spec linter on the bootstrap manifest. Expected output: `vibe check: 0 errors, 0 warnings, 0 info`.

If you only want the spec linter without the build/test prelude, run step 3 directly. Note: pre-built binaries under `target/release/` and `target/debug/` may be out of date relative to the source tree (e.g. built before a subcommand was added); the script always goes through `cargo run` so the binary is guaranteed to match `HEAD`.

CI wiring: a single `bash tools/self-check.sh` line is enough. Local development: run before opening a PR; for quick iteration during a feature, run the relevant slice directly (`cargo test -p vibe-foo`) and reserve `self-check.sh` for "is the tree shippable right now?".

If you `vibe install <pkgref>` against this manifest by accident, the install will succeed — there are no boot-prefix collisions today (`spec/boot/` carries only `00-core.md` and `90-user.md`). It will, however, materialise package files into `spec/flows/`, `spec/feats/`, or `spec/stacks/` and rewrite `vibe.lock` with `[[package]]` entries; revert with `vibe uninstall` (or `git restore vibe.lock spec/`) before committing.

## 7. Troubleshooting

(Populated as real issues arise. Empty today.)
