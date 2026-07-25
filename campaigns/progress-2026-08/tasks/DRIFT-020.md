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

- implemented 2026-07-26. Both layers landed; both controls run and
  reverted. Details below.

### Layer 1 — what was built

`crates/vibe-test-support` is the new dev-dependency crate. It carries
`UserScratch` and `vibe()` verbatim from `crates/vibe-cli/tests/common/mod.rs`
(doc comment and all — it holds the F-055/F-056/F-057 history), plus
`cargo_bin(name)` for the non-`vibe` binaries, plus `isolated_home()`.
`common/mod.rs` now `pub use`s the two moved names, so the six already-converted
`vibe-cli` files are untouched.

The initialiser is a hand-rolled platform constructor in
`crates/vibe-test-support/src/isolate.rs` — a `#[used] static` of type
`extern "C" fn()` placed in `.CRT$XCU` (MSVC), `__DATA,__mod_init_func`
(Mach-O) or `.init_array` (ELF); the mechanism the `ctor` crate packages,
inlined rather than depended on. It points `VIBE_SETTINGS`,
`VIBE_REGISTRY_CACHE` and `VIBEVM_SEARCH_CACHE_DIR` at
`<temp>/vibevm-test-homes/p<pid>-<nanos>/…`.

**A lazy `isolate()` from the helpers was considered and rejected, and the
reason is the interesting one:** libtest runs test bodies on many threads, and
`std::env::set_var` while another thread reads the environment is precisely the
unsoundness that made those functions `unsafe` in edition 2024. A constructor
runs single-threaded before `main`, which is the one moment the mutation is
sound. So the crate deliberately has **no** runtime env-mutation entry point.
`rust-ai-native-env-audit::EnvGuard` cannot serve here either: it restores on
drop and holds a process-wide lock, both wrong for an isolation that must
outlive the call and never block a test. `vibe-test-support` is therefore
registered in `conform.toml` as the second `audit_crates` entry, with that
reasoning recorded inline.

**Linkage is the whole opt-in/opt-out.** The constructor fires in any test
binary that *references* the crate; `cli_live_e2e.rs` simply does not, and now
carries the reason at its `cargo_bin` site. There is no env escape hatch to get
wrong. Verified empirically that a bare `pub use` is enough:
`cli_workspace_publish.rs` names only `mod common`, never `UserScratch`, and
still minted a per-process home.

### The layer-1 positive control (§6), verbatim

A temporary `crates/vibe-cli/tests/drift020_control.rs` ran `vibe init` through
a bare `assert_cmd::Command::cargo_bin("vibe")` — no `UserScratch`, no `.env()`
— in a binary that links the support crate:

```
CONTROL isolated_home  = C:\Users\olegc\AppData\Local\Temp\vibevm-test-homes\p60240-1785018230646799400\settings
CONTROL real home      = C:\Users\olegc\.vibe
CONTROL VIBE_SETTINGS  = Ok("C:\\Users\\olegc\\AppData\\Local\\Temp\\vibevm-test-homes\\p60240-1785018230646799400\\settings")
CONTROL isolated home BEFORE init = []
CONTROL real home BEFORE init: 265 paths
CONTROL isolated home AFTER init  = ["config.toml", "registry.toml"]
CONTROL real home AFTER init: 265 paths
CONTROL real home NEW paths = []
```

The un-isolated child wrote **two** files, and both landed in the temp home.
A matching negative control in a binary that references nothing from the crate
(`drift020_control_neg.rs`) reported all three variables `Err(NotPresent)`,
so the isolation came from the constructor and not from an ambient value.
Both files deleted after the run.

A permanent regression guard replaced them: the unit test
`isolate::tests::the_constructor_ran_before_this_test_body`. The constructor is
the one silently-breakable part of layer 1 — a toolchain change that stopped
honouring `#[used]` in a section would drop it and nothing else would notice.

### Layer 2 — the tripwire

