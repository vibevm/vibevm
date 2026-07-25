# DRIFT-018 — the last two test files stop touching the real user home {#root}

<status stage="impl" state="plan" ref="DRIFT-018"/>

**Status:** queued (blocked on DRIFT-016 — both may touch `tests/common/mod.rs`)
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (vibe-cli e2e tests)
**Unit-stability check:** test-isolation defects; no spec anchor moves.

## 1. Goal {#goal}

`cli_init.rs` and `cli_search.rs` join the four files DRIFT-012 already
isolated, so no test in this workspace reads or writes the developer's real
`~/.vibe/`.

## 2. Contract {#contract}

> **Never let a manual test touch real user state.** Every run isolates its
> project into a scratch directory and redirects the tool's per-user cache
> into that scratch. A test that mutates the real per-user state is a bug in
> the test.
> — `spec://org.vibevm.world/manual-tests/flows/manual-tests/…#never`

Findings realised: **F-056** (`cli_init.rs`) and **F-057** (`cli_search.rs`).

## 3. Current state {#current}

Verified 2026-07-25 — do not re-discover.

**F-056, `crates/vibe-cli/tests/cli_init.rs`.** Its own local builder at
`:10-16` sets `VIBE_NO_DEFAULT_REGISTRY` and nothing else. Two halves, of
different severity, and the distinction matters:

- **Reading — unconditional and live.** All 11 tests run `vibe init`, whose
  non-interactive author resolves through
  `commands/init/prompts.rs:99-110`: `--author` → `user_config.init.last_author`
  → `detect_git_author()`. No test passes `--author`, so the author comes
  out of the developer's real `~/.vibe/config.toml` and the tests' behaviour
  depends on the machine.
- **Writing — real but conditional.** `maybe_save_author`
  (`commands/init/mod.rs:178`) sits *outside* the `if interactive` branch so
  it runs every time, but `prompts.rs:132` writes only when the computed
  author differs from the stored one. On this machine `last_author` is
  already set, so nothing is written today. On a fresh machine or in CI —
  `last_author` unset, `git config user.name` set — the first test run
  **does** write into the real user config.

**The production behaviour is correct and out of scope.** `vibe init`
remembering the last author is the feature; the owner confirmed it. Nothing
in `commands/init/` changes.

