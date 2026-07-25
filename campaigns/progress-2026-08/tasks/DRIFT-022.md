# DRIFT-022 — the `[env]` promotion stops being able to set anything {#root}

<status stage="impl" state="plan" ref="DRIFT-022"/>

**Status:** queued — **§4.1 is an owner decision; confirm before executing**
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
