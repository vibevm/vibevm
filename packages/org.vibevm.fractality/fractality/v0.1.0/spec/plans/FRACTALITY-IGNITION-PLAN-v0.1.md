# FRACTALITY-IGNITION-PLAN v0.1 — from zero to a metered GLM swarm under mission-control

_Status: PLANNED · written 2026-07-09 against host tree `05d3b1c` plus the
same-day ignition bootstrap commits · cold-executable: any phase boundary is
a safe stop; the fractality floor (§11) is green at every boundary. Format:
`flow:org.vibevm/campaign-plans` (one file, five roles)._

## 2. Execution record

_(Empty at authoring. Prepended at close: commit range, per-phase deltas
against §4, a verdict on every §7 prediction.)_

## 3. The mandate (owner, 2026-07-09, verbatim)

Commission, from the opening message:

> «Я хочу научить Claude Code думать качественно лучше, путём запуска
> субагентов на дешёвых нейросетях чуть более низкого класса. […] Первый —
> с моей основной подпиской Claude Max x20. Второй — подключив туда GLM 5.2
> и GLM-5-Turbo в качестве модели. […] Ну по сути, не только 2 раза, а
> много раз — сколько нужно. […] Я хочу сделать про это отдельный пакет, в
> packages, который будет написан серьёзно, хорошо и кроссплатформенно на
> Rust (минимум всяких Bash, Python и другой ерунды) и по сути является
> отдельным программным продуктом. Пока внутри VibeVM, пока не обкатаем
> более глобально. Сам по себе проект не зависит от VibeVM […] У проекта
> должен быть свой собственный WAL, Continue, и так далее. […]
> 1. Умеем делегировать работу в GLM, запущенную в параллельном Claude Code
> с другим провайдером (изолированно). […] достаточно универсальные
> паттерны проектирования, чтобы в будущем мочь подключать другие
> инструменты. Но пока — только Claude Code.
> 2. Учим запускать более 1 такого агента. […] целый сворм агентов.
> Стратегию как плодить сворм можно сделать пока очевидной: отдать на
> усмотрение модели.
> 3. Умеем правильно настраивать правила — что делегировать, что не
> делегировать. Нужен Clean Room Implementation для [steipete codex-first]
> […] пройтись по каждому из пунктов и улучшить их […] сделать в несколько
> раз более крутым.
> 4. Важная особенность GLM — […] её максимальная эффективность заключается
> в больших и сложных one-shot решениях задач (надо проверить, рекомендую
> поискать в интернете). […] GLM-5-Turbo стоит делегировать всё что
> маленькое (по типу того, что ты отдал бы в Haiku или Sonnet). […] любые
> документы из интернета нужно качать локально, а не использовать web-read
> […] (всего 4000 вызовов MCP в месяц). Должно быть какое-то расширение для
> общей инструкции из пункта 2, которое описывает шаблоны использования для
> разных моделей. С возможностью расширения […] когда-нибудь […] Codex.
> 5. Делаем систему, которая вынуждает модель чаще и более креативно
> использовать агентов. […] Clean Room Implementation [barkain workflow
> orchestration] но это дискуссионное. Конечно же на Rust […]
> 6. Учим делать инструкции, поощряющие и поддерживающие паттерн RLM. […]
> Запускать его нужно только тогда, когда задача тому способствует.
> [arXiv 2512.24601; alexzhang13/rlm] Нам нужна реализация без всего этого
> Python ужаса, нужен нормальный Rust. […]
> Всё это Clean Room Implementation на Rust, без использования копипасты из
> изначальных проектов. […] прямая копипаста приведёт к юридическим
> последствиям. […] На выходе получится продукт, который позволяет
> использовать подписку Claude Code Max x20 во много раз более эффективно,
> при этом не создавая вреда серверам Anthropic и не нарушая никаких
> условий использования.»

Rulings, from the answers message:

