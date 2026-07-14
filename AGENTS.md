# vibevm — read this first

Every session in this repository begins by reading this file, then every file in `spec/boot/` in filename order, then `spec/WAL.md`, then any relevant PROP/FEAT documents under `spec/common/` and `spec/modules/` for the task at hand. Only after that, start work.

The repository's commit-and-push discipline — human-authored **attribution** (never mark any part of this repository as AI-authored), **Conventional Commits**, **atomicity**, and commit **autonomy** (routine proceeds; non-routine stops and asks) — is the `git-practices` family, a dependency of this project loaded first and verbatim from `spec/boot/INLINE.md`. The rules live in that inline lane, not restated here. Authoritative record: [spec://vibevm/common/PROP-000#commits](spec/common/PROP-000.md#commits).

Authoritative record: [spec://vibevm/common/PROP-000#commits](spec/common/PROP-000.md#commits).

## Delegation-first — spend Claude on judgment, run execution on fractality

**The directive is now installed.** The standing posture — Claude's context and reasoning are the scarcest, most expensive resource in the room; the cheap worker slots sit idle, already paid for, so **delegate execution by default and keep Claude for architecture, planning, judgment, and review** (a session that codes, bulk-edits, or reads-and-summarizes work a worker could do is spending the very budget this directive exists to save) — is the `delegation-first` flow, a static dependency of this project. It carries the directive in full — the scarce-resource thesis and the ~5%-boss / ~95%-worker target, delegate-by-default, GLM-5.2 as the `big` worker slot, first-level swarm and RLM handling, the never-delegate set, and the obligations (always review; surface the analysis out loud; announce the harness). The decidable calculus it sits above — *delegate when verification is cheaper than generation*, scored on four axes (error cost / context / verifiability / size) with the verdict steps and per-model playbooks — is the `delegation-rules` flow it pulls, now **installed** as a dependency rather than read in-place: `spec://org.vibevm.fractality/delegation-rules/flows/delegation-rules/DECISION-MATRIX#root`.

What follows is **only** vibevm's operational specifics on that directive — the exact fractality entry points, how Rules 1 & 4 bind delegated work, and the live operating-facts ledger. The directive itself — delegate by default, GLM-5.2, RLM, swarms, review, surface, announce — is the package above, not repeated here.

**Running fractality here.** The first-level usage lives in the package; the
verified operating facts (profiles, tokens, packet schema, build state) are the
ledger below. The entry points between them: the launcher is
`packages/org.vibevm.fractality/fractality.ps1` (PowerShell) / `fractality.sh`
(Bash), built once via `cargo build -p fractality-cli` from
`packages/org.vibevm.fractality/fractality/v0.1.0/` against the global
`~/.fractality` home. Drive it — `./fractality.ps1 run --packet <task.toml>`
(sync) or `spawn … ; wait <id>` (async); free `route` / `gate` helpers (no
daemon, no spend); no-packet interim route
`opencode run -m zai-coding-plan/glm-5.2 "<task>"`. RLM's need-gate is
`fractality gate …`; its recursive-descent machinery is
`packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-RLM-PLAN-v0.1.md`
(Campaign 3 Stage B, maturing). On Claude Code, `ultracode` / the Workflow tool
cannot spawn GLM workers directly, so a swarm under them still routes through
fractality.

**Rules 1 & 4 bind delegated work exactly as direct work.** A worker is a tool, never credited — the authored surface of this repository stays human (Rule 1); and non-routine work (Rule 4's ask-first list — history rewrites, force-push, large blobs, CI / signing / secrets, anything whose reversal costs work) stops for the owner *before* it is delegated, not only when done directly. The never-delegate set is narrower than that list and never replaces it.

*(The fractality workspace runs the strong, mechanized form of this — its
⛔ DELEGATION LAW + live-observation protocol in
`packages/org.vibevm.fractality/CLAUDE.md`. The delegation-first package above is
the general form for all vibevm sessions; a workspace session follows its own contract.)*

### Operating facts — the in-place fractality ledger (owner-authorised, keep current)

**Owner grant (2026-07-12):** maintain this ledger **autonomously** — whenever
a session verifies a durable operational fact about running fractality /
delegation, record it here immediately (no need to ask), so no future session
re-learns it. This is an explicit, narrow exception to Rule 4's ask-first for
this sensitive file: it authorises *appending and curating verified
operational facts in this subsection only*, never rewriting the rules above.
Keep it current-state; prune stale lines.

- **Build / run:** `cargo build -p fractality-cli` (also
  `-p fractality-mission-control -p fractality-pod`) from
  `packages/org.vibevm.fractality/fractality/v0.1.0/`; drive via the launcher;
  global home `~/.fractality`. Binaries verified built 2026-07-12.
- **Daemon:** `mc start` is idempotent; read-verbs auto-start it
  (`connect_or_start`); one daemon already runs live on the global home
  (2026-07-12). A different home ⇒ a separate daemon (own lock/port).
- **Profiles** (`~/.fractality/profiles.toml`): profile `glm` → z.ai gateway
  `https://api.z.ai/api/anthropic`, `big = glm-5.2[1m]`, `small = glm-5-turbo`,
  token by PATH `~/.vibevm/zai.api.token` (never inline/echo it);
  `config_dir = "auto"` (fresh per-run `CLAUDE_CONFIG_DIR`).
- **Free vs paid:** `route` / `gate` are pure calculus — no daemon, no spend;
  `run` / `spawn` / `advise` spawn a real GLM worker (z.ai). **Standing owner
  grant (2026-07-13): fractality runs are NOT treated as paid — do not ask
  before spawning; use them freely and by default, the more delegation the
  better (this is the whole point of the delegation-first directive).** Rule 4's
  non-routine red lines still bind the *work* a run performs (history rewrites,
  force-push, large blobs, CI / signing / secrets, irreversible ops) — those
  stop for the owner whether done directly or via a worker, and the never-delegate
  set still governs what may be handed off at all.
- **Interim route (no packet):** `opencode run -m zai-coding-plan/glm-5.2 "<task>"`
  — opencode 1.17.14, z.ai creds in its auth store (2026-07-12); use **only**
  `zai-coding-plan/*` (the `opencode/*` Zen gateway is unpaid here and errors).
- **Packets** (TOML, schema 1): `[task]` goal/acceptance,
  `[workspace] mode = "worktree" | "dir"` (worktree default → `repo`/`base`,
  deliverable branch), `[output]`, `[budget]`, `[routing]` profile/model.
  Golden: `…/fractality/v0.1.0/spec/examples/hello-glm.toml`. Workers **cannot
  run git** — the boss commits/merges the `fractality/<id>` branch.
- **Enable RLM (worker recursion):** profile `allow_tools = ["Bash"]` (worker
  may itself call `fractality spawn`) and/or `ask_boss = true` — both off by
  default. Need-gate verdicts: `inline | route | fold-local | spawn | escalate`.
- **F19 gotcha:** `git worktree add` of THIS host repo overflows Windows
  MAX_PATH on deep `vibedeps/` paths → provisioning uses
  `-c core.longpaths=true`; only a deep real repo catches it.
- **Filing fractality bugs:** operational / behavioural bugs found while running
  fractality go to `packages/org.vibevm.fractality/plans/external/E-BUG-NNN.md`
  (stable id in the filename), in the **E-BUG format** — *what happened · what I
  wanted · what I got · why they differ · ideas on the cause · ideas on the fix ·
  workaround · references* — worked during fractality's own development. First:
  `E-BUG-001` (acceptance quote-mangling).
- **Acceptance gotcha (E-BUG-001):** a packet's `acceptance` mangles quoted
  multi-word commands — `findstr /C:"a b c"` false-fails (each word parsed as a
  filename, `acceptance: 0/N`). Prefer single-token matches; the boss-side
  `diff` / `grep` is the real gate — acceptance is advisory until the diff is read.
- **Delegated-run mechanics (verified 2026-07-13, first real host delegation —
  the wal-test migration on `glm`/`big`):** a `worktree`-mode worker gets its
  **own cold `target/`** (provisioning shares nothing with the host), so an
  edit-and-verify task pays a full `cargo build` — hand such a worker a
  **`cargo check`** self-verify (not the full suite), set `wall_secs` high, and
  expect a long run. **`max_turns` blows easily on a many-edit task** (80 did
  not cover ~40 edits + iterative build-verify): the run then ends
  `state=failed exit=1` **though the work may be complete** — never discard on
  "failed"; review the worktree first. **`show`/`ps` usage (in/out tokens) does
  not flush until terminal** — `in=0/out=0` mid-run is *not* a stall; judge
  liveness by `runs/<id>/worker-stdout.jsonl` growth + `git -C runs/<id>/wt
  status`. Review path (workers can't git): `git -C runs/<id>/wt diff` → read as
  a PR → `git apply` it into the host tree (worker touches disjoint files → it
  applies clean) → boss runs the real gate (`self-check`) → boss commits +
  pushes. **Workers don't `cargo fmt`** → run `cargo fmt --all` after applying
  (fmt is self-check's fail-fast first gate). A background `fractality wait <id>`
  yields a clean completion notification. Net: `big` executed the ~40-edit,
  map-guided migration faithfully (0 stale values); the only boss fixes were
  fmt + 2 behavioural edge cases — exactly the "boss verifies + finishes the
  tail" split.
- **License state (keep current):** our shipped surface is **fully UPL-1.0**. The
  canonical `packages/org.vibevm.*/**` (redbook family, discipline stack,
  fractality, delegation-rules, wal-specspaces) were relicensed by MT-05 firing #2
  (merges `893e314` / `79938ab`); the host root `LICENSE.md` was relicensed
  2026-07-12 (MT-05 run `01KXBEHEYJCQ1RNJ5657Q31HVA`; host crates inherit via
  `license-file.workspace`). The `"EULA"` strings that remain are all **off-limits
  for relicensing**: `refs/**` (third-party), `vibedeps/**` + `.vibe/cache/**`
  (regenerated dep copies), `fixtures/**` + `crates/**` test data (tests assert on
  `"EULA"`), the `licensing` package (legitimate eula-template), and
  `VIBEVM-SPEC.md` + specs (owner-frozen / historical mentions). Dogfood spec:
  `…/fractality/v0.1.0/spec/manual-tests/MT-05-dogfood-relicense.md`.

## Specspaces — nested projects with their own WAL

This repository can host **specspaces**: sub-projects registered in [`SPECSPACES.md`](SPECSPACES.md) that carry their own boot contract, WAL, and `CONTINUE.md`, worked on as independent projects. Canon (grammar, target resolution, the five laws) is the installed flow `flow:org.vibevm.world/wal-specspaces` — its snippet is slot 11 of `spec/boot/INDEX.md`, and the full protocol is `spec/flows/wal-specspaces/SPECSPACES-PROTOCOL.md` inside that package. This section is the signpost; two rules bind regardless:

- **Target resolution.** A **bare** session phrase (`восстанови сессию` / `RESUME SESSION` with no name) targets the `default:` declared in `SPECSPACES.md` if one is set, and otherwise **this host project** — never a specspace by accident. Name a specspace (`восстанови сессию fractality` / `RESUME SESSION fractality`, `заверши сессию fractality` / `END SESSION fractality`) to target it; an explicit name or directory always overrides the default. Registered today: `fractality` (`packages/org.vibevm.fractality/`).
- **Boot scoping.** A specspace session reads the host's Rules 1–4 above (repo-wide, they bind every commit) plus the specspace's own boot contract → its WAL → its `CONTINUE.md` → the active plan its WAL names. It does **not** read the host `spec/boot/`, `spec/WAL.md`, or host specs, and does not scan the host tree — unless the task explicitly crosses into the host project, and then it says so first. A specspace wind-down refreshes that specspace's one-line status in `SPECSPACES.md`; the host WAL is updated only if host files changed in the same session.

## Memory discipline: project facts stay in the project

Facts about *this project* — its design, conventions, decisions, milestones, open questions, owner preferences that govern technology choices — live **inside this repository**. The canonical homes are:

- `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` (kept identical; the four rules and the few directives that must hit every harness on session boot).
- `MEMORY.md` at repo root (currently a pointer to [`spec/boot/90-user.md`](spec/boot/90-user.md), the user-owned boot snippet).
- `TASKS.md` at repo root, if one is warranted (not present today).
- Authoritatively, the `spec/**/*.md` tree — PROP / FEAT documents, `spec/WAL.md`, `spec/boot/*`.

Project facts do **not** belong in the running harness's global per-user auto-memory (whatever tool-specific path that happens to be). A teammate who clones the repo will never see global user-memory, and anything they need to know about the project must live in the repo.

Global user-memory is reserved for facts about *this developer's machine* — shell quirks, SSH-agent setup on this box, installed-tool specifics that persist across sessions but are not universal.

**Default:** when uncertain whether a fact is project-scoped or machine-scoped, treat it as project-scoped and write it into the repo. Moving a fact from the project into user-memory later is cheap; the reverse has already silently cost a teammate context.

## Session-end checkpoint command — `ЗАВЕРШИ СЕССИЮ` / `END SESSION`

When the user issues any of the trigger phrases below, treat it as a structured wind-down request — the user is about to close the conversation and may continue from a fresh context (different machine, new session, post-compaction). The job is to capture *everything a cold reader would need* into durable repository state.

**Trigger phrases** (case-insensitive; exact wording not required, recognise the intent):

- Russian: `ЗАВЕРШИ СЕССИЮ КОДИРОВАНИЯ`, `ЗАВЕРШИ СЕССИЮ`, `КОНЕЦ СЕССИИ`, `ЗАКАНЧИВАЕМ СЕССИЮ`, `ЗАВЕРШАЕМ СЕССИЮ`, `СВОРАЧИВАЕМСЯ`, `ФИКСИРУЕМ И ЗАКАНЧИВАЕМ`.
- English: `END SESSION`, `WRAP UP SESSION`, `WRAP UP`, `FINISH SESSION`, `CLOSING SESSION`, `CHECKPOINT AND CLOSE`.

**Required behaviour** when a trigger phrase fires:

1. **Overwrite `CONTINUE.md` at repo root** with a comprehensive cold-resume document. If the file exists, replace it wholesale (do not append — staleness compounds otherwise). The body must include, at minimum:
   - A short TL;DR / executive summary at the top.
   - Where work currently stands (branch, ahead/behind origin, working tree status).
   - The active blocker (if any) and the exact human action that unblocks it.
   - Exact next-steps recipe (commands, file paths, line numbers) for whoever picks up cold.
   - Any non-obvious findings discovered this session (API quirks, config gotchas, vendor-specific surprises).
   - A repository map (top-level layout + what each crate / directory holds).
   - The list of important architectural / policy decisions still in force, in long form.
   - The recent commit chain (last ~25 commits, oneline format) so cold reader sees velocity.
   - Quick-start commands for the workspace.
   - A pointer noting the WAL is the canonical *living* state and supersedes this snapshot if they diverge.
2. **Update `spec/WAL.md`** with the current checkpoint — bump the date line, refresh the "Current phase" / "Next" / "Known issues" sections, record any new findings or commits since the last WAL update.
3. **Commit the changes in topic-grouped commits** per Rule 3. Typical shape: one `docs(continue): cold-resume checkpoint` for `CONTINUE.md`, one `docs(wal): session-end checkpoint` for `spec/WAL.md`. If the same session-end run also lands a code or boot-file change, that is a separate third commit.
4. **Push to `origin/main`** — routine per Rule 4, since the user invoked the wind-down explicitly. (If push would be non-fast-forward or otherwise risky, stop and ask first per Rule 4's escape hatch.)
5. **Emit a TL;DR / executive summary in the chat response** describing what this command did: which files were written / updated, which commits were created, push status, what the next session should pick up first. Keep it short enough to scan in one screen, but include enough detail (file paths, commit subjects, blockers) that the user can verify nothing was missed without opening the files.

The point of this command is to make session-boundary loss-of-context cheap: any session can be ended at any time and resumed from `CONTINUE.md` + `spec/WAL.md` with no degradation. Treat it as a hard contract, not a courtesy.

## Session-resume command — `ВОССТАНОВИ СЕССИЮ` / `RESUME SESSION`

When the user issues a resume trigger phrase, the job is to **restore context and report — nothing else**. Recognise the intent, not the exact wording:

- Russian: `ВОССТАНОВИ СЕССИЮ`, `ВОССТАНОВИ КОНТЕКСТ`, `ПРОДОЛЖАЕМ С ТОГО ЖЕ МЕСТА`.
- English: `RESUME SESSION`, `RESTORE SESSION`, `RESTORE CONTEXT`.

**Required behaviour** when a resume phrase fires:

1. Run the full boot sequence (this file → `spec/boot/INDEX.md` and its files → `spec/WAL.md`), read `CONTINUE.md`, and verify repository state empirically (branch, sync with origin, working tree, recent commits).
2. **Emit a status report in the chat**: where work stands, gate-panel state as last recorded, active blockers, and the candidate next steps (typically the plan pointer from the WAL / `CONTINUE.md`).
3. **Stop and wait for direction.** No code edits, no plan-phase execution, no commits, no pushes. The owner reads the report and decides what the session does. Any "resume work at …" pointer in `CONTINUE.md` or the WAL names the *candidate* next step for the report — it is not authorisation to start it.

Rationale: the resume boundary exists so the owner can inspect the restored state and steer — possibly somewhere other than the recorded next step. A session that boots straight into execution takes that decision away (rule recorded 2026-06-12 after exactly that misfire).

<vibevm>
<!-- Generated by vibe — do not edit inside this block; it is rewritten on `vibe install`. Text outside the <vibevm> markers is yours. -->

# Session boot

This project's boot sequence is computed by vibe (the PROP-009 loading
model). To begin a session, read these files in order:

1. `spec/boot/STATIC.md` — if it exists. The static (priority) lane: read it
   first and in full.
2. `spec/boot/INDEX.md` — a generated TOML manifest. Read every file named
   by its `[[entry]]` tables, in the listed order. A `kind = "static"`
   entry is read directly; a `kind = "dynamic"` entry is an INCLUDE
   resolved at boot, and one carrying a `when` condition is read only when
   that condition holds for the current session.

Boot is pure file-reading — there is nothing to execute.
</vibevm>
