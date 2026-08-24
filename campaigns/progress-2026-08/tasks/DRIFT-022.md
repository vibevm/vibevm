# DRIFT-022 — the `[env]` promotion stops being able to set anything {#root}

<status stage="impl" state="plan" ref="DRIFT-022"/>

**Status:** ready — **the owner picked (a); §4.1 is settled, execute it**
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (process bootstrap)

## 1. Goal {#goal}

A `[env]` table in a settings file can no longer set an arbitrary
environment variable in the vibevm process, so inheriting one cannot reach
a database, a cloud credential, or a kubeconfig.

## 2. Contract {#contract}

**Owner's ruling, 2026-07-25, verbatim:** «Вероятно, так делать для тестов
не нужно. Иначе мы когда-нибудь удалим живую базу на продакшене или еще
что-то такое, в рамках теста.»

Finding realised: the second half of **F-061**.

## 3. Current state {#current}

- `crates/vibe-cli/src/main.rs:51` — `promote_user_config_env()` runs before
  dispatch for **every** subcommand and promotes the settings file's `[env]`
  table into the process environment.
- So any invocation that has not isolated `$VIBE_SETTINGS` reads the real
  config and adopts whatever it declares. Ten test files were in that state
  when this was found; DRIFT-020 fixes the test half.
- Inert on this machine only because the table happens to be empty. That is
  not a property anyone chose.

## 4. Required behavior {#behavior}

1. **The decision, which is the owner's:**
   - **(a) Allowlist — the reviewer's recommendation.** The promotion
     accepts only names matching `VIBE_*` / `VIBEVM_*`; anything else is
     ignored with one warning naming the rejected key (never its value —
     it may be a secret). The feature keeps working for what it was built
     for, and stops being able to reach anything outside vibevm.
   - **(b) Remove the promotion entirely.** Simpler, and it deletes a
     documented capability someone may be relying on.
   (a) is recommended because it is the reversible half: widening an
   allowlist later is one line, resurrecting a deleted feature is an
   argument. **Do not start until the owner has picked.**
2. Whichever is chosen, a rejected or removed key is **reported once, by
   name only**. A silently dropped variable is a debugging nightmare, and a
   printed value is a secrets-hygiene violation.
3. Document the rule where the feature is documented, in code doc — not in
   spec text.

Edge cases: an empty or absent `[env]` ⇒ unchanged, silent. A key that
matches the allowlist but is already set in the real environment ⇒ keep the
existing precedence, whatever it is today; this task does not change who
wins.

Error paths: a malformed `[env]` behaves exactly as it does now.

## 5. Boundaries {#boundaries}

- Do not change the settings chokepoint, the precedence, or any other
  bootstrap step.
- Never print an environment value — names only, always.
- Never edit spec text. If the promotion is described in a spec, record the
  text in §9 for the reviewer's sync-from-code pass.

## 6. Acceptance {#acceptance}

```bash
cargo test --workspace
bash tools/self-check.sh
```

- New test: a settings file declaring `DATABASE_URL` and `VIBE_THING` —
  under (a) the process sees `VIBE_THING` and **not** `DATABASE_URL`; under
  (b) neither.
- New test: the rejection is reported by name and the value appears
  nowhere in stdout or stderr. Assert on the absence of the value.
- Discipline: `cargo fmt --all`, clippy clean, atomic commits, no AI
  attribution.

## 7. Analogies {#analogies}

`crates/vibe-publish/src/token.rs`'s precedence documentation is the
register for describing "what wins and why" in code doc.

## 8. Stop rule {#stop}

**Do not execute §4 until the owner has chosen (a) or (b).** Beyond that:
if the promotion turns out to have a consumer that depends on a non-`VIBE_*`
name, STOP, name it in §9, return — that is a fact about how the feature is
actually used and it changes the answer.

Budget signal: past ~3 files, stop and return.

## 9. Log {#log}

- queued 2026-07-26 (Fable). The owner named the failure mode exactly: a
  test that deletes a live database. This task makes that unreachable
  rather than unlikely.

- executed 2026-07-26, option **(a)**, per the owner's pick. Four files:
  `crates/vibe-cli/src/main.rs` (the rule + its doc + unit tests),
  `crates/vibe-core/src/user_config.rs` (the parse-side doc, which
  promised more than the promotion now delivers),
  `crates/vibe-cli/tests/cli_registry_mgmt.rs` (the end-to-end test,
  filed next to the four `show config` layering tests already there),
  and `docs/commands/show.md` (see "drift I created", below).

- **Behaviour.** `PROMOTABLE_ENV_PREFIXES = ["VIBE_", "VIBEVM_"]`,
  case-sensitive, prefix match. `partition_env_promotions` splits the
  table; the admitted half goes through the unchanged live-env guard,
  the refused half is reported once and never written. Verbatim, for a
  table declaring `DATABASE_URL` and `VIBE_THING`:

  ```
  vibe: warning: user config `[env]` may only set VIBE_* / VIBEVM_* names; ignored: DATABASE_URL
  ```

  Both prefixes are needed: `VIBEVM_HOME` does not start with `VIBE_`.
  The allowlist is checked **before** the live-env guard, so the verdict
  is a property of the name and the diagnostic does not come and go with
  the operator's ambient environment. Precedence itself is untouched:
  live env still wins over an admitted `[env]` value.