> «Имя — fractality. Сделать отдельную группу: org.vibevm.fractality, её
> корневой пакет с которого всё начинается — org.vibevm.fractality/fractality
> и все пакеты класть туда (лучше декомпозировать проект на библиотеки и так
> далее, чем валить всё в одну кучу). […] передачу всех результатов нужно
> делать ТОЛЬКО через файлы на диске. Поэтому фаза 1 на самом деле
> распадается на 2 фазы — вначале мы учимся просто делегировать задачу и
> класть на диск, вторая часть — учимся забирать назад ответы. […] отдельное
> приложение (тоже на Rust) которое бы запускалось как сервер, висело в фоне
> и хранило на себе состав агентов, стек вызовов, и другие штуки — примерно
> те же штуки которые языки программирования используют при вызове функций.
> Это будет такой scheduler нашей "агентной операционной системы". В далёком
> будущем мы приделаем к нему разную аналитику, GUI, и сможем использовать
> его как источник для observability и meta-cognition как для людей, так и
> для агентов. Возможно мы даже научим запускаться эту штуку на выделенном
> сервере и сможем объединять агентов, работающих на разных компьютерах. […]
> Прямо сейчас […] агент регистрируется в этом приложении и ждёт возврата —
> возврата можно ждать синхронно и асинхронно, на самом первом этапе только
> синхронно чтобы проверить что наши идеи вообще работают. Субагент кладёт
> файл и как-то рапортует что он завершил работу. Приложение назовём
> fractality-mission-control. Стек — поищи сам, вся самая свежая экосистема
> Rust, вероятно Axum с нативной интеграцией Tokio и так далее. […] Все
> процессы регистрируются в fractality-mission-control, поэтому […] легко
> смотреть деревья вызовов, делать утилиты для рекурсивного kill (вероятно
> должно быть частью fractality-mission-control), и это кроссплатформенно.
> По сути мы строим крутую распределённую агентскую операционную систему
> […] проведи аналогию с теми годами когда ядро Linux и утилиты GNU были
> молодыми. […] Токен zai лежит в ~/.vibevm/zai.api.token […] Разработай
> самую лучшую и крутую систему, barkain был просто как ранний прототип
> одного из решений. Но меня очень беспокоит, что если мы не сделаем
> какую-то систему инициативы для Opus, он просто не будет использовать
> спавн наших агентов. В принципе, чем меньше заточки на Claude Code тем
> лучше […] будет Opus внутри других агентов (включая наш собственный
> будущий агент VibeVM Pixel который будет работать специально для Opus)
> […] А та ерунда, что написана у barkain это ужасное питоноговно, и так
> делать не нужно.»

Addenda, same day:

> «Напиши хороший, отличный план, продумай всё.» · «идея про "scoreboard
> вместо принуждения": mission control может сохранять метаданные
> профилировки и ориентироваться по ним, и отдавать эти метаданные всем,
> кому нужно.»

### 3b. Programme map — owner phases → campaigns

The mandate's six product phases map onto a campaign chain (lineage law:
each campaign's deferrals seed the next mandate):

| Owner phase | Home |
|---|---|
| 1 — delegate to GLM (split: fire / collect) | **this plan**, Phases 2 + 3 |
| 2 — swarm of agents | **this plan**, Phase 4 |
| 3 — delegation rules (codex-first clean-room, improved) | **this plan**, Phase 5 |
| 4 — per-model playbooks, tariff hygiene, extensibility | **this plan**, Phases 0 (facts) + 5 |
| new — mission-control scheduler | **this plan**, Phases 1–4 (backbone) |
| 5 — initiative system (scoreboard-driven) | **Campaign 2**, seeded in §15 |
| 6 — RLM protocol | **Campaign 3**, seeded in §15 |

## 4. Target arithmetic

Baseline (2026-07-09, after the ignition bootstrap): packages in group
`org.vibevm.fractality` = **1** (spec-only; 0 crates, 0 binaries); policy
flow packages = **0**; boss-side boot snippets/skills = **0**; recorded E2E
proofs = **0**; mission-control = does not exist.

Exit state of this campaign:

- **1 code-bearing package** `fractality` v0.1.0 with exactly **5 crates**
  (`fractality-core`, `fractality-mission-control`, `fractality-mc-client`,
  `fractality-backend-claude-code`, `fractality-cli`) and **2 binaries**
  (`fractality`, `fractality-mission-control`).
- **1 policy flow package** `org.vibevm.fractality/delegation-rules` v0.1.0
  with ≥ 3 playbooks (`glm-5.2`, `glm-5-turbo`, `_template`).
- **1 boss boot snippet + ≥ 1 skill** in the fractality package (Phase 6).
- **4 recorded E2E proofs** (manual-tests style, outputs saved under
  `spec/manual-tests/`): single sync delegation; a 3-worker swarm; a
  recursive kill; a stats readout that reconciles with transcripts.
- Floor green (§11), host self-check green, **0 clean-room violations**
  (every studied source has license + study note in the inventory before
  any implementation that draws on it).

Reconciliation: crates/binaries rise in Phases 1–4; proofs in 2–4 and 6;
the policy package in 5; boss-side artifacts in 6. Nothing else moves.

## 5. Current-state facts (verified 2026-07-09; do not re-discover)

- Host tree `05d3b1c`, clean, both mirrors synced (GitVerse `origin`,
  GitHub `github`), root `Cargo.toml:7` has
  `exclude = ["packages", "vibedeps"]` — a package version dir is its own
  Cargo workspace (PROP-024 precedent: `rust-ai-native-lang/v0.7.0`).
- `claude` CLI **2.1.202** on PATH; `git` 2.52.0.windows.1; Windows 11 Pro
  for Workstations. Machine quirks (host `spec/boot/90-user.md`): editor-tool
  edits only; heredoc commits; Git Bash not WSL; UAC blocks test binaries
  named `*install*` — **never name a crate/binary/test `*install*`**.
- `~/.vibevm/zai.api.token` **exists** (existence verified; content never
  read).
- Host `/refs/` is gitignored wholesale (`.gitignore:37`) — reference clones
  live there uncommitted; the committed record is
  [`../refs/INVENTORY.md`](../refs/INVENTORY.md).
