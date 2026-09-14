# WORKER-REPORT-POST-O3 — `--offline` reaches `vibe self` (B-160)

Packet: `campaigns/docs-2026-09/findings/PACKET-POST-O3.md`. Worktree
`C:\Users\olegc\git\v\vibevm-docs`, branch `research-preview-1-docs`. Not pushed.
`specmap.json` not regenerated. Nothing outside `crates/vibe-cli/**` and the one spec file
was touched; the web worker's files were never staged.

## Commits

| hash | subject |
| --- | --- |
| `a031e804ee19e912c6baf0554c1c750947b02433` | `fix(self): honour the offline posture on every self verb` |
| `09f647d7c3ccc1ddf7fab821399b9df2914cd8e1` | `docs(spec): say that self obeys the offline posture` |

Both committed with explicit paths (`git commit -F - -- <paths>`, the message on stdin so a
multi-line body survives the shell; no `git add`, since every file was already tracked). No
trailers, no mention of a model or an agent.

**Q — `a031e804`** (10 files): `main.rs`, `main_tests.rs`, `commands/vvm/mod.rs`,
`commands/vvm/error.rs`, `commands/vvm/builder.rs`, `commands/vvm/bundle.rs`,
`commands/vvm/bundle/network_tests.rs`, `commands/vvm/bundle/release_tests.rs`,
`commands/vvm/tests.rs`, `commands/vvm/relocate/tests.rs`.

**R — `09f647d7`** (2 files): `vibevm/vibespecs/common/PROP-019-version-manager.xml`,
`commands/vvm/tests.rs` (the single `verifies` edge).

## Where the offline posture is resolved, and how it was reused

**It already existed and I wrote no second copy.** The ladder is
`crate::output::resolve_offline(cli_flag, config_offline)` in
`crates/vibe-cli/src/output.rs:111` — `cli_flag || env_offline() || config_offline`, where
`env_offline()` reads `VIBE_OFFLINE` with the shared truthy dialect. Its five existing
callers are `install/direct.rs:85`, `update/prepare.rs:72`, `reinstall/prepare.rs:71`,
`cache/add.rs:247`, `lifecycle.rs:203` and `mcp/mod.rs:112`, each pairing it with
`UserConfig::load()?.net.offline`.

The version manager now uses the same function, called at the composition root:

```rust
// crates/vibe-cli/src/main.rs
fn vvm_offline(cli_offline: bool) -> bool {
    output::resolve_offline(
        cli_offline,
        UserConfig::load().map(|cfg| cfg.net.offline).unwrap_or(false),
    )
}
```

and the `Command::Vvm` arm fills the new `VvmEnv { …, offline: vvm_offline(cli.offline) }`.

It is resolved in `main.rs` rather than inside `commands/vvm/` because **both lower rungs
are ambient reads** — `VIBE_OFFLINE` from the process environment, `[net] offline` through
`UserConfig::load()`'s own `$VIBEVM_USER_CONFIG` / settings-dir resolution — and `VvmEnv`'s
contract is that the vvm domain reads no ambient state itself. The registry commands can
resolve in their own module because they are already allowed those reads; vvm is not. The
one behavioural nuance I had to choose: a malformed user config is **already** reported once
by `promote_user_config_env()` at startup, so `vvm_offline` does not fail the command a
second time over an optional layer — that rung falls back to its own default (online) and
the two rungs above it still decide. `main_tests.rs` pins all four cases against a config
file the test owns.

## Semantics as implemented

**The guard is at the seam, not at the top of each verb.** Every network call the version
manager makes goes through `Downloader::download`, and there are exactly two places that
reach it, so the posture is checked at those two and cannot be escaped by a new caller:

- `bundle.rs::fetch_aggregate` — the release lane's **first** request. Covers
  `self update`, `self install stable` (both via `move_to_newest_release`),
  `self install X.Y.Z` and `self reinstall` (both via `install_release_version`).
- `bundle.rs::install_selected`, immediately before the bundle download and before the
  download path is allocated. Covers `self bootstrap`, whose aggregate is read off local
  disk so the bundle is its first request.

This placement is what makes the packet's `installed_from_published_bundle` clause true
rather than accidental: a machine holding the newest release *could* be served from disk,
but the only thing that can say a release is the newest is the aggregate manifest, and
reading it is the act being refused — so `update` stops before it and the held instance
rescues nothing. The release-lane test seeds exactly that machine to prove it.

