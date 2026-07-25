# DRIFT-021 — the legacy `~/.vibevm` read leg goes away {#root}

<status stage="impl" state="plan" ref="DRIFT-021"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** common (settings home + token precedence)
**Unit-stability check:** this one **does** move a spec surface — the
precedence list is normative. See §4.5.

## 1. Goal {#goal}

vibevm stops reading `~/.vibeVM/` — the pre-consolidation settings
directory — so there is exactly one per-user home, and exactly one place a
credential can be read from.

## 2. Contract {#contract}

> The pre-consolidation `<home>/.vibevm` survives only as a **read-only**
> migration fallback.
> — `crates/vibe-core/src/settings.rs:14`

**Owner's ruling, 2026-07-26, verbatim:** «удали чтение из этой директории.
Это легаси, оно нам не нужно.»

The reason it matters beyond tidiness: `$VIBE_SETTINGS` relocates the
canonical home but **deliberately does not relocate the legacy one**
(`settings.rs:78-79`). So every isolation mechanism this campaign built —
`UserScratch`, and DRIFT-020's load-time default — is blind to this leg by
construction. It is the one path by which an isolated test can still reach a
real credential.

## 3. Current state {#current}

Measured 2026-07-26 — do not re-discover:

- `crates/vibe-core/src/settings.rs` — `LEGACY_DIR = ".vibevm"` (`:36`), the
  legacy path accessor (`:73`), and the note that `$VIBE_SETTINGS` does not
  move it (`:79`). A test pins the constant (`:172`).
- `crates/vibe-publish/src/token.rs` — the host-aware precedence has **two**
  legacy legs: `~/.vibevm/<host-prefix>.publish.token` (documented at `:20`,
  read at `:219-220` via `dot_vibevm_per_host_token_path`) and
  `~/.vibevm/git.publish.token` (`:26`, read at `:236` via
  `dot_vibevm_token_path`).
- On this machine `~/.vibevm/` **does not exist**, so removing the legs
  changes nothing observable here. That is exactly why it should go now
  rather than after it matters.

## 4. Required behavior {#behavior}

1. Delete the legacy directory from the settings module: the constant, the
   accessor, and any caller. `settings_dir()` resolves `$VIBE_SETTINGS` then
   the canonical `~/.vibe` and nothing else.
2. Delete both legacy legs from the token precedence, and renumber the
   documented list so it describes what the code does.
3. **Migration is not this task's job, and pretending otherwise is worse
   than skipping it.** If `~/.vibevm/` exists on a machine, the operator
   moves their own files. Do not copy, do not merge, do not touch anything
   under it — a tool that silently relocates a credential file is a worse
   bug than the one being fixed.
4. Consider a one-line notice when `~/.vibevm/` exists and the canonical
   home does not: tell the operator where their files should go, and read
   nothing. Decide yes or no and say why in §9 — a notice that fires on a
   fresh machine forever is noise, and silence when someone's tokens have
   just stopped being found is cruel.
5. **The precedence list is normative and lives in the spec.** Removing legs
   changes it. Do **not** edit spec text — record in §9 exactly which spec
   text now over-describes the code, quoting it, so the reviewer can run the
   sync-from-code flow with the owner. That is the one thing this task must
   hand back rather than do.

Edge cases: a `~/.vibevm` that exists and holds a token ⇒ it stops being
read, which is the point; say so in §9 with the path so the reviewer can
tell the owner. Tests that pin `LEGACY_DIR` are deleted with it.

Error paths: unchanged.

## 5. Boundaries {#boundaries}

- **Never read, copy, move, or delete anything under `~/.vibevm/`.** Not to
  migrate, not to check its contents, not to report what is in it. Its
  existence is the only fact this task may observe.
- Never print a token value or a token file's contents (secrets-hygiene).
- Do not change the canonical home, `$VIBE_SETTINGS`, or the other
  precedence legs.
- Never edit spec text — §4.5.

## 6. Acceptance {#acceptance}

```bash
cargo test --workspace
cargo xtask conform check
bash tools/self-check.sh
```

- `grep -rn "vibevm\"" crates/*/src/` shows no `.vibevm` directory literal
  outside a comment describing its removal.
- A test asserting the precedence no longer contains a legacy leg.
- The spec text §4.5 identifies, quoted verbatim in §9, with its file and
  anchor.
- Discipline: `cargo fmt --all`, clippy clean, atomic commits, no AI
  attribution.

## 7. Analogies {#analogies}

`crates/vibe-core/src/settings.rs` is deliberately the single authority for
these paths — the module doc says so at `:7`. Removing a path means removing
it there and letting the compiler find the callers.

## 8. Stop rule {#stop}

If removing a leg breaks a test that turns out to be asserting real
behaviour someone depends on: STOP, name it in §9, return. And if you find
any *writer* to `~/.vibevm/` anywhere — the current state says there is
none — stop immediately: that would make this a data-migration question,
not a read-path removal, and it is the owner's call.

Budget signal: past ~5 files, stop and return.

## 9. Log {#log}

- queued 2026-07-26 (Fable), on the owner's explicit ruling. It is the one
  credential path no isolation mechanism in this campaign can reach.
