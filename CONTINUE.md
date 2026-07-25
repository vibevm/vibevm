# CONTINUE — cold-resume checkpoint

_Written 2026-07-26 (session end: **Phases D and E driven end to end** —
the drift ledger emptied from 311 rows to 1, twelve DRIFT tasks executed,
the floor green with no workaround for the first time). `spec/WAL.md` is
the canonical living state and supersedes this snapshot wherever they
diverge._

## TL;DR

Progress-Control campaign (PROP-043, plan
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`): **Phase D
(stitching) ran to completion and Phase E (coding) drained its queue.**
The spec tree measured **93.0 % true** at the Phase C gate this morning;
it now measures **4 486 confirmed / 1 drift / 3 unverifiable of
4 490 — 99.9 %**. Findings **61 of 64 resolved**. The one remaining drift
row (`FACT-GRAIN-EVIDENCE`) cannot close in this repository at all — it
waits on a package release, and that is wave 2's first phase.

**The floor is green plainly** — `bash tools/self-check.sh` exits 0
against the developer's real `~/.vibe/`, with no `VIBE_SETTINGS`
override. That was not true this morning and is the single most useful
fact in this file.

Phase lane reads **E**. Gate panel in `campaign.json`: floor / check /
specmap / conform all green.

## What the next session should decide first

Three things are parked on the owner and nothing else is blocked:

1. **DRIFT-022 needs a letter.** The `[env]` promotion in
   `crates/vibe-cli/src/main.rs:51` can set *any* environment variable
   from a settings file. Owner's words for the risk: «Иначе мы
   когда-нибудь удалим живую базу на продакшене … в рамках теста.»
   Choose **(a)** allowlist `VIBE_*` / `VIBEVM_*` (recommended — widening
   later is one line, resurrecting a deleted feature is an argument) or
   **(b)** remove the promotion entirely. The task is written and refuses
   to start without the answer.
2. **DRIFT-020 is ready to run** and needs no decision — say go.
3. **F-063 needs a sync-from-code diff** the owner approves, and one edit
   only he may make (`spec/boot/90-user.md` is user-owned).

## Where the campaign actually stands

| Phase | State |
|---|---|
| A, B, L, C | closed earlier |
| **D — stitching** | **complete.** 311 drift rows → 1. Waves d1, d2a–d2h. |
| **E — coding** | **queue drained.** DRIFT-006…021 executed; 015 superseded before it ran. |
| F — plans from views | **never opened.** Three owner plans: release/productization, improvement, global idea ledger. |
| G — documentation | **never opened.** User Guide + Package Author Guide, written from proven behaviour. |
| close-out | **not started.** `baseline.json`, deferrals, and the REPORT against §8's six predictions — four are already checkable. |

**Wave 2 is planned and unratified:**
`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` — 37 packages,
294 files, 28 733 lines across `org.vibevm.world` and
`org.vibevm.ai-native`. Owner's reason, verbatim: «без проверки самой
ai-native всё это выглядит как профанация.» Its Phase A2 is the re-mint
that unblocks wave 1's last drift row.

## The three open findings

- **F-061** — `cli_workspace_publish.rs` still runs `vibe init` against
  the real settings home (four tests), and `main.rs:51` promotes the real
  config's `[env]` for every subcommand. Closed by DRIFT-020 + DRIFT-022.
- **F-063** — *security-adjacent spec drift.* `PROP-002`
  `##PUB-TOKEN-LOADING` and `spec/boot/90-user.md`
  `##TOKEN-FILE-CONVENTION` both say `VIBEVM_PUBLISH_TOKEN` is the
  highest-precedence source. It is not: `VIBEVM_PUBLISH_TOKEN_<HOST>`
  sits above it (`crates/vibe-publish/src/token.rs:7-11`) and neither
  document mentions it. `90-user.md` is user-owned — the owner's edit.
- **F-064** — `crates/vibe-core/src/user_config.rs:285`
  `legacy_xdg_config_path()`, read at `:168`: a second config home from
  `$XDG_CONFIG_HOME` / `%APPDATA%` / `$HOME` that `$VIBE_SETTINGS` does
  not relocate. Same shape as the leg DRIFT-021 removed, one severity
  lower (config, not a credential).

## Queued tasks, ready to dispatch

`campaigns/progress-2026-08/tasks/INDEX.md` is the register.

- **DRIFT-020** — test isolation stops being a convention. Two layers:
  a load-time default so a forgotten helper is harmless (a spawned binary
  inherits the test process's environment), and a floor tripwire that
  fails if anything in the real `~/.vibe` moved. Both carry a control
  that proves the mechanism can fire.
- **DRIFT-022** — blocked on the letter above.

## Non-obvious findings from this session

- **Substring matching lied three times in one day**, including to me.
  `"parsed"` appeared to be a cache key and was not; `"verdicts"`
  appeared to be in the sidecar and was a marker's comment text;
  `updated_at` appears inside a verdict's own prose in `corpus.json`.
  Structural walks and byte-anchored needles — never `grep` — for any
  claim about a data file.
- **Two measurements corrected each other.** DRIFT-010's ×1.44–2.06
  speed-up was a *debug* profile; in release the parse is 10.3 ms against
  7.5 ms of payload serde. DRIFT-017 then corrected that in turn: the
  writes were ~14 % of a warm run, not the bulk, because comparing
  requires serialising — only write+fsync are saved. **The next
  performance lever is the serialisation, not the IO.**
- **The token "leak" was investigated and closed: no rotation needed.**
  `search.rs:426` loads the token only after the registry URL is
  confirmed a GitHub org, then sends it to `api_base`. Exactly one test
  reaches that path and points `api_base` at a 127.0.0.1 mock. The 47-byte
  `Authorization` header measured went to loopback.
- **`~/.vibe/` holds four credential files** (`github.publish.token`,
  `git.publish.token`, `zai.api.token`, `zai.api.token.2`). Ten test
  files still call `cargo_bin` with no isolation — all six of
  `vibe-index`'s among them, which cannot reach the helper because it
  lives in `vibe-cli`'s test dir.
- **Commit delegated work on the completion notification, never on
  journal evidence.** Executors write their §9 log as they go; committing
  on it captured an intermediate state and left the tree conform-red for
  twenty minutes.
- **A gate never seen to go red is not known to work.** Two executors ran
  positive controls (an oversized throwaway file; a one-byte write into
  the guarded home) before trusting a green result.
- The campaign cache is now split: verdicts in git
  (`run/cache.json`, 2.68 MB), the parse payload outside the repository at
  `~/.vibe/progress-cache/<repo-id>/<branch-slug>/<campaign>/`. Deleting
  the sidecar is silent and harmless — verified by doing it.
- Two consecutive `progress scan`s now leave `git status` byte-identical
  ("0 written, 7 unchanged and skipped").

## State

- Branch `main`; **GitHub synced through `97c26bf6`**; working tree clean
  (untracked `.zcode/` only).
- **GitVerse SSH down all day** — banner-exchange timeout, network-level,
  not divergence. Recovery: plain `cargo xtask mirror`; **never**
  `--force`.
- `progress check` = 0 · `conform check` = 0 findings · specmap ratchet 37
  gated orphans, unmoved · `self-check` exit 0 with no override.
- Findings 64 (next free **F-065**), in
  `campaigns/progress-2026-08/run/state/findings.json`.

## Standing decisions in force

No fractality for this campaign (owner decision — Fable does markup,
verification, stitching, task authoring and **all review**; Opus executes
DRIFT tasks; engine pin `claude-opus-5`). `reality-mismatch` closes only
through sync-from-code with owner approval. Verdicts live in the cache's
campaign maps, never in markup; mutate by load-and-merge only. Campaign
zone excluded from scan. `legacy-spec/` is an archive — never cited
normatively. The four CLAUDE.md rules bind every executor, worker output
is never credited. Amend only unpushed commits.

## Quick start

```bash
bash tools/self-check.sh                                  # exits 0, no override needed
cargo run -q -p vibe-cli --bin vibe -- progress check      # must stay 0
cargo run -q -p vibe-cli --bin vibe -- progress scan       # "0 written, N unchanged and skipped"
cargo xtask conform check                                  # 0 findings
GIT_SSH_COMMAND="ssh -o ConnectTimeout=15 -o BatchMode=yes" cargo xtask mirror
```

Ledger read-out:

```bash
python -c "import json,io,collections; c=json.load(io.open('campaigns/progress-2026-08/run/cache.json',encoding='utf-8')); t=collections.Counter(v['v'] for r in c['files'].values() for v in ((r.get('campaign') or {}).get('verdicts') or {}).values()); print(dict(t))"
```

## Repository map (top level)

- `spec/` — the living corpus (58 files, all verified) + `WAL.md` + both
  campaign plans in `spec/terraforms/`.
- `legacy-spec/` — the archive; zero normative inbound refs.
- `crates/` — the workspace (17 crates).
- `campaigns/progress-2026-08/` — journal, state (findings 64), cache with
  the FULL verdict maps (**load-bearing**), `tasks/` (DRIFT-001…022).
- `packages/` — `org.vibevm.world` (27) + `org.vibevm.ai-native` (10) —
  wave 2's corpus.
- `tools/self-check.sh` — the floor.

A pointer worth repeating: **the WAL is the canonical living state.** If
this file and `spec/WAL.md` disagree, the WAL wins.
