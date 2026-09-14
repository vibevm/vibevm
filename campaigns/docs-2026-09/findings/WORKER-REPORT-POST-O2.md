# WORKER-REPORT-POST-O2 — `vibe self update` moves forward, `vibe self reinstall` refetches

Packet: `campaigns/docs-2026-09/findings/PACKET-POST-O2.md` (plus the coordinator's
mid-task refinement to step 1: an unchanged version number is not a no-op, and an
unchanged digest must skip the bundle download).

**Commit:** `02d586e3` — `feat(self): update moves a binary install forward, reinstall refreshes the current one`
(10 files, +695 / −98; not pushed). Another worker's `818247cd` landed on top of it
afterwards; both are on `research-preview-1-docs`.

## Files touched (all inside the packet's perimeter)

- `crates/vibe-cli/src/cli/vvm.rs`
- `crates/vibe-cli/src/commands/vvm/mod.rs`
- `crates/vibe-cli/src/commands/vvm/bundle.rs`
- `crates/vibe-cli/src/commands/vvm/bundle/archive.rs`
- `crates/vibe-cli/src/commands/vvm/bundle/tests.rs`
- `crates/vibe-cli/src/commands/vvm/bundle/network_tests.rs`
- `crates/vibe-cli/src/commands/vvm/bundle/release_tests.rs` *(new)*
- `crates/vibe-cli/src/commands/vvm/tests.rs`
- `crates/vibe-cli/src/commands/vvm/error.rs`
- `vibevm/vibespecs/common/PROP-019-version-manager.xml`

Nothing outside it. `specmap.json`, `schemas/**` and the manual are untouched — see
*Gates* for the consequence.

## Semantics as implemented

**`vibe self update`, binary execution.** Reads
`https://github.com/vibevm/vibevm/releases/latest/download/DISTRIBUTIONS.json`
(cache-busted, same nonce scheme as before), parses `version`, and compares it with the
running instance's logical SemVer.

- *Newer* → `select_platform` → `install_selected` from `…/releases/download/v<newest>`,
  the same verified path an explicit `X.Y.Z` takes; `--force` passes through. Prints
  `updating 1.0.0 → 1.1.0` before the install.
- *Equal* → **not** a no-op. The manifest's declared bundle digest is compared with the
  installed instances' `payload_sha256` and the local copy is re-verified in place
  (`archive::installed_from_published_bundle`, which is exactly the reuse test
  `install_bundle` already ran after downloading). Unchanged → **the bundle is never
  fetched**, the instance is reused and activated, line
  `newest release is 1.0.0 — already installed`. Changed (a release rebuilt under its own
  number) → the bundle is downloaded and a fresh immutable `#N` is installed, line
  `newest release is still 1.0.0, rebuilt since this install — reinstalled`.
  `--force` skips the pre-check and always lands a fresh `#N`.
- *Older than installed* (a withdrawn release) → nothing is fetched, nothing moves; one
  line naming the deliberate way down (`vibe self install <older>`).

**`vibe self reinstall`** — new verb. Binary execution: the manifest of **its own**
version (`…/releases/download/v<current>/DISTRIBUTIONS.json`) and
`install_selected(force = true)`, so always a fresh `#N` and never a question about what
is newest. Source execution: exactly what `update --force` did for source — rebuild
`latest` with `force`, same `--profile` / `--release`. No `--force` flag is declared
(a fresh instance is the whole verb). `--json` envelope is byte-identical in shape to
`update`'s; no new fields, no schema touched.

**`vibe self install stable`, binary execution** — routes into the same newest-release
function (`binary_lane` → `BinaryLane::Newest`), instead of falling through to
`choose_mirror` → clone → `cargo`. `install latest` still refuses, with the text now
naming all three verbs.

**Pre-download reuse check.** `install_selected` gained the manifest-only reuse test in
front of the download. It therefore also benefits `self bootstrap` and
`self install X.Y.Z` without `--force`: an already-installed, still-verifying release is
recognised without refetching ~75 MB. Every *actual* install still walks the full
verified path (fetch → outer hash → embedded-manifest cross-check → per-component hash →
safe extraction). The install lock is taken before activation on the reuse path too; its
scope on the download path is unchanged (still acquired after the download, not across
it, because `InstallLock::acquire` is a fail-fast `try_lock`).

**Norm.** `##CMD-UPDATE` rewritten on the owner's word (forward movement, the three
outcomes, the digest rule, `stable` entering the same path); `##CMD-REINSTALL` added
beside it (`impl/done`, `action="continue" actionstage="doc" audience="user"`).
`##SEL-STABLE` untouched, as instructed.

## Help texts changed — for the manual's `derived` block

Only these two lines changed on the `vibe self` help surface; nothing else was touched.