**`self bootstrap` with a bundle it already holds byte-for-byte still installs offline.**
It reaches `install_selected`'s reuse shortcut before the guard, costs no request, and
therefore refuses nothing. I read the packet's rule as "no `self` verb issues a network
request under the posture", not "no `self` verb succeeds", and this matches the source-lane
rule the packet states in the same sentence (*rebuild if it is possible from local state*)
and PROP-010's own `OFFLINE-LOCAL-ONLY`. Flagging it as the one judgement call in Q.

**Source lane, two network needs, both closed:**

- *The mirror.* `run_install_cmd`'s third branch is the only place the source lane clones
  or fetches. The refusal is raised there, **before `source::choose_mirror`** — so an
  interactive run is not asked to pick a mirror whose answer it could not use — naming the
  requested `--mirror` when one was given and the default (`gitverse`) otherwise. The
  in-tree and linked-source branches never reach it and rebuild untouched.
- *The crates.* `CargoBuilder` gained an `offline` field and `cargo_build_args` pushes
  `--offline`. A rebuild the machine's registry cache cannot satisfy fails with cargo's own
  error naming the crate, which I took to be the "honest refusal" for that case; the
  packet's own parenthetical names `cargo build --offline` as the mechanism.

**The refusal** is `VvmError::Offline { verb, address }` in `commands/vvm/error.rs`, in the
house grammar of its neighbours — `<what> (violates spec://…#surface; fix: …)`. It names the
verb as the operator typed it (`self:update` → `` `vibe self update` ``), the exact address,
and all three rungs in the fix hint. It is an ordinary `VvmError` returned as `Err`, so the
`--json` caller gets the ordinary error envelope through `main`'s single error arm — there
is no second path to keep in step, and no capture seam was added to `output.rs` (the JSON
error payload is built inline in `error_with_suffix` and `eprintln!`ed, so the envelope is
proven by the live run below rather than by a unit test).

**One consequence beyond the flag, reported deliberately.** Naming the verb required the
verb's label to reach `run_install_cmd`, which is shared by `self install` and — through
`rebuild_latest` — the source lanes of `self update` / `self reinstall`. It is now a
parameter, used both for the refusal and for the existing `activate_record` label. So a
**source-lane `self update` / `self reinstall` now reports `"command": "self:update"` /
`"self:reinstall"` in `--json` instead of `"self:install"`**. That is a corrected mislabel,
not a new behaviour, but it is a JSON-surface change the packet did not ask for; it is in
the commit body.

**Tests added** (all in the packet's perimeter):

- `bundle/release_tests.rs` — `UnaskedServer`, a `Downloader` that **panics** on any call:
  every release-lane verb refuses with verb + address + rule against a **seeded** machine,
  the forced (`--force`) case refuses rather than allocating a generation, and one online
  case pins that the guard changes nothing when the posture is false.
- `bundle/network_tests.rs` — `self bootstrap` refuses before the bundle request; the
  downloader records no URL and no instance directory is created.
- `commands/vvm/tests.rs` — the refusal's shape (verb, address, rule, all three rungs in
  the hint), and the source lane's mirror refusal for both the named and the default mirror,
  asserting the managed clone was never created.
- `commands/vvm/builder.rs` — `--offline` is added to cargo's argv iff the posture holds.
- `main_tests.rs` — the composition root hands down a *resolved* posture: all three rungs
  plus the unparseable-config fallback.

## Spec (commit R)

`##CMD-OFFLINE` added to `<surface>` immediately after `CMD-REINSTALL`, carrying the
packet's text verbatim. The section's content changed non-editorially, so
`req-surface` went `req r4` → `req r5`, and the single edge
`#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#surface", r = 4)]` in
`commands/vvm/tests.rs:84` moved to `r = 5` in the same commit.

Re-read of that test against the new text: it is
`a_binary_execution_routes_stable_to_the_newest_release_and_latest_nowhere`, which asserts
the selector routing of a binary execution (`CMD-INSTALL`, `CMD-UPDATE`, `SEL-STABLE`). The
added fact does not touch selector routing, so the test attests to revision 5 unchanged; I
left its body and doc comment as they were. No new `verifies` edges were added anywhere —
the packet says "the single edge", and extra edges would only add churn to a map the boss
has to regenerate.

**The fact carries no `audience`.** With `audience="user"` the manual gate goes red exactly
as the packet anticipated:

