# FRACTALITY-IGNITION-PLAN v0.1 — from zero to a metered GLM swarm under mission-control

_Status: **EXECUTING** (Phase 0 landed 2026-07-09, floor green, next:
Phase 1 — workspace skeleton + mission-control core) · written 2026-07-09
against host tree `05d3b1c` plus the same-day ignition bootstrap commits ·
cold-executable: any phase boundary is a safe stop; the fractality floor
(§11) is green at every boundary. Format: `flow:org.vibevm/campaign-plans`
(one file, five roles)._

_ACCEPTED with owner amendments, 2026-07-09 (same day): supervision
topology is now MC → **pod** → worker (D3 rewritten; sixth crate;
Phase 4b inserted; §4 arithmetic updated); a UNIX-ergonomics law (D17) and
the non-yolo interaction stack (D18) added; RP1, RP2, and RP4 RESOLVED
(RP4: yolo de-scoped from v0.1 — the D18 stack is the way of life); DEF-2
carries the owner's RLM hypothesis; DEF-11 and DEF-12 (the Entire.io-like
checkpoints horizon) added; I2 re-scoped — mission-control is the command
bus, files are the persistence plane, never the medium (D4/D10/D18
aligned); D19 added — claim-check FileRefs + beacon-proven filesystem
identity + node identity as the bulk-data law; the interim opencode+GLM
paradigm recorded in the workspace contract and verified live. Only RP3
(publish) remains open; Phase 0 is fully unblocked._

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

Clarifications (owner, 2026-07-09, second message; operative fragments):

> «Кажется, в RLE основная фишка в том, что каждый раз у нас новый свежий
> контекст и это означает спавн нового агента с чистого листа. Но это моё
> поверхностное представление, это надо проверять.» [→ DEF-2]
> «То что мы делаем в mission-control — подумай сразу, чтобы этим было
> удобно пользоваться, эргономично. Например, как удобно пользоваться
> командной строкой Linux и утилитами GNU — что агентам, что людям. […]
> позаимствовать у них часть UNIX вайба.» [→ D17]
> «Ты можешь уже сейчас экономить контекст с помощью GLM-5.2, у тебя в
> PATH есть opencode […] всякий шлак типа рефакторингов отправлять в GLM
> […] Ты можешь эту парадигму где-нибудь зафиксировать, например, в
> Claude.md в текущем Workspace и пользоваться ей.» [→ workspace contract]
> «Да, в качестве догфуд задачи можно переколбасить все наши EULA на
> UPL-1.0 […] если что-то меняется в самом корневом VibeVM, верифицируй
> это, на случай если воркер сойдёт с ума. Должна быть хотя бы минимальная
> приемка.» [→ RP1, Phase 6]
> «wal-workspaces можно включить в redbook» [→ RP2, DEF-11]
> «я не понимаю, как мы будем делегировать в неинтерактивные задачи
> воркеры в любом режиме кроме как yolo? можем ли мы сделать какой-то
> протокол обмена ответами не останавливая запущенный воркер […] Агент у
> тебя спавнится не напрямую, а вначале стартует отдельное приложение —
> "под", и этот под хранит на себе управление вещами типа
> stdout/stderr/stdin. Именно pod общается с mission control. […] потоки
> ввода-вывода очень хрупкие, и они совершенно точно не работают сквозь
> сетевые соединения […] Плюс […] мы можем работать с зависшими агентами,
> не убивая процесса на уровне mission control — это такие безопасные
> контейнеры, которые если что могут перезапустить нативно упавшего
> агента, поменять ему параметры или ещё что-то.» [→ D3, D18, Phase 4b]

Refinement (owner, 2026-07-09, third message):

> «Если у нас босс общается с агентом (через прослойку из mission control
> и pod) я хочу чтобы весь командный интерфейс проходил через mission
> control. Файлы — это просто форма гарантированного, ультимативного
> персистенса, потенциально распределённого между нодами (если положить
> на nfs или ceph), но это не средство коммуникации.»
> [→ I2 re-scoped; D4/D10/D18 aligned]

Optimization directive (owner, 2026-07-09, fourth message):

> «чтобы не передавать какие-то большие данные, мы можем как способ
> оптимизации сохранять временный файл, а через командный интерфейс
> передавать не весь результат текстом, а только путь до файла и
> смещение/длину (или смещение с начала и с конца для супербольших
> данных непонятной длины) […] только если оба агента которые общаются,
> или агент и босс, используют одну и ту же файловую систему. Поэтому
> наверное нужно везде выставить супер важный параметр — у агента должно
> быть каким-то образом возможно узнать на каком компьютере он работает
> сам, и откуда взялась его файловая система […] для NFS IP тоже имеет
> смысл — если разные ноды подключились к одному и тому же NAS у них
> будут разные свои IP но один и тот же IP файловой системы. Хотя
> возможно, ты придумаешь какие-то более крутые средства идентификации.»
> [→ D19, I7]

Follow-up (owner, 2026-07-09, fifth message):

> «для файловых операций я бы посмотрел как сделано чтение и адресация
> больших файлов у Amazon S3 например, и вдохновлялся этим подходом.
> Возможно, нам нужно нечто более мощное, чем просто files?path&range»
> [→ D19 upgraded: RFC 7233 ranges, ETag/If-Match, presigned refs]

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