- Redbook boot-slot grid: 03,05,10,15,17,25,35,40,42,44,45,50,52,55,57,60,
  62,65,67,70 (+ 20/30 reserved); `wal-workspaces` took **11** in the
  bootstrap. The fractality boss snippet (Phase 6) will use **75** —
  outside the redbook grid, no collision.
- **NOT verified (authoring-model knowledge ends 2026-01):** the z.ai
  Anthropic-compatible base URL; exact model ids for GLM 5.2 / GLM-5-Turbo;
  which env vars Claude Code 2.1.x honors for model-slot mapping
  (`ANTHROPIC_MODEL` / `ANTHROPIC_SMALL_FAST_MODEL` vs newer
  `ANTHROPIC_DEFAULT_*_MODEL` names); whether a fresh `CLAUDE_CONFIG_DIR`
  works headless with env auth and no onboarding; z.ai plan quotas (the
  «4000 MCP calls/month» figure) and pricing; the «GLM is strongest at big
  one-shot tasks» claim. **All are Phase 0 targets; D6/D7/D12 carry best
  guesses and are rewritten in place by findings.**

## 6. Decisions

### D1 — package topology: one code package now, group designed for many
- (α) one crate, one binary — rejected: the owner explicitly wants library
  decomposition and mission-control as its own application.
- (β) two packages now (`fractality` + `mission-control`) — rejected for
  v0.1: the MC API is not yet stable enough to be a package boundary;
  cross-package path-deps break PROP-024 self-containment, and
  contract-first type duplication is dead weight at this stage.
- (γ) **CHOSEN:** one code-bearing package `fractality` v0.1.0 whose Cargo
  workspace holds 5 well-cut crates (§4); `mission-control` splits into its
  own package at the API-stability milestone (deferral DEF-4). Policy
  content ships as a separate sibling flow package from day one (Phase 5),
  because *its* boundary (prose vs code) is already stable. Consequence:
  registry repo names will read `org.vibevm.fractality.<name>` at publish
  (PROP-008 Fqdn) — accepted; publish is owner-gated anyway.

### D2 — delegation locus: process-level provider isolation
- (α) proxy/router that rewrites one session's model per request —
  rejected: opaque to ToS review, single point of failure, and it cannot
  give workers different tools/permissions/workspaces.
- (β) Claude Code native subagents — rejected: they cannot switch provider
  per agent (account-level auth), which is the entire point.
- (γ) **CHOSEN:** each worker is a separate headless Claude Code process
  (`claude -p`) with environment-injected provider config
  (`ANTHROPIC_BASE_URL`, auth token, model mapping) and its own
  `CLAUDE_CONFIG_DIR`. The documented enterprise-gateway surface — the
  paved road.

### D3 — spawner locus: mission-control owns the children
- (α) CLI spawns, MC only registers — rejected: kill-tree handles and pipe
  ownership scatter across short-lived CLI processes; adoption after a CLI
  death is impossible; federation dead-end.
- (β) **CHOSEN:** the CLI is a thin client; MC spawns every worker, owns
  its process group / Windows Job Object, captures stdout to the run dir,
  enforces budgets, performs kills (recursive by tree), adopts-or-reaps on
  restart via journal replay. CLI auto-starts the daemon when absent
  (lockfile probe). Phase 0 s5 validates the mechanism; if Job-Object
  inheritance under nested `claude → node → shell` trees fails, fallback is
  MC-side `taskkill /PID /T /F` (+ `sysinfo` sweep assert) — the decision
  point is *which mechanism*, not *which process owns it*.

### D4 — content channel: files on disk, run dirs under `~/.fractality`
Owner ruling (I2). Run dir: `~/.fractality/runs/<ulid>/` holding
`packet.toml`, `worker-stdout.jsonl` (raw stream-json), `result.md`
(worker-authored final report), `files/` (non-code artifacts),
`usage.json`, `status.json`. Code deliverables travel as **git branches in
worktrees** (D8), not file copies. MC state holds pointers only. Rejected:
per-project run dirs (scatters the journal; MC is machine-global), sockets
or stdout as content channels (owner ruling; transcripts are *records*, not
channels).

### D5 — worker environment: clean-slate whitelist (invariant I1)
Construct from scratch: minimal OS set (`PATH`, `HOME`/`USERPROFILE`,
`TEMP`/`TMP`, `SystemRoot`, `COMSPEC`, locale) + profile injections
(base URL, auth token read from `token_file` at spawn, model mapping,
`CLAUDE_CONFIG_DIR`) + fractality context (`FRACTALITY_RUN_ID`,
`FRACTALITY_DEPTH`). Explicitly **never** passed through: `ANTHROPIC_*`,
`CLAUDE_*`, `CLAUDECODE*` inherited values. A unit test constructs an env
from a poisoned parent and asserts the poison is gone; CI-grade, not
optional. Rejected: inherit-and-override — one forgotten variable silently
routes a swarm to the boss's subscription.

