# DRIFT-012 — the e2e harness stops drinking the developer's settings {#root}

<status stage="impl" state="plan" ref="DRIFT-012"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (vibe-cli e2e tests)
**Unit-stability check:** F-055 is a test-isolation defect, not a spec
obligation — no anchor moves and no marker changes.

## 1. Goal {#goal}

`bash tools/self-check.sh` is green on a developer machine that has a real
`~/.vibe/`, so the floor stops needing a `VIBE_SETTINGS` incantation to tell
the truth.

## 2. Contract {#contract}

> **Never let a manual test touch real user state.** Every run isolates its
> project into a scratch directory and redirects the tool's per-user cache
> into that scratch. A test that mutates the real per-user state is a bug in
> the test.
> — `spec://org.vibevm.world/manual-tests/flows/manual-tests/…#never`

The law is written for the manual tier; it binds the automated tier at least
as hard, and this is the inbound direction of the same rule — a test that
*reads* real user state is as broken as one that writes it.

Finding realised: **F-055**.

## 3. Current state {#current}

Root-caused and proven in-session on 2026-07-25 — do not re-discover:

- `crates/vibe-cli/tests/cli_pkg_cycle.rs` isolates `VIBE_REGISTRY_CACHE`
  (seven `.env(…)` sites: lines 791, 873, 966, 1032, 1048, 1097, 1111) and
  **nothing else**.
- A real `~/.vibe/registry.toml` appeared on this machine on 2026-07-25 at
  15:37 carrying `vibespecs` + `vibespecs-gitverse`. Those global registries
  merge into the "hermetic" resolver, so the run mints three cache buckets
  where the test asserts one.
- The failure is the `assert_eq!(clone_dirs.len(), 1, "expected one registry
  cache bucket")` at `cli_pkg_cycle.rs:836`.
- Proof it is the settings chokepoint and nothing else: with `VIBE_SETTINGS`
  pointed at an empty directory the same binary passes, repeatedly.

## 4. Required behavior {#behavior}

1. Every `Command` this test file builds gets `VIBE_SETTINGS` pointed into
   the test's own scratch directory, beside the existing
   `VIBE_REGISTRY_CACHE`. Not just the failing test — **all seven sites**: a
   test that passes today only because the developer's settings happen to be
   empty is a latent version of the same bug.
2. Prefer one helper over seven copies: if the file already has a command
   builder, add the env there; if it does not, add one and route the sites
   through it. Seven copies of an isolation rule is seven chances to forget
   the eighth.
3. Sweep the rest of the e2e surface: any other test in
   `crates/vibe-cli/tests/**` that constructs a `vibe` command and asserts on
   resolution, cache layout, or registry contents gets the same treatment.
   Report which files you touched and which you judged unaffected, with the
   reason.
4. The scratch `VIBE_SETTINGS` directory must be inside the per-test
   tempdir, so it dies with the test and two tests never share one.

Edge cases: a test that *deliberately* exercises global settings (if one
exists) must keep doing so — point its `VIBE_SETTINGS` at a fixture it owns,
never at the real home. If you find such a test, name it in §9.

Error paths: none — this is test-side isolation only.

## 5. Boundaries {#boundaries}

- **Do not change production code.** The chokepoint's behaviour is correct:
  merging global registries is what `ensure_default_global_registry` is for.
  The defect is that a hermetic test did not opt out of it.
- Do not weaken the `assert_eq!(…, 1, …)` — the assertion is right, the
  environment was wrong. An assertion relaxed to make a red test green is
  the failure mode this whole campaign exists to catch.
- Never edit spec text.

## 6. Acceptance {#acceptance}

```bash
bash tools/self-check.sh                 # green WITHOUT any VIBE_SETTINGS override
cargo test -p vibe-cli --test cli_pkg_cycle
```

- The floor is green on this machine **with the developer's real `~/.vibe/`
  in place** — that is the whole point, so run it that way, not isolated.
- New assertion or comment at the isolation helper naming why both env vars
  are set, so the next person does not delete one.
