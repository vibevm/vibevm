# Corporate environment campaign — log

### 2026-11-08 — planned

Plan written against `0ec3c96de` from a read-only code survey (PLAN.md section 2). Phase 0 spikes S1–S4 next; implementation starts after VIBE-NEEDS closes, beginning with phases whose files do not overlap the VIBE-NEEDS workers.

### 2026-11-08 — Phase 0 spikes

S1 confirmed P1 and found the release downloader bypasses source policy/proxy; S2 found no GitVerse release assets and a token-only API; S3 showed monorepo resolution works over SSH but lacks lock provenance (Phase 2 split); S4 found the Windows build broken at HEAD (fixed separately as a 1.0.12 blocker). PLAN.md §3.8 records the changes.

### 2026-11-08 — Phases 2c/2d: index inside layout registries, `vibe registry mirror`; real-service experiment

Built: `_index/` inside `monorepo`/`localdir` registries (CORP-FMT-012), `vibe registry mirror` with the batch publisher (CORP-FMT-013), exact Git trees and the reverse-mirror record (CORP-FMT-014). Read-only experiment against the real registries; nothing was pushed or published to GitHub/GitVerse, no token was read.

**Setup.** Isolated HOME `/fast/vibevm-test-tmp/claude-int/mirror-exp/home` (`VIBE_SETTINGS`, `VIBE_REGISTRY_CACHE`, empty `GIT_CONFIG_GLOBAL`, defaults seeded: `vibespecs` on GitHub, `vibespecs-gitverse`), the debug `vibe` of this change. Targets: a local bare repository, a working copy, and the loopback sshd fixture (`ssh_fixture.py`, `VIBE_SSH_FIXTURE_WRITABLE=1`) reached through an `~/.ssh/config` alias `corp-git`.

**Commands and results.**