```
$ target/debug/vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0 --coverage --min 100
  UNCOVERED user — vibevm/vibespecs/common/PROP-019-version-manager.xml#CMD-OFFLINE
    vibevm/vibespecs/common/PROP-019-version-manager.xml:118 — no page cites it
  user    310/311 (99%)
  author  220/220 (100%)
  dev     19/19 (100%)
  agent   40/40 (100%)
coverage: 534 of 534 obligation(s) told, 99% of 590 audience pair(s) (threshold 100%), 0 unreadable page(s)
error: the documentation does not tell everything the specifications promised: 99% of 590 audience pair(s) covered, 100 required, 0 page(s) unreadable (violates spec://org.vibevm.core/vibevm/common/PROP-057#OBS-COVERAGE-GATE; fix: write the page — the gate closes on a page for that audience citing the rule, never on a list of pages)
```

So `audience` was removed and `action="continue" actionstage="doc"` kept — the fact still
says it owes documentation. **The prose is the central session's to write**, in both
languages, in the atom that restores `audience="user"`. With the attribute off, both manuals
are green:

```
$ target/debug/vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0 --coverage --min 100
  user    310/310 (100%)
  author  220/220 (100%)
  dev     19/19 (100%)
  agent   40/40 (100%)
coverage: 534 of 534 obligation(s) told, 100% of 589 audience pair(s) (threshold 100%), 0 unreadable page(s)

$ target/debug/vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs-ru/v1.0.0 --coverage --min 100
  user    310/310 (100%)
  author  220/220 (100%)
  dev     19/19 (100%)
  agent   40/40 (100%)
coverage: 534 of 534 obligation(s) told, 100% of 589 audience pair(s) (threshold 100%), 0 unreadable page(s)
```