### D6 — profiles: `~/.fractality/profiles.toml` (+ per-project override)
```toml
schema = 1
[profile.glm]
backend = "claude-code"
base_url = "https://api.z.ai/api/anthropic"   # VERIFY Ф0.s3
token_file = "~/.vibevm/zai.api.token"
claude_binary = "claude"                       # or absolute path
config_dir = "auto"        # ~/.fractality/profiles/glm/claude-config
[profile.glm.models]
big = "glm-5.2"            # VERIFY Ф0.s3 exact ids
small = "glm-5-turbo"
haiku_slot = "glm-5-turbo" # CC-internal small-model traffic → cheap model
[profile.glm.limits]
max_concurrent = 4
[profile.glm.permissions]
mode = "accept-edits"      # allowlist posture; see RP4
deny_tools = ["WebFetch", "WebSearch"]   # tariff hygiene, D12
[profile.glm.pricing]      # metrics only; flat=true for subscription plans
flat = true
```
Token by *reference* (path), per host secrets-hygiene. Rejected: tokens or
provider secrets inside profiles; one global hardcoded provider.

### D7 — task packet: versioned TOML, the universal seam
```toml
schema = 1
[task]
title = "…"
goal = """full, self-contained task text (markdown)"""
acceptance = ["cargo test -p …"]        # commands run in the workspace
[context]
files = ["src/config.rs"]                # visible via worktree or copied
notes = "…"
[workspace]
mode = "worktree"                        # worktree | dir | none
repo = "."
base = "main"
[output]
result = "result.md"                     # contract: worker's final report
branch = "fractality/<run-id>"           # worktree mode deliverable
[budget]
wall_secs = 1800
max_turns = 40
max_output_tokens = 200000               # cumulative → kill on exceed
[routing]
profile = "glm"
model = "big"                            # slot name, resolved by profile
```
The packet, not the backend trait, is the future-proof seam (Codex or
VibeVM Pixel consume packets unchanged). Golden serde tests pin `schema=1`.
Rejected: JSON (house style is TOML; comments matter for humans), CLI-flag
soup (unreviewable, unrecordable).

### D8 — worker workspace: git worktree by default
`worktree` mode: MC runs `git worktree add <run-dir>/wt -b fractality/<id>
<base>` in the target repo; the worker works there; the branch is the
deliverable; boss reviews/merges; MC removes the worktree on collect (keeps
it on failure for autopsy; `fractality gc` reaps — deferral DEF-7 for
richer policies). `dir` mode for analysis tasks (scratch dir, `files/` out).
Rejected: workers editing the live checkout (parallel writes corrupt), full
repo copies (slow, fat).

### D9 — mission-control storage: append-only JSONL journal
`~/.fractality/journal/events.jsonl` (rotated by size): every event —
`registered`, `spawned`, `state`, `usage`, `completed`, `killed`, `error` —
one line, serde-typed. In-memory state = replay at startup (adopt-or-reap
for runs whose PIDs died). This journal **is** the profiling-metadata store
of I3 (owner addendum); `GET /v0/metrics` aggregates from it. Rejected:
SQLite (dep + migrations before the schema has settled), in-memory-only
(loses the telemetry that the whole initiative system feeds on).

### D10 — mission-control API: versioned localhost HTTP
Bind `127.0.0.1:0` (ephemeral); lockfile `~/.fractality/mc.lock` = `{port,
pid, bearer}` (0600); bearer required on every call (defense against other
local users). Endpoints v0: `GET /v0/health` · `POST /v0/runs` (spawn from
packet) · `GET /v0/runs` (filters) · `GET /v0/runs/:id` ·
`GET /v0/runs/:id/tree` · `POST /v0/runs/:id/kill` (`recursive` flag) ·
`GET /v0/metrics` (aggregates by profile/model/day). Sync `run` = spawn +
long-poll (SSE is DEF-6). Path-versioned (`/v0/`) so federation-era changes
don't strand old CLIs. Rejected: gRPC (heavy for v0.1), Unix sockets /
named pipes (two transport stacks; HTTP is uniform cross-platform).

### D11 — stack (pin exact versions in Ф0.s8)
`tokio` + `axum` (server), `reqwest`/rustls (client), `serde` + `toml` +
`serde_json`, `clap` (derive), `tracing` + `tracing-subscriber`, `ulid`,
`camino` (UTF-8 paths), `thiserror`, `insta` (goldens), process-group
control: candidate crates `command_group` / `win32job` / `sysinfo` — the
Ф0.s5 spike picks. Rejected: actix (no advantage here), async-std (ecosystem
gravity is tokio), heavyweight config frameworks (plain serde+toml).

### D12 — tariff hygiene is mechanism, not prose
Workers get `WebFetch`/`WebSearch` (and web MCP tools) denied via profile;
`fractality fetch <url> --out <path>` (plain reqwest, boss- and
human-callable) downloads documents once into the workspace; MC counts
web-ish tool events from transcripts into a monthly quota metric (the
«4000 MCP calls» budget — exact quota VERIFY Ф0.s3). The Phase 5 playbooks
then *describe* what the mechanism already *enforces*.

