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

```sh
cargo run -p vibe-cli -- check --path . --quiet
```

The expected output is `vibe check: 0 errors, 0 warnings, 0 info`. The six v0 checks (manifest validity, WAL freshness, WAL well-formedness, boot directory, lockfile/disk consistency, REVIEW marker aging) all run.

If you `vibe install <pkgref>` against this manifest by accident, the install will succeed — there are no boot-prefix collisions today (`spec/boot/` carries only `00-core.md` and `90-user.md`). It will, however, materialise package files into `spec/flows/`, `spec/feats/`, or `spec/stacks/` and rewrite `vibe.lock` with `[[package]]` entries; revert with `vibe uninstall` (or `git restore vibe.lock spec/`) before committing.

## 7. Troubleshooting

(Populated as real issues arise. Empty today.)