- **1 code-bearing package** `fractality` v0.1.0 with exactly **6 crates**
  (`fractality-core`, `fractality-mission-control`, `fractality-pod`,
  `fractality-mc-client`, `fractality-backend-claude-code`,
  `fractality-cli`) and **3 binaries** (`fractality`,
  `fractality-mission-control`, `fractality-pod`).
- **1 policy flow package** `org.vibevm.fractality/delegation-rules` v0.1.0
  with ≥ 3 playbooks (`glm-5.2`, `glm-5-turbo`, `_template`).
- **1 boss boot snippet + ≥ 1 skill** in the fractality package (Phase 6).
- **5 recorded E2E proofs** (manual-tests style, outputs saved under
  `spec/manual-tests/`): single sync delegation; a 3-worker swarm; a
  recursive kill; a permission/question round-trip on a live parked
  worker; the dogfood relicensing run with a stats readout that
  reconciles with transcripts.
- Floor green (§11), host self-check green, **0 clean-room violations**
  (every studied source has license + study note in the inventory before
  any implementation that draws on it).

Reconciliation: crates/binaries rise in Phases 1–4b; proofs in 2–4b and
6; the policy package in 5; boss-side artifacts in 6. Nothing else moves.

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
- `opencode` **1.17.14** on PATH with the owner's z.ai credentials in its
  auth store (`Z.AI` + `Z.AI Coding Plan` entries). **Verified live
  2026-07-09:** `opencode run -m zai-coding-plan/glm-5.2 "Reply with
  exactly: OK"` → `OK`. The catalog also carries
  `zai-coding-plan/glm-5-turbo`. The `opencode/*` (Zen gateway) route is
  blocked — no payment method; the default model (lmstudio, local) is
  down unless LM Studio runs. **Use `zai-coding-plan/*` only.** This
  confirms `glm-5.2` / `glm-5-turbo` as live z.ai catalog names; the
  CC-side env mapping still needs Ф0.s3.
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
  workspace holds 6 well-cut crates (§4, incl. `fractality-pod` — the
  per-worker supervisor, D3); `mission-control` splits into its
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

### D3 — supervision topology: mission-control → pod → worker
- (α) CLI spawns, MC only registers — rejected: kill-tree handles and pipe
  ownership scatter across short-lived CLI processes; adoption after a CLI
  death is impossible; federation dead-end.
- (β) MC spawns workers directly — the plan's original choice, **rejected
  by owner amendment (2026-07-09)**: stdio is process-local and fragile,
  while every other leg (boss↔MC, MC↔runner) is message-shaped HTTP that
  must survive a network hop someday; direct spawn also couples MC
  restarts to worker lifetimes and pushes per-platform process mess into
  the daemon.
- (γ) **CHOSEN (owner amendment):** MC spawns a **pod**
  (`fractality-pod`, one per run); the pod spawns the worker and owns what
  only a local parent can own — the child's stdin/stdout/stderr (streamed
  to run-dir files), the Job Object / process group, budget watchdogs,
  kill, and later native restart of a crashed worker with amended
  parameters. The pod speaks to MC over the same localhost HTTP (register,
  heartbeat, events, exit report), so an MC restart strands nobody: pods
  keep supervising and re-register on reconnect — adoption is a protocol
  feature, not journal archaeology. This is the containerd-shim /
  systemd-per-service shape, and it is the federation seam: tomorrow MC is
  remote and pods are the machine-local runners, zero rearchitecture. The
  CLI stays a thin MC client; MC auto-starts on first call (lockfile
  probe). Cost accepted: one more binary and one protocol leg — bought
  back by simpler adoption (R6 shrinks) and a platform-clean daemon.
  Phase 0 s5 validates the pod-side kill mechanism; fallback stays
  `taskkill /PID /T /F` + `sysinfo` sweep.

### D4 — two planes: mission-control is the bus, files are persistence
Owner rulings (I2, both 2026-07-09): results are *delivered* as files,
but files are **not the communication medium** — «весь командный
интерфейс проходит через mission control»; the run dir is the
guaranteed, ultimate persistence of what flowed (federation era: shared
storage such as NFS/Ceph). Run dir: `~/.fractality/runs/<ulid>/` holding
`packet.toml`, `worker-stdout.jsonl` (raw stream-json), `result.md`
(worker-authored final report), `files/` (non-code artifacts),
`usage.json`, `status.json`, `question.md`/`answer.md` (D18). Code
deliverables travel as **git branches in worktrees** (D8). Every CLI
verb resolves through MC's API — summaries, questions, answers, and
result metadata ride the bus inline; bulk artifacts are referenced by
path (same box in v0.1; a file-serving endpoint is the named federation
extension under DEF-6). Rejected: files-as-mailbox polling (fragile,
slow, unobservable — the bus carries events), per-project run dirs
(scatters the journal; MC is machine-global), sockets or stdout as
content channels (transcripts are *records*, not channels).

