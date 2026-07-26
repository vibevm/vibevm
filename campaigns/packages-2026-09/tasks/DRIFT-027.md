# DRIFT-027 — one config home, the way there is one credential home {#root}

<status stage="impl" state="plan" ref="DRIFT-027"/>

**Status:** ready — owner said do it now, 2026-07-26
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** vibe-core (the settings chokepoint)
**Unit-stability check:** the spec side (`VIBEVM-SPEC.md` §9.5, F-066) is the
reviewer's, landed after this. No anchor moves here.

## 1. Goal {#goal}

`$VIBE_SETTINGS` relocates the user config the way it already relocates every
credential — completely — so an isolated run cannot reach the operator's real
`config.toml` by any path.

## 2. Contract {#contract}

> Both file legs hang off the one settings dir, so `$VIBE_SETTINGS` redirects
> every on-disk credential read together.
> — `crates/vibe-publish/src/token.rs`, after DRIFT-021

Finding realised: **F-064**. Its sibling **F-066** (the spec still names the
old path) is the reviewer's, and lands right after this.

## 3. Current state {#current}

Measured 2026-07-26 — contradict me if a number is wrong:

- `UserConfig::default_path()` (`crates/vibe-core/src/user_config.rs:163-183`)
  reads, in order: `$VIBEVM_USER_CONFIG`, then the canonical
  `<settings-dir>/config.toml`, then — **only if the canonical is absent** —
  `legacy_xdg_config_path()`.
- That legacy leg (`:295`) resolves `$XDG_CONFIG_HOME/vibe/config.toml`, else
  `%APPDATA%\vibe\config.toml` on Windows, else `$HOME/.config/vibe/config.toml`.
  **`$VIBE_SETTINGS` does not relocate any of them.**
- So a run that isolated `$VIBE_SETTINGS` — every test, after DRIFT-020 — still
  reads the operator's real config through this leg whenever the isolated
  settings dir has no `config.toml`, which is the normal case for a fresh temp
  home.
- **It is exactly the shape DRIFT-021 removed** for credentials, one severity
  lower: config rather than a token. That task's reasoning applies verbatim.
- The invariant test DRIFT-021 added
  (`every_accessor_is_rooted_in_the_one_settings_dir`, `settings.rs`) **does
  not cover `user_config.rs`**, which is why this survived it.
- Only two sites mention the leg: its definition and its one caller.

## 4. Required behavior {#behavior}

1. **Delete the leg.** `legacy_xdg_config_path()` goes, and `default_path()`
   resolves `$VIBEVM_USER_CONFIG` → `<settings-dir>/config.toml` and nothing
   else. One home, the way there is one credential home.
2. **Do not read, move, or delete the operator's legacy file.** DRIFT-021 set
   this precedent for tokens and it holds here: a file still sitting in the old
   location is the operator's to move, and vibevm neither reads it nor touches
   it. Say so in the doc-comment.
3. **Say it once, rather than switching silently.** If the canonical config is
   absent *and* a file exists at the old location, emit **one** warning naming
   the old path and the canonical one. A user whose config quietly stopped
   being read is the failure this rule exists to prevent — and it is the one
   thing this task adds rather than removes, so keep it to a single line and
   never print the file's contents.
4. **Extend the invariant test to cover `user_config.rs`**, so the next leg of
   this shape is caught by a gate rather than by a campaign. That is the
   durable half of this task; the deletion is the cheap half.

Edge cases: no settings dir resolvable ⇒ `None`, unchanged. `$VIBEVM_USER_CONFIG`
set ⇒ wins verbatim, unchanged — it is an explicit override, not a home.

Error paths: unchanged. A malformed config still reports and continues.

## 5. Boundaries {#boundaries}

- **Never edit `spec/**` or `VIBEVM-SPEC.md`.** The spec still names
  `~/.config/vibe/config.toml` as the user-level config; that is F-066 and the
  reviewer lands it. Quote the exact line in §9 for that pass.
- Do not touch the token precedence, the settings chokepoint's own resolution,
  or `$VIBEVM_USER_CONFIG`.
- Do not add a migration that copies the file. Reading it was the defect;
  copying it silently would be a bigger one.

## 6. Acceptance {#acceptance}

```bash
cargo test --workspace
bash tools/self-check.sh
```

- `grep -rn "legacy_xdg_config_path\|XDG_CONFIG_HOME" crates/ --include=*.rs`
  returns **nothing** outside a test that asserts the leg is gone. Report what
  it returns.
- New test: with `$VIBE_SETTINGS` pointed at an empty temp dir and a config
  planted at the legacy location, `default_path()` returns the canonical path
  (or `None`) and **never** the legacy one. This is the test that would have
  failed before the change — run it against the old code first and report that
  it fails, so we know it tests something.
- New test: the one-line warning fires exactly once and names both paths, and
  the file's contents appear nowhere in the output.
- The extended invariant test **fails** if `user_config.rs` grows a path that
  is not rooted in the settings dir. Prove it fires by adding such a path
  temporarily, then revert — a gate never seen to go red is not known to work.
- Discipline: `cargo fmt --all`, clippy clean, no AI attribution.

## 7. Analogies {#analogies}

DRIFT-021 is the precedent in full: it removed the pre-consolidation credential
legs, left the operator's files alone, and recorded why in
`crates/vibe-publish/src/token.rs`'s doc-comment. Read that comment before
writing this one — the register and the reasoning both transfer.

## 8. Stop rule {#stop}

If removing the leg breaks a test that asserts the legacy path is read, that
test encodes the defect: **STOP, name it, and return.** Do not edit it to pass —
whether that behaviour was ever intended is a question for the reviewer.

Budget signal: past ~3 files, stop and return.

## 9. Log {#log}

- queued 2026-07-26 (Fable). Surfaced by DRIFT-021 as the sibling it was
  forbidden to absorb, and left with a note that the invariant test added in
  the same breath does not reach the module this lives in — which is why §4.4
  matters more than §4.1.
