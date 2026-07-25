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