### D5 — worker environment: clean-slate whitelist (invariant I1)
Construct from scratch: minimal OS set — **Ф0.s2-verified for Windows:**
`PATH`, `HOME`/`USERPROFILE`, `TEMP`/`TMP`, `SystemRoot`, `COMSPEC`,
**`APPDATA`, `LOCALAPPDATA`** (CC needs the last two on Windows; omitting
them was the one gap the spike surfaced), locale — plus profile
injections (base URL, auth token read from `token_file` at spawn, the
`ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU}_MODEL` mapping, any `profile.env`
extras, `CLAUDE_CONFIG_DIR`) + fractality context (`FRACTALITY_RUN_ID`,
`FRACTALITY_DEPTH`, `FRACTALITY_NODE_ID`). Explicitly **never** passed
through: `ANTHROPIC_*`, `CLAUDE_*`, `CLAUDECODE*` inherited values. A unit
test constructs an env from a poisoned parent and asserts the poison is
gone; CI-grade, not optional. Rejected: inherit-and-override — one
forgotten variable silently routes a swarm to the boss's subscription.

### D6 — profiles: `~/.fractality/profiles.toml` (+ per-project override)
All VERIFY items resolved by Ф0.s3/s4 (see the Phase 0 findings block):
```toml
schema = 1
[profile.glm]
backend = "claude-code"
base_url = "https://api.z.ai/api/anthropic"   # ✅ confirmed (z.ai docs + live smoke)
token_file = "~/.vibevm/zai.api.token"
claude_binary = "claude"                       # or absolute path
config_dir = "auto"        # fresh CLAUDE_CONFIG_DIR; ✅ headless onboarding needs no TTY
[profile.glm.models]
# ✅ mapping is via ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU}_MODEL for this provider,
# NOT the legacy ANTHROPIC_MODEL / ANTHROPIC_SMALL_FAST_MODEL pair.
big = "glm-5.2[1m]"        # ✅ the [1m] suffix selects the 1M-context variant
small = "glm-5-turbo"
haiku_slot = "glm-5-turbo" # CC-internal small-model traffic → cheap model
[profile.glm.env]          # extra provider env, injected verbatim (D5)
API_TIMEOUT_MS = "3000000"                     # z.ai-recommended for long turns
CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1" # cut telemetry chatter to the gateway
[profile.glm.limits]
max_concurrent = 4         # ≤ tier's 5-hour prompt budget; see D12
[profile.glm.permissions]
mode = "acceptEdits"       # ✅ exact CC value (not "accept-edits"); allowlist posture, RP4
deny_tools = ["WebFetch", "WebSearch"]   # tariff hygiene, D12
[profile.glm.pricing]      # metrics only; flat=true for subscription plans
flat = true
plan = "max"               # informs the quota metric (D12): 1600 prompts/5h, 4000 MCP/mo
```
Token by *reference* (path), per host secrets-hygiene. Rejected: tokens or
provider secrets inside profiles; one global hardcoded provider; the
legacy single-model env vars (wrong surface for this provider).

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
one line, serde-typed. In-memory state = replay at startup, reconciled
against live pod re-registrations (D3): a run whose pod returns is adopted
live; a run whose pod is gone is reaped from journal evidence. This
journal **is** the profiling-metadata store
of I3 (owner addendum); `GET /v0/metrics` aggregates from it. Rejected:
SQLite (dep + migrations before the schema has settled), in-memory-only
(loses the telemetry that the whole initiative system feeds on).

### D10 — mission-control API: versioned localhost HTTP
Bind `127.0.0.1:0` (ephemeral); lockfile `~/.fractality/mc.lock` = `{port,
pid, bearer}` (0600); bearer required on every call (defense against other
local users). Endpoints v0: `GET /v0/health` · `GET /v0/node` (node identity +
attached fs scopes, D19) · `POST /v0/runs` (spawn from
packet) · `GET /v0/runs` (filters) · `GET /v0/runs/:id` ·
`GET /v0/runs/:id/tree` · `POST /v0/runs/:id/kill` (`recursive` flag) ·
`GET /v0/runs/:id/question` · `POST /v0/runs/:id/answer` (D18) ·
`GET /v0/runs/:id/result` (I2: read verbs resolve through the bus) ·
`GET /v0/metrics` (aggregates by profile/model/day). Pod leg:
`POST /v0/pods/register` · `POST /v0/pods/:id/heartbeat` ·
`POST /v0/pods/:id/event` (state/usage/question/exit). Run states gain
`waiting_on_boss`. Sync `run` = spawn + long-poll (SSE is DEF-6).
Path-versioned (`/v0/`) so federation-era changes
don't strand old CLIs. Rejected: gRPC (heavy for v0.1), Unix sockets /
named pipes (two transport stacks; HTTP is uniform cross-platform).