(The packet's `vibevm-docs-ru/v0.1.0` does not exist; the Russian package is at `v1.0.0`.)

## Gates — verbatim

`CARGO_TARGET_DIR` was **not** overridden; everything ran against the shared `target/`.

```
$ cargo fmt --all -- --check
FMT-EXIT=0                                 # no output, no diff
```

```
$ cargo clippy -p vibe-cli --all-targets -- -D warnings
warning: C:\Users\olegc\git\v\vibevm-docs\crates\vibe-cli\Cargo.toml: unused manifest key: build
help: build is a valid .cargo/config.toml key
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.27s
CLIPPY-EXIT=0
```

(The `unused manifest key: build` warning is pre-existing and unrelated.)

```
$ cargo test -p vibe-cli vvm -j 2
running 124 tests
test result: ok. 124 passed; 0 failed; 0 ignored; 0 measured; 642 filtered out; finished in 20.83s
# every integration target: 0 passed; 0 failed  (nothing in them matches `vvm` by name;
# tests/vvm.rs itself reports `0 passed … 4 filtered out`, as it did before this change)
```

```
$ cargo test -p vibe-cli offline -j 2
running 21 tests
test commands::vvm::builder::tests::an_offline_build_passes_the_posture_to_cargo ... ok
test output::tests::offline_config_applies_when_env_is_falsy ... ok
test commands::short_name::tests::qualify_locked_unknown_is_not_installed_and_offline ... ok
test cli::tests::offline_defaults_to_false_on_the_root ... ok
test cli::tests::install_local_offline_flag_still_parses ... ok
test cli::tests::offline_flag_parses_on_the_root ... ok
test commands::vvm::tests::the_offline_refusal_names_the_verb_the_address_and_the_rule ... ok
test commands::vvm::bundle::tests::network_tests::an_offline_bootstrap_is_refused_before_the_bundle_is_requested ... ok
test cli::tests::root_offline_reaches_the_install_command ... ok
test commands::show::source_path::tests::embedded_root_uses_exact_lock_row_and_threads_offline ... ok
test commands::install::resolver::flag_tests::env_offline_alone_bails_before_the_network_with_the_same_message ... ok
test output::tests::offline_default_false_with_no_flag_no_env_no_config ... ok
test output::tests::offline_cli_flag_wins_over_falsy_env ... ok
test output::tests::offline_env_falsy_values_or_empty_or_unset ... ok
test output::tests::offline_env_truthy_values ... ok
test output::tests::offline_env_wins_with_no_flag_no_config ... ok
test tests::the_version_manager_is_handed_the_resolved_offline_posture ... ok
test commands::vvm::bundle::tests::release_tests::a_forced_offline_update_is_refused_rather_than_allocating_a_generation ... ok
test commands::vvm::bundle::tests::release_tests::the_offline_posture_refuses_every_release_verb_before_its_first_request ... ok
test commands::vvm::bundle::tests::release_tests::an_online_run_is_untouched_by_the_offline_guard ... ok
test commands::vvm::tests::the_offline_posture_refuses_the_source_lane_before_git_reaches_a_mirror ... ok
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 745 filtered out; finished in 0.52s

# and two integration reds, both pre-existing and green:
test an_offline_pkgref_install_keeps_the_rest_of_the_world ... ok
test validate_rejects_a_malformed_manifest_under_the_offline_posture ... ok
```

```
$ cargo build -p vibe-cli -j 2
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.55s
BUILD-EXIT=0
```

```
$ cargo xtask specmap --check
Error: `C:\Users\olegc\git\v\vibevm-docs\specmap.json` is out of date relative to the tree.
  drift: revision bump: `spec://org.vibevm.core/vibevm/common/PROP-019#surface` r4 → r5
  drift: units added: 1
Run `rust-ai-native-specmap` (or your project's wrapper), review the drift, and commit the result.
SPECMAP-EXIT=1
```

**Red, and left red on purpose** — the packet forbids regenerating the map, and the drift is
exactly the two things commit R did: the `r4 → r5` bump, and one unit added (the
`vvm_offline` composition-root function). The repository's convention is a separate
`chore(specmap): regenerate the map after …` commit, which is the boss's.

## Live run — verbatim

Isolated root under the session scratchpad (`VIBEVM_INSTALL_ROOT=<scratch>\post-o3-root`).
Nothing under `~/.vibe/opt` was written, and — unlike POST-O2 — **the durable user
environment was never touched**, because the imported instance was inventoried **without
`--use`** and every probe below refuses before activation:

```
$ VIBEVM_INSTALL_ROOT=<scratch>\post-o3-root  target\debug\vibe.exe self import target\debug\vibe.exe --tag 1.0.0
  ✓ created  <scratch>\post-o3-root\opt\vibevm\versions\tag\1.0.0\1
imported tag:1.0.0#1
EXIT=0
```

Every probe below was then invoked from that instance's own path, so `derive_self` resolves
it and `running_record` really is `origin=binary`:

```
$ …\tag\1.0.0\1\vibe.exe self current
 > tag:1.0.0#1  origin=binary source="local-import" commit=unknown sha256=095866389f6e63579050f1d3e1bb75d8efec309163145455961e936fc44da9b0 profile=release
EXIT=0
```

### The release lane refuses, before the network, with the address and the rule

```
$ …\tag\1.0.0\1\vibe.exe --offline self update
error: learning the newest published release: `vibe self update` needs the network and this run is offline — it would have fetched `https://github.com/vibevm/vibevm/releases/latest/download/DISTRIBUTIONS.json` (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; fix: re-run it without `--offline`, with no truthy `VIBE_OFFLINE`, and without `[net] offline = true` in the user config — or stay on a version this machine already holds: `vibe self ls` lists them and `vibe self use <selector>` activates one)
EXIT=1

$ …\tag\1.0.0\1\vibe.exe --offline self install stable
error: learning the newest published release: `vibe self install` needs the network and this run is offline — it would have fetched `https://github.com/vibevm/vibevm/releases/latest/download/DISTRIBUTIONS.json` (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; fix: …)
EXIT=1

$ …\tag\1.0.0\1\vibe.exe --offline self install 1.0.0
error: `vibe self install` needs the network and this run is offline — it would have fetched `https://github.com/vibevm/vibevm/releases/download/v1.0.0/DISTRIBUTIONS.json` (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; fix: …)
EXIT=1

$ …\tag\1.0.0\1\vibe.exe --offline self reinstall
error: `vibe self reinstall` needs the network and this run is offline — it would have fetched `https://github.com/vibevm/vibevm/releases/download/v1.0.0/DISTRIBUTIONS.json` (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; fix: …)
EXIT=1
```

(The `fix:` clause is identical in all four and is elided after the first for length; the
first is quoted in full.)

### The source lane refuses before git reaches a mirror

Run from the worktree binary, which is a *source* execution, so it takes `run_install_cmd`'s
mirror branch:

```
$ VIBEVM_INSTALL_ROOT=<scratch>\post-o3-root  target\debug\vibe.exe --offline self install 1.2.3 --mirror github
error: `vibe self install` needs the network and this run is offline — it would have fetched `https://github.com/vibevm/vibevm.git` (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; fix: …)
EXIT=1
```

### The read-only verbs still work

```
$ …\tag\1.0.0\1\vibe.exe --offline self ls
  → > tag:1.0.0#1  origin=binary source="local-import" commit=unknown sha256=095866389f6e63579050f1d3e1bb75d8efec309163145455961e936fc44da9b0 profile=release
1 instance(s) installed.
EXIT=0

$ …\tag\1.0.0\1\vibe.exe --offline self which vibe
<scratch>\post-o3-root\opt\vibevm\versions\tag\1.0.0\1\vibe.exe
EXIT=0
```

### `--json` is the ordinary error envelope

```
$ …\tag\1.0.0\1\vibe.exe --json --offline self update
{"ok":false,"error":"learning the newest published release: `vibe self update` needs the network and this run is offline — it would have fetched `https://github.com/vibevm/vibevm/releases/latest/download/DISTRIBUTIONS.json` (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; fix: re-run it without `--offline`, with no truthy `VIBE_OFFLINE`, and without `[net] offline = true` in the user config — or stay on a version this machine already holds: `vibe self ls` lists them and `vibe self use <selector>` activates one)"}
EXIT=1
```

`ok:false` + `error` and nothing else — the same shape every other `VvmError` produces.

### The environment rung reaches the version manager too

```
$ VIBE_OFFLINE=1  …\tag\1.0.0\1\vibe.exe self update          # no flag at all
error: learning the newest published release: `vibe self update` needs the network and this run is offline — it would have fetched `https://github.com/vibevm/vibevm/releases/latest/download/DISTRIBUTIONS.json` (violates …)
EXIT=1
```

### Inventory unchanged, machine unchanged

```
$ …\tag\1.0.0\1\vibe.exe --offline self ls        # after every probe above
  → > tag:1.0.0#1  origin=binary source="local-import" commit=unknown sha256=095866389f6e63579050f1d3e1bb75d8efec309163145455961e936fc44da9b0 profile=release
1 instance(s) installed.
```

```
PS> (Get-ItemProperty HKCU:\Environment -Name Path).Path -like '*post-o3-root*'
False
PS> (Get-ItemProperty HKCU:\Environment -Name VIBEVM_HOME).VIBEVM_HOME
C:\Users\olegc\.vibe\opt\vibevm\versions\branch\main\71
```

The durable user `Path` has no scratch entry and `VIBEVM_HOME` still points at the owner's
own active instance — nothing to restore, unlike POST-O2. `<scratch>\post-o3-root` was
deleted afterwards.

## Anomalies

1. **`LINK : fatal error LNK1102: out of memory`** on the first `cargo test -p vibe-cli vvm`
   at default parallelism — several ~100 MB test binaries linking at once on a loaded
   machine (a web worker is building in the same tree). Re-running with `-j 2` succeeded
   every time. Environment, not code; every gate in this report that builds used `-j 2`.
   Worth knowing for the next Rust packet on this machine.
2. **The packet's Russian-manual path is stale.** `vibevm-docs-ru/v0.1.0` does not exist;
   the package is `vibevm-docs-ru/v1.0.0` (the English one is `vibevm-docs/v0.1.0`). Not a
   packet defect for POST-O3 — the packet did not name it — but the P4/W1 packets that do
   name a path should be checked.
3. **`bundle.rs` is over the 600-line budget** and my change adds to it (627 → 640). It was
   already over before this packet; I kept the addition to the two guards and put every new
   test in the `#[path]`-included test files so none of it lands in a production file.
4. The `--offline` **online** path was proven by the test suite
   (`an_online_run_is_untouched_by_the_offline_guard`) rather than live, deliberately: a
   real online `self update` from the imported instance would download ~75 MB **and
   activate**, which writes the durable Windows `Path` and `VIBEVM_HOME` — the exact damage
   POST-O2 had to repair by hand. I judged that not worth re-inflicting on the owner's
   machine to re-prove a path this packet does not change.

## Not done, and why

- **`specmap.json` not regenerated** — forbidden by the packet; the red is quoted above with
  its exact drift, which is precisely commit R's bump plus one added unit. Boss's
  `chore(specmap)` commit.
- **No manual prose for `##CMD-OFFLINE`** — the fact ships without `audience`, exactly as
  the packet's fallback prescribes. Writing that page in both languages and restoring
  `audience="user"` is the central session's atom.
- **No new `verifies` edges** — the packet says the single edge; extra edges would only
  enlarge the drift the boss has to review.
- **`tools/self-check.sh` not run** — outside the packet's self-verify list, and it would
  fail on the specmap red anyway.
- **`git push` not run**, no process I did not start was stopped, `CARGO_TARGET_DIR` was not
  overridden, and no file under `vibevm/vibepacks/org.vibevm.doc/web/**` was read, edited or
  staged.

## For the backlog, if the boss wants it

`self bootstrap` under the offline posture will *succeed* when the bundle it names is
already installed byte-for-byte (see **Semantics as implemented**). I believe that is right,
and it is the behaviour PROP-010's `OFFLINE-LOCAL-ONLY` describes — but it is the one place
where "no `self` verb makes a request" and "every release-lane verb refuses" are not the
same sentence, so it is worth an owner's eye before it hardens into precedent.
