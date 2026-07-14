# PROP-016: Decentralized source mirrors — the vibevm setup {#root}

**Status:** accepted 2026-06-14 — owner-requested; in force. The target set (§2) is **living** — it grows as hosts are added.
**Related:** [PROP-000 §7](PROP-000.md#registry) (the package-registry split-host — a *different* concern, see §3), [`spec/boot/90-user.md`](../boot/90-user.md) (this machine's repository-access record), [`mirrors.toml`](../../mirrors.toml) (the target registry), `xtask/src/mirror.rs` (`cargo xtask mirror`), [`CLAUDE.md`](../../CLAUDE.md) (the attribution and force-push rules this model never crosses).

The **general model** — why multi-homing a git source across several hosts invites multi-master divergence, and how a single-writer mainline with every host as a downstream read-replica dissolves it (the benevolent-dictator / hub-and-spoke shape, the never-`--force` law, what it buys and costs, the maintainer's daily loop) — is the `source-mirrors` flow this project depends on: `spec://org.vibevm.world/source-mirrors/flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL#root` (fan-out mechanics and the reference script: `spec://org.vibevm.world/source-mirrors/flows/source-mirrors/fanout-mechanics#root`; the maintainer's day: `spec://org.vibevm.world/source-mirrors/flows/source-mirrors/daily-loop#root`). This PROP records only what is **specific to vibevm**: the concrete host set, its relationship to the package registry, and the tooling that fans out to it.

## 1. vibevm's hosts {#hosts}

vibevm's source is multi-homed across two public hosts, both canonical for reading, kept in step under the source-mirrors single-writer model (mainline is the maintainer's integrated local `main`; no host is primary; every host is a downstream read-replica):

- **GitVerse** — `git@gitverse.ru:vibevm/vibevm.git` (web `https://gitverse.ru/vibevm/vibevm`), region **RU**. `origin` on the maintainer's machine points here — a single-host convenience remote; fan-out is the manifest, not `git push origin`.
- **GitHub** — `git@github.com:vibevm/vibevm.git` (web `https://github.com/vibevm/vibevm`), region **US**.

Authentication is the maintainer's per-host SSH keys in the agent — never a token, never in the manifest.

## 2. `mirrors.toml` — the target set {#manifest}

`mirrors.toml` at the repo root is the committed, shared, credential-free target registry (its schema and the `push` / `self-pull` mode semantics are the flow's `spec://org.vibevm.world/source-mirrors/flows/source-mirrors/fanout-mechanics#manifest`). vibevm's current set:

```toml
schema = 1

[[target]]
name = "gitverse"
url = "git@gitverse.ru:vibevm/vibevm.git"
mode = "push"            # the maintainer pushes mainline here
refs = ["main", "tags"]
region = "ru"

[[target]]
name = "github"
url = "git@github.com:vibevm/vibevm.git"
mode = "push"
refs = ["main", "tags"]
region = "us"
```

Adding a host is one `[[target]]` block, committed — the set is living.

## 3. Relationship to the package-registry split-host {#registry}

This PROP governs the **source repository**; it is orthogonal to the **package registry**, and the two must not be conflated.

- **Source mirrors** (this PROP): the vibevm *source* is multi-homed across GitVerse (`vibevm/vibevm`) and GitHub (`vibevm/vibevm`), kept in step by `cargo xtask mirror`. Auth is the maintainer's **per-host SSH keys**.
- **Package registry** ([PROP-000 §7](PROP-000.md#registry), [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish)): published *packages* live in the GitHub `vibespecs` org. Auth is the **`~/.vibevm/github.publish.token`**, used *only* by `vibe registry publish`, scoped strictly to `vibespecs`.

So `vibevm/vibevm` (a source mirror) and `github.com/vibespecs/*` (the package registry) are different GitHub orgs serving different purposes with different credentials. The publish token is never used to push source; an SSH key is never used to publish a package. The original split-host rationale (GitVerse's API does not expose org-scoped repo creation, which the publisher needs; GitHub's does) holds for the registry and is untouched.

## 4. Tooling {#tooling}

The fan-out engine is `xtask/src/mirror.rs`, driven by `cargo xtask mirror` (the model's mechanics — fast-forward-only push, tracking-ref refresh, fail-loud drift handling — are the flow's `spec://org.vibevm.world/source-mirrors/flows/source-mirrors/fanout-mechanics#root`):

- `cargo xtask mirror` — push mainline (`main` + tags) to every `push` target, **fast-forward-only, never `--force`**; verify `self-pull` targets; refresh the matching local remote-tracking refs so `git status` is clean after a green rollout. This — not `git push origin` — is the standard rollout.
- `cargo xtask mirror --check` — verify every target equals mainline; push nothing (read-only, non-zero exit on drift).
- `cargo xtask mirror --from <name>` — fast-forward mainline to a host's accepted-PR merge before fanning out (the bridge for a PR merged via that host's web UI).
- `cargo xtask health --mirrors` — run the `--check` probe as an advisory `mirrors` block in the Discipline sweep; **off by default**, so the committed health snapshot stays deterministic and offline (mirror sync is network state, not a property of the source tree).

The never-`--force` invariant is **runnable capital**, not prose: `push_args` is a pure function and the `push_args_never_force` unit test asserts it never emits `--force`, `-f`, or a `+`-prefixed force refspec for any ref shape; `--from`'s `git merge --ff-only` enforces the same at runtime. This honours the `CLAUDE.md` force-push red line by construction.

## 5. Open questions {#open}

1. **Server-side mirroring.** When a host must originate writes outside `cargo xtask mirror` (e.g. heavy web-UI merging on one host), add one-directional server-side mirroring (a GitHub Action mirroring GitHub→GitVerse, or GitVerse's own pull-mirror for the reverse). It touches CI secrets (an owner act), so it is deferred until needed.
2. **`self-pull` adoption.** No target uses `self-pull` yet; the mode exists for the first host that ships a built-in mirror.
3. **A `vibe`-level mirror surface.** The fan-out shape (one source → many heterogeneous targets with per-target capability and credential) mirrors vibevm's own multi-registry publish domain (`[[registry]]`, `RepoCreator` adapters). Whether the two should share code is a FEAT worth opening if the target set grows large.

## 6. Version history {#history}

- **2026-06-14 — authored, in force.** Owner decision: the source becomes multi-homed (GitVerse + GitHub `vibevm/vibevm`, both public, canonical for reading; US↔GitHub, RU↔GitVerse), kept in sync by the maintainer's fan-out. The `mirrors.toml` registry and `cargo xtask mirror` (`--check`, `--from`) plus the off-by-default `health --mirrors` probe are defined here. Supersedes the interim multi-push-remote and the abandoned bidirectional-multi-master sketch.
- **2026-06-14 — fan-out refreshes tracking refs.** `cargo xtask mirror` updates the local remote-tracking ref of any remote matching a target after a successful branch push (pushing by raw URL otherwise leaves `refs/remotes/<remote>/<branch>` stale, so `git status` falsely read "ahead of origin/main" right after a green rollout). A local, best-effort convenience; no model change.
- **2026-07-14 — general model extracted to the `source-mirrors` flow.** The problem statement, the hub-and-spoke model, what it buys and costs, the daily loop, and the fan-out mechanics moved into the installable `source-mirrors` package (reaching vibevm through the redbook dependency); this PROP was thinned to vibevm's concrete host set, the registry distinction, and the tooling. No behaviour changed — `mirrors.toml` and `cargo xtask mirror` are untouched.