- `update` — **was:** `Refresh and activate the current logical version. Binary executions
  refetch their mutable release; source executions rebuild latest.`
  **now:** `Move a binary installation to the newest release; a source execution rebuilds
  `latest`.`
- `reinstall` — **new:** `Refetch and reinstall the current version: a binary execution
  redownloads its own release, a source execution rebuilds it.`

`vibe self reinstall --help` carries `--profile`, `--release` and the global
`--json` / `--quiet` / `--invoked-by` / `--agent-mode` / `--unattended` / `--offline`.

The `VvmError::BinaryFetchUnavailable` message also changed (it is an error, not help):
`fix: use `vibe self update` for the newest release, `vibe self install X.Y.Z` for a
specific one, `vibe self reinstall` to refresh the current one, or `--mirror` for a
source build`.

## `--offline` — it does not reach `vvm` (packet item 4)

Measured, not inferred. The root `--offline` parses and appears in `vibe self … --help`,
but `main.rs` never hands it to the version manager:

```rust
Command::Vvm(args) => {
    let vvm_env = commands::vvm::VvmEnv { root: …, cwd: …, home: …, shell: …, path_var: … };
    commands::vvm::run(&ctx, args, vvm_env)   // no `cli.offline`
}
```

`output::Context` carries no offline flag either, and the vvm domain is documented as
never reading the process environment itself (composition-root rule, `VvmEnv`). Live
probe on the isolated root:

```
=== vibe --offline self update (flag parses, never reaches vvm) ===
note: a running `vibe-index serve` keeps its old process; restart it to use this instance.
durable PATH was already configured; reopen this shell to pick it up.
reused tag:1.0.0#3 — active
newest release is 1.0.0 — already installed
EXIT=0
```

The command went to the network and succeeded. Per the packet I did **not** invent the
channel: the honest `--offline` errors for `update` / `stable` / `reinstall` are not
implemented, and cannot be until someone threads the resolved offline posture into
`VvmEnv` (one field at the composition root, then three call sites in `bundle.rs`).

## Gates — verbatim

Every Rust gate was run with `CARGO_TARGET_DIR` pointed at a private scratch directory
(the shared `target/` belongs to another worker); that directory has been deleted
(40.3 GB reclaimed).

```
$ cargo fmt --all -- --check
FMT CLEAN                                  # exit 0, no diff
```

```
$ cargo clippy -p vibe-cli --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 10s
                                           # exit 0, no warnings
```

```
$ cargo test -p vibe-cli vvm
    Finished `test` profile [unoptimized + debuginfo] target(s) in 55.36s
running 117 tests
test result: ok. 117 passed; 0 failed; 0 ignored; 0 measured; 641 filtered out; finished in 22.53s
   # …plus 38 further test binaries, each "0 passed; 0 failed" (name filter)
```

The six new cases, listed by name:

```
$ cargo test -p vibe-cli --bin vibe vvm::bundle
running 19 tests
test commands::vvm::bundle::tests::release_tests::an_unchanged_newest_release_is_settled_by_the_manifest_without_a_download ... ok
test commands::vvm::bundle::tests::release_tests::a_release_rebuilt_under_its_own_number_is_refetched_and_installed ... ok
test commands::vvm::bundle::tests::release_tests::a_newer_release_is_installed_and_activated_by_the_verified_path ... ok
test commands::vvm::bundle::tests::release_tests::a_withdrawn_newest_release_never_walks_the_machine_backwards ... ok
test commands::vvm::bundle::tests::release_tests::an_unreadable_newest_release_manifest_fails_the_command_and_changes_nothing ... ok
test commands::vvm::bundle::tests::release_tests::reinstall_refetches_the_running_version_without_asking_what_is_newest ... ok
test commands::vvm::bundle::tests::network_tests::a_named_release_uses_mutable_assets_and_force_allocates_a_fresh_instance ... ok
   # + the 12 pre-existing bundle/archive cases
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 739 filtered out; finished in 0.97s
```

Two more cases live in `commands::vvm::tests` (`cargo test … vvm` above ran them):
`a_binary_execution_routes_stable_to_the_newest_release_and_latest_nowhere` (the
`binary_lane` classification — `stable` → newest, `latest` → refusal, `1.2.3`/`v1.2.3` →
that release, a branch → source lane) and `the_latest_refusal_names_every_release_verb`.

```
$ cargo test -p vibe-cli cli_help
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 41s
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 758 filtered out; finished in 0.00s
   # …and 0 in every other target: the filter matches nothing.
```

**The `self` help is NOT pinned by any `vibe-cli` test.** `cli_help` occurs in this
repository only as a module name in `crates/vibe-doc/src/derived.rs` (the derived-docs
generator), never as a test name.

```
$ cargo build -p vibe-cli
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 01s   # exit 0
```