### D13 — CLI verb set (v0.1)
`fractality mc start|stop|status` · `run --packet <file>` (sync; exit code
mirrors worker outcome; prints run-dir path + one-screen summary) · `spawn`
(async) · `await <id>` · `ls` · `show <id>` · `tree [<id>]` ·
`kill <id> [--tree]` · `stats` · `fetch <url> --out <path>` ·
`packet new [--template <name>]`. Human-drivable first: every flow
debuggable from a terminal without any boss involved.

### D14 — error surface and testing
One `thiserror` enum per crate layer; messages carry the violated invariant
(I-number or spec anchor) and a fix surface. Tests: unit (env constructor
I1, packet schema goldens, journal replay), fixture goldens for stream-json
parsing (recorded in Ф0/Ф2 from live runs, then frozen), E2E as
**manual-tests** (house flow): networked/paid procedures documented with
recorded outputs under `spec/manual-tests/`, never wired into CI. The
stream-json parser is tolerant: unknown event kinds are preserved as
`Other` and logged, never fatal (VERIFY drift risk R2).

### D15 — discipline level for this workspace
Floor-lite (fmt + clippy `-D warnings` + tests) as the campaign gate;
full AI-Native machinery (conform/specmap/tcg) is DEF-9, adopted when the
tree is worth gating. Production-grade bar still applies: no stub
subcommands in the shipped surface, no skipped edge cases justified by
scope (owner's standing quality directive, 2026-07-07).

### D16 — telemetry consumers read MC, full stop (I3)
`fractality stats` (Phase 6) is a thin client over `GET /v0/metrics`; the
Campaign-2 initiative system, future GUIs, and meta-cognition tooling read
the same API. No shadow accounting anywhere — including in the boss's own
notes. Owner addendum, 2026-07-09, verbatim in §3.

## 7. Predictions (checked one by one at close)

- **P1** — nested headless spawn works on this box: a `claude -p` child
  with a clean-slate env completes under a parent Claude Code session
  without console/conpty surprises. (Falsifier: hangs or auth failures
  traceable to env construction.)
- **P2** — GLM-served Claude Code emits stream-json complete enough for
  metering: per-message `usage` fields present and a final `result` event
  carries the final text. (Falsifier: missing usage → metering degrades to
  event counts; D14 fallback engages.)
- **P3** — the result-file contract holds: ≥ 9 of the first 10 real packets
  produce a non-empty `result.md` without a follow-up nudge turn.
  (Falsifier: workers ignore the output contract → Phase 3 adds a finalizer
  turn to the backend.)
- **P4** — a 3-worker swarm on disjoint modules completes with zero
  worktree conflicts and wall-clock < 1.6× the slowest single run.
- **P5** — recursive kill terminates the full worker tree (`claude` +
  `node` + shells) in < 2 s with zero orphans, verified by process sweep,
  on Windows.
- **P6** — with only Phase 6's boot snippet + skill (no initiative system
  yet), a live boss session delegates ≥ 50% of eligible grunt tasks in the
  dogfood exercise. This is the *baseline* Campaign 2 must beat; recording
  it honestly matters more than it being high.
- **P7** — the campaign lands in ≤ 14 commits matching the planned
  subjects (drift recorded in the ledger, not silently absorbed).

## 8. Phases

### Phase 0 — spikes and probes (no commits; findings rewrite Decisions)

1. **s1 env sanity:** `claude --version`; `test -f ~/.vibevm/zai.api.token`.
   (Both already verified at authoring; re-confirm at execution.)
2. **s2 nested-spawn spike:** in a scratch dir, run
   `claude -p "Reply with exactly: OK" --output-format json` (a) with
   inherited env — baseline; (b) with a hand-built clean-slate env — the
   D5 whitelist, still on the default account (one cheap prompt, deliberate
   variable isolation: nested-spawn vs provider config).
3. **s3 provider facts:** download (never web-read from workers; boss-side
   fetch is fine here) the z.ai Claude Code integration docs and pricing
   pages plus the Anthropic CC docs for env/model config and headless
   flags, into `/refs/src/zai-docs/` and `/refs/src/cc-docs/`; extract:
   base URL, model ids, env var names honored by CC 2.1.x, onboarding
   behavior of a fresh `CLAUDE_CONFIG_DIR`, plan quotas (the 4000-MCP
   figure), prices, and any documented one-shot-strength guidance for GLM.
   **Rewrite D6/D7/D12 in place**; note findings under this phase.
4. **s4 GLM smoke + stream-json fixtures:** repeat s2(b) with the GLM env
   (base URL + token file + model mapping): expect a GLM-authored "OK".
   Then one small real task with `--output-format stream-json` captured to
   a fixture file — the golden corpus for the Phase 3 parser. Early P2
   check.
5. **s5 kill-tree spike:** scratch Rust bin (uncommitted) spawns a
   long-running `claude -p`, enumerates the child tree, kills via the D11
   candidate crates; pick the mechanism (Job Objects vs taskkill-sweep);
   assert no orphans via `sysinfo`. **Rewrite D3/D11 in place.**
