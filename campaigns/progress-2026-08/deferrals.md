# Deferrals — campaign `progress-2026-08`

_Written at close-out, 2026-07-26._ Open tails the campaign is **choosing** not
to close: obligations ruled deferred, work that cannot close in this
repository, and sign-offs only a person may give. The next campaign's mandate
drains this file.

A tail is listed here **only** if someone decided to leave it. Anything still
being worked is in `tasks/INDEX.md`, not here.

## 1. Cannot close in this repository {#blocked}

- **`FACT-GRAIN-EVIDENCE` — the ledger's single surviving drift row.** The
  spec promises fact-grain evidence joining; the shipped specmap engine is
  unit-grain. It closes when `rust-ai-native-lang` v0.8.0 re-vendors the
  fact-aware engine, which is **wave 2's Phase A2** — the first thing that
  campaign does. Nothing in this repository can close it sooner, and softening
  the spec to match would delete a promise the engine is about to keep.
  *Deferred to wave 2, by construction rather than by choice.*

- **`spec/boot/90-user.md` `##TOKEN-FILE-CONVENTION` — the owner's half of
  F-063.** The line states `VIBEVM_PUBLISH_TOKEN` is the highest-precedence
  token source; it is not — `VIBEVM_PUBLISH_TOKEN_<HOST>` outranks it. The
  PROP-002 half landed under sync-from-code this session. `90-user.md` is
  user-owned (`00-core` `NOTOUCH-90-USER`), so no session may edit it; the
  corrected line was handed to the owner in full. *Deferred to the owner, not
  to a campaign.*

## 2. Found, understood, deliberately not taken {#not-taken}

- **F-064 — the second config home.** `user_config.rs:285`
  `legacy_xdg_config_path()`, read at `:168`, resolves a config home from
  `$XDG_CONFIG_HOME` / `%APPDATA%` / `$HOME` that `$VIBE_SETTINGS` does not
  relocate — so an isolated run can still read the operator's real
  `config.toml` through it. Same shape as the credential leg DRIFT-021
  removed, one severity lower: config, not a token. Left because DRIFT-021's
  §5 forbade touching other precedence legs, and widening a task mid-flight is
  how a reviewed diff stops being reviewable. **Note for whoever takes it:**
  the invariant test DRIFT-021 added
  (`every_accessor_is_rooted_in_the_one_settings_dir`, `settings.rs`) does
  **not** cover `user_config.rs`, so it would not catch this. *Ready to
  execute; wants a task, not a decision.*

- **The three cache files from May in the real search-cache.** Surfaced by
  DRIFT-018's investigation; payloads are verbatim mock fixtures. Left in
  place because deleting a developer's files is not an executor's call, and
  the residue is inert. *Owner's to remove or keep.*

## 3. Awaiting a person, not a session {#human}

- **MT-02 (`vibe tree` TUI) and MT-03 (`vibe prefs ui`) are unsigned.** Both
  were re-authored in Phase D against the shipped F-key surface (the old steps
  walked a human through `n`/`x`/`t`/`Tab`/`q`, none of which exist). An agent
  may pre-run them; **only a person signs off**, so the run status did not
  move and `spec/WAL.md` names them as outstanding — which is itself what
  makes `MT-WAL-NAMES` true rather than merely worded.

## 4. Standing conditions carried forward {#standing}

- **specmap ratchet: 37 gated orphans, host-side, unmoved all campaign.** The
  gate holds them; nothing regressed. Not campaign work — it is the standing
  ratchet, and it shrinks as tags land.
- **vibespecs 401 on this machine.** redbook + rust-ai-native resolve through
  vibe-embedded; consuming lockfiles carry `source_kind = "embedded"`.
  Environmental, not a defect in the tree.
- **GitVerse SSH down 2026-07-25/26** — banner-exchange timeout at the network
  level, not divergence. GitHub carries everything. Recovery is a plain
  `cargo xtask mirror`; **never** `--force`.
- **Parked follow-ups predating this campaign**, unchanged and still parked:
  vibe-vvm / term-vvm conformance-golden; Linux/macOS install smoke;
  arbitrary user-repos design-doc; the `vibe doctor` project-local row.

## 5. Predictions left unscored {#unscored}

Recorded so the REPORT's honesty survives contact with the next campaign —
see §11 of the plan for the full scoring.

- **§8 prediction 6's "F+G ≈ 1 week overlapping E" clause** could not be
  scored at the time the REPORT was written, because close-out ran *before*
  Phases F and G at the owner's direction. The row is explicitly re-openable.
