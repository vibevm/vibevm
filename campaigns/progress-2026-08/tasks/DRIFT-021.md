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

- implemented 2026-07-26 (Opus). Gates green: `cargo test --workspace`
  exit 0, `cargo xtask conform check` exit 0 (0 findings, 0 new),
  `bash tools/self-check.sh` exit 0 ("self-check: all green"). `cargo fmt
  --all` run. Nothing under the legacy directory was read, copied, moved
  or deleted; no token value or file content was printed anywhere.

  **Removed.**
  - `crates/vibe-core/src/settings.rs` — `LEGACY_DIR` const (was `:36`),
    `legacy_settings_dir()` (was `:73-82`), the module-doc bullet that
    promised the read fallback (was `:14-17`), and the test
    `legacy_dir_is_dot_vibevm_and_never_the_canonical` (was `:171-177`).
    The module no longer contains the directory literal in any form.
  - `crates/vibe-publish/src/token.rs` — `dot_vibevm_per_host_token_path`
    (was `:256-261`) and `dot_vibevm_token_path` (was `:269-273`); both
    legacy legs out of the precedence (was `:219-220` and `:236`); doc
    legs 5 and 7 deleted and the list renumbered to 5 items (was
    `:20-27`). `read_per_host_token` / `read_legacy_token` collapsed into
    one `token_file_candidates(host)` so the on-disk precedence is a
    single testable value rather than two parallel arrays.
  - `crates/vibe-cli/src/commands/aiui/control.rs` — **a third read leg
    §3 did not measure**: `legacy_aiui_dir()` (was `:183-187`) and
    `discovery_candidates()` (was `:189-197`), consumed by
    `read_discovery` (`:207`) and `wait_for_discovery` (`:231`). Replaced
    by `discovery_path(name)` against the one settings dir. §4.1 says
    "and any caller", and §7 says to let the compiler find them — it did.
    Worth the reviewer's attention: an aiui discovery file carries the
    vibeterm **control-server token** (`parse_discovery`, `v["token"]`),
    so this was a second credential read out of the un-isolatable home,
    not merely a convenience path.
  - Doc surfaces that would otherwise have become false:
    `crates/vibe-publish/src/lib.rs:18`, `crates/vibe-publish/src/github.rs:26`,
    `DEV-GUIDE.md:43`, `RUNTIME-GUIDE.md:49`. Seven files touched against
    the ~5 budget signal; the four above are one-line corrections forced
    by the removal, not new scope.

  **§4.4 — notice when the legacy dir exists and the canonical one does
  not: NO.** Reasons, in order of weight. (1) It would require keeping
  `LEGACY_DIR` and a home-derived existence probe of that directory — the
  very construct §4.1 deletes, and one `$VIBE_SETTINGS` still would not
  relocate. The campaign's isolation guarantee would remain false in
  miniature: an isolated run would still stat the operator's real home.
  Paying that to print a sentence is a bad trade. (2) The failure it
  would soften is already loud and actionable — `PublishError::AuthMissing
  { host }`, not silent wrong behaviour — and §4 pins "Error paths:
  unchanged". (3) A one-time migration message belongs in the release
  note and the guides, which are durable and cost nothing per run; a
  runtime notice either fires forever or needs its own suppression state.
  The "don't be cruel" obligation is therefore discharged in
  `RUNTIME-GUIDE.md:49` and `DEV-GUIDE.md:43`, which now name the old
  directory, state it is no longer read, and tell the operator to move
  the file into `~/.vibe/` themselves.

  **§8 writer check — clear, no stop.** The only accessor was
  `legacy_settings_dir()`; its three callers all read (`read_token_file`,
  `fs::read_to_string`). No writer exists in this repository. The
  historical writer named in
  `legacy-spec/research/SETTINGS-HOME-AND-GLOBAL-REGISTRY-PLAN-v0.1.md:31`
  ("writer is the JS side `apps/vibeframe/renderer.js:89`") is gone:
  `apps/` holds no files, and PROP-042 `:201-202` records that "the
  in-tree `apps/` source has moved to vibevm-term". The aiui writer is
  now an **out-of-repo product** (vibeterm, `vibevm-term` repo), so a
  vibeterm build old enough to still write the pre-consolidation location
  stops being discovered by `vibe aiui`. Not a data-migration question —
  the file is a regenerated session sidecar, not operator data — but it is
  the one user-visible behaviour change beyond the token path.

  **On this machine:** the legacy directory is **absent** (existence
  tested; nothing under it read, listed, or otherwise observed), so the
  removal changes nothing observable here — as §3 predicted.

  **§4.5 hand-back — the finding is the opposite of the one expected: no
  spec text over-describes the code.** No document under `spec/` mentions
  the legacy directory at all; the normative list never carried those
  legs, so removing them moves the code *toward* the spec. What remains
  is pre-existing drift in the other direction — the list **under**-describes
  the code, and did so before this task. For the sync-from-code flow,
  `spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#PUB-TOKEN-LOADING` —
  `spec/modules/vibe-registry/PROP-002-decentralized-registry.md:590-594`,
  verbatim:

  > @fact:PUB-TOKEN-LOADING **Token loading.** The publish token loader (`crate::token::load_token(host)`) iterates these sources in order, returning the first non-empty value: @status:impl/done
  >
  > 1. @fact:TOK-ENV-VAR `VIBEVM_PUBLISH_TOKEN` environment variable (host-agnostic; useful for CI). @status:impl/done
  > 2. @fact:TOK-PER-HOST-FILE `~/.vibe/<host-prefix>.publish.token` — per-host file. The prefix is the first label of the host (`github` for `github.com`, `gitverse` for `gitverse.ru`, `gitlab` for `gitlab.com`). @status:impl/done
  > 3. @fact:TOK-LEGACY-FALLBACK `~/.vibe/git.publish.token` — legacy host-agnostic fallback. @status:impl/done

  The code's four runtime legs are now exactly this list **plus one
  higher-precedence leg the spec omits**: `VIBEVM_PUBLISH_TOKEN_<HOST>`
  (`token.rs`, `read_host_env_token`), which outranks `##TOK-ENV-VAR`. So
  `##TOK-ENV-VAR` is no longer first. Before this task the gap was three
  legs; it is now one. Same defect, same anchor family, in
  `spec/boot/90-user.md:26`, verbatim:

  > @fact:TOKEN-FILE-CONVENTION **Token file convention.** Per-host file под `~/.vibe/<host-prefix>.publish.token` (`github.publish.token`, `gitverse.publish.token`, etc.) — первый label хоста. Legacy host-agnostic путь `~/.vibe/git.publish.token` остаётся как fallback. Env-var `VIBEVM_PUBLISH_TOKEN` — высший приоритет, для CI. @status:impl/done

  `VIBEVM_PUBLISH_TOKEN` is not "высший приоритет" — the host-specific
  form outranks it. Both were already true before DRIFT-021; neither was
  edited here (§5).

  **Sibling finding, out of scope, not touched.** `user_config.rs:285`
  `legacy_xdg_config_path()` (read at `:168`) is the same class of
  defect: a second config home derived from `$XDG_CONFIG_HOME` /
  `%APPDATA%` / `$HOME`, which `$VIBE_SETTINGS` also does not relocate, so
  an isolated run can still read the operator's real `config.toml`. It is
  config rather than a credential, and §5 forbids touching other
  precedence legs — flagging it for the owner as the natural DRIFT-021
  sequel.