- Verify the fix actually bites: temporarily add a second registry to a
  scratch `VIBE_SETTINGS` fixture and confirm the test still sees one bucket.
  Report that check in §9; do not leave the fixture behind.
- Discipline: `cargo fmt --all`, clippy clean, atomic commits, no AI
  attribution.

## 7. Analogies {#analogies}

The existing `.env("VIBE_REGISTRY_CACHE", &cache)` sites in this same file
are the shape — you are adding the sibling nobody added when the settings
home landed.

## 8. Stop rule {#stop}

If isolating `VIBE_SETTINGS` makes a *different* test fail, that test was
also drinking user state and its expectations were calibrated against this
developer's machine: STOP, list them in §9, and return. Do not fix them by
guessing what they meant to assert.

Budget signal: past ~4 files or ~250 lines, stop and return.

## 9. Log {#log}

- queued 2026-07-25 (Fable). F-055 has been the floor's only red all day.
- implemented 2026-07-25 (Opus).

  **Red reproduced first, verbatim** — `cargo test -p vibe-cli --test
  cli_pkg_cycle`, no override: `install_from_git_registry ... FAILED`,
  `panicked at crates\vibe-cli\tests\cli_pkg_cycle.rs:836`, `left: 3 /
  right: 1`. The real `~/.vibe/registry.toml` (2026-07-25 15:37) carries
  `vibespecs` + `vibespecs-gitverse`; each merges in and mints a bucket.

  **The helper** — `crates/vibe-cli/tests/common/mod.rs:34-110`, new
  `UserScratch`: an owned per-test `tempfile::TempDir` exposing `settings`
  (`$VIBE_SETTINGS`) and `cache` (`$VIBE_REGISTRY_CACHE`), a `vibe()`
  builder that sets **both**, and an `init_project()` that runs `vibe
  init` under them. Its doc comment names why each var is there so
  neither gets deleted (§6). `common::vibe()` keeps a pointer note that
  `VIBE_NO_DEFAULT_REGISTRY` only suppresses *seeding* a fresh
  `registry.toml` and does nothing about one that already exists — that
  gap is exactly F-055.

  **§4.4 note.** The scratch is a tempdir the `UserScratch` value owns
  rather than a subdirectory of the existing `project` tempdir: it still
  dies with the test and no two tests share one, but it cannot perturb
  the project tree, several of whose tests assert on re-install freshness
  scans (`cli_pkg_cycle.rs:251-254`, `:351-353`).

  **Files swept** (4 + the helper), every `Command` routed through the
  scratch — counts are `scratches / isolated commands`
  (`user.vibe()` + `user.init_project()`):
  `cli_pkg_cycle.rs` 20 / 49 (the seven hand-rolled
  `.env("VIBE_REGISTRY_CACHE", …)` folded into the helper; the bucket
  count now reads `user.cache`), `cli_mcp_kind.rs` 2 / 7 (had **zero**
  env isolation and asserts on exact-pin resolution),
  `cli_redirect.rs` 8 / 13, `cli_registry_mgmt.rs` 67 / 148.
  All 97 tests across the four binaries pass.

  **Boundaries honoured.** No production code touched; the
  `assert_eq!(clone_dirs.len(), 1, …)` is unchanged. The 17 sites in
  `cli_registry_mgmt.rs` that deliberately point `VIBE_REGISTRY_CACHE` at
  their own tempdir were left alone — their `.env` still wins over the
  helper's default, which is the intended layering.

  **§6 "does it bite" check, both directions.**
  *Inbound:* seeding a probe `registry.toml` into the scratch settings
  dir made the count go `1 → 2` (`left: 2 / right: 1`), proving
  `$VIBE_SETTINGS` is genuinely the directory the binary reads and the
  green is not accidental. Probe removed; nothing left behind.
  *Outbound:* after running the swept suites (59 `vibe init` calls among
  them), the real `~/.vibe/config.toml` still shows mtime 2026-07-24
  00:29 and `~/.vibe/registries/` 2026-07-20 20:07 — untouched.
  (Note for the reviewer: §6's literal wording — "add a second registry
  to a scratch `VIBE_SETTINGS` fixture and confirm the test still sees
  one bucket" — cannot hold while the global merge stays correct, since
  a registry in the *scratch* must be merged just like a real one. The
  check above is the meaningful form: real home ignored, scratch obeyed.)

  **Not swept, with reasons.** `cli_search.rs`, `cli_workspace_publish.rs`,
  `tree_fixture.rs`, `tree_json.rs`, `vvm.rs`, `cli_live_e2e.rs` — only
  `install` / `update` / `reinstall` read the global layer
  (`GlobalRegistryConfig::load()` at `install/mod.rs:109`,
  `update.rs:117`, `reinstall.rs:191`); `search`, `outdated`, `registry
  list`, `publish` and `tree` read the project manifest only, and
  `cli_live_e2e.rs` is entirely `#[ignore]`.

  **Two follow-ups found, deliberately NOT fixed (§8 budget: 4 files).**
  1. `cli_init.rs` — all 11 tests call `vibe init`, which *writes*
     `last_author` back to the real `~/.vibe/config.toml`
     (`commands/init/mod.rs:177` → `prompts.rs:128-137`
     `maybe_save_author` → `user_config.save()`). That is the §2 contract
     broken in the outbound direction on every run. It has its own local
     `vibe()` builder (`cli_init.rs:10-16`), so it needs the same
     `UserScratch` treatment. Out of §4.3's stated scope (it asserts on
     the project manifest, not on resolution / cache / registry
     contents), so it is not red — just leaking.
  2. `cli_search.rs:1077,1135,1157,1181` — four bare `vibe()` sites with
     no `VIBEVM_SEARCH_CACHE_DIR`, so they fall back to the real
     `~/.vibe/search-cache/`.

  Nothing else failed under isolation, so §8's stop rule never fired.

  **§6 acceptance — `bash tools/self-check.sh`, `$VIBE_SETTINGS` unset,
  the developer's real `~/.vibe/registry.toml` in place.** F-055 is gone:
  `cargo test --workspace` passes. Re-run with `--keep-going` so every
  gate reports; **130 test binaries `test result: ok`, zero
  `test result: FAILED` anywhere**, and exactly one failure line in the
  whole floor:

  ```
  === cargo fmt --all --check ===
  === cargo test --workspace ===
  === cargo clippy --workspace --all-targets -- -D warnings ===
  === cargo run -p vibe-cli -- check --path . --quiet ===
  vibe check: 0 errors, 1 warning, 0 info
  === cargo xtask conform check ===
    conform: NEW file-length crates/vibe-cli/src/commands/progress.rs:1 — violates REQ discipline://rust-ai-native-lang/guide#surface-form: 892 lines exceeds the 600-line file budget; fix surface: split along the file's responsibility seams into module-grain cells
  conform check: 1 finding(s) in scope <workspace> ({"file-length": 1}), 0 frozen in baseline, 1 new; SARIF at target\conform\report.sarif.
  self-check: `cargo xtask conform check` failed (exit 1)
  === cargo xtask sync-engines --check ===
  … (core-ai-native, rust-ai-native-lang, rust-ai-native-mcp,
     typescript-ai-native-mcp: fmt + test + clippy + specmap gates, all clean)
  self-check: failures above (exit 1)
  ```

  That one red is **not this task's** and must not be swept into it:
  `crates/vibe-cli/src/commands/progress.rs` is owned by a concurrent
  session (mtime 2026-07-25 21:56, alongside `progress-core/src/cache.rs`
  at 21:59) and grew past the 600-line budget while this task ran. This
  task touched only `crates/vibe-cli/tests/**`, which conform does not
  scan (`roots = ["crates/*", "xtask"]` over crate source; the 3800-line
  `cli_registry_mgmt.rs` has never been a finding). The floor returns to
  green for DRIFT-012's purposes the moment that file is split by its
  owner — no action here.