**F-057, `crates/vibe-cli/tests/cli_search.rs:1077, 1135, 1157, 1181.** Four
bare `vibe()` sites with no `VIBEVM_SEARCH_CACHE_DIR`, falling back to the
real `~/.vibe/search-cache/`.

**The shape of the fix already exists:** `crates/vibe-cli/tests/common/mod.rs`
carries `UserScratch` (DRIFT-012), an owned per-test tempdir that sets
`VIBE_SETTINGS` and `VIBE_REGISTRY_CACHE` together.

## 4. Required behavior {#behavior}

1. Route every command in both files through `UserScratch`, deleting
   `cli_init.rs`'s local `vibe()` builder rather than adding a second env
   var to it. One helper, not N copies — that is the lesson DRIFT-012 paid
   for, and a builder that sets *some* of the isolation is how F-056 was born.
2. If `UserScratch` needs to grow `VIBEVM_SEARCH_CACHE_DIR` for the search
   tests, add it there, beside the other two, with the same style of comment
   naming why it exists. Do not set it ad-hoc at four call sites.
3. **Prove the reading half was real.** Before the fix, run `cli_init.rs`
   with `VIBE_SETTINGS` pointed at an empty scratch and confirm the
   resolved author changes — that is what shows the tests were reading the
   developer's config, and it is the observation the finding rests on.
   Report it in §9.
4. **Prove the writing half is closed.** With the fix in place, run the full
   `cli_init.rs` suite against a scratch settings home whose `last_author`
   is *unset* and `git config user.name` set — the condition that fires on a
   fresh machine — and confirm the real `~/.vibe/config.toml` mtime does not
   move while the scratch one does. That is the acceptance; a green suite is
   not.
5. Sweep whatever else the two files reach. DRIFT-012 judged
   `cli_search.rs`, `cli_workspace_publish.rs`, `tree_fixture.rs`,
   `tree_json.rs`, `vvm.rs` and `cli_live_e2e.rs` unaffected for
   *resolution*; `cli_search.rs` turned out to have a different leak. Check
   the others for the same class — a cache, a config, any per-user path —
   and report which you cleared and why.

Edge cases: a test that deliberately exercises real user state (if any
exists) points at a fixture it owns. Name it in §9 if you find one.

Error paths: none — test-side only.

## 5. Boundaries {#boundaries}

- **Do not change production code.** Not `commands/init/`, not the search
  cache resolution. Both behave correctly; the tests do not isolate them.
- Do not weaken an assertion to make a test pass under isolation. If a test
  fails once isolated, it was asserting against this developer's machine —
  that is §8, not a fix.
- Never edit spec text.

## 6. Acceptance {#acceptance}

```bash
cargo test -p vibe-cli --test cli_init --test cli_search
bash tools/self-check.sh          # green, no VIBE_SETTINGS override
```

- The §4.3 and §4.4 observations, both reported in §9 with their numbers.
- After the full `vibe-cli` suite: real `~/.vibe/config.toml` and
  `~/.vibe/search-cache/` mtimes unchanged. Record them before and after.
- A grep over `crates/vibe-cli/tests/**` for `Command::cargo_bin` shows every
  site routed through the helper, or names each exception with its reason.
- Discipline: `cargo fmt --all`, clippy clean, atomic commits, no AI
  attribution.

## 7. Analogies {#analogies}

`crates/vibe-cli/tests/common/mod.rs` `UserScratch`, and the four files
DRIFT-012 converted — `cli_pkg_cycle.rs` is the fullest example.

## 8. Stop rule {#stop}

If isolating either file makes a test fail: STOP, list them in §9, return.
A test calibrated against this machine is a finding, not something to guess
the intent of.

Budget signal: past ~4 files or ~300 lines, stop and return.

## 9. Log {#log}

- queued 2026-07-25 (Fable). Both found by DRIFT-012's sweep and left for
  their own task; F-056's severity was corrected after the owner asked which
  direction actually leaks.

- **implemented 2026-07-25.** Three files touched, all test-side:
  `crates/vibe-cli/tests/common/mod.rs` (`UserScratch` grows a third
  variable), `crates/vibe-cli/tests/cli_init.rs` (local builder deleted),
  `crates/vibe-cli/tests/cli_search.rs` (both local builders deleted). No
  production code, no spec text, no assertion weakened. `cargo test -p
  vibe-cli --test cli_init --test cli_search`: 11 + 15 passed, 0 failed —
  no test failed under isolation, so §8 never fired.

- **§4.3 — the reading half was real.** Measured on the built binary
  before the fix, `vibe init` into a fresh project, reading `authors` back
  out of the generated `vibe.toml`:

  | # | environment | resolved `authors` |
  |---|---|---|
  | A | pre-fix test env (`VIBE_NO_DEFAULT_REGISTRY=1` only) | `["Oleg Chirukhin"]` |
  | B | + `VIBE_SETTINGS` = empty scratch | `["Oleg Chirukhin"]` |
  | C | + `VIBE_SETTINGS` = scratch seeded `last_author = "DRIFT-018 Scratch Author"` | `["DRIFT-018 Scratch Author"]` |
  | D | + `VIBE_SETTINGS` = empty scratch, `git config user.name` suppressed | `[]` |
  | E | real `~/.vibe`, `git config user.name` suppressed | `["Oleg Chirukhin"]` |

  **B is the trap.** On this machine `git config user.name` and
  `[init] last_author` are the same string, so the empty-scratch check the
  task sketched comes out *unchanged* and reads as a false negative. C
  (the resolved author tracks whatever `<settings>/config.toml` says) and
  the D-vs-E pair (identical command, git identity silenced in both, only
  the settings home differs → `[]` vs `["Oleg Chirukhin"]`) are what
  actually prove the read was live. Anyone re-running this check should
  use C or D/E, not B.

- **§4.4 — the writing half is closed.** Fresh-machine condition
  throughout: `last_author` UNSET in the settings home, `git config
  user.name` = `Oleg Chirukhin`. mtimes are `LastWriteTimeUtc.Ticks`.

  | observation | before | after |
  |---|---|---|
  | scratch `config.toml`, `UserScratch::vibe()` env shape | `<absent>` | `639206067240932551`, body `[init] last_author = "Oleg Chirukhin"` |
  | real `~/.vibe/config.toml`, same run | `639204389589244541` | `639204389589244541` |
  | counterfactual: `<home>/.vibe/config.toml` under the **pre-fix** builder shape (`VIBE_NO_DEFAULT_REGISTRY=1` and nothing else), `HOME` redirected so the write is observable | `<absent>` | `639206067619793458`, body `[init] last_author = "Oleg Chirukhin"` |

  The counterfactual is the point: the deleted builder *does* create the
  settings-dir `config.toml` on a machine where `last_author` is unset —
  it was only inert here because this developer's is already set. (First
  attempt at that counterfactual mis-measured: redirecting `HOME` also
  moves git's global config, which silenced `detect_git_author()` and
  produced `authors = []` and no write. `GIT_CONFIG_GLOBAL` has to be
  pinned back at the real `~/.gitconfig`.)

- **Real per-user state after the full `vibe-cli` suite** (401 unit + 143
  integration, all green) and again after the whole floor: every path
  under `~/.vibe` byte-identical in path set and mtime. `config.toml`
  `639204389589244541` → unchanged; `search-cache/`
  `639137006268636522` → unchanged; `registry.toml` `639205798576113406`
  → unchanged.

- **F-057 is worse than "a cache".** Two things the finding did not name:

  1. *The residue is still on disk.* `~/.vibe/search-cache/primary/` and
     `…/secondary/` hold three JSON entries dated 2026-05-06T21:43:46Z
     whose payloads are verbatim `cli_search.rs` mock fixtures
     (`"description": "ok."` from
     `search_reports_unreachable_registry_without_aborting`;
     `"description": null, "score": 1` from
     `search_filters_to_one_registry_via_flag`), filed under this file's
     mock registry names. Left in place — deleting a developer's files is
     not this task's call.
  2. *It leaked a credential, not just cache bytes.*
     `commands/search.rs:427` calls
     `vibe_publish::token::load_token_for_host("github.com")`, whose
     4th precedence leg is `<settings-dir>/github.publish.token` — a file
     that exists here (41 bytes). `search_full_scan_finds_matching_packages_in_github_org`
     clears `VIBEVM_PUBLISH_TOKEN` / `…_GITHUB` (legs 2–3) but could not
     reach leg 4. Measured against a loopback listener that records only
     header names and value length: pre-fix shape → request headers
     `accept, x-github-api-version, authorization, user-agent, host`,
     Authorization value **47 bytes** (`Bearer ` + a 40-char token, matching
     the file); `UserScratch` shape → `accept, x-github-api-version,
     user-agent, host`, **no Authorization at all**. The suite was handing
     the developer's real GitHub token to its own localhost mock.

- **§4.5 sweep — what the two files still reach, and the other files.**

  - **Universal, every `vibe` subprocess:** `main.rs:51`
    `promote_user_config_env()` runs before dispatch and `UserConfig::load()`s
    `<settings-dir>/config.toml`, promoting its `[env]` table into the
    process env. So *every* un-isolated `Command::cargo_bin("vibe")` in
    this tree reads the developer's real `config.toml`, whatever the
    subcommand. Inert today (this machine's `[env]` is empty); a developer
    with `[env] VIBE_LOG` or `VIBEVM_INDEX_URL_*` set would have it
    injected into every test.
  - **Residual in the two converted files:** `detect_git_author()` shells
    out to `git config user.name`, which reads the developer's real global
    gitconfig. Cleared — it is git's user state, not vibevm's, no test in
    `cli_init.rs` asserts on `authors`, and pinning `GIT_CONFIG_GLOBAL`
    would only matter if one ever did.
  - **`~/.vibevm/` legacy leg:** `read_per_host_token` / `read_legacy_token`
    fall back to `~/.vibevm/<host>.publish.token`, which `$VIBE_SETTINGS`
    deliberately does **not** relocate (`settings.rs:78`). Cleared here —
    the directory does not exist on this machine — but it is a per-user
    path no scratch can close, worth knowing before someone claims full
    isolation.
  - **`cli_workspace_publish.rs` — NOT clear, a new finding.** It imports
    the *free* `common::vibe` and the *free* `common::init_project`
    (`:9`), so four tests (`:27`, `:113`, `:206`, `:253`) run `vibe init`
    against the real settings home: the F-056 mechanism verbatim, in a
    third file. It also runs `vibe registry publish`, which reaches
    `<settings-dir>/<host>.publish.token`. Left unconverted — §1 scopes
    this task to two files and §8 caps the budget — but it should be
    queued; the fix is mechanical (`UserScratch` + `user.vibe()`), and
    until then §6's "real `config.toml` unchanged after the full suite"
    holds here only because *this* machine's `last_author` is already set.
  - **`tree_json.rs:34` / `tree_fixture.rs:156`** — bare
    `Command::cargo_bin`. `commands/tree/mod.rs:40-41` builds
    `TreeSettings::new()` + `settings.load()` *before* the `--json`
    branch, and L1 is `<settings-dir>/settings.toml` (present here, 154
    bytes), so both **read** the developer's prefs. Cleared as low
    severity: `--json` takes the early return at `:47`, never reaching
    `record_last_project`, so nothing is written, and neither test
    asserts on anything prefs can move. Named as an exception, not clean.
  - **`vvm.rs:15`** — local builder pinning `VIBEVM_INSTALL_ROOT` and
    dropping `VIBEVM_HOME`; `vibe self` is excluded from
    `needs_global_registry`. Cleared: the state it owns is already
    scratched, only the universal `[env]` promotion remains.
  - **`cli_live_e2e.rs:65`** — local builder, same F-056 shape, but all
    three tests are `#[ignore]`d and the file exists to reach the live
    internet with the operator's real SSH key. Out of the hermetic suite;
    isolating it blindly could break the credentials it needs. Named as a
    deliberate exception.
  - `cli_pkg_cycle.rs`, `cli_mcp_kind.rs`, `cli_redirect.rs`,
    `cli_registry_mgmt.rs`, `cli_progress_sidecar.rs` — already
    `UserScratch`-routed, no bare `cargo_bin` site. Clear.

- **Floor.** Run plainly, no `VIBE_SETTINGS` override:
  `cargo fmt --all --check`, `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `vibe check --path .` (0 errors, 1 warning), `sync-engines --check` and
  every package gate all pass. `cargo xtask conform check` reports 3 new
  `no-unwrap-in-domain` findings, all three in
  `crates/vibe-cli/src/commands/progress/tests/writes.rs` — an untracked
  file belonging to a different, concurrent task. Nothing in this task's
  diff contributes a finding.