### D11 — stack (pinned Ф0.s8, MSRV-checked against this box's rustc 1.93.1)
`tokio` 1.52, `axum` 0.8 (server), `reqwest` 0.13 + rustls (client),
`serde` 1.0 + `toml` + `serde_json`, `clap` 4.6 (derive), `tracing` 0.1
+ `tracing-subscriber`, `ulid` 1.2, `camino` 1.2 (UTF-8 paths),
`thiserror` 2.0, `insta` 1.48 (goldens). Process control (Ф0.s5 spike
PASSED, mechanism chosen): **`win32job` 2.0** (Windows Job Object with
`KILL_ON_JOB_CLOSE`) as the primary kill-tree + crash-safety mechanism,
**`sysinfo` `=0.37.2`** for the orphan-sweep assertion, `command-group`
5.0 for the POSIX process-group path (cfg-gated). **MSRV finding
(binding on D15/CI):** `sysinfo` 0.39 requires rustc ≥ 1.95; this box
runs **1.93.1**, so the workspace pins `sysinfo =0.37.2` (builds clean)
and either sets a `rust-version = "1.93"` floor or the owner upgrades
the toolchain — recorded as a Phase 1 opening step. Rejected: actix (no
advantage), async-std (tokio gravity), heavyweight config frameworks
(plain serde+toml), `taskkill` shelling as the *primary* mechanism
(kept only as the pod-loss fallback — Job Objects are cleaner and were
proven).

### D12 — tariff hygiene is mechanism, not prose
Workers get `WebFetch`/`WebSearch` (and web MCP tools) denied via profile;
`fractality fetch <url> --out <path>` (plain reqwest, boss- and
human-callable) downloads documents once into the workspace; MC counts
web-ish tool events from transcripts into a monthly quota metric. **Ф0.s3
resolved the quota:** it is tier-scoped, not a flat 4000 — Lite 100 / Pro
1 000 / **Max 4 000** MCP calls per month (the owner's figure = the Max
tier), and Web-Search + Web-Reader + Zread MCP *share* that pool; prompt
budgets are Lite ~80 / Pro ~400 / Max ~1 600 per 5 h. So the quota metric
is parameterized by `profile.pricing.plan` (D6), and `max_concurrent`
respects the 5-hour prompt budget. The Phase 5 playbooks then *describe*
what the mechanism already *enforces*.

### D13 — CLI verb set (v0.1): UNIX muscle memory
`fractality mc start|stop|status` · `run --packet <file>` (sync; exit code
mirrors worker outcome; prints run-dir path + one-screen summary) · `spawn`
(async) · `wait <id>…` (shell-`wait` semantics) · `ps` (list runs; `ls`
kept as alias) · `show <id>` · `tree [<id>]` · `logs <id> [-f]` (tail the
transcript/stderr) · `kill <id> [--tree]` · `questions` (pending
`waiting_on_boss` items) · `answer <id> [<text>|--file <f>]` · `stats` ·
`node` (machine + fs-scope identity, D19) ·
`fetch <url> --out <path>` · `packet new [--template <name>]`.
Human-drivable first: every flow debuggable from a terminal without any
boss involved. Ergonomics law: D17.

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

### D17 — ergonomics law: GNU/UNIX grammar for humans and agents
Owner amendment (2026-07-09): the toolchain must feel like the classic
userland, «что агентам, что людям». Concretely: verbs mirror coreutils
muscle memory (`ps`, `kill`, `wait`, `logs -f`, `tree`); default output is
quiet, plain-text, one-record-per-line, stable-ordered — grep/awk-able by
construction; `--json` on every read verb for agents; exit codes are
semantic (0 ok · 1 worker failed · 2 infra error · 3 budget-killed ·
4 parked on a question past its wait) and documented per verb; color only
on a TTY and never under `NO_COLOR`; everything-is-a-file stays literal —
run dirs are the API of last resort, `show`/`logs` are conveniences over
them; `--help` at man-page quality. Rejected: TUI-first or JSON-only
surfaces — the fabric must compose with pipes and with agents equally.

### D18 — non-yolo delegation: permissions and questions as a protocol
The owner's question («как делегировать в неинтерактивные воркеры в любом
режиме кроме yolo?») has a paved-road answer, **Ф0.s3 CONFIRMED** on CC
2.1.202: *two* usable surfaces exist — `--permission-prompt-tool <mcp
tool>` (a broker MCP tool answers each prompt in headless mode) and the
**PreToolUse hook** returning
`hookSpecificOutput.permissionDecision ∈ {allow, deny, ask, defer}`
(multi-hook precedence `deny > defer > ask > allow`; `updatedInput` can
rewrite the call). The **`defer` value is a gift**: it "exits to be
resumed later (`-p` only)" and the final `result` event carries
`deferred_tool_use` — i.e. CC has a *native* park-and-resume primitive
that maps exactly onto `waiting_on_boss`, so the broker can defer a
blocked call to MC and resume it on `answer` without holding a process
frozen mid-turn. Both `--max-turns` and `--max-budget-usd` exist (print
mode) for budget enforcement (D6/Phase 4). The stack, layered:
1. **Static allowlist** per profile (D6) — the boring majority:
   pre-approved edit/test/git tools inside the worktree; web tools denied
   (D12). Unlisted actions fail closed, they never hang.
2. **Pod permission broker:** the worker's permission requests route to a
   broker the pod serves; profile policy auto-decides allow/deny
   patterns; anything else escalates — pod → MC event → run state
   `waiting_on_boss`: the question rides the bus and is persisted as
   `question.md` in the run dir (I2). The worker stays alive, blocked on
   that one tool result.
