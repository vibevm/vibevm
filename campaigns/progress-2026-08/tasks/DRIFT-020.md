# DRIFT-020 — test isolation stops being a convention {#root}

<status stage="impl" state="plan" ref="DRIFT-020"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (test support + the floor)
**Unit-stability check:** no spec anchor moves.

## 1. Goal {#goal}

A test that forgets to isolate its environment is harmless, and a test
that touches the developer's real `~/.vibe/` fails the floor.

## 2. Contract {#contract}

> **Never let a manual test touch real user state.** … A test that mutates
> the real per-user state is a bug in the test.
> — `spec://org.vibevm.world/manual-tests/flows/manual-tests/…#never`

> Never print, echo, quote, or paste a secret value … Never commit or
> persist a secret anywhere but the one sanctioned per-user,
> permission-protected location.
> — `spec://org.vibevm.world/secrets-hygiene/…#never`

**Owner's ruling, 2026-07-25/26, verbatim:** «Я вот вообще не верю, что
какой-то тест не перезапишет мой `~/.vibe/settings.json` и еще множество
файлов, которые там лежат, например - ключи.»

## 3. Current state {#current}

The distrust is justified — measured 2026-07-26, do not re-discover:

- The real `~/.vibe/` holds **four credential files**:
  `github.publish.token`, `git.publish.token`, `zai.api.token`,
  `zai.api.token.2` — plus `config.toml`, `settings.toml`, `registry.toml`,
  `registries/`, `search-cache/`, `progress-cache/`, `aiui/`.
- **Ten test files call `Command::cargo_bin` without `UserScratch`**:
  `vibe-cli`'s `cli_live_e2e.rs`, `tree_fixture.rs`, `tree_json.rs`,
  `vvm.rs`, and **all six of `vibe-index`'s** (`cli_lifecycle`, `cli_read`,
  `cli_write`, `from_github_e2e`, `help_smoke`, `scanner_e2e`).
- `UserScratch` lives in `crates/vibe-cli/tests/common/mod.rs`, so
  `vibe-index` cannot reach it at all. Isolation today is **opt-in, in one
  crate**.
- Three findings in a row (F-055, F-056, F-057) were the same forgotten
  discipline, each caught by accident rather than by a gate.

## 4. Required behavior {#behavior}

Two layers. Each catches what the other cannot, and the task is not done
with only one.

**Layer 1 — make the safe path the default.**

1. Move `UserScratch` into a shared workspace dev-dependency crate (e.g.
   `crates/vibe-test-support`), so every crate's tests can use it. Keep the
   API; `vibe-cli`'s existing six converted files must not change behaviour.
2. Add a **load-time initialiser** to that crate so the isolation happens
   before any `#[test]` runs: point `VIBE_SETTINGS`, `VIBE_REGISTRY_CACHE`
   and `VIBEVM_SEARCH_CACHE_DIR` at a per-process temp directory. The
   mechanism matters — `Command::cargo_bin` inherits the **test process's**
   environment, so once the parent is isolated, a bare `cargo_bin` child is
   isolated too and forgetting the helper stops being dangerous.
3. Route the ten bare files through it. Where a test deliberately needs
   real state (`cli_live_e2e.rs` is `#[ignore]`d and exists to reach the
   live internet with the operator's key), say so at the call site rather
   than isolating it blind.

**Layer 2 — the tripwire, which is the actual guarantee.**

4. Add a floor step that snapshots the real per-user settings home — every
   path plus a content hash — before and after `cargo test --workspace`,
   and **fails the floor** if anything moved. DRIFT-018 proved the
   technique by hand over 266 paths; this makes it a gate.
5. The snapshot must **never print a file's contents**, and must never
   include a token's bytes in any output — hash and path only. The four
   credential files are surface secrets, and a tripwire that leaks what it
   guards is worse than none.
6. On failure the message names the moved paths and says what to do. A
   tripwire nobody can act on gets disabled.

Edge cases: no `~/.vibe` at all (a fresh machine) ⇒ the tripwire passes
trivially, and says so. A path that legitimately moves during a run (none
known — say so if you find one) ⇒ surface it, do not add an exception list
without recording why.

Error paths: the tripwire failing to read the home is a warning and a pass,
never a false red — it must not become the thing that blocks work.

## 5. Boundaries {#boundaries}

- **Do not change production code.** Not the settings chokepoint, not the
  token precedence — DRIFT-021 owns the one production change in this area.
- Do not weaken any assertion to make a test pass under isolation. A test
  that fails once isolated was asserting against this machine, and that is
  §8.
- Never edit spec text.

## 6. Acceptance {#acceptance}

```bash
cargo test --workspace
bash tools/self-check.sh          # green, and now carrying the tripwire
```

- **The proof that layer 1 works:** add a deliberately un-isolated test that
  runs `vibe init` through a bare `cargo_bin`, confirm it writes into the
  per-process temp home rather than the real one, then delete it and report
  the observation in §9. A default nobody has tested is not a default.
- **The proof that layer 2 works:** temporarily make a test write one byte
  into the real settings home, confirm the floor goes red and names the
  path, then revert. A gate that has never been seen to fire is not known
  to work — the same control DRIFT-015 ran on conform.
- A grep for `cargo_bin` across `crates/*/tests/**` shows every site either
  routed through the helper or carrying a one-line reason.
- Discipline: `cargo fmt --all`, clippy clean, atomic commits, no AI
  attribution.

## 7. Analogies {#analogies}

`crates/vibe-cli/tests/common/mod.rs` `UserScratch` (DRIFT-012, extended by
DRIFT-018) is what you are promoting. `tools/self-check.sh`'s existing steps
are the shape of the tripwire.

## 8. Stop rule {#stop}

If the load-time initialiser cannot be made to run before every test in a
binary without a new runtime dependency you are uncomfortable adding: STOP,
say which mechanisms you tried and why each failed, and return with layer 2
alone. Layer 2 is the guarantee; layer 1 is the ergonomics, and shipping the
guarantee without the ergonomics is far better than the reverse.

Budget signal: past ~10 files, stop and return.

## 9. Log {#log}

- queued 2026-07-26 (Fable), on the owner's ruling. He did not ask for a
  convention; he asked for something he does not have to trust.