6. **s6 refs intake + licenses:** shallow-clone the three repos
   (steipete/agent-scripts; barkain/claude-code-workflow-orchestration;
   alexzhang13/rlm) into `/refs/src/`, download arXiv 2512.24601 into
   `/refs/papers/`; record commit pins + LICENSE verdicts in the
   inventory. Write the **codex-first study note** (decisions-only, no
   text reuse) — Phase 5's prerequisite. Deep study of barkain/rlm is
   Campaigns 2/3, not now.
7. **s7 landscape one-pager:** shallow comparative scan (claude-code-router,
   claude_swarm, claude-squad, and whatever s3 surfaces) →
   `spec/refs/notes/landscape.md`: what exists, what fractality does
   differently (scheduler + provider isolation + files + metering).
8. **s8 crate pins:** resolve current versions for the D11 set; record.
9. **s9 host-gate probe:** run host `bash tools/self-check.sh` and confirm
   it ignores package workspaces (expected per PROP-024); record runtime.

*Exit:* findings list appended below this phase; every VERIFY in §5/§6
resolved or explicitly re-scoped; affected Decisions rewritten in place.
*Prediction:* P1 and the P2 early check pass in spikes.
*Commits:* exactly one — `docs(fractality): plan v0.1 amended with Phase 0
findings` — which also flips the status line to `EXECUTING`.

### Phase 1 — workspace skeleton + mission-control core

1. Create the Cargo workspace in `fractality/v0.1.0/` with the five D1
   crates; wire the floor; add `spec/examples/hello-glm.toml` (packet).
2. `fractality-core`: ULID run ids, `RunState` machine, packet schema
   (serde + goldens), journal event types, API DTOs, `WorkerBackend` trait.
3. `fractality-mission-control`: journal append + replay (adopt-or-reap),
   run registry, lockfile + bearer, `GET /v0/health`, run CRUD minus spawn,
   graceful shutdown.
4. `fractality-cli` + `fractality-mc-client`: `mc start|stop|status`,
   auto-start on first client call, `ls`, `show`.

*Exit:* floor green; `mc start` → `status` healthy → `stop` clean on this
box; journal survives a kill-and-restart cycle with state intact.
*Prediction:* journal replay covers restart with zero manual repair.
*Commits:* `feat(fractality): cargo workspace + core model` ·
`feat(fractality): mission-control — journal, registry, lifecycle`.

### Phase 2 — delegate-out (fire a worker, results land on disk)

1. Profile loading (D6) + validation; `config_dir = "auto"` provisioning
   (seed per s3 findings so headless works on first spawn).
2. The D5 env constructor **with the poisoned-parent test**.
3. Worktree manager (D8) via git CLI.
4. Backend `claude-code`: build headless invocation (prompt from packet
   goal + output contract preamble; flags per s3: output format, max
   turns, permission posture, tool denies).
5. MC spawn path: `POST /v0/runs` accepts a packet, spawns owned child,
   streams stdout to `worker-stdout.jsonl`, tracks state; `fractality run`
   drives it sync end-to-end (spawn → wait → exit code), collection still
   minimal (exit code + transcript on disk).

*Exit:* E2E — a GLM worker executes a trivial file-producing packet; run
dir holds packet + transcript; run visible in `ls`/`show`; **the
poisoned-env test is green**.
*Prediction:* P2 confirmed on real transcripts (usage fields present).
*Commits:* `feat(fractality): profiles + clean-slate worker env` ·
`feat(fractality): spawn path — mc-owned workers and worktrees`.

### Phase 3 — collect-back (returns as first-class files)

1. Stream-json incremental parser (fixture-driven, tolerant per D14):
   state transitions, usage accumulation, final-result extraction.
2. Collection: `result.md` (worker-written per contract; fall back to
   final-message extraction when absent — and record which happened),
   `usage.json`, `status.json`; acceptance commands from the packet run in
   the workspace, pass/fail recorded.
3. `fractality run` prints the one-screen summary (state, wall time,
   tokens, result path, branch, acceptance verdicts); `show` renders any
   historical run the same way.

*Exit:* E2E — packet "implement a small Rust function + test in a scratch
repo", GLM-5.2 worker, acceptance `cargo test` green, boss-side artifact =
`result.md` + branch only. Recorded as manual-test #1.
*Prediction:* P3 measured over the first 10 packets (record the count).
*Commits:* `feat(fractality): result collection, metering, sync run` ·
`test(fractality): stream-json goldens from live fixtures`.

### Phase 4 — swarm (many workers, budgets, the tree, the kill)

1. Async verbs: `spawn`, `await`, `tree`; per-profile `max_concurrent`
   admission; queueing.
2. Budget enforcement in MC: wall clock watchdog, `--max-turns`
   passthrough, cumulative token cap → kill with `killed(budget)` state.
3. Nesting: `FRACTALITY_RUN_ID` / `FRACTALITY_DEPTH` in worker env; a
   worker calling `fractality spawn` registers a **child** run — the tree
   becomes real (fractal property demonstrated with a depth-2 run).
4. `kill --tree` per the s5 mechanism; orphan sweep assertion.
5. `GET /v0/metrics` v0 aggregates (runs by state/profile/model/day;
   tokens; wall time; web-tool-quota counter per D12).