3. **`ask_boss` guidance channel:** the same broker exposes an explicit
   question tool; the packet preamble instructs the worker to use it when
   genuinely stuck instead of guessing — especially before anything
   destructive. The boss (or a human) triages with `questions` and
   replies with `answer`; the reply returns as the tool result and the
   run resumes. Wait bounded by the packet budget.
`yolo` (skip-permissions) is **de-scoped from v0.1 by RP4's ruling** —
the D18 stack is the way of life; if yolo ever returns, it returns as
explicitly named, worktree-restricted profiles. Rejected as the primary
channel: stdin stream-json injection — the pod owns stdin so it stays
available as a fallback (structured-output injection included), but
tool-call semantics are in-distribution for models, atomic, and
file-recordable; raw stream surgery is neither.

### D19 — claim-check references and filesystem identity
Owner directive (2026-07-09, §3): don't haul bulk data over the bus —
pass a temp-file reference (path + offset/length, or head/tail offsets
for data of unknown length), valid only when both parties share a
filesystem; and give every agent a way to know its machine and where
its filesystem comes from.
- **FileRef v1** (core DTO, baked into the API from Phase 1):
  `{fs: <scope-id>, path: <scope-relative, forward-slash>, range:
  whole | {offset, len} | {skip_head, skip_tail}, etag?, sha256?,
  grant?}`. Range vocabulary is deliberately RFC 7233 (the S3 model,
  owner directive): `{offset, len}` ↔ `bytes=a-b`, pure tail ↔ the
  suffix form `bytes=-N`; the combined head/tail trim resolves
  `last-byte-pos = size−1−skip_tail` at stat time (length pins then;
  non-atomicity for still-growing files documented). `etag` is a cheap
  version fingerprint (size+mtime hash, MC-stamped) — the reader sends
  `If-Match` semantics and a stale copy or a mutated file fails loudly
  instead of returning silently wrong bytes (this also closes the
  copy-staleness caveat at the *read* end). `sha256` stays optional
  strong integrity for immutable payloads. `grant` is reserved: a
  presigned capability (HMAC over scope|path|range|expiry with MC's
  key, S3-presigned-URL style) so a ref can be handed to a party that
  has no MC bearer — cross-node pods, future GUIs; federation builds
  it, v0.1 only reserves the field. Bus messages inline payloads up to
  a threshold (default 64 KiB, configurable) and switch to FileRef
  above it.
- **Scope identity — the rendezvous beacon (authoritative):** MC stamps
  `<scope-root>/.fractality-fsid` with a random UUID + issuing-MC id +
  timestamp, rotated periodically. Two parties share a scope **iff they
  read the same live beacon** — a behavioral proof that sidesteps every
  platform quirk. Mount metadata is recorded as corroboration and
  diagnostics: volume serial / volume GUID (Windows), fsid/UUID (Unix),
  and for network mounts the **server identity + export** — the owner's
  NAS-IP intuition, generalized (NFS `server:/export`, SMB
  `\\server\share`, Ceph cluster fsid). Copy-staleness is the named
  caveat: a byte-copied tree carries a beacon copy; beacon rotation
  plus MC authority (scope-id ↔ current nonce) bounds the window; on
  the v0.1 single box it is moot.
- **Node identity:** stable machine id (`/etc/machine-id` Linux /
  `MachineGuid` Windows / `IOPlatformUUID` macOS) + hostname +
  addresses, captured at registration, exposed as `FRACTALITY_NODE_ID`
  in run envs and by `fractality node`. Raw IPs are **labels, not
  identity** (DHCP, multiple NICs, VPNs) — rejected as the primary key,
  kept in metadata for humans.
- **Dereference rule:** scope match proven → read locally (the
  zero-copy fast path); no match → fetch over the bus:
  `GET /v0/runs/:id/files/{path}` honoring standard `Range` headers
  (RFC 7233 incl. suffix ranges), `ETag`/`If-Match`, and part-aligned
  parallel ranged reads for bulk (the S3 byte-range-fetch pattern) —
  shape reserved now, built with federation under DEF-6. **A reference
  is an optimization, never a requirement** — I2's bus law survives
  intact.
- Rejected: absolute paths inside references (the same NAS mounts at
  different points on different nodes; separators differ per OS —
  scope-relative forward-slash paths only); IP-as-identity (see above);
  content-addressing everything (hashing bulk on every hop costs more
  than it buys at v0.1 scale; `sha256` stays optional for immutable
  payloads).

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
- **P7** — the campaign lands in ≤ 17 commits (15 planned) matching the
  planned subjects (drift recorded in the ledger, not silently absorbed).
- **P8** — the pod layer costs < 1 s of spawn overhead per worker, and an
  MC kill-and-restart during a live run loses nothing: the pod
  re-registers and the run completes (zero orphans, zero lost
  transcripts).

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
   figure), prices, any documented one-shot-strength guidance for GLM,
   and the headless permission surface (permission-prompt tool /
   PreToolUse decision shape) that D18 leans on.
   **Rewrite D6/D7/D12/D18 in place**; note findings under this phase.
4. **s4 GLM smoke + stream-json fixtures:** repeat s2(b) with the GLM env
   (base URL + token file + model mapping): expect a GLM-authored "OK".
   Then one small real task with `--output-format stream-json` captured to
   a fixture file — the golden corpus for the Phase 3 parser. Early P2
   check.