```
$ cargo xtask specmap --check
Error: `C:\Users\olegc\git\v\vibevm-docs\specmap.json` is out of date relative to the tree.
  drift: unbumped-hash: `spec://org.vibevm.core/vibevm/common/PROP-019#surface` content changed while the revision stayed at r3 — editorial, or forgot to bump? (bump `r`, or mark the commit body `spec-editorial: surface`)
  drift: units added: 1
  drift: edges added: 20
Run `rust-ai-native-specmap` (or your project's wrapper), review the drift, and commit the result.
                                           # exit 1
```

**This is red and I left it red — deliberately.** See *Open for the boss*.

## Live probe — verbatim

Isolated root (`VIBEVM_INSTALL_ROOT=<scratch>\post-o2-root`), nothing written to
`~/.vibe/opt`. The release's own bootstrap was verified against the manifest before it
ran (declared `sha256:ad0d79b9…30bc`, 74 073 600 bytes — both matched the download).

```
$ vibe-bootstrap.exe self bootstrap --manifest …\DISTRIBUTIONS.json --version 1.0.0 \
      --release-base https://github.com/vibevm/vibevm/releases/download/v1.0.0
note: a running `vibe-index serve` keeps its old process; restart it to use this instance.
durable PATH updated; reopen the shell to resolve the stable shims.
installed tag:1.0.0#1 — active
EXIT=0
```

**Making my build a real binary execution.** The packet anticipated this: a binary built
into a private `target/` is *not* a binary execution — `provenance::running_identity` →
`selfloc::derive_self` recognises a managed instance purely by path shape
(`…/opt/vibevm/versions/<kind>/<id>/<N>/[bin/]vibe.exe`), and anything else falls back to
source detection. I did not work around the branch; I entered it properly, with
`self import`, which inventories a ready-built executable as `origin=binary` under a
release tag:

```
$ vibe.exe self import <built vibe.exe> --tag 1.0.0 --use
imported tag:1.0.0#2
active → tag:1.0.0#2

$ <root>\opt\vibevm\versions\tag\1.0.0\2\vibe.exe self current
*> tag:1.0.0#2  origin=binary source="local-import" commit=unknown sha256=9cefb9ca… profile=release
```

Every probe below was then invoked from that instance's own path, so `running_record`
really is `origin=binary`, tag `1.0.0`. Nothing was stubbed or faked.

```
$ … \tag\1.0.0\2\vibe.exe self update
note: a running `vibe-index serve` keeps its old process; restart it to use this instance.
durable PATH was already configured; reopen this shell to pick it up.
reused tag:1.0.0#1 — active
newest release is 1.0.0 — already installed
EXIT=0
```

Newest published release really is 1.0.0, so this is the *equal, unchanged digest* case:
it read only the `releases/latest/download/DISTRIBUTIONS.json` manifest, matched the
digest against the bootstrap's instance `#1`, skipped the 75 472 108-byte bundle, and
reused and activated `#1`. (Compare the old behaviour the packet measured: the bundle
was downloaded first and only then found unchanged.)

```
$ … \tag\1.0.0\2\vibe.exe self install stable
note: a running `vibe-index serve` keeps its old process; restart it to use this instance.
durable PATH was already configured; reopen this shell to pick it up.
reused tag:1.0.0#1 — active
newest release is 1.0.0 — already installed
EXIT=0

$ … \tag\1.0.0\2\vibe.exe self install latest
error: `latest` is a source-branch selector, not a binary release version (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; fix: use `vibe self update` for the newest release, `vibe self install X.Y.Z` for a specific one, `vibe self reinstall` to refresh the current one, or `--mirror` for a source build)
EXIT=1
```

`stable` is now byte-identical to `update` — no clone, no `cargo`.

```
$ … \tag\1.0.0\2\vibe.exe self reinstall
note: a running `vibe-index serve` keeps its old process; restart it to use this instance.
durable PATH was already configured; reopen this shell to pick it up.
installed tag:1.0.0#3 — active
EXIT=0
elapsed: 23s        # a real ~75 MB refetch, versus the sub-second update above

$ … self ls
  →   tag:1.0.0#1  origin=binary source="…\1\source" commit=a6b8c774… sha256=b260e66e… profile=release
  → > tag:1.0.0#2  origin=binary source="local-import" commit=unknown sha256=9cefb9ca… profile=release
  → * tag:1.0.0#3  origin=binary source="…\3\source" commit=a6b8c774… sha256=b260e66e… profile=release
3 instance(s) installed.
```

`#3` is the fresh instance `reinstall` produced; the packet predicted `#2`, which is off
by one only because `#2` is my imported build.

```
$ … \tag\1.0.0\2\vibe.exe --json self update
{
  "ok": true,
  "command": "self:update",
  "selector": "tag:1.0.0#3",
  "instance": 3,
  "home": "…\\versions\\tag\\1.0.0\\3",
  "source": "…\\versions\\tag\\1.0.0\\3\\source",
  "payload_sha256": "b260e66e17ec19b208126706ff2ba3a0dbe7eafec9d212d0754667e24efa35f0",
  "reused": true,
  "vibe_index_restart_required": true,
  "path_on_current_process": false,
  "durable_path_changed": false,
  "reopen_shell": true,
  "advisory_home_warning": null
}
```

**Not verifiable live:** the *newer release* and *withdrawn release* branches — GitHub
publishes exactly one release (1.0.0), so there is no second version to move to or fall
back from. Both are covered by the substituted-`Downloader` tests above, which assert the
real URLs (`…/releases/latest/download/DISTRIBUTIONS.json` for the question,
`…/releases/download/v1.1.0/vibevm-1.1.0-…zip` for the answer), the installed bytes, the
activation, and the preserved `previous` pointer.

## Cleanup

- Private `CARGO_TARGET_DIR` deleted (40.3 GB).
- `<scratch>\post-o2-root`, the downloaded bootstrap/manifest, and two helper dirs deleted.
- Durable user `Path` restored **byte-for-byte** to its pre-probe snapshot (the
  `…\post-o2-root\opt\bin` entry the bootstrap appended is gone).
- The probe also set the durable user `VIBEVM_HOME` (advisory) to an instance inside the
  scratch root — the packet did not anticipate this. I repointed it to the owner's own
  active instance, read from `C:\Users\olegc\.vibe\opt\vibevm\current`:
  `C:\Users\olegc\.vibe\opt\vibevm\versions\branch\main\71`. Nothing under `~/.vibe/opt`
  was written; no token was read; no process I did not start was stopped;
  `tools/self-check.sh` was not run.

## Open for the boss (not done, and why)

1. **`cargo xtask specmap --check` is red.** The map must be regenerated and the
   `#surface` revision question answered — and both are outside the packet's perimeter
   (`specmap.json` … не трогать). Two concrete decisions:
   - *Regenerate:* `cargo xtask specmap` (no `--check`). Note this repository's own
     convention is a separate `chore(specmap): regenerate the map after …` commit — the
     last five commits touching `specmap.json` are all of exactly that shape — which is
     why folding it into a feature commit would have been wrong anyway. A further reason
     to leave it: while I worked, another session had `specmap.json` modified in this
     shared worktree, so regenerating from my side would have baked in their in-flight
     state.
   - *The `r` bump:* the drift says PROP-019 `#surface` changed while its revision stayed
     `r3`. This change is **not** editorial — it adds a verb and changes what `update`
     does — so the honest answer is bumping `req r3` → `r4` in §2.2, not marking the
     commit `spec-editorial: surface`. I did not bump it: a revision bump invalidates
     every existing `#[verifies("…#surface", r = 3)]` edge across the tree, and I could
     not measure that blast radius without regenerating the map. My two new
     `#surface` verifications are written against `r = 3`; if you bump, they move with
     the rest.
2. **The manual's `derived` block** — the two help lines above changed; the packet
   reserves that rewrite for the central session.
3. **`--offline` for the release lane** — see above; a channel exists nowhere and I did
   not invent one.
4. **Two pre-existing-style failures in `crates/vibe-cli/tests/vvm.rs`**, caused by my
   environment, not by this change. `ls_on_a_fresh_root_still_identifies_the_direct_source_execution`
   and `which_reports_the_direct_source_executable_without_an_active_version` assert that
   the test binary's own path resolves to a vibevm source root
   (`source::find_source_root` walks up from `current_exe` looking for `Cargo.toml` +
   `crates/vibe-cli/`). With `CARGO_TARGET_DIR` outside the repository — which the packet
   required — there is no such root above the binary. Demonstrated directly: the *same*
   binary, run from the scratch target, prints `(no versions installed …)`; copied under a
   vibevm-shaped directory it prints exactly the asserted line:
   ```
   →  > source origin=external source="…\fakeroot" executable="…\fakeroot\target\debug\vibe.exe" commit=unknown sha256=- selector=-
   ```
   The packet's own filter (`cargo test -p vibe-cli vvm`) does not reach this target
   anyway — it filters by test *name*, and none of these four names contains `vvm`. They
   need a target dir inside the checkout to run, which belongs to the boss.

## Deviations from the packet

One, deliberate, and it is the coordinator's own refinement: step 1's "equal → `reused`"
became the three-way digest rule (equal + unchanged → reuse, no download; equal + changed
→ fresh `#N`), with the reuse check moved in front of the download for every release
install. Everything else is as written. The `#3`-instead-of-`#2` numbering in the live
probe (step 8) follows from importing my build as `#2`, as the step itself allowed for.