*Exit:* manual-tests #2 (3-worker swarm on disjoint modules) and #3
(recursive kill of a depth-2 tree) recorded with outputs.
*Prediction:* P4 and P5, measured.
*Commits:* `feat(fractality): swarm — async lifecycle, budgets, kill-tree`
· `feat(fractality): mission-control call tree + metrics`.

### Phase 5 — the policy layer (delegation-rules package)

1. From the s6 study note (never from the source text): author
   `packages/org.vibevm.fractality/delegation-rules/v0.1.0/` — kind=flow,
   UPL-1.0, boot snippet slot **77**.
2. Core doc `DECISION-MATRIX.md`: the delegation calculus — **delegate
   when verification is cheaper than generation**; axes: task size ×
   error cost × context transferability × verifiability; hard
   never-delegate set (secrets, destructive ops, owner-court decisions,
   tasks whose context cannot be packaged); sizing guidance (one-shot
   coarse-grained for big-model workers); the boss-as-reviewer loop
   (acceptance criteria travel IN the packet). Deliberately broader and
   sharper than the studied skill — the improvement the mandate asks for.
3. `playbooks/glm-5.2.md`, `playbooks/glm-5-turbo.md`, `playbooks/_template.md`:
   per-model cards — strengths, task shapes, budget defaults, tariff
   rules (facts from s3, mechanisms from D12), routing hints
   (`model = "big"` vs `"small"`, `haiku_slot`). The template is the
   extension surface the mandate requires (future: `codex.md`).

*Exit:* package parses (host manifest check), matrix + 3 playbooks
complete, every claim either s3-verified or marked as a measurable
hypothesis; inventory marks codex-first as "studied, note on file,
implemented clean".
*Prediction:* the matrix yields a **decidable** verdict (no judgment call
needed) for ≥ 8 of 10 randomly drawn recent host-repo tasks — else it is
prose, not policy; rewrite until it decides.
*Commits:* `feat(fractality): delegation-rules — matrix + model playbooks`.

### Phase 6 — boss integration + the scoreboard v0

1. Boot snippet `spec/boot/75-tool-fractality.md` in the fractality
   package: the trigger table (when a task smells delegable → consult the
   matrix → `fractality run/spawn`), the never-delegate set, the
   one-command examples. Add `boot_snippet` to `vibe.toml`.
2. Skill `spec/skills/fractality-delegate/SKILL.md`: guided packet
   authoring → run → collect → verify loop, swarm variant included.
3. `fractality stats`: thin client over `/v0/metrics` (D16) — tokens by
   model/profile, delegated-run counts, quota counters, boss-minutes saved
   estimate (declared as an estimate).
4. Dogfood: one real grunt task from the host repo executed through the
   swarm end-to-end (RP1 names the candidate; owner picks), recorded as
   manual-test #4 and as the P6 baseline measurement.

*Exit:* a live boss session (this harness) delegates the dogfood task via
the skill without hand-holding; stats reconcile with the run transcripts.
*Prediction:* P6, measured and recorded (the honest baseline).
*Commits:* `feat(fractality): boss boot snippet + delegation skill` ·
`feat(fractality): stats — the mission-control scoreboard`.

### Close

Run §12 acceptance on a green floor; prepend the §2 execution record;
verdict every prediction; update the workspace WAL/CONTINUE and the host
`WORKSPACES.md` line; seed Campaigns 2–3 from §15.

## 9. Risks and fallbacks

- **R1 provider-facts drift** (cutoff-stale guesses). *Detect:* s3/s4.
  *Fallback:* D6/D7/D12 rewritten before any code depends on them; profile
  schema keeps provider specifics in data, not code.
- **R2 stream-json schema drift across CC versions.** *Detect:* golden
  fixtures break on a CC upgrade. *Fallback:* tolerant parser (D14),
  `claude --version` recorded per run in `status.json`, known-good range
  documented.
- **R3 Windows kill-tree gaps** (Job Objects vs conpty vs node children).
  *Detect:* s5 orphan sweep. *Fallback:* taskkill /T /F + sysinfo sweep;
  worst case, kill is best-effort plus a loud orphan report — never
  silent.
- **R4 GLM quality below the delegation threshold.** *Detect:* Phase 3/6
  rework counts (boss redoes the task). *Fallback:* playbooks narrow GLM
  to task shapes it wins; the fabric is model-agnostic — swap backends,
  the product survives.
- **R5 fresh CLAUDE_CONFIG_DIR onboarding blocks headless.** *Detect:*
  s2/s4. *Fallback:* seed the config dir (settings.json) per s3 findings;
  documented in the backend.
- **R6 MC crash orphans workers.** *Detect:* journal replay finds
  running-state runs with dead/alive PIDs. *Fallback:* adopt-or-reap at
  startup (D9), designed in Phase 1, tested by kill-and-restart.
- **R7 z.ai rate limits mid-swarm.** *Detect:* 429s in transcripts.
  *Fallback:* per-profile concurrency caps + retry-with-backoff at spawn
  admission only (never mid-task); 429 counts surface in metrics.