5. **s5 kill-tree spike:** scratch Rust bin (uncommitted) plays the pod:
   spawns a long-running `claude -p`, owns its Job Object / process
   group, streams stdout to a file, kills the tree via the D11 candidate
   crates; assert no orphans via `sysinfo`; verify a dying parent
   (mock-MC) takes down neither pod nor worker. **Rewrite D3/D11 in
   place.**
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

#### Phase 0 findings — EXECUTED 2026-07-09 (all green; no commits)

- **F1 (s1) env sanity — GREEN.** `claude` **2.1.202** on PATH; `git`
  2.52.0; `~/.vibevm/zai.api.token` exists (content never read).
- **F2 (s2) nested spawn — GREEN, P1 CONFIRMED.** `claude -p` under this
  boss session returns `OK` both with inherited env and with a hand-built
  clean-slate whitelist (`PATH HOME USERPROFILE TEMP TMP SystemRoot
  COMSPEC APPDATA LOCALAPPDATA`). No conpty/console surprise. **Finding:**
  `APPDATA`/`LOCALAPPDATA` must be in the D5 whitelist on Windows (CC
  needs them) — added to D5's set. `env -i` in Git Bash must re-pass
  `PATH` or `claude` isn't found (documented for the env constructor).
- **F3 (s3) provider facts — RESOLVED, D6/D12/D18 rewritten.** Base URL
  `https://api.z.ai/api/anthropic` ✅. Model mapping via
  `ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU}_MODEL` (the legacy
  `ANTHROPIC_MODEL`/`ANTHROPIC_SMALL_FAST_MODEL` pair is the wrong
  surface for this provider). Big model id `glm-5.2[1m]` (the `[1m]`
  suffix = 1M context), small `glm-5-turbo`, haiku-slot `glm-4.7`/turbo.
  Recommended extra env: `API_TIMEOUT_MS=3000000`,
  `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`. Quota is **tier-scoped**
  (the owner's "4000 MCP" = the **Max** tier): Lite 100 / Pro 1 000 / Max
  4 000 MCP calls per month, shared across Web-Search + Web-Reader +
  Zread; prompts ≈ 80/400/1 600 per 5 h. CC permission surface confirmed
  (see F6). CC budget flags `--max-turns` and `--max-budget-usd` exist
  (print mode).
- **F4 (s4) GLM smoke + fixtures — GREEN, P2 CONFIRMED.** A clean-slate
  env + z.ai base URL + token + model mapping + a **fresh
  `CLAUDE_CONFIG_DIR`** ran headless first try: `claude -p "…state your
  model name"` → `OK, glm-5.2`. A real write task under
  `--output-format stream-json --verbose --permission-mode acceptEdits`
  created `hello.txt` and emitted a **122-event** transcript; captured as
  the first golden fixture. Event `type`s seen: `system` (incl.
  `system/init`), `assistant`, `user`, `text`, `thinking`, `tool_use`,
  `tool_result`, `create`, `result`. The `json` result carries
  `usage.{input_tokens,output_tokens,cache_creation_input_tokens,
  cache_read_input_tokens}` and `total_cost_usd` — metering is viable.
  **This also resolves R5:** a fresh config dir onboards headless with
  **no interactive step** (it created `backups/ projects/ sessions/`
  itself), so `config_dir = "auto"` is safe.
- **F5 (s5) kill-tree — GREEN, mechanism chosen, D3/D11 rewritten.** A
  `win32job` 2.0 Job Object armed with `KILL_ON_JOB_CLOSE`, child
  assigned immediately after spawn, stdout streamed to a file: closing
  the job reaps the tree (PASS, zero descendants). **The stronger proof:**
  in `--orphan-test` the parent *exits without any explicit kill* and the
  OS auto-reaps the assigned child (verified recycling-safe by image
  name; zero `timeout.exe`/`cmd.exe` orphans). So **a pod crash leaks no
  worker** — the pod's core safety property is guaranteed by the OS, not
  by our cleanup code. `taskkill /T /F` demoted to the pod-loss fallback.
- **F6 (s3/CC) permission surface — CONFIRMED, D18 strengthened.** CC
  2.1.202 offers both `--permission-prompt-tool` and a PreToolUse hook
  with `permissionDecision ∈ {allow,deny,ask,defer}` (precedence
  `deny>defer>ask>allow`, `updatedInput` rewrites the call). `defer`
  natively exits-to-resume in `-p` mode with `deferred_tool_use` in the
  result — a native `waiting_on_boss` primitive. settings.json
  `permissions.{allow,ask,deny}` with `Tool(specifier)` rule syntax backs
  the static allowlist (D6).
- **F7 (s6) intake — DONE, all MIT, clean-room intact.** S1–S4 cloned/
  downloaded at pinned commits (inventory updated); all three repos MIT.
  codex-first fully studied → `spec/refs/notes/codex-first-study.md`
  (DC1–DC6 + the mandated improvements). barkain/rlm/paper deep study
  deferred to Campaigns 2/3 (licenses cleared).
- **F8 (s7) landscape — DONE.** `spec/refs/notes/landscape.md`: fractality
  is neither a per-request router (claude-code-router) nor a session
  orchestrator (claude-swarm/squad) — it is the scheduler layer they lack.
- **F9 (s8) crate pins — DONE, MSRV finding (binding).** Versions pinned
  in D11. **This box runs rustc 1.93.1**; `sysinfo` 0.39 needs ≥ 1.95, so
  the workspace pins `sysinfo =0.37.2` (builds clean) and Phase 1 opens by
  setting a `rust-version` floor or asking the owner to bump the
  toolchain. Caught here exactly as Phase 0 is meant to.
- **F10 (s9) host gate — GREEN.** Host `bash tools/self-check.sh` was
  green at ignition and the new package lives under the excluded
  `packages/` tree; re-confirmed green after all Phase 0 spec edits (see
  the amendment commit's acceptance).
- **Delegation dogfood (interim paradigm, live this session):** two grunt
  tasks were delegated to `zai-coding-plan/glm-5.2` via opencode and
  **boss-verified**: (a) extracting the CC permission/headless fact sheet
  from local docs — spot-checked against the raw docs, accurate; (b)
  drafting the kill-tree spike program — it needed one boss fix (the
  `sysinfo` MSRV pin) but then **passed on the real machine**. First field
  data for the Phase 5 playbooks: GLM-5.2 is strong at bounded,
  well-specified one-shot code and doc-extraction; MSRV/version currency
  is a known blind spot the boss must check.

### Phase 1 — workspace skeleton + mission-control core

1. Create the Cargo workspace in `fractality/v0.1.0/` with the six D1
   crates; wire the floor; add `spec/examples/hello-glm.toml` (packet).
2. `fractality-core`: ULID run ids, `RunState` machine (incl.
   `waiting_on_boss`), packet schema (serde + goldens), journal event
   types, API DTOs (client and pod legs), `FileRef` / fs-scope / node
   types (D19), `WorkerBackend` trait.
3. `fractality-mission-control`: journal append + replay + pod
   reconciliation (D9), run registry, lockfile + bearer, `GET /v0/health`,
   run CRUD minus spawn, pod register/heartbeat endpoints, node identity +
   the runs-root scope beacon (D19), graceful shutdown.
4. `fractality-pod` skeleton: spawn a child from a spec, stream its stdio
   to files, heartbeat + exit report to MC (loopback-tested with a stub
   child process, no Claude Code yet).
5. `fractality-cli` + `fractality-mc-client`: `mc start|stop|status`,
   auto-start on first client call, `ps`, `show` (D17 output rules from
   day one).

*Exit:* floor green; `mc start` → `status` healthy → `stop` clean on this
box; a stub run driven through pod → MC survives an MC kill-and-restart
with state intact (early P8 signal).
*Prediction:* journal replay + pod re-registration cover restart with
zero manual repair.
*Commits:* `feat(fractality): cargo workspace + core model` ·
`feat(fractality): mission-control — journal, registry, lifecycle` ·
`feat(fractality): pod — child supervision skeleton`.

### Phase 2 — delegate-out (fire a worker, results land on disk)

1. Profile loading (D6) + validation; `config_dir = "auto"` provisioning
   (seed per s3 findings so headless works on first spawn).
2. The D5 env constructor **with the poisoned-parent test**.
3. Worktree manager (D8) via git CLI.
4. Backend `claude-code`: build headless invocation (prompt from packet
   goal + output contract preamble; flags per s3: output format, max
   turns, permission posture, tool denies).
5. Spawn path: `POST /v0/runs` accepts a packet; MC provisions the
   worktree (D8) and launches the pod with the run spec; the pod builds
   the D5 env, spawns the worker, streams stdout to
   `worker-stdout.jsonl`, heartbeats state; `fractality run` drives it
   sync end-to-end (spawn → wait → exit code), collection still minimal
   (exit code + transcript on disk).

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
   tokens, the result as a FileRef (D19), branch, acceptance verdicts);
   `show` renders any historical run the same way.

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
4. `kill --tree` delegated to the pod (s5 mechanism); orphan sweep
   assertion; pod-loss fallback: MC tree-kills the pod's process group.
5. `GET /v0/metrics` v0 aggregates (runs by state/profile/model/day;
   tokens; wall time; web-tool-quota counter per D12).

*Exit:* manual-tests #2 (3-worker swarm on disjoint modules) and #3
(recursive kill of a depth-2 tree) recorded with outputs.
*Prediction:* P4 and P5, measured.
*Commits:* `feat(fractality): swarm — async lifecycle, budgets, kill-tree`
· `feat(fractality): mission-control call tree + metrics`.

### Phase 4b — the interaction layer (delegation without yolo)

1. Broker in the pod per D18: serve the permission surface verified in
   Ф0.s3 plus the explicit `ask_boss` tool; profile policy auto-decides
   allow/deny patterns; everything else escalates to MC.
2. MC + core: `waiting_on_boss` transitions, question/answer endpoints
   (D10) — the bus carries them; `question.md` / `answer.md` persist
   them in the run dir (I2).
3. CLI: `questions`, `answer`, the exit-code-4 path (D17); `run` prints a
   loud one-liner when its worker parks on a question.
4. The packet preamble gains the question protocol: use `ask_boss` when
   genuinely stuck, always before anything destructive; never guess.

*Exit:* E2E — a worker hits a non-allowlisted action, parks in
`waiting_on_boss`, the boss answers via `fractality answer`, the run
resumes and completes. Recorded as manual-test #4.
*Prediction:* a parked worker survives ≥ 10 minutes idle and resumes
cleanly (no worker-side session timeout at v0.1 scale).
*Commits:* `feat(fractality): pod broker — permissions and questions` ·
`feat(fractality): waiting-on-boss lifecycle + answer verbs`.

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
4. Dogfood (RP1 RESOLVED): relicense the host's straggler manifests
   (`license = "EULA"` → `"UPL-1.0"`) through the swarm, in worktrees.
   Acceptance, per the owner's «минимальная приемка»: the boss reviews
   the diff; `grep -r 'license = "EULA"'` over the target set returns
   zero; `git diff --stat` touches only the intended manifests; host
   `bash tools/self-check.sh` green **before** merge. Recorded as
   manual-test #5 and as the P6 baseline measurement.

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
- **R5 fresh CLAUDE_CONFIG_DIR onboarding blocks headless.** **RESOLVED
  GREEN in Ф0.s4** — a fresh config dir onboarded with no interactive
  step (created `backups/ projects/ sessions/` itself; the GLM smoke ran
  first try). No seeding needed; `config_dir = "auto"` is safe. Kept as a
  regression watch, not an open risk.
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
fractality stats                                                     # ≥5 completed runs, ≥1 swarm parent w/ 3 children
fractality questions                                                 # empty at close
ls spec/manual-tests/                                                # 5 recorded procedures with outputs
(cd ../../../.. && bash tools/self-check.sh); echo "EXIT=$?"          # host floor green
```

## 13. Review points

1. **RP1 — dogfood target for Phase 6.** RESOLVED (owner, 2026-07-09):
   «Да, в качестве догфуд задачи можно переколбасить все наши EULA на
   UPL-1.0, это кажется супер простой задачей. Но пожалуйста, если что-то
   меняется в самом корневом VibeVM, верифицируй это, на случай если
   воркер сойдёт с ума. Должна быть хотя бы минимальная приемка.» —
   Phase 6 step 4 carries the acceptance recipe.
2. **RP2 — does `wal-workspaces` join redbook?** RESOLVED (owner,
   2026-07-09): «wal-workspaces можно включить в redbook» — rides the
   next redbook edition (0.3.0), host-side work outside this campaign
   (DEF-11).
3. **RP3 — publish timing** for anything in `org.vibevm.fractality`. OPEN,
   standing hold; §10.
4. **RP4 — default worker permission posture.** RESOLVED (owner,
   2026-07-09): «RP4 зависел от того как мы будем жить без yolo. Мы
   изобрели поды, ask_boss и инъекцию structured output в stdin, так что
   мы вполне справимся на некоторое время без yolo, пока не работаем в
   worktree.» Applied reading: the D18 stack (allowlist + pod broker +
   `ask_boss`) is the way of life; **`yolo` is de-scoped from v0.1
   entirely** — if it ever returns, it returns as explicitly named,
   worktree-restricted profiles. D18's closing paragraph amended in
   place.

## 14. Execution ledger

### Phase 0 — EXECUTED (2026-07-09); commit map

- `6317cff` docs(fractality): plan v0.1 amended with Phase 0 findings.
  Single amendment commit per the phase-gates law (Phase 0 commits no
  tree changes — only findings into the plan, the study note, the
  landscape note, and the inventory). Flips the status line to
  EXECUTING. Confirms P1 (F2) and the P2 early check (F4); resolves R5
  (F4); rewrites D3/D5/D6/D11/D12/D18 in place from findings; records
  the rustc-1.93.1 MSRV constraint (F9) as binding on Phase 1. No
  prediction falsified. Floor: host `self-check.sh` green; the pod
  kill-tree spike passed on the real machine (scratch, discarded).
- Interim-paradigm note: two GLM-5.2 delegations (fact extraction, spike
  draft) were boss-verified during this phase — first Phase-5 field data.

_(Later phases fill their own maps at each boundary.)_

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
  тогда, когда задача тому способствует»). Owner hypothesis to check
  against the paper first (2026-07-09, verbatim): «Кажется, в RLE
  основная фишка в том, что каждый раз у нас новый свежий контекст и это
  означает спавн нового агента с чистого листа. Но это моё поверхностное
  представление, это надо проверять.»
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
- **DEF-11 — redbook edition 0.3.0** pinning `wal-workspaces` (RP2
  resolved): host-side redbook wave, not this campaign.
- **DEF-12 — the checkpoints layer** (owner, 2026-07-09, verbatim): «в
  будущем мы будем использовать систему, похожую на Checkpoints от
  Entire.io и наш контроль за областью выполнения и историей будет ещё
  лучше. […] система уже есть, я просто не хочу её пока здесь
  использовать по причине того, что она слишком молодая и сырая».
  Per-turn workspace + history checkpointing with rewind/audit over
  runs; deliberately unadopted in v0.1; enters via its own future
  campaign once the system matures. Inventory row S8.
