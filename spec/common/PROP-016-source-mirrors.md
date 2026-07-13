# PROP-016: Decentralized source mirrors — the integration model {#root}

**Status:** accepted 2026-06-14 — owner-requested; in force. The target set (§3.1) is **living** — it grows as hosts are added.
**Related:** [PROP-000 §7](PROP-000.md#registry) (the package-registry split-host posture — a *different* concern, see §5), [`spec/boot/90-user.md`](../boot/90-user.md) (this machine's repository-access record), [`mirrors.toml`](../../mirrors.toml) (the target registry), `xtask/src/mirror.rs` (`cargo xtask mirror`), [`CLAUDE.md`](../../CLAUDE.md) (Rule 1 attribution; Rule 4 force-push red line this model never crosses).

---

## 1. Motivation {#motivation}

The vibevm source is consumed from two regions: contributors and readers in the United States reach **GitHub**, those in Russia reach **GitVerse**. Both must always carry the same history, both are public, and both are canonical *for reading*. The naive way to keep two writable repositories in step — let each accept writes and mirror to the other — is **multi-master replication**: two independent writes to the same branch diverge, and something must merge or one must be force-overwritten. That is the failure mode this PROP exists to avoid.

The avoidance is structural, not operational. Adopting the **benevolent-dictator / hub-and-spoke** model (the Linux-kernel workflow) makes mainline **single-writer**: there is exactly one integration point — the maintainer's decision that a change is in. Every host is then a *downstream read-replica* of mainline, never an independent writer, so two writes to the same ref cannot race. The cost of "both repos canonical" is paid once, in the model, not continuously, in conflict resolution.

## 2. The model {#model}

### 2.1 Mainline is the maintainer's local tree; there is no primary {#mainline}

Mainline is the maintainer's integrated local `main`. It has **no primary host** — it is not "the GitHub copy" or "the GitVerse copy"; it is what the maintainer has blessed, replicated equally to every target. Because only the maintainer advances mainline, and they do so serially, concurrent divergent writes to `main` do not occur: the multi-master problem is absent by construction, not patched after the fact.

### 2.2 Targets are downstream read-replicas {#targets}

Every host in [`mirrors.toml`](../../mirrors.toml) is a downstream replica, canonical for *reading* in its region (GitVerse/RU, GitHub/US, more later). Nobody writes a target directly. A direct write to a target — a stray push, a force-push — makes it **diverge** from mainline; the tooling detects that and fails loud rather than papering over it (§4.4). The model tolerates heterogeneous hosts: a `push` target is one the maintainer pushes to; a `self-pull` target is one that mirrors itself from elsewhere (we only verify it keeps up, §3.1).

### 2.3 Contributions arrive anywhere; the maintainer integrates {#contributions}

A change reaches mainline only by the maintainer integrating it. Proposals arrive however is convenient — a GitHub web PR, a GitVerse web PR, a branch on a fork, an emailed patch — and are reviewed where they land. Accepting one means bringing its commits into local mainline (§4.2), then fanning out (§4.3). The web PR UIs are *inboxes and review surfaces*, not the merge authority; the merge authority is the maintainer's tree. This is exactly the kernel's "patches by email, integrated in the maintainer's tree, pushed to a hub that mirrors out" — the web UIs are merely nicer inboxes than a mailing list.

## 3. Components {#components}

### 3.1 `mirrors.toml` — the shared target registry {#manifest}

`mirrors.toml` at the repo root is the **committed, shared** list of targets. A machine-local git remote cannot serve this role: it is not shared with contributors and cannot be verified in CI or a sweep. The manifest carries **no credentials** — authentication is the maintainer's per-host SSH keys in the agent (§5). Schema:

```toml
schema = 1

[[target]]
name = "gitverse"
url = "git@gitverse.ru:vibevm/vibevm.git"
mode = "push"            # the maintainer pushes mainline here
refs = ["main", "tags"]  # what to mirror
region = "ru"

[[target]]
name = "github"
url = "git@github.com:vibevm/vibevm.git"
mode = "push"
refs = ["main", "tags"]
region = "us"
```

A `self-pull` target (a host that mirrors itself, e.g. via its own CI or a built-in pull-mirror) is listed with `mode = "self-pull"`: the tool does not push to it, only verifies it is level with mainline. Adding a host is one `[[target]]` block — the set is living.

### 3.2 `cargo xtask mirror` — the fan-out engine {#engine}

`xtask/src/mirror.rs`:

- `cargo xtask mirror` — push mainline (`main` + tags) to every `push` target, **fast-forward-only, never `--force`**. A non-fast-forward means a target diverged → fail loud (§4.4), reconcile by hand. `self-pull` targets are verified, not pushed. After each successful branch push it refreshes the matching local remote-tracking ref (e.g. `origin/main`), so `git status` reflects the rollout without a manual `git fetch` (§4.3).
- `cargo xtask mirror --check` — verify every target equals local mainline; push nothing. Read-only; non-zero exit on drift.
- `cargo xtask mirror --from <name>` — fast-forward local mainline to a host's accepted-PR merge (`git fetch` + `git merge --ff-only`) before fanning out: the bridge for a PR merged via that host's web UI (§4.2).

### 3.3 The optional health probe {#health}

`cargo xtask health --mirrors` runs the §3.2 `--check` probe as an advisory fact inside the Discipline-sweep collector (it adds a `mirrors` block to `terraform/health/latest.json`). It is **off by default** — the default `cargo xtask health` stays deterministic and offline (mirror sync is network state, not a property of the source tree), so the committed health snapshot never depends on remote reachability. Run it when you want drift surfaced alongside the other sweep facts.

## 4. Usage — the maintainer's guide {#usage}

This is the day-to-day loop, written for you, the maintainer.

### 4.1 The loop in one line {#loop}

> A PR arrives somewhere → you review and accept it there → you bring it into local `main` → `cargo xtask mirror` → it is everywhere.

### 4.2 Bringing an accepted change into mainline {#integrate}

Two ways, depending on where you accepted it:

- **You merged it via a host's web UI** (clicked "Merge" on GitHub or GitVerse). That host's `main` is now ahead. Bring it home:
  ```sh
  cargo xtask mirror --from github     # or --from gitverse
  ```
  This fast-forwards your local `main` to that host's `main`, then fans out to all targets (the host you pulled from is a no-op; everyone else catches up). It refuses if your local `main` cannot fast-forward — meaning your tree has commits the host lacks; reconcile by hand first.
- **You integrate locally** (a fork branch, or an emailed patch). On `main` with a clean tree:
  ```sh
  git fetch <contributor-url> <their-branch>   # or: git am < patch.eml
  git merge --ff-only FETCH_HEAD               # or your chosen merge
  ```
  then fan out (§4.3).

### 4.3 Rolling out everywhere {#rollout}

```sh
cargo xtask mirror
```
Pushes `main` + tags to every target, fast-forward-only. Output lists each target `ok`; a failure is loud and explained. This — not `git push origin` — is how a change reaches all hosts. (`origin` on this machine is a single-host convenience remote, GitVerse; fan-out is the manifest, not a multi-push remote, so the target set has one source of truth.)

Because the fan-out pushes by **URL** (the manifest's form), git would normally leave the local remote-tracking refs untouched — a raw-URL push updates no `refs/remotes/<remote>/<branch>`. So `mirror` refreshes them itself: after each successful branch push it moves the tracking ref of any remote pointing at that target (e.g. `origin/main`) up to the just-pushed commit, printing a `track` line. A green fan-out therefore leaves `git status` clean, with no stray "ahead of origin/main" that a manual `git fetch` would otherwise be needed to clear. The refresh is best-effort and local — it never fails a rollout whose pushes already landed, and `git fetch <remote>` remains the fallback.

### 4.4 Checking sync and handling drift {#drift}

```sh
cargo xtask mirror --check            # or: cargo xtask health --mirrors
```
`sync` on every line means all hosts equal mainline. `DRIFT` on a host means it carries a `main` your mainline does not — almost always a direct write to that host (someone pushed to it, or a force-push). The tool never force-overwrites; you reconcile deliberately: fetch the host, inspect the divergent commits, merge what is wanted into mainline, then fan out. **A diverged target is a signal to investigate, never something to silently clobber.**

### 4.5 Adding a host {#adding}

Append a `[[target]]` block to `mirrors.toml` (§3.1), commit it, ensure your SSH key has write access (for `push`) or the host is configured to self-mirror (for `self-pull`), then `cargo xtask mirror`. Done.

## 5. Relationship to the package-registry split-host {#registry}

This PROP governs the **source repository**; it is orthogonal to the **package registry**, and the two must not be conflated.

- **Source mirrors** (this PROP): the vibevm *source* is multi-homed across GitVerse (`vibevm/vibevm`) and GitHub (`vibevm/vibevm`), kept in step by `cargo xtask mirror`. Auth is the maintainer's **per-host SSH keys**.
- **Package registry** ([PROP-000 §7](PROP-000.md#registry), [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish)): published *packages* live in the GitHub `vibespecs` org. Auth is the **`~/.vibevm/github.publish.token`**, used *only* by `vibe registry publish`, scoped strictly to `vibespecs` (the token-secrecy and scope discipline in PROP-000 §20 / 90-user.md are unchanged).

So `vibevm/vibevm` (a source mirror) and `github.com/vibespecs/*` (the package registry) are different GitHub orgs serving different purposes with different credentials. The publish token is never used to push source; an SSH key is never used to publish a package. The original split-host rationale (GitVerse's API does not expose org-scoped repo creation, which the publisher needs; GitHub's does) still holds for the registry and is untouched.

## 6. Safety and limits {#safety}

- **Never `--force`.** Fan-out and `--from` are fast-forward-only. The tool cannot silently lose a commit — a divergence fails loud (§4.4). This honours the `CLAUDE.md` Rule 4 force-push red line by construction. The fan-out's push command is built in one pure function (`push_args`) and the `push_args_never_force` unit test asserts it never emits `--force`, `-f`, or a `+`-prefixed force refspec for any ref shape — the invariant is **runnable capital**, not a prose promise (the Discipline's "a rule with no checker is a WISH"). `--from`'s `git merge --ff-only` enforces the same at runtime, bailing on anything but a clean fast-forward.
- **Deletions do not auto-propagate.** Deleting a branch on one host does not delete it elsewhere — a safety choice (an accidental deletion must not cascade). Delete on each host deliberately.
- **The fan-out hub is the maintainer's machine.** Server-side auto-mirroring (a GitHub Action, a GitVerse pull-mirror) is deliberately *not* required: because mainline is single-writer and the maintainer runs the fan-out, the rollout is a deliberate act, not a daemon. Server-side mirroring is an open option (§7) for the day a host originates writes the maintainer does not funnel.

## 7. Open questions {#open}

1. **Server-side mirroring.** When a host must originate writes outside the maintainer's `cargo xtask mirror` (e.g. heavy web-UI merging on one host), add one-directional server-side mirroring: a GitHub Action mirroring GitHub→GitVerse (needs a GitVerse deploy key as a GitHub secret), or GitVerse's own pull-mirror for the reverse. Loop termination is free — a no-op push fires no event — but it touches CI secrets (an owner act), so it is deferred until needed.
2. **`self-pull` adoption.** No target uses `self-pull` yet; the mode exists for the first host that ships a built-in mirror.
3. **A `vibe`-level mirror surface.** The fan-out shape (one source → many heterogeneous targets with per-target capability and credential) mirrors vibevm's own multi-registry publish domain (`[[registry]]`, `RepoCreator` adapters). Whether the two should share code is a FEAT worth opening if the target set grows large.

## 8. Version history {#history}

- **2026-06-14 — authored, in force.** Owner decision: the source becomes multi-homed (GitVerse + GitHub `vibevm/vibevm`, both public, canonical for reading; US↔GitHub, RU↔GitVerse), kept in sync always and automatically by the maintainer's fan-out. The benevolent-dictator / hub-and-spoke model (no primary, single-writer mainline), the `mirrors.toml` registry, and `cargo xtask mirror` (`--check`, `--from`) plus the off-by-default `health --mirrors` probe are defined here. Supersedes the interim multi-push-remote and the abandoned bidirectional-multi-master sketch.
- **2026-06-14 — fan-out refreshes tracking refs (§3.2, §4.3).** `cargo xtask mirror` now updates the local remote-tracking ref of any remote matching a target after a successful branch push. Pushing by raw URL otherwise leaves `refs/remotes/<remote>/<branch>` stale, so `git status` falsely read "ahead of origin/main" right after a green rollout (the host was actually level — `mirror --check`, which queries the host, saw the truth). No model change: a local, best-effort convenience that makes the maintainer's working-tree view match the hosts without a manual `git fetch`.
