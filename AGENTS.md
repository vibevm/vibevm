# vibevm — read this first

Every session in this repository begins by reading this file, then the boot lane in the order the generated block at the end of this file prescribes (the generated static lane `vibevm/vibespecs/boot/STATIC.xml` — or `STATIC.md` in a Markdown-lane project — first and in full, then every file `vibevm/vibespecs/boot/INDEX.md` names), then any relevant PROP/FEAT documents under `vibevm/vibespecs/common/` and `vibevm/vibespecs/modules/` for the task at hand. An owner-facing central session additionally resolves its exact user-local context under `~/.vibe/steward/` as the installed multi-user-planning flow prescribes; a worker packet never claims that context. Only after that, start work.

**A task carrying `##subagent-quiet-clause` does not run that sequence.** A delegated worker, a review-only agent or a consulted subagent reads exactly the files its packet names — including whichever standing rules bind that particular task, named file by file — and its packet is the rest of its instruction surface. Selecting those files is the packet author's job, not the worker's: a worker that needs a rule its packet did not name reports a packet defect rather than reading the lane itself. The full lane costs roughly 145k tokens before such a session reaches its first instruction, and nearly all of it governs decisions a worker never makes.

The repository's commit-and-push discipline — human-authored **attribution** (never mark any part of this repository as AI-authored), **Conventional Commits**, **atomicity**, and commit **autonomy** (routine proceeds; non-routine stops and asks) — is the `git-practices` family, a dependency of this project loaded first and verbatim from the generated static lane in `vibevm/vibespecs/boot/`. The rules live in that static lane, not restated here. Authoritative record: [spec://org.vibevm.core/vibevm/common/PROP-000#commits](vibevm/vibespecs/common/PROP-000.xml#commits).

