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
