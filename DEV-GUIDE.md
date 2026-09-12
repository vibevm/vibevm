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

### 2.6 Node and pnpm for the site

Needed only for the site package `org.vibevm.doc/web` — the Qwik application that carries both the landing and the documentation reader. Nothing else in this repository needs Node, and a reader's machine never does ([PROP-057 `##STACK-NODE-SERVER-ONLY`](vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml#stack)).

The versions are pinned exactly, together with the framework, by [PROP-057 §12 `##STACK-QWIK`](vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml#stack): **Node 24.18.0** and **pnpm 10.33.2**. The package declares both — `engines.node` and `packageManager` — so `corepack enable` is all it takes to get the right pnpm:

```sh
corepack enable
cd vibevm/vibepacks/org.vibevm.doc/web/v0.1.0
pnpm install --frozen-lockfile
```

`pnpm-lock.yaml` is committed; `node_modules/`, `dist/`, `server/` and `tmp/` never are — the package is published as source ([PROP-024 §2.2](vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml)).

**The floor.** The web package is authored under the `typescript-ai-native` discipline, and its seven steps (`prettier → tsc → tests → eslint → conform → specmap → test-gate`) plus the APCA contrast audit of both themes run as one command from the package root:

```sh
pnpm floor
```

The seven steps are a compiled tool, not an npm package. `pnpm floor` finds it in `$TYPESCRIPT_AI_NATIVE`, then on `PATH`, then in this checkout's built slot — so inside this repository it works after the tool has been built once:

```sh
target/debug/vibe.exe bin build typescript-ai-native --assume-yes
```

From the repo root the same floor runs without pnpm at all, which is the form `tools/self-check.sh` uses:

```sh
vibevm/vibedeps/org.vibevm.ai-native.typescript-ai-native-lang/1.0.0/target/release/typescript-ai-native.exe \
  floor --path vibevm/vibepacks/org.vibevm.doc/web/v0.1.0
node vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/design/audit/contrast.mjs
```

**Building the site.** `pnpm build:static` prerenders every route for the server; `pnpm build:embedded` builds only the documentation routes for the shell `vibe` embeds. Both count the pages the generator reports against the pages the manifest declares and fail on a mismatch — the static generator under-generates silently and still exits 0 ([`##STACK-PAGE-COUNT-GATE`](vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml#stack)).

Neither is how the deployment builds the site: there the renderer runs `vibe doc build-site`, which hands the rendered documentation to this same build and moves its output into place. Node is therefore needed on a build machine and in one container, and nowhere else — see [§8.1](#81-building-the-site-in-docker).

**Windows caveat.** Git Bash (MSYS2) rewrites anything that looks like a POSIX path, in arguments and in environment variables alike — a base path passed as `/doc/` reached the generator as `/Program Files/Git/doc/`. When a path or a base has to be passed to a build script from Git Bash, prefix the command with `MSYS_NO_PATHCONV=1`, or pass a value with no leading slash. PowerShell and `cmd` are unaffected.

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

Full design: [PROP-002 §2.10](vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml#publish). User-facing reference: [`docs-legacy/commands/registry-publish.md`](docs-legacy/commands/registry-publish.md).

**Routine usage:**

```sh
# Dry-run first (read-only — only hits GET /repos/...).
cargo run --release -p vibe-cli -- registry publish vibevm/vibepacks/org.vibevm.world/wal/v1.0.0 --dry-run

# Apply.
cargo run --release -p vibe-cli -- registry publish vibevm/vibepacks/org.vibevm.world/wal/v1.0.0
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

If you `vibe install <pkgref>` against this manifest by accident, the install will succeed — there are no boot-prefix collisions today (`vibevm/vibespecs/boot/` carries only `00-core.xml` and `90-user.xml`). It will, however, materialise package files into `spec/flows/`, `spec/feats/`, or `spec/stacks/` and rewrite `vibe.lock` with `[[package]]` entries; revert with `vibe uninstall` (or `git restore vibe.lock spec/`) before committing.

## 7. Troubleshooting

(Populated as real issues arise. Empty today.)

## 8. Documentation

The user-facing documentation of vibevm is being rebuilt as a *documentation package* — `org.vibevm.core/vibevm-docs`, kind `doc` — by the `campaigns/docs-2026-09` campaign. The contract is [PROP-057](vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml); the plan is `campaigns/docs-2026-09/PLAN.md`.

**What a documentation package is.** A package that documents other packages (its *subjects*): it names them in `[[documents]]`, carries a `title` and an `abstract`, and is read, never installed. `vibe install` refuses it; `vibe cache add <coordinate>` warms it into the machine store, and the local reader (`vibe doc serve`) and `vibe explain` read the store.

**Where things live, and when they arrive.**

- The pages: `vibevm/vibepacks/org.vibevm.core/vibevm-docs/` — opened by phase P of the campaign.
- The old `docs/` tree moves to `docs-legacy/` in phase 3 and stays readable; nothing cites it as normative.
- The machinery: `vibe doc build | check | manifest | serve | shell` ships now. The maintenance verbs (`todo`, `surface`, `diff`) ship with them.

**The four verbs.** Each is a thin surface over the `vibe-doc` library, which is where everything with content in it lives ([PROP-057 §10](vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml)); nothing below duplicates a rule, and `--path` defaults to the current directory everywhere.

```sh
# Check it. Every flag is independent; ask for the ones you want.
cargo run -p vibe-cli -- doc check --examples --citations --derived \
    --translations --coverage --media \
    --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0

# Render it into a directory that can be served as it stands.
cargo run -p vibe-cli -- doc build --out .vibe/doc --format html

# What a machine reads: the page manifest, or one `llms` tier.
cargo run -p vibe-cli -- doc manifest --json
cargo run -p vibe-cli -- doc manifest --llms index

# Read it locally. Loopback only, and the pages come dressed in the
# reader's shell — the real one when this binary carries it, the bare one
# otherwise.
cargo run -p vibe-cli -- doc serve --port 8413

# Which shell does this binary carry, and is it the one the build pinned?
cargo run -p vibe-cli -- doc shell status
```

`--style` joins `doc check` with the prose linter, later in the same phase.

**The whole site, not one package: `doc build-site`.** Every verb above answers about the one package you point `--path` at. `build-site` answers about the site — it reads two sources named in a `site.toml`, and it is the command the renderer container runs.

```sh
# Read the sources and print what would be rebuilt, writing nothing.
cargo run -p vibe-cli -- doc build-site --config site.toml --dry-run

# Render what moved, then build the domain on the result.
cargo run -p vibe-cli -- doc build-site --config site.toml --out /srv/site

# Render the documentation trees and stop — for a machine with no Node.
cargo run -p vibe-cli -- doc build-site --config site.toml --out /srv/site --no-web
```

The two sources are the ones [PROP-057 §9.2](vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml) names, each in the form a project already names a `[[registry]]`: a package **registry**, whose index is the change feed, and the **host's own repository**, which is not a package at all — its root is a `[project]` — and so is read as a checkout the deploy keeps current on disk. A commented example with every default written out is `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site.example.toml`; the shape is `schemas/doc_site_config.jtd.json`, registered as the format `doc-site-config`.

Nothing in that file authorises anything. The site reads every source anonymously, so there is no `auth`, no token and no environment name in the shape — a configuration that *could* carry a credential invites one onto a renderer with no use for it.

**What a run does, in order.** It polls both sources for the set of «coordinate · version · content hash» they hold *now*; compares that with `<out>/.vibe-site/state.json`, which records what was rendered and what it was rendered from; and rebuilds only the pairs that are new, moved or last failed. Each rebuilt pair is warmed into the machine store the way `vibe cache add` warms one — subjects and all, so its `spec://` citations resolve offline — then composed into a documentation package and built in all three projections under `<out>/.vibe-site/trees/`. Finally the site package is run over every standing tree and its output replaces everything in `<out>` except `.vibe-site`.

Two consequences worth knowing before you point it at a directory. **The state file is inside the output**, because the question it answers is «what is in *this* directory»; a serving configuration has to refuse `/.vibe-site/` the way it refuses any other kitchen. And **a package that will not render becomes a page** rather than a failure: the reason is composed into a card and a page at the same address, the row records the failure, and the next run tries again — a registry of hundreds will always hold one broken package, and a builder that stopped for it would publish nothing at all.

### 8.1 Building the site in Docker

The deployment is two containers and a directory between them ([PROP-057 `##SITE-TWO-CONTAINERS`](vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml#site)): a **renderer**, which carries `vibe` built from this checkout and the site package with its dependencies, runs `doc build-site` into the directory and exits; and a **serving** container, stock nginx with `docker/nginx.conf`, which serves that directory and knows nothing else. A new render is a new directory, never a new image.

Both are stages of one file, `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/docker/Dockerfile`, and its build context is **the repository root** — the renderer needs the binary built from this source, the site package, and the checkout itself, which is one of the two sources it reads. What of the root actually enters the context is an allow-list in `docker/Dockerfile.dockerignore` beside it; a working tree's `target/` alone is hundreds of gigabytes, so a deny-list that forgot one entry would send it all to the daemon.

To build and run the whole stack locally:

```sh
cd vibevm/vibepacks/org.vibevm.doc/web/v0.1.0
docker compose -f docker/compose.yaml up --build   # renders, then serves on 127.0.0.1:8080
docker compose -f docker/compose.yaml down
```

The service names and the published port in that file are **placeholders**: the real ones belong to the server's own arrangement and are never written here ([`##SITE-WHO-COMMITS-WHAT`](vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml#site)). What is not a placeholder is everything below them — one volume, the renderer filling it, the serving side waiting for the render to finish rather than racing it.

The renderer's own configuration is `docker/site.toml`: the two public sources and the domain, with an empty analytics website id, because that id names a live property and belongs to the deployment rather than to the repository. A deployment that needs different values mounts its own file over `/site.toml`.

Three things about the build worth knowing before the first one.

- **The first build is slow and the rest are not.** It compiles `vibe` from source and installs the site package's dependencies; both are cached layers afterwards, and editing a page recompiles nothing.
- **The render needs the network** — it reads the package registry and warms what it renders — and it needs a resolver its containers can actually reach. On a machine whose resolver answers containers with addresses they cannot route, the symptom is every fetch failing at connect time inside the build; a throwaway `docker buildx` builder configured with a public resolver is enough to get past it.
- **A Rust edit can hide from the cache mount.** `cargo` inside the image judges freshness by modification time, and a file edited before the previous build reached compilation arrives older than the artefacts in the cache: the build then reports the binary finished in a fraction of a second, and the image carries the old one. A checkout on the server never has this, because `git pull` stamps the files; it is a trap of local iteration only, and `docker buildx build --no-cache-filter vibe-build …` forces the compile.
- **`docker compose build --no-cache` is for the site's services only.** Nothing in this repository rebuilds anybody else's.
- **A redeploy is three commands in this order, or it is a render's worth of downtime.** `docker compose up -d vibevm-org` alone recreates the serving container *before* it waits for the renderer — compose creates first and starts later — so the old container stops and the domain answers 502 until the first render is done (eight minutes on the first live switch, 2026-09-12). Build both images, run the renderer as a one-shot (`up --no-deps --exit-code-from <renderer> <renderer>`, which also stops on a red render), and only then `up -d --no-deps <serving>`: the old container answers until the last step, which takes seconds.

**Level 0 is every package, from its own bytes.** A package that ships no documentation still gets pages: the manifest as a reference page, the README, and the boot snippet when it declares one, beside every specification it carries — copied at its own path, because the path is the address a `spec://` citation resolves to. A `doc` package gets the same treatment and its authored pages besides, which is why there is one rendering path and not two.

**What each check asks.** `--examples` runs every documented command against the debug binary in a sandbox and compares the output exactly, after the fixture's declared normalisation; `tools/self-check.sh` runs them as golden tests, and a red example is fixed in the normalisation rules or in the product, never by loosening the comparison. `--citations` asks one question of every `spec://` a page cites — does the anchor exist. `--derived` rebuilds every generated block and compares it with the record in the package; the cure for a red one is `--derived --accept`. `--translations` checks an adaptation against the documentation it adapts, structure only. `--coverage` requires every spec fact marked `actionstage="doc"` with an audience to be cited by a page for that same audience, and `--min <percent>` lowers the bar for an intermediate run. `--media` judges the card's images by their bytes.

**Two things `doc build` will do that may surprise you.** It runs the product — `derived` blocks are generated from `vibe … --help` and the schemas, one process per block — so a build takes a few seconds; `--no-derived` skips that and marks the blocks as the gaps they are. And it writes the site's own address map (`<group>/<name>/<version>/<document>/index.html`, the projections beside it, `manifest.json` and the four `llms` files at the root), so the output directory is servable as it stands.

**The local reader binds `127.0.0.1` and only that.** There is no host flag and there is not going to be one: the reader serves proprietary packages' documentation, and one reachable from another machine is serving it to them. It sends a content policy naming no external source, no CORS header at all, and `frame-ancestors 'none'` unless `--frame-ancestor <origin>` names an editor's webview. Every inline script the shell carries is named in that policy by the sha256 of its own bytes, computed at start-up from the shell the binary is holding — so a rebuilt shell cannot leave a stale hash behind.

**The reader's shell, and why your build has the bare one.** A page a person reads is the *island* — the content HTML the Rust pipeline renders — inside the *shell*: the head, the styles, the navigation, the behaviour, which are a build of the site package `org.vibevm.doc/web`. A release build compiles that shell into `vibe`; a plain `cargo build` on a machine with no Node compiles the **bare shell** instead, which is typography and no scripts and still a page you can read. `vibe doc shell status` says which one you have and whether it matches `crates/vibe-doc-shell/doc-shell.lock`.

To carry the real one locally:

```sh
# Build the shell from the web package and put it where the crate can
# compile it in. Needs Node and pnpm (§2.6); writes
# crates/vibe-doc-shell/shell/, which is gitignored, and re-pins
# doc-shell.lock.
cargo xtask embed-doc-shell

cargo build -p vibe-cli --features vibe-doc-shell/embedded-shell
```

On a release binary built from source there is a third way: `vibe doc shell install` downloads the shell that release was built with, verifies it against the pin, and stores it under `~/.vibe/opt/vibevm/doc-shell/<sha256>/`. It **asks first, every time**, and nothing in `vibe doc` ever reaches the network without that answer — which is the whole promise of the local mode.

Node is not needed to build or run the product. The site package `org.vibevm.doc/web` (Qwik 2.0) is built on the server and by `cargo xtask embed-doc-shell`; its Node and pnpm pins, the floor and the MSYS path caveat are in [§2.6](#26-node-and-pnpm-for-the-site).
