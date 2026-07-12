# vibevm — read this first

Every session in this repository begins by reading this file, then every file in `spec/boot/` in filename order, then `spec/WAL.md`, then any relevant PROP/FEAT documents under `spec/common/` and `spec/modules/` for the task at hand. Only after that, start work.

The four rules below are non-negotiable and apply to every session. Authoritative reference: [spec://vibevm/common/PROP-000#commits](spec/common/PROP-000.md#commits).

## Rule 1 — Attribution: keep this repository human-authored

Never attribute authorship of any part of this repository to an artificial-intelligence (AI) or neural-network–based system of any kind — no commit messages, no Git trailers (`Co-Authored-By`, `Signed-off-by: <model-name>`, etc.), no branch names, no worktree branch names, no code comments, no README lines, no release notes. This applies to every such system regardless of provider, model family, or whether it is local or cloud-hosted.

**Why:** Some jurisdictions regulate or criminalize machine authorship of software. We are not currently subject to such regulation and are not violating any present law; the owner chooses this policy proactively so that any future regulation finds no hook on this project. The surface this repository presents is: a human wrote this code, full stop.

**This paragraph (and its copy in [PROP-000 §12.1](spec/common/PROP-000.md#commits)) is the single place in the entire project where AI tooling is discussed in the attribution sense.** Everywhere else — commits, code, docs, branches, CI, signing — assume human authorship only. `VIBEVM-SPEC.md` discusses AI integration as a feature of the vibevm product (what the tool does for its users); that is product scope, not attribution, and is not covered by this rule.

## Rule 2 — Conventional Commits

Every commit follows the [Conventional Commits](https://www.conventionalcommits.org/) specification.

```
type(scope): short imperative subject line

Longer body — a sentence, a paragraph, or a mini-article depending
on how much reasoning the change carries. Explain WHY this change
was made and what follows from it. The diff already shows what
changed; the value of the commit message is the reasoning and the
downstream consequences that a future reader cannot reconstruct
from the diff alone.

Cite `spec://…` URIs where relevant.
```

- Keep the subject short (target ≤ 60 characters, hard limit 72) so Git web UIs render it without truncation.
- Body is free-form; prefer paragraphs over bullet lists when reasoning is continuous.
- `type` is one of `feat`, `fix`, `chore`, `docs`, `build`, `test`, `refactor`, `perf`, `style`, `ci`, `revert`.
- `scope` names the most affected crate, package, or subsystem (e.g. `core`, `install`, `wal`, `registry`, `spec`).

## Rule 3 — Group commits by meaning

When the working tree carries changes spanning multiple concerns, split them into separate commits grouped by topic — never by file name or time of edit. Each commit is one logical unit. A working set containing "fix typo in README" + "refactor the planner" + "update the manifest schema" is **three** commits, not one.

## Rule 4 — Autonomy on routine changes only

Routine large changes — implementing a planned milestone, finishing a feature slice, touching many files for one coherent reason — may be committed and pushed without first asking the user, using rules 1–3.

Stop and ask the user first for anything **non-routine**:

- rewriting published history (rebase of pushed commits, `git commit --amend` of pushed work),
- `git push --force` or `--force-with-lease`,
- bringing in large binary blobs,
- changing CI, signing, or secrets configuration,
- any operation whose reversal would cost work.

When uncertain, ask.

## Delegation-first — spend Claude on judgment, run execution on fractality

**Standing directive (owner-commissioned 2026-07-12).** Claude's context and
reasoning are the scarcest, most expensive resource in the room; cheap worker
models (GLM-5.2 / GLM-5-turbo via z.ai) sit idle, already paid for. So for
**every** task — one the user asks for, or one you or another plan/agent set —
the first question is: **can this be delegated to fractality?** Delegate
execution by default; keep Claude for architecture, planning, judgment, and
review. A session that codes, bulk-edits, or reads-and-summarizes work a
worker could do is spending the very budget this directive exists to save.

**The calculus is `delegation-rules` — read it in-place before delegating.**
One law: *delegate when verification is cheaper than generation.* The full
procedure (four axes — error cost / context / verifiability / size — the
verdict steps, the never-delegate set, per-model playbooks) lives in the
delegation-rules package. Until it is installed as a dependency, read it where
it lies (in-place, per owner directive — this moves once fractality graduates):

- `packages/org.vibevm.fractality/delegation-rules/v0.1.0/spec/boot/77-flow-delegation-rules.md`
- `…/spec/flows/delegation-rules/DECISION-MATRIX.md`
- `…/spec/flows/delegation-rules/playbooks/{glm-5.2,glm-5-turbo}.md`

**Don't fear the big model.** GLM-5.2 (the `big` slot) is for substantial
one-shot work — a whole module with its tests and a self-verify command, a
long document distilled — not just trivia; `glm-5-turbo` (`small`) takes
bounded mechanical transforms. Size the packet coarsely (goal, exact
paths/APIs, non-goals, an acceptance command): big models earn coarse
one-shots.

**How to run it, in-place (no global install).** fractality runs from the
working-tree build against the global `~/.fractality` home (a mission-control
daemon + `profiles.toml` already live there). Build it once with
`cargo build -p fractality-cli` from
`packages/org.vibevm.fractality/fractality/v0.1.0/`, then drive it through the
launcher — `packages/org.vibevm.fractality/fractality.ps1` (PowerShell) or
`fractality.sh` (Bash). In the command sketches below, `fractality` means that
launcher (`./fractality.ps1` / `./fractality.sh`), not a binary on PATH:

- `./fractality.ps1 run --packet <task.toml>` (sync) or
  `./fractality.ps1 spawn … ; ./fractality.ps1 wait <id>` (async) —
  mission-control spawns an isolated GLM worker; results come back as files.
  Golden sample packet:
  `packages/org.vibevm.fractality/fractality/v0.1.0/spec/examples/hello-glm.toml`.
- No-packet interim route: `opencode run -m zai-coding-plan/glm-5.2 "<task>"`.
- Free decision helpers (no daemon, no spend): `./fractality.ps1 route …` /
  `./fractality.ps1 gate …`.

**Enable RLM when a task needs it.** The need-gate is `fractality gate …`: it
prints one of five verdicts — `inline | route | fold-local | spawn | escalate`
— on a task's signals (VISION §V2; the recursive descent/ascent machinery is
`packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-RLM-PLAN-v0.1.md`,
Campaign 3 Stage B — built but still maturing). A worker only recurses —
sub-delegates or escalates — when its profile grants the capability:
`allow_tools = ["Bash"]` (so the worker can itself call `fractality spawn`)
and/or `ask_boss = true` in `profiles.toml`, both off by default. If a task
needs that sub-delegation or escalation, enable it for the worker's class
rather than reclaiming the task by reflex.

**Swarm modes route through fractality too — including `ultracode` and
Workflow.** Whenever you would fan out a swarm of agents, prefer one of:

- **(a) fractality *is* the swarm** *(the default)* — the parallel workers
  are fractality-spawned GLM agents (`spawn` + `wait`/`tree`), not Claude
  subagents. Heavy execution runs on cheap GLM; you orchestrate and review.
- **(b) big-class launcher agents drive fractality** — reach for this only
  when the mode forces Claude subagents (`ultracode` / the Workflow tool give
  you no direct GLM spawn) or the orchestration itself needs Claude-level
  judgment. Give the launcher a *big-class* model (e.g. Opus) but keep its
  job *thin*: drive fractality and review the workers' files. The cost win
  holds **only while the launcher stays thin** — a big-model agent that
  merely orchestrates fractality moves few tokens, so it beats a mid-model
  agent carrying the token-heavy coding at max reasoning; the moment the
  launcher does the work itself, that arithmetic inverts. The rule under
  both: never let Claude carry the token-heavy execution.

Default a swarm to route through fractality unless the work sits in the
never-delegate set.

**Always review; the ask-first gates still bind.** Delegated output is
advisory until you read the diff as a contributor PR and the gate/acceptance
is green — whatever the worker claimed; delegation without review is
abandonment. Never hand off the never-delegate set: secrets or credentials,
destructive or irreversible operations, architecture/spec/plan authoring,
ambiguity-that-is-design, the review itself, or sub-minute edits. **Rule 1
still governs** — delegation is a working method; the authored surface of this
repository stays human, and workers are tools, never credited. **Rule 4 still
governs** — non-routine work (its ask-first list: history rewrites,
force-push, large blobs, CI / signing / secrets, anything whose reversal costs
work) stops for the owner *before* it is delegated or done; the never-delegate
set is narrower than that list and never replaces it.

*(The fractality workspace runs the strong, mechanized form of this — its
⛔ DELEGATION LAW + live-observation protocol in
`packages/org.vibevm.fractality/CLAUDE.md`. This host directive is the general
form for all vibevm sessions; a workspace session follows its own contract.)*

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
  `run` / `spawn` / `advise` spawn a real GLM worker = **z.ai spend**. The
  blanket paid-run pre-authorisation ended with the 2026-07-12 five-task goal —
  a new paid run needs the owner's word (a task that commissions a run is that
  word).
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
- **License state (keep current):** our shipped surface is **fully UPL-1.0**. The
  canonical `packages/org.vibevm/**` (redbook family, discipline stack,
  fractality, delegation-rules, wal-workspaces) were relicensed by MT-05 firing #2
  (merges `893e314` / `79938ab`); the host root `LICENSE.md` was relicensed
  2026-07-12 (MT-05 run `01KXBEHEYJCQ1RNJ5657Q31HVA`; host crates inherit via
  `license-file.workspace`). The `"EULA"` strings that remain are all **off-limits
  for relicensing**: `refs/**` (third-party), `vibedeps/**` + `.vibe/cache/**`
  (regenerated dep copies), `fixtures/**` + `crates/**` test data (tests assert on
  `"EULA"`), the `licensing` package (legitimate eula-template), and
  `VIBEVM-SPEC.md` + specs (owner-frozen / historical mentions). Dogfood spec:
  `…/fractality/v0.1.0/spec/manual-tests/MT-05-dogfood-relicense.md`.

## Workspaces — nested projects with their own WAL

This repository hosts **workspaces**: sub-projects registered in [`WORKSPACES.md`](WORKSPACES.md) that carry their own boot contract (`CLAUDE.md` at the workspace root), their own WAL, and their own `CONTINUE.md`, and are worked on as independent projects. Canon: `flow:org.vibevm/wal-workspaces` (authored under `packages/org.vibevm/`, like the rest of the redbook family).

- A session-end or session-resume phrase carrying a workspace name — e.g. `восстанови сессию fractality` / `RESUME SESSION fractality`, `заверши сессию fractality` / `END SESSION fractality` — targets **that workspace**, not this host project. The same required behaviours apply (resume = restore, report, stop; wind-down = WAL + cold-resume + commits + push), but they operate on the workspace's own files.
- **Workspace boot replaces the host boot.** A workspace session reads: Rules 1–4 above (repo-wide, they bind every commit), then the workspace's `CLAUDE.md` → its WAL → its `CONTINUE.md` → the active plan its WAL names. It does **not** read `spec/boot/`, `spec/WAL.md`, or host specs, and does not scan the host tree — unless the task explicitly crosses into the host project, and then it says so first.
- A workspace wind-down also refreshes that workspace's one-line status in `WORKSPACES.md`. The host WAL is updated only if host files changed in the same session.

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

1. `spec/boot/INLINE.md` — if it exists. The priority lane: read it first
   and in full.
2. `spec/boot/INDEX.md` — a generated TOML manifest. Read every file named
   by its `[[entry]]` tables, in the listed order. A `kind = "static"`
   entry is read directly; a `kind = "dynamic"` entry is an INCLUDE
   resolved at boot, and one carrying a `when` condition is read only when
   that condition holds for the current session.

Boot is pure file-reading — there is nothing to execute.
</vibevm>