- **§8 — checked, and there are none.** Every `[env]` table anywhere in
  the tree declares only `VIBE_*` / `VIBEVM_*` keys: the two unit
  fixtures in `crates/vibe-core/src/user_config.rs`, four test fixtures
  in `crates/vibe-cli/tests/cli_registry_mgmt.rs`, and the doc example
  in `docs/commands/show.md`. No `config.toml` with an `[env]` table
  exists on disk outside `refs/`; there is no `cfg.env.insert(…)` and no
  `UserConfig { env: … }` literal in the workspace. So nothing depends on
  a non-`VIBE_*` promoted name, and the stop rule did not fire.

  Two near-misses, recorded because they are the ones a future widening
  would be about:

  - `VIBETERM` / `VIBEFRAME` (`crates/vibe-cli/src/commands/tree/host.rs:20`)
    are vibevm's own variables and fall **outside** the allowlist — no
    underscore after `VIBE`. Correctly so: a vibe desktop terminal sets
    them in the PTY it spawns, so a config file claiming one would be
    lying to `vibe tree` about where it is running. Not a consumer of
    the promotion.
  - A manifest may override `token_env` on a `[[registry]]`
    (`crates/vibe-registry/src/git_package_registry/mod.rs:300`) or a
    `[redirect]` (`crates/vibe-publish/src/redirect_sync.rs:269`) with
    an arbitrary name. The defaults —
    `VIBEVM_REGISTRY_TOKEN_<HOST>` / `VIBEVM_TARGET_TOKEN_<HOST>` — are
    admitted; an arbitrary override supplied through `[env]` would now
    be refused. No such configuration exists in this repository, and a
    token's home is `~/.vibe/<host>.publish.token` (PROP-000 §20) rather
    than a config file regardless. Named in the code doc as the one edge
    to widen for, if it ever appears.

- **§5 — spec text for the reviewer's sync-from-code pass. Not edited.**
  No spec file defines the `[env]` table or its promotion. Four places
  touch it:

  `VIBEVM-SPEC.md:1072-1080` — the whole of §9.5, which never names an
  `[env]` table and never mentions promotion. Its line 1075 already
  scopes the env layer the way this change does:

  > ```
  > ### 9.5 Configuration sources, in precedence order {#configuration-sources-in-precedence-order}
  >
  > 1. Command-line flags (highest precedence).
  > 2. Environment variables (`VIBE_*` prefix).
  > 3. Project `vibe.toml`.
  > 4. User-level config at `~/.config/vibe/config.toml`.
  > 5. Built-in defaults (lowest precedence).
  >
  > `vibe show config` prints the effective configuration with provenance for each value.
  > ```

  (Line 1077's `~/.config/vibe/config.toml` is stale against the
  settings-dir consolidation — the canonical path is `~/.vibe/config.toml`.
  Pre-existing, not this task's; noted because the reviewer is in the file.)

  `spec/modules/vibe-registry/PROP-010-local-package-cache.xml:68` —

  > ```
  > @fact:USER-LEVEL-REGISTRIES **Decision.** A **user-level default registry configuration** — `[[registry]]` (and `[[mirror]]`) entries in the existing user config (`~/.config/vibe/config.toml`, the `UserConfig` layer that already promotes `[env]` per `VIBEVM-SPEC.md` §9.5). It supplies registry configuration when no project does, and seeds a new one: @status:spec/done
  > ```

  `spec/modules/vibe-registry/PROP-010-local-package-cache.xml:74` —

  > ```
  > - @fact:PROJECT-OVERRIDES Project-level `[[registry]]` always overrides the user-level default — the same precedence the `UserConfig` `[env]` layer already follows (the project / live value wins). @status:spec/done
  > ```

  Both are incidental cross-references; neither states which names are
  permitted, so the allowlist falsifies neither. Nothing under
  `spec/modules/vibe-workspace/` describes the table at all (PROP-011,
  which `user_config.rs` cites, covers only `[install] slot_integrity`).

  `spec/WAL.xml:22-24` and `:111-114` describe this task as parked on the
  owner's letter; both go stale once this lands.

- **Drift I created and then fixed, outside spec.**
  `docs/commands/show.md:163` read "**Runtime injection.** Every value
  listed in `[env]` is promoted into the process environment…" — false
  the moment the allowlist landed. Edited: the sentence now says
  "Values listed in `[env]` are promoted", followed by the allowlist
  rule, the verbatim warning, and the `VIBETERM` prefix note. `docs/` is
  command reference, not spec, so §5's boundary does not cover it and
  leaving a freshly-false sentence would have been worse. Flagged here
  because it is the fourth file and past §8's budget signal.

- **Evidence.** Against a fixture config declaring `DATABASE_URL`,
  `VIBE_THING` and `VIBE_LOG`, `vibe --json show config` exits 0, prints
  the one warning above on stderr, and reports `VIBE_LOG` with
  `provenance = user-config` / `value = vibe_registry=info` — which is
  set only for names the startup promotion actually wrote, read back out
  of the live process environment, so an admitted name demonstrably
  still lands. The refused value occurs 0 times across stdout and
  stderr.

- **Not absorbed.** The stale `~/.config/vibe/config.toml` path in
  `VIBEVM-SPEC.md:1077` and in `docs/commands/show.md:139,146,154-159`
  (canonical is `~/.vibe/config.toml`); the two `spec/WAL.xml` passages
  that call this task parked. Neither is this task's contract.