- **R8 permission-bypass hazard on autonomous workers.** *Mitigation:*
  default allowlist posture (RP4), worktree isolation, web tools denied;
  `yolo` mode exists only per-profile, opt-in, worktree-only.

## 10. Non-goals (named, with disposition)

- Initiative system — **Campaign 2** (seeded §15). RLM — **Campaign 3**.
- MCP surface for the boss — deferred (DEF-6 family); Bash + skill suffice
  to prove the loop.
- mission-control package split, GUI, federation, remote MC — horizons
  (PROP-001 §7); designed-for, not built.
- Publish to the registry — owner-word-only (standing policy).
- macOS/Linux *validation* — the code is written portable (camino, no
  path/encoding assumptions, cfg-gated process control), but this campaign
  validates on Windows only; CI matrix is future work (DEF-8).
- Codex/other backends — the trait exists; no second implementation until
  the first one earns it.
- vibe-native dispatch (`vibe bin exec`) — later; plain cargo builds now.

## 11. Quick-start for the executing session

```sh
cd packages/org.vibevm.fractality
git log --oneline -5          # expect the ignition bootstrap commits
head -20 WAL.md               # PLANNED/EXECUTING + next phase pointer
# Floor, from Phase 1 on (before crates exist: host self-check instead):
cd fractality/v0.1.0
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

## 12. Whole-campaign acceptance

```sh
cd packages/org.vibevm.fractality/fractality/v0.1.0
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace   # exit 0
fractality mc start && fractality mc status                          # healthy
fractality run --packet spec/examples/hello-glm.toml                 # exit 0
test -s ~/.fractality/runs/<that-run>/result.md                      # non-empty result
fractality stats                                                     # ≥4 completed runs, ≥1 swarm parent w/ 3 children
ls spec/manual-tests/                                                # 4 recorded procedures with outputs
(cd ../../../.. && bash tools/self-check.sh); echo "EXIT=$?"          # host floor green
```

## 13. Review points

1. **RP1 — dogfood target for Phase 6.** OPEN. Candidate: the host's
   license-straggler alignment (old fixture manifests `EULA` → `UPL-1.0`) —
   real, boring, parallelizable, host-visible. Executor recommends it;
   needs the owner's word because workers would edit host-repo files (in
   worktrees, boss-reviewed).
2. **RP2 — does `wal-workspaces` join redbook edition 0.3.0?** OPEN, owner
   court; no phase depends on it.
3. **RP3 — publish timing** for anything in `org.vibevm.fractality`. OPEN,
   standing hold; §10.
4. **RP4 — default worker permission posture.** OPEN. Executor recommends:
   allowlist/accept-edits default; `--dangerously-skip-permissions`-class
   modes only in explicitly named `yolo` profiles restricted to worktree
   workspaces. Ratify or amend; D6 carries the current draft.

## 14. Execution ledger

_(Filled at each phase boundary: `EXECUTED <date>` + commit map — hash,
planned subject, what it confirmed or falsified. Empty at authoring.)_

## 15. Deferrals ledger (seeds the next campaigns)

- **DEF-1 — CAMPAIGN 2: the initiative system.** Mandate seed (owner,
  verbatim): «если мы не сделаем какую-то систему инициативы для Opus, он
  просто не будет использовать спавн наших агентов» + the scoreboard
  addendum (§3). Shape: agent-neutral policy core fed by MC metrics (I3);
  per-harness adapters at the edges (Claude Code hooks first — hook target
  is a fractality subcommand, never a script zoo); scoreboard-first
  (visible wins), contextual injection by threshold, P6 baseline to beat.
  Prerequisite study: barkain clean-room note (repo already in refs).
- **DEF-2 — CAMPAIGN 3: the RLM protocol.** Paper + reference impl in
  refs; core hypothesis to test: *Claude Code already is the REPL* — RLM
  here = a context-store layout + recursion discipline (depth/budget via
  `FRACTALITY_DEPTH` + MC call tree) + an `rlm` packet template, with an
  embedded-scripting variant (Rhai/Lua) as the ADR alternative. Trigger
  rule: only when the task's context shape warrants it (owner: «только
  тогда, когда задача тому способствует»).
- **DEF-3 — publish** anything (owner-gated; includes wal 0.2.0 ordering
  constraint noted in wal-workspaces' manifest).
- **DEF-4 — mission-control package split** at API stability (D1).
- **DEF-5 — `vibe bin exec` / lockfile dispatch integration.**
- **DEF-6 — async richness:** SSE event stream, await-any/all, MCP surface
  for the boss.
- **DEF-7 — worktree GC policies** (`fractality gc`, failure autopsies).
- **DEF-8 — CI matrix** (Linux/macOS validation of the portable design).
- **DEF-9 — full AI-Native discipline adoption** for this workspace
  (conform/specmap/tcg) once the tree is worth gating.
- **DEF-10 — token-file permissions hardening** (owner machine policy:
  0600-equivalent ACL on `~/.vibevm/*.token`, MC lockfile ACL audit).