1. `vibe registry mirror vibespecs-gitverse --into gv.git --dry-run` → refused: no index answers, `--package` required. With `--package org.vibevm.world/wal`: `access denied (401, walked past — auth=none)`. GitVerse's `vibespecs` organisation is not anonymously readable (every `git ls-remote https://gitverse.ru/vibespecs/<repo>.git` asks for credentials; no `index` repository). **Not feasible without a token**, which this experiment may not use.
2. `vibe registry mirror vibespecs --into corp.git --name corp --dry-run` → listing from the GitHub index (`https://raw.githubusercontent.com/vibespecs/index/main`): 87 to copy, 1.7 s, bare repository untouched.
3. Save into a bare repository: 86 copied, **1 unavailable** (`org.vibevm.world/zap@1.0.0`: the index lists it, its repository is not anonymously readable), one commit, 1m49s (first version of the command: 3 min and then aborted the whole run on that one package — fixed: `unavailable` row, the rest copied, exit non-zero). All 86 `content_hash` values in the target's `_index/primary.jsonl` equal the source index's (recipe `sha256-tree/1`). Provenance per entry: upstream `source_url`, tag, commit (`wal`: `320698409fa5…`, matching `git ls-remote`).
4. Full-tree save (after the owner's reverse-mirror requirement) into `corp2.git`: 86 copied, `tree_verified = true` for all 86; independently, `git rev-parse HEAD:<group>/<name>/v<version>` equals the recorded source tree for 86/86; content hashes 86/86 equal. All 86 source tags are annotated; tagger, tag message, commit, tree, author, committer, message recorded in `_index/mirrors/vibespecs.toml`. 1m55s.
5. Rerun: 86 unchanged (nothing fetched — decided by the index hashes), no commit, branch tip unchanged, 4.5 s.
6. Working copy (fresh clone of an empty repo, `--group org.vibevm.fractality`): 3 copied, committed locally, the bare remote untouched; with an untracked file present → refused (now before any fetch); after removing it → 3 unchanged.
7. `--keep-history --group org.vibevm.fractality`: 3 bundles (9.8 KB, 1.2 MB, 1.8 MB) under `_index/mirrors/bundles/vibespecs/`; rerun with `--keep-history` → no commit.
8. Mirror into the SSH monorepo (`ssh://corp-git/…/repo.git`): 86 copied + 1 unavailable, pushed, 2m22s.
9. Project installed from GitHub (`org.vibevm.world/wal`, `org.vibevm.fractality/delegation-rules`), then `~/.vibe/registry.toml` = only `corp` (`type = "monorepo"`, the SSH url) with `[source_policy] exclude_hosting = ["github", "gitverse"]`; `strace -f -e trace=connect vibe --unattended update --sources-only` → exit 0; both lock rows now `registry = "corp"`, `source_url = <ssh url>#<path>`, `resolved_commit` = the monorepo commit, **content hashes identical**; connects: only `127.0.0.1` (3).
10. Cold `vibe --unattended install` (store, registry cache and `vibedeps` removed) under the same strace: exit 0, 74 connects, **all to 127.0.0.1**.
11. `vibe --json search playbooks` (a keyword only in `delegation-rules`' manifest `keywords`): `registries_searched = ["corp"]`, hit `delegation-rules`, `source = "index"`, matched `playbooks`; only loopback connects. `vibe registry test`: `corp … reachable (auth=ssh) — index: current`.

**Problems found.**

- P1 (fixed in this change): a single unreadable repository aborted the whole mirror after minutes of fetching → `unavailable` rows; the rest is copied; exit non-zero.
- P1 (fixed): short names never resolved from `localdir`/`monorepo` registries (`resolve_name_candidates` only asked per-package indexes); `vibe registry sync` skipped monorepo lock rows as an unknown registry; `vibe search`/`registry test`/`sync` read only `vibe.toml`, so registries in `~/.vibe/registry.toml` (the corporate case) were invisible to them.
- P2 (open): the GitHub index lists `org.vibevm.world/zap`, whose repository is not public (index/repository drift at the source).
- P2 (open): the HTTPS failure on `zap` is reported as «ssh authentication failed for `https://…` — check your ssh-agent» (git backend error classification).
- P2 (open): a `file://` monorepo cannot be used while any `exclude_hosting` is active: «native context unsupported: selected source requires a separately qualified controlled transport» (the controlled transport admits HTTP/HTTPS/SSH only). A corporate monorepo on an NFS path plus exclusion of the public hosts therefore does not work today; use SSH, or a `localdir` copy.
- P2 (open): `vibe install` and `vibe update --sources-only` disagree on the closure: the install materialised the 2 requested packages, the update (against GitHub too — not mirror-specific) re-resolved 32 (the `access = "private"` requires of `delegation-rules`).
- P3 (open, performance): the cold install from the SSH monorepo took 88 s (11 min under `strace -f`): the 79 MB repository was captured three times (once per resolution/fetch stage) by the controlled transport.
- P3 (environment): OpenSSH reads `~/.ssh/config` from the passwd home, not `$HOME`; plain `git` in an isolated HOME needed `GIT_SSH_COMMAND='ssh -F …'`.
- P3: the frozen/snapshot rules could not be exercised against the real registry (every vibespecs version is a snapshot and unchanged); they are covered by `crates/vibe-cli/tests/cli_registry_mirror.rs` and the plan unit test.

**What a reverse mirror (monorepo → per-package repositories) would still need.** With the default record it can recreate, per package, a repository whose tag (same name, same kind, same tagger and message) points at a commit with the identical tree and the recorded author/committer/message; the commit hash differs because parents are not kept. With `--keep-history` the bundle restores the exact commits, tags and branches. Out of scope, not recorded: repository description, topics, visibility and settings; host releases and their assets; issues/PRs; branches other than via a bundle; signatures on tags/commits beyond what a bundle carries; repository creation on the host (needs the host API and a token). The reverse command itself is not implemented.