Authoritative record: [spec://org.vibevm.core/vibevm/common/PROP-000#commits](vibevm/vibespecs/common/PROP-000.xml#commits).

## Delegation-first — spend Claude on judgment, run execution on fractality

**The directive is now installed.** The standing posture — Claude's context and reasoning are the scarcest, most expensive resource in the room; the cheap worker slots sit idle, already paid for, so **delegate execution by default and keep Claude for architecture, planning, judgment, and review** (a session that codes, bulk-edits, or reads-and-summarizes work a worker could do is spending the very budget this directive exists to save) — is the `delegation-first` flow, a static dependency of this project. It carries the directive in full — the scarce-resource thesis and the ~5%-boss / ~95%-worker target, delegate-by-default, GLM-5.2 as the `big` worker slot, first-level swarm and RLM handling, the never-delegate set, and the obligations (always review; surface the analysis out loud; announce the harness). The decidable calculus it sits above — *delegate when verification is cheaper than generation*, scored on four axes (error cost / context / verifiability / size) with the verdict steps and per-model playbooks — is the `delegation-rules` flow it pulls, now **installed** as a dependency rather than read in-place: `spec://org.vibevm.fractality/delegation-rules/flows/delegation-rules/DECISION-MATRIX#root`.

What follows is **only** vibevm's operational specifics on that directive — the exact fractality entry points, how Rules 1 & 4 bind delegated work, and the live operating-facts ledger. The directive itself — delegate by default, GLM-5.2, RLM, swarms, review, surface, announce — is the package above, not repeated here.

**Running fractality here.** The first-level usage lives in the package; the
verified operating facts (profiles, tokens, packet schema, build state) are the
ledger below. The entry points between them: the launcher is
`vibevm/vibepacks/org.vibevm.fractality/fractality.ps1` (PowerShell) / `fractality.sh`
(Bash), built once via `cargo build -p fractality-cli` from
`vibevm/vibepacks/org.vibevm.fractality/fractality/v1.0.0/` against the global
`~/.fractality` home. Drive it — `./fractality.ps1 run --packet <task.toml>`
(sync) or `spawn … ; wait <id>` (async); free `route` / `gate` helpers (no
daemon, no spend); no-packet interim route
`opencode run -m zai-coding-plan/glm-5.2 "<task>"`. RLM's need-gate is
`fractality gate …`; its recursive-descent machinery is
`vibevm/vibepacks/org.vibevm.fractality/fractality/v1.0.0/vibevm/vibespecs/plans/FRACTALITY-RLM-PLAN-v0.1.xml`
(Campaign 3 Stage B, maturing). On Claude Code, `ultracode` / the Workflow tool
cannot spawn GLM workers directly, so a swarm under them still routes through
fractality.

**Rules 1 & 4 bind delegated work exactly as direct work.** A worker is a tool, never credited — the authored surface of this repository stays human (Rule 1); and non-routine work (Rule 4's ask-first list — history rewrites, force-push, large blobs, CI / signing / secrets, anything whose reversal costs work) stops for the owner *before* it is delegated, not only when done directly. The never-delegate set is narrower than that list and never replaces it.

*(The fractality workspace runs the strong, mechanized form of this — its
⛔ DELEGATION LAW + live-observation protocol in
`vibevm/vibepacks/org.vibevm.fractality/CLAUDE.md`. The delegation-first package above is
the general form for all vibevm sessions; a workspace session follows its own contract.)*

### Operating facts — the in-place fractality ledger (owner-authorised, keep current)

**Owner grant (2026-07-12):** maintain this ledger **autonomously** — whenever
a session verifies a durable operational fact about running fractality /
delegation, record it here immediately (no need to ask), so no future session
re-learns it. This is an explicit, narrow exception to Rule 4's ask-first for
this sensitive file: it authorises *appending and curating verified
operational facts in this subsection only*, never rewriting the rules above.
Keep it current-state; prune stale lines.

- **Harness delegation surface (verified 2026-08-30):** native
  `Agent` / `Task` / `Workflow` tools inherit their harness family; they
  offload context but do not select the machine's external GLM lane. General
  non-campaign GLM execution still routes through fractality, with bare
  `opencode run` only as the last-resort fallback below. **During the active
  OpenAI ChatGPT lifecycle campaign, PROP-055 is the scoped override:** healthy
  `claudez` is the first execution lane, root accepts, and a correction
  continues with `claudez -c` in the exact same dedicated cwd. The installed
  `C:/Users/olegc/opt/bin/claudez.ps1` is a thin Claude Code launcher over the
  z.ai Anthropic-compatible gateway; it currently maps the large aliases to
  `glm-5.3[1m]`, keeps state in `~/.claude-glm`, and loads its bearer from
  the token file without exposing it. Launch fresh bounded work with
  `claudez -p <pointer> --permission-mode bypassPermissions`; use ordinary
  text output plus the durable report for long work, never `claudez2` unless
  the owner explicitly re-enables it. PROP-055 and launcher re-verified
  2026-08-30 after an incorrect `opencode` fallback was caught in review.
- **Codexrunner quiet-packet guard (verified 2026-08-30):** a plain
  `codexrunner exec <pointer>` auto-loaded the repository `AGENTS.md` before
  reading the named packet, then began the forbidden full 145k boot despite the
  packet's `##subagent-quiet-clause`. Launch a boot-limited worker as
  `codexrunner exec --strict-config --ignore-user-config -c
  project_doc_max_bytes=0 -c model_reasoning_effort='"xhigh"' <pointer>`;
  set the shared `CARGO_TARGET_DIR` in that process environment before launch.
  The strict-config probe returned exact `QUIET_OK` without a tool call, and
  the real retry read the packet first, loaded only its six named standing
  files, resolved `gpt-5.6-sol` / `xhigh`, and left product work unstaged.
  Without the late effort override this machine's user layer reported `ultra`
  despite the launcher's earlier xhigh default.
- **Post-clean v1 build workaround (verified 2026-08-30):** the repository-root
  `fractality.ps1` is stale despite the v1 ledger below — it still hardcodes
  `fractality/v0.1.0/target/debug/fractality.exe`, so it cannot launch the
  current slot after a clean. Build v1 directly from `fractality/v1.0.0/`,
  temporarily junctioning that slot's root `vibedeps` to its tracked
  `vibevm/vibedeps` because the Cargo path dependencies still name the old
  root layout; build `fractality-cli`, `fractality-mission-control` and
  `fractality-pod`, remove the junction, then invoke
  `v1.0.0/target/debug/fractality.exe` directly. **Do not use the current
  `vibe install` as this repair:** against the old slot it migrates the tracked
  dependency/boot projection to the new coordinate layout and creates a large
  unrelated diff without satisfying Cargo's old root path. This failed route
  was restored and its generated `org.*`/`STATIC.md` residue removed exactly.
- **Build / run:** `cargo build -p fractality-cli` (also
  `-p fractality-mission-control -p fractality-pod`) from
  `vibevm/vibepacks/org.vibevm.fractality/fractality/v1.0.0/`; drive via the launcher;
  global home `~/.fractality`. Binaries verified built 2026-07-12.
- **Daemon:** `mc start` is idempotent; read-verbs auto-start it
  (`connect_or_start`); one daemon already runs live on the global home
  (2026-07-12). A different home ⇒ a separate daemon (own lock/port).
- **Profiles** (`~/.fractality/profiles.toml`): profile `glm` → z.ai gateway
  `https://api.z.ai/api/anthropic`, `big = glm-5.2[1m]`, `small = glm-5-turbo`,
  token by PATH `~/.vibe/zai.api.token` (never inline/echo it);
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
  **Last resort only:** outside the PROP-055 campaign prefer the fractality
  launcher; inside the active ChatGPT campaign prefer healthy `claudez`.
  Bare `opencode run` is used only when the applicable primary launcher is
  genuinely unavailable, never merely because it is convenient.**
- **Packets** (TOML, schema 1): `[task]` goal/acceptance,
  `[workspace] mode = "worktree" | "dir"` (worktree default → `repo`/`base`,
  deliverable branch), `[output]`, `[budget]`, `[routing]` profile/model.
  Golden: `…/fractality/v1.0.0/vibevm/vibespecs/examples/hello-glm.toml`. Workers **cannot
  run git** — the boss commits/merges the `fractality/<id>` branch.
- **Enable RLM (worker recursion):** profile `allow_tools = ["Bash"]` (worker
  may itself call `fractality spawn`) and/or `ask_boss = true` — both off by
  default. Need-gate verdicts: `inline | route | fold-local | spawn | escalate`.
- **F19 gotcha:** `git worktree add` of THIS host repo overflows Windows
  MAX_PATH on deep `vibedeps/` paths → provisioning uses
  `-c core.longpaths=true`; only a deep real repo catches it.
- **Filing fractality bugs:** operational / behavioural bugs found while running
  fractality go to `vibevm/vibepacks/org.vibevm.fractality/plans/external/E-BUG-NNN.md`
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
  canonical `vibevm/vibepacks/org.vibevm.*/**` (redbook family, discipline stack,
  fractality, delegation-rules, wal-specspaces) were relicensed by MT-05 firing #2
  (merges `893e314` / `79938ab`); the host root `LICENSE.md` was relicensed
  2026-07-12 (MT-05 run `01KXBEHEYJCQ1RNJ5657Q31HVA`; host crates inherit via
  `license-file.workspace`). The `"EULA"` strings that remain are all **off-limits
  for relicensing**: `refs/**` (third-party), `vibedeps/**` + `.vibe/cache/**`
  (regenerated dep copies), `fixtures/**` + `crates/**` test data (tests assert on
  `"EULA"`), the `licensing` package (legitimate eula-template), and
  `VIBEVM-SPEC.md` + specs (owner-frozen / historical mentions). Dogfood spec:
  `…/fractality/v1.0.0/vibevm/vibespecs/manual-tests/MT-05-dogfood-relicense.xml`.

## User-local central stewardship — `~/.vibe/steward/`

Personal execution state is not a project setting. An owner-facing central
session resolves the exact repository/worktree/revision binding under
`~/.vibe/steward/contexts/*/binding.toml`, then reads the context's
`settings.toml`, `custody.toml`, complete `plan.toml`, and latest acknowledged
handoff. The selected campaign's derived `GOAL.md` and bounded
`GOAL-CLAUDE.txt` live beside them. Missing global defaults are created as
`interaction_mode = "collab"`
and `planning_profile = "standard"`; context overrides may differ by branch or
worktree. The owner may switch either setting in chat and the central session
updates the current context immediately. `auto` versus `collab` controls
narration; `ultra` versus `standard` controls planning scaffolding. Neither
axis changes correctness, authority or safety rules.

`AGENT-MODE.toml` at the repository root is legacy migration input and is no
longer authoritative. It remains temporarily only until the first multi-user
handoff is receipted and its explicit owner value has been preserved locally.

Workers and reviewers do not read, create or claim central custody. They always
follow their explicit packet and `##subagent-quiet-clause`; worker reports are
evidence, never acceptance. Canon:
`spec://org.vibevm.world/multi-user-planning/flows/multi-user-planning/MULTI-USER-PLANNING-PROTOCOL#root`.

## Harness-scoped ChatGPT campaign runner

This selector is intentionally byte-identical in `CLAUDE.md`, `AGENTS.md` and
`GEMINI.md`. **Only** an owner-facing central session running under OpenAI
ChatGPT/Codex reads
[`PROP-055`](vibevm/vibespecs/common/PROP-055-chatgpt-campaign-execution.xml)
after the common boot and local stewardship context while the
lifecycle/extensions campaign is active.
Claude Code, `claudez`, Gemini, every other non-OpenAI harness, and every task
carrying `##subagent-quiet-clause` MUST NOT read or apply PROP-055; their common
boot and explicit worker packet remain the whole instruction surface.

## Specspaces — stable topology, user-local continuity

This repository can host **specspaces**: nested projects registered in
[`SPECSPACES.md`](SPECSPACES.md) by stable name, root and boot contract. The
registry is project topology, not a shared personal cursor. Its historical WAL,
`CONTINUE.md`, default and live-status columns are legacy migration input; do
not update them as central-session state.

- **Target resolution.** Explicit owner name or directory wins; otherwise an
  unambiguous task/cwd inside a registered root wins; otherwise the current
  user-local context wins; otherwise target the host project. An unknown or
  ambiguous target is surfaced, never guessed. Registered today: `fractality`
  (`vibevm/vibepacks/org.vibevm.fractality/`).
- **Boot scoping.** A specspace session reads the host's repo-wide commit and
  safety rules plus the target's own boot contract and relevant specs. Its
  plan/custody/handoff come from a context bound to that target under
  `~/.vibe/steward/`; it does not load unrelated host/specspace plans. A task
  crossing boundaries says so before touching the other project.

## Memory discipline: project facts stay in the project

Facts about *this project* — its design, conventions, decisions, milestones, open questions, owner preferences that govern technology choices — live **inside this repository**. The canonical homes are:

- `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` (kept identical; the four rules and the few directives that must hit every harness on session boot).
- `MEMORY.md` at repo root (currently a pointer to [`vibevm/vibespecs/boot/90-user.xml`](vibevm/vibespecs/boot/90-user.xml), the user-owned boot snippet).
- `TASKS.md` at repo root — a legacy/integrator-owned shared projection of the
  current project slice, never a contributor's personal adaptive plan.
- `BACKLOG.md` at repo root — findings the work surfaced and deliberately did
  not act on, severity-triaged P1/P2/P3, drained by the next wave. The opposite
  genre to `TASKS.md`: nobody is working on these yet, and they are kept so the
  decision to start can be taken deliberately (owner directive 2026-07-26).
- Authoritatively, the `vibevm/vibespecs/**` tree — PROP / FEAT documents and
  generated/user boot sources. The existing `vibevm/vibespecs/WAL.xml` is
  legacy migration evidence, not the current central-session authority.

Project facts do **not** belong in the running harness's global per-user auto-memory (whatever tool-specific path that happens to be). A teammate who clones the repo will never see global user-memory, and anything they need to know about the project must live in the repo.

User-local stewardship is reserved for this developer's preferences, exact
checkout/branch context, personal execution plan, central custody/handoffs and
machine facts. **Classification test:** if another contributor needs the fact
to build, review or understand the project, promote it into the repository; if
it only resumes this developer's central session, keep it local. A worker
report or handoff is never the sole home of an accepted project fact.

## Local session checkpoint — `ЗАВЕРШИ СЕССИЮ` / `END SESSION`

When the user issues any trigger phrase below, treat it as a structured local
wind-down. It checkpoints the current user's exact context; it does not create
or rewrite a repository-global personal WAL.

**Trigger phrases** (case-insensitive; exact wording not required, recognise the intent):

- Russian: `ЗАВЕРШИ СЕССИЮ КОДИРОВАНИЯ`, `ЗАВЕРШИ СЕССИЮ`, `КОНЕЦ СЕССИИ`, `ЗАКАНЧИВАЕМ СЕССИЮ`, `ЗАВЕРШАЕМ СЕССИЮ`, `СВОРАЧИВАЕМСЯ`, `ФИКСИРУЕМ И ЗАКАНЧИВАЕМ`.
- English: `END SESSION`, `WRAP UP SESSION`, `WRAP UP`, `FINISH SESSION`, `CLOSING SESSION`, `CHECKPOINT AND CLOSE`.

**Required behaviour:** verify Git/tree state; update the complete local
`plan.toml` and current atom; backscan mandate, candidates and unpromoted facts;
write a uniquely named local session record with accepted boundary, blockers,
next atom and evidence pointers; refresh custody heartbeat or release it only
when the owner requested release; report the checkpoint and stop. Commit/push
only product work already ready under the ordinary git rules—wind-down itself
does not manufacture shared-state commits. A transfer to a different central
agent uses the separate two-phase handoff below.

## Central custody handoff — `HANDOFF CENTRAL TO <target>`

A planned model/harness change, session rollover or owner-authorized recovery
uses `steward-handoff` and
`spec://org.vibevm.world/multi-user-planning/flows/multi-user-planning/custody-and-handoff#root`.
Outgoing: verify and backscan, write/hash an immutable offer plus comprehensive
`HANDOFF.md`, set custody `offering`, then become repository/plan read-only.
Incoming: read cold, verify hashes and tree, classify every candidate, write a
receipt, advance epoch once, report restored state, and wait for owner
direction. Custody transfers; unaccepted work never does.

The owner never pastes a handoff body. In a new/resumed central session,
`ACCEPT HANDOFF FROM <context-id>` / `ПРИМИ ХЭНДОФФ ИЗ <context-id>` resolves
exactly one unreceipted offer stored under that local context (multiple offers
require an explicit handoff id), reads its generated `HANDOFF.md`, and performs
receipt. After claim, refresh the deterministic goal, print full `GOAL.md`, then
print the exact single-line `GOAL-CLAUDE.txt` command. Claude Code slash commands
are user-only: the human pastes that `/goal …` line; an agent never claims it
set the client goal itself.

After creating and sealing `HANDOFF CENTRAL TO <target>`, the outgoing session
must print `HANDOFF CREATED`, the literal `context-id`, the literal `handoff-id`,
and the exact receive command. A cancelled offer has a sibling
`cancellation.toml` and is not considered open; never make the owner search the
local directory for an id.

## Session-resume command — `ВОССТАНОВИ СЕССИЮ` / `RESUME SESSION`

When the user issues a resume trigger phrase, the job is to **restore context and report — nothing else**. Recognise the intent, not the exact wording:

- Russian: `ВОССТАНОВИ СЕССИЮ`, `ВОССТАНОВИ КОНТЕКСТ`, `ПРОДОЛЖАЕМ С ТОГО ЖЕ МЕСТА`.
- English: `RESUME SESSION`, `RESTORE SESSION`, `RESTORE CONTEXT`.

**Required behaviour** when a resume phrase fires:

1. Run the full project boot, resolve the exact user-local context, and read its
   settings, custody, complete plan and latest acknowledged handoff; verify
   branch, sync, working tree, recent commits and named artifacts empirically.
2. **Emit a status report in the chat**: context/profile, custody epoch/holder,
   accepted boundary, candidates, full remaining epic route, expanded current
   frontier, gate-panel state, blockers, and the **judging debt** while a
   campaign is live (`python campaigns/<zone>/tasks/judging-debt.py` — reporting
   is not paying it). Name the candidate next atom from the local plan.
3. **Stop and wait for direction.** No code edits, plan-phase execution,
   custody takeover, commits or pushes. A stored next atom is a candidate, not
   authorization to choose the owner's priority.

Rationale: the resume boundary exists so the owner can inspect the restored state and steer — possibly somewhere other than the recorded next step. A session that boots straight into execution takes that decision away (rule recorded 2026-06-12 after exactly that misfire).

<vibevm>
<!-- Generated by vibe — do not edit inside this block; it is rewritten on `vibe install`. Text outside the <vibevm> markers is yours. -->

# Session boot

This project's boot sequence is computed by vibe (the PROP-009 loading
model). To begin a session, read these files in order:

1. `vibevm/vibespecs/boot/STATIC.xml` — if it exists. The static (priority) lane: read it
   first and in full.
2. `vibevm/vibespecs/boot/INDEX.md` — a generated TOML manifest. Read every file named
   by its `[[entry]]` tables, in the listed order. A `kind = "static"`
   entry is read directly; a `kind = "dynamic"` entry is an INCLUDE
   resolved at boot, and one carrying a `when` condition is read only when
   that condition holds for the current session.

Boot is pure file-reading — there is nothing to execute.
</vibevm>