`tools/user-home-tripwire.sh` (`snapshot` / `compare`) resolves the settings
home exactly as `vibe_core::settings::settings_dir()` does, then records one
line per path: `dir <rel>`, `link <rel>`, `file <rel> <sha256>`. Directories are
included so a minted-but-empty registry-cache bucket still counts as movement.
It emits hash and path only, never contents. `tools/self-check.sh` snapshots
once at step 0 and compares twice — step 2b (right after `cargo test
--workspace`, so a failure points at the workspace suite specifically) and step
12 (after the four package suites in steps 7-10). Unresolvable or unreadable
home ⇒ warning and pass; absent home ⇒ "trivially green", both by construction.

### The layer-2 positive control (§6), verbatim

A temporary `drift020_tripwire_control.rs` — deliberately in a binary linking
nothing from `vibe-test-support`, so it resolved the real home the way
production does — wrote one byte to `~/.vibe/drift020-tripwire-control`.
`bash tools/self-check.sh` then exited **1** at the new step:

```
=== user-home tripwire (after cargo test --workspace) ===
user-home tripwire: FIRED — the real per-user settings home changed during this run.
  home: /c/Users/olegc/.vibe
  paths that moved (+ appeared, - vanished, ~ contents changed):
    + file drift020-tripwire-control
  What this means: something in this run read-modified-wrote the operator's
  real settings home instead of an isolated one. That home carries publish
  tokens and API keys; a test must never touch it.
  What to do:
    1. Find the test. `crates/vibe-test-support` isolates a test process at
       load time — a test binary that links it cannot reach the real home.
       A binary that does NOT link it is the likely culprit.
    2. Route it through `vibe_test_support::UserScratch` (or add
       `use vibe_test_support as _;` so the load-time isolator links in).
    3. Restore whatever moved by hand if it mattered, then re-run.
  This gate is not advisory. Do not add an exception list without recording
  why the path legitimately moves (§4 of DRIFT-020: none are known).
self-check: `user-home tripwire (after cargo test --workspace)` failed (exit 1)
```

Reverted: the test file and the byte it wrote are both gone, and the real home
compares byte-identical against the snapshot taken before any of this work
began.

### Findings

- **No path legitimately moves during a run.** Measured before any change: a
  full `cargo test --workspace` left the real `~/.vibe` unchanged (compare
  exit 0). So the tripwire ships with **no exception list**, which §4 asked to
  confirm rather than assume. `~/.vibe/aiui/` was the one candidate — it is
  vibeterm's live control-session discovery dir — and it did not move.
- **§3's measurements all hold**, with one number to correct: the home records
  **265** paths here, not the 266 §4.4 cites from DRIFT-018 (that figure was a
  hand count on a different day; nothing is wrong with either). The four
  credential files and the ten bare `cargo_bin` files were exactly as stated.
- **`vibe init` writes `registry.toml` too, not just `config.toml`.** §3 and the
  `UserScratch` doc frame F-056 around `config.toml`/`last_author`; the control
  shows a bare `vibe init` also seeds a default `registry.toml` (the write
  `VIBE_NO_DEFAULT_REGISTRY=1` suppresses). On a machine with no `registry.toml`
  yet, a forgotten-isolation `vibe init` would have created the operator's
  global registry list as a side effect of a test. That is a second F-055-shaped
  hazard from the same call.
- **Nine other crates have `tests/` dirs** (`progress-core`, `vibe-check`,
  `vibe-install`, `vibe-mcp`, `vibe-publish`, `vibe-registry`, `vibe-resolver`,
  `vibe-settings`, `vibe-spec`). None spawns a binary and none names a
  settings-home path today, so none was given the dev-dependency — deliberately
  out of scope per §8, and layer 2 covers them regardless. If one ever calls
  `vibe_core::settings::settings_dir()` and writes, the tripwire catches it and
  the fix is one dev-dependency line.
- **The per-process homes are not dropped**, because a constructor has no drop
  point. `prune_stale` collects siblings older than six hours on the way in
  instead. Worth knowing before someone finds ~19 directories per full test run
  under `<temp>/vibevm-test-homes/` and files a bug.
