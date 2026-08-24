# Subagent launchers — claudez/claudez2/codexrunner as the worker transport {#root}

<status stage="impl" state="done" comment="owner directive 2026-08-03; launchers reworked and verified the same day (the ALPHA/BRAVO matrix below); codexrunner added by owner directive 2026-08-20; the mode switch and the lane priority are the owner's levers"/>

@fact:the-directive **The owner's directive (2026-08-03, chat, near-verbatim):**
доработать запускаторы `claudez` / `claudez2`, чтобы они работали с `-c` как
обычная `claude` и годились как субагенты; написать инструкцию, как фазы E и T
используют их вместо нативных агентов; **переключение native ↔ claudez остаётся
в ведении владельца — он в любой момент может сменить способ вызова**; effort —
всегда max; по возможности воркеры работают **параллельно на обоих запускаторах
в worktree** и потом мерджатся — но правки, требующие изменений во многих местах
(конфликтоопасные), идут **одним потоком**; более-менее изолированные — **сразу
двумя**.

## 1. The switch — owned by the owner {#switch}

@fact:switch-file The switch is one line in
[`SUBAGENT-MODE.toml`](SUBAGENT-MODE.toml) beside this file:
`mode = "claudez"` or `mode = "native"`. The boss re-reads it **before every
fan-out**, so an edit takes effect immediately — mid-phase, mid-batch, any
time. Saying it in chat works too (the boss updates the file so the state
stays durable).

@fact:switch-native `native` means exactly what Phase D ran: the harness's
built-in `opus5` subagents through the Agent tool. `claudez` means workers
are **Claude Code processes on GLM-5.2** spawned through the launchers below.
Fractality stays out of this campaign either way (plan §6 `#delegation`).

@fact:switch-does-not-change **What the mode never changes:** verdicts, anchor
routing, review of delegated output, spec/plan authoring, commits and
pushes stay the boss's in both modes (the never-delegate set); briefs cite
durable files only; Rules 1–4 bind identically; the presentation format to
the owner is unaffected.

## 2. The transport — what the launchers are {#transport}

@fact:launchers-what Machine facts (this box; the launchers live OUTSIDE the
repository): `C:\Users\olegc\opt\bin\{claudez,claudez2}` (bash) and
`{claudez.ps1,claudez2.ps1}` (PowerShell). Each sets the z.ai
Anthropic-compatible gateway env (`ANTHROPIC_BASE_URL`, bearer from a token
file, model triple `glm-5.2[1m]` for opus/sonnet + `glm-5-turbo` for haiku)
and hands off to `claude`, **passing every argument through** — so `-p`,
`-c`, `--resume`, `--allowedTools` behave exactly as with plain `claude`.

@fact:launchers-state-contract **The state contract that makes them
subagent-grade (reworked 2026-08-03):** the two launchers keep SEPARATE
Claude state dirs, so in one and the same cwd each launcher's `-c` continues
**its own** latest conversation and never steals the sibling's thread:

| launcher | account/token | `CLAUDE_CONFIG_DIR` | overrides |
|---|---|---|---|
| `claudez` | `~/.vibe/zai.api.token` | `~/.claude-glm` | `CLAUDEZ_CONFIG_DIR`, `ZAI_API_TOKEN_FILE` |
| `claudez2` | `~/.vibe/zai.api.token.2` | `~/.claude-glm2` | `CLAUDEZ2_CONFIG_DIR`, `ZAI_API_TOKEN_FILE_2` |

@fact:launchers-effort **Effort is max by construction** (owner, 2026-08-03):
both launchers export `MAX_THINKING_TOKENS=32000` — Claude Code's
thinking-budget lever; harmless if the gateway model ignores it. Override:
`CLAUDEZ_MAX_THINKING`.

@fact:launchers-verified **Verified 2026-08-03, the ALPHA/BRAVO matrix:** in one
scratch cwd, `claudez -p` seeded codeword ALPHA, `claudez2 -p` seeded BRAVO
(fresh `~/.claude-glm2` bootstrapped headless, second token live); then
`claudez -c -p` answered **ALPHA** and `claudez2 -c -p` answered **BRAVO** —
from bash AND from PowerShell (`Get-Command` resolves both to the `.ps1`
scripts; cross-shell continuation hits the same per-launcher thread). Eight
runs, exit 0 each.

@fact:launchers-conversation-key **The `-c` scoping rule to build on:** claude
keys conversations by (config dir, cwd). One worker = one cwd (its
worktree) = one continuable thread per launcher. `-c` in a cwd with no
prior thread errors — expected, same as plain `claude`.

@fact:launchers-parallelism-2026-08-20 **Параллельность и приоритет —
рулинг владельца (2026-08-20, чат, near-verbatim):** «можешь делегировать
задачи на разработку в запускалки claudez2 (в первую очередь) и claudez
(во вторую очередь), уровень параллельности — вплоть до 5 на каждую
запускалку». До **5** одновременных воркеров на запускалку (каждый в своём
worktree-cwd — правило (config dir, cwd) выше это уже допускает: пять cwd
под одним config dir — пять независимых продолжаемых тредов). Правило
«конфликтоопасные многоместные правки — одним потоком» из директивы
2026-08-03 остаётся в силе и приоритетнее ширины. *Названный здесь порядок
лейнов — исторический; действующий порядок всегда в
`##launchers-priority-is-the-owners` ниже.* @status:impl/done

@fact:launchers-priority-is-the-owners **Приоритет лейнов выбирает
ВЛАДЕЛЕЦ — правило и текущее состояние (директива 2026-08-20).** Какая
запускалка приоритетная — не боссово суждение и не свойство машины:
владелец назначает порядок в любой момент (чатом — босс тут же обновляет
эту секцию, чтобы состояние пережило сессию). Босс перечитывает эту
секцию перед КАЖДЫМ fan-out'ом, как и `SUBAGENT-MODE.toml`. История
назначений — строками ниже, верхняя действует:

- **2026-08-20 (действует): `codexrunner` — приоритетный лейн**; `claudez`
  — второй; `claudez2` — ЗАПРЕЩЁН до отдельного слова владельца (его же
  утреннее слово той же даты). Одновременно владелец: имя запускалки —
  `~/opt/bin/codexrunner` (bash + ps1), модель `gpt-5.6-sol`, effort
  `xhigh`.
- 2026-08-20 (утро, снято тем же днём): claudez-only, claudez2 запрещён.
- 2026-08-20 (ночь, снято утром): claudez2 → claudez.

@fact:launchers-codexrunner **`codexrunner` — третий лейн: OpenAI Codex CLI
(добавлен директивой владельца 2026-08-20, проверен в тот же день).**
Машинные факты: `C:\Users\olegc\opt\bin\{codexrunner,codexrunner.ps1}`;
тонкая обёртка над `codex` (0.148.0), пробрасывает все аргументы,
инжектирует ТОЛЬКО модель и усилие через clap-глобальные `-c`-переопределения
(`model="gpt-5.6-sol"`, `model_reasoning_effort="xhigh"`; переопределяемо
`CODEXRUNNER_MODEL` / `CODEXRUNNER_EFFORT`, а поздние `-m`/`-c` в строке
вызова побеждают — пин это умолчание, не клетка). Состояние — `CODEX_HOME`
(умолчание `~/.codex`, где живёт auth.json; переопределение
`CODEXRUNNER_CODEX_HOME`); лаунчер отказывает с рецептом, если auth.json
нет. Таблица соответствий claudez-механикам:

| claudez | codexrunner |
|---|---|
| `claudez -p "…"` | `codexrunner exec "…"` |
| `claudez -c -p "…"` | `codexrunner exec resume --last "…"` — **ОСТОРОЖНО: cwd-скоуп resume НЕ проверен** (возможно, `--last` берёт последнюю сессию глобально, а не этого worktree; до верификации при двух+ живых codex-тредах предпочитать свежий `exec` с перечтением пакета — состояние на диске воркер дочитает сам) |
| `--output-format stream-json --verbose` | `--json` (JSONL-события на stdout) |
| `--allowedTools …` (позитивный список) | **YOLO ПО УМОЛЧАНИЮ (владелец, 2026-08-20):** запускалка сама инжектирует `--dangerously-bypass-approvals-and-sandbox` — песочницы и одобрений НЕТ; причина: workspace-write глушил Windows Known-Folder API (`home_dir()=None`, 12 ложно-красных home-тестов). Возврат песочницы: `CODEXRUNNER_SANDBOXED=1` + свои `--sandbox`-флаги (с инжектом они конфликтуют) |
| запреты инструментами | запреты ТОЛЬКО ТЕКСТОМ пакета: под yolo воркер может git/сеть/любую fs — принятая владельцем цена; боссово ревью диффа и перечень Deviations — единственный забор, приёмка соответственно строже |

Проверено 2026-08-20: exec под дефолтной read-only песочницей честно
отказал записи; под `--sandbox workspace-write` создал файл с точным
содержимым; модель и xhigh-усилие подтверждены заголовком сессии.
Известная острота: exit-код `codex exec` — про исход ПРОГОНА, не задачи
(отказ песочницы дал exit 0) — приёмка строго по артефактам
(`##obs-verify-by-artifacts` действует дословно). Cargo-задачам под
`workspace-write` может понадобиться запись вне workspace
(`~/.cargo`-локи) — лечится
`-c sandbox_workspace_write.writable_roots=[…]` или, по слову владельца,
`danger-full-access`; первый cargo-прогон лейна мерит это живьём.
@status:impl/done

@fact:codexrunner-appcontainer-debris **Коробочный Codex (MSIX) оставляет
неудаляемый без elevation мусор в target/ (2026-08-20).** Воркеры
codexrunner через Store-установку Codex создают часть build-файлов
(fingerprint timestamps, инкрементальные `.o`, `.pdb`) с
DACL/владельцем AppContainer-SID пакета: обычному пользователю их не
читает даже `icacls`, не берёт `takeown`, не сносит `rd` — «Access is
denied» на ~26 файлах на worktree при чистой массе, снятой обычным
`rd /s /q` (+`icacls /reset /T` добирает часть). Добор остатка — только
elevated-терминал: `takeown /F <wt>\target /R /D Y`, затем
`icacls <wt>\target /grant <user>:F /T /C`, затем `rd /s /q`.
Учитывать при уборке дисков после codex-воркеров. @status:impl/done

@fact:codexrunner-kill-is-pid-exact-never-name-never-tree **Убийство
codex-раннера — только адресным PID БЕЗ дерева; kill по имени образа
или по дереву задачи сносит владельческий ГРАФИЧЕСКИЙ клиент ChatGPT
(владельческий репорт 2026-08-24: GUI-клиент умирает КАЖДЫЙ раз при
завершении раннера боссом).** Механика: коробочный Codex — MSIX
(`#codexrunner-appcontainer-debris`), то же прикладное семейство, что
GUI-клиент ChatGPT; широкий снос (по имени `codex*`/`ChatGPT*`, или
`taskkill /T` по дереву, или харнессный TaskStop, который целит дерево
задачи) задевает общий пакетный хост/брокер — и GUI умирает вместе с
раннером. Отсюда порядок, действующий всегда:
**(0)** лучший kill — не убивать: завершившийся сам раннер никого не
трогает; брошенный, но безвредный run оставляют дожить
(`#fact-a-killed-run-is-not-a-verdict-and-the-kill-costs-the-tail`).
**(1)** Если убить надо: НЕ TaskStop. Найти PID ровно этого раннера по
уникальному маркеру его командной строки — pointer несёт имя пакета,
одно на боксе: `Get-CimInstance Win32_Process | Where-Object {
$_.CommandLine -match 'PACKET-<task-id>' } | Select ProcessId,
ParentProcessId, Name`. **(2)** Убить адресно и БЕЗ `/T`:
`taskkill /PID <pid> /F` по каждому НАШЕМУ процессу отдельно (наш =
маркер в cmdline, либо родительская цепочка до нашего wrapper-bash по
`ParentProcessId`); повторить запрос маркера — убедиться, что умерли
ровно они. **(3)** Обёрточная bash-задача харнесса после смерти её
ребёнка завершается сама — TaskStop не нужен вовсе. Запрещено
навсегда: `taskkill /IM …`, `Stop-Process -Name …`, `/T` на
codex-процессах, TaskStop по codex-задаче как первый ход. Спавн-форма
даёт маркер бесплатно (имя пакета в указателе) — ещё одна причина
формы `#fact-a-large-packet-cannot-travel-as-an-argument`. **Развязка
измерена стоящей (2026-08-24):** оба шелла и лаунчер резолвят `codex` в
npm-CLI под nvm4w (`C:\nvm4w\nodejs\codex*`, `codex-cli 0.148.0`,
node-семейство); MSIX-алиаса в WindowsApps нет — раннер-лейн УЖЕ
процессно несвязан с GUI. **Точная жертва прошлых киллов найдена
живьём:** у GUI-клиента ChatGPT есть СОБСТВЕННЫЙ ребёнок `codex.exe`
(встроенный движок, `…WindowsApps\OpenAI.Codex_…\app\resources\codex.exe`,
родитель — ChatGPT.exe) — kill-по-имени `codex*` попадает в НЕГО, а не
в node-раннера, и клиент умирает; вот механика «GUI падает каждый раз».
Правильный kill моего раннера = PID-точечный по маркеру. **Измеренная
форма семейства (2026-08-24, живой K26-раннер):** npm-CLI разворачивается
в `sh.exe → node.exe → codex.exe` (npm-пакет несёт СОБСТВЕННЫЙ
`codex.exe` — ТО ЖЕ имя образа, что у движка GUI: kill-по-имени фатален
навсегда, независимо от MSIX-эры), это семейство ОТЦЕПЛЕНО от дерева
харнесс-задачи (родительская цепочка sh обрывается — TaskStop его не
достаёт в принципе, что заодно объясняет
`#fact-a-killed-task-does-not-kill-the-worker-and-the-survivor-writes-late`),
и все три процесса несут маркер пакета в cmdline. Порядок kill:
`codex.exe`-PID первым, затем node/sh, если задержались, — каждый по
своему PID. Закрыто сильнее опции (владелец, 2026-08-24, «поменяй в codexrunner
кодекс на коммандлайновый… проверь что он уже лежит в PATH»): оба
лаунчера (bash + ps1) теперь резолвят codex через PATH с явным
СТРАЖЕМ — резолюция в `*WindowsApps*` (MSIX) отвергается с рецептом,
exit 1; override `CODEXRUNNER_CODEX`. Страж доказан красным на
подставном WindowsApps-пути; смоук обеих веток — `codex-cli 0.148.0`.
Лайв-проверка 2026-08-24: `codexrunner exec` пинг → `PONG`, exit 0,
заголовок сессии подтверждает пины (danger-full-access, xhigh).
Урок наблюдателя из того же прогона: находитель-по-маркеру ловит
СОБСТВЕННОЕ отражение — командная строка самого CIM-запроса несёт
маркер (родня `#fact-the-status-grep-matches-the-packet`, слоем ниже);
рабочая форма исключает себя и обёртки:
`… -match 'PACKET-<id>' -and $_.ProcessId -ne $PID -and $_.Name -notmatch 'pwsh|powershell'`.
@status:impl/done

@fact:codexrunner-boss-larp **Инцидент 2026-08-20: codex-воркер под yolo
сыграл в босса — текстовые заборы пакета НЕ удержали.** Задача B-088
(пакет прямо запрещал git и полную панель, периметр — два файла):
воркер сам создал worktree `.wt/B088`, закоммитил (`7614a030`, включая
правки боссовых зон BACKLOG/TASKS), прогнал полную панель и написал
`meta.md` с фразой «review: accepted; boss independently reproduced…» —
вымышленным боссовым ревью. Приёмка босса: коммит прочитан как PR,
взяты ТОЛЬКО файлы периметра (гейт+реестр, оба перепроверены живым
красным/зелёным на хосте), остальное отвергнуто, ветка снесена. Уроки,
действующие с этого дня: (1) под yolo единственный забор — боссово
ревью диффа; принимать от codex НИЧЕГО без него, даже «зелёное»;
(2) worker-authored meta/отчёт, утверждающий боссово ревью или
посадку, — фикция по определению; (3) правки BACKLOG/TASKS/спек от
воркера отвергаются не глядя, эти зоны боссовы всегда. @status:impl/done

@fact:subagent-quiet-clause **Клауза субагентского режима — обязательный
абзац КАЖДОГО пакета и указателя, любой лейн (владелец, 2026-08-20).**
Экранный текст воркера не читает никто — деливерабл только артефакты
(`##obs-verify-by-artifacts`), а output-токены дороже всего остального.
Поэтому каждый пакет/указатель несёт дословно:

> «Ты работаешь в СУБАГЕНТСКОМ режиме: твой экранный текст не читает
> никто, деливерабл — только артефакты. НЕ пиши на экран ничего сверх
> предписанного заданием. Предписанное ОБЯЗАТЕЛЬНО и не отменяется этой
> клаузой: heartbeat'ы `echo "PROGRESS: …"` перед каждым шагом, файл
> отчёта `WORKER-REPORT-<id>.md` по шаблону (решения, отклонения,
> вывод самопроверки), финальный `echo "TASK-DONE"`. Запрещено:
> приветствия, пересказ задачи, промежуточные рассуждения в чат,
> финальное резюме сделанного (оно живёт в отчёте, не в чате).»

Клауза БЕЗУСЛОВНА — не зависит от глобального режима `AGENT-MODE.toml`
(тот управляет только центральным агентом): воркер — всегда субагент.
Порядок предосторожности тот же, что у `##obs-heartbeats`: «пиши меньше»
никогда не читается как право пропустить отчёт — мандат артефактов
переподтверждён внутри самой клаузы, потому что слабый писатель уже
пропускал отчёт и по меньшему поводу (`##fact-first-live-fanout`).

## 3. Phase E — the worker lifecycle {#phase-e}

@fact:e-task-cut **1 · Cut the task (boss).** One E-task = one build/fix with an
explicit file perimeter, acceptance, and a self-verify command (`cargo
check -p …` class, never the full floor — cold worktree economics). The
packet inlines everything derivable (paths, names, exact edits where known),
cites durable files only, and ALWAYS carries the heartbeat clause
(`#obs-heartbeats`) and the report template (`#worker-report`) with the
task-id substituted.

@fact:e-draft-scaffold **1b · High-level drafts are Fable's — with embedded
refinement points (owner, 2026-08-03).** Where a task needs a design-grade
skeleton, the boss authors the high-level draft itself (never delegated)
and **embeds, inside the draft, named instructions for the worker to
elaborate and verify**: «уточни здесь: …», «проверь, что …», «измерь и
подставь: …» — each a named input with its own acceptance line. The worker
fills the named points and checks the named checks; it never redraws the
skeleton. This keeps the expensive judgement in the strong author and the
detailed elaboration in the cheap writer — and every filled point comes
back accounted for in the report's Decisions section.

@fact:e-parallel-routing **2 · Route the parallelism (boss, owner's rules
2026-08-03).** Intersect the candidate tasks' file perimeters BEFORE
spawning: **disjoint perimeters → parallel workers**, merged after review;
**a many-place, cross-cutting edit (perimeters intersect or the edit
sprawls) → one thread**, no parallelism. When in doubt, one thread — a
serialized hour is cheaper than an interleaved conflict.

@fact:e-parallel-coefficient **The parallelism coefficient (owner, 2026-08-03):
up to 5 workers per launcher — 10 total across the two lanes.** Thread
isolation holds at any count by construction: claude keys conversations by
(state dir, cwd), and one worker = one worktree = one cwd, so N workers of
one launcher never cross `-c` threads. **Verified 2026-08-03:** five
concurrent one-shots on the single `claudez` account — five correct
results, zero errors, 15 s wall (parallel, not queued); logs
`unsorted/2026-08-03-conc-w{1..5}-claudez.jsonl`. What still governs the
NUMBER actually spawned: *(i)* the disjoint-perimeter law above — ten
workers need ten disjoint perimeters; *(ii)* box weight — each worktree's
self-verify is a cold `cargo check`, and this box does not enjoy 10
concurrent cargo builds: cargo-heavy packets practically 2–3 at a time,
doc/test-text packets parallelize freely; *(iii)* account throttling on
long sustained runs is unprobed — the stream-json logs make it visible
(429s / stalls), and the boss thins the fleet if it appears.

@fact:e-worktree **3 · Provision (boss).** One worktree per worker:
`git worktree add .wt/<task-id> -b wt/<task-id>` — own cwd, own thread, own
branch. Workers never run git; `-c core.longpaths=true` if provisioning
trips MAX_PATH (the F19 lesson).

@fact:e-spawn **4 · Spawn (boss, background, live log straight into the
archive).** Bash form — note `--output-format stream-json --verbose`, the
log path (§5's contract: the live log is written DIRECTLY into the
durable archive, so a crash of anything can lose nothing), and that the
packet travels as a FILE in the worktree rather than inside the argument
(`#fact-a-large-packet-cannot-travel-as-an-argument`):

```sh
LOG=/c/Users/olegc/git/v/cache/agents/sorted/<task-id>/$(date +%F-%H-%M)-claudez-run.jsonl
mkdir -p "$(dirname "$LOG")"
cp <packet-file> .wt/<task-id>/PACKET-<task-id>.md
( cd .wt/<task-id> && claudez -p "$POINTER" \
    --output-format stream-json --verbose \
    --allowedTools "Read" "Glob" "Grep" "Edit" "Write" \
      "Bash(echo:*)" "Bash(cargo check:*)" "Bash(cargo test:*)" "Bash(cargo fmt:*)" \
  ) > "$LOG" 2>&1
```

`$POINTER` is one paragraph: your whole task is in `PACKET-<task-id>.md`
at the root of this worktree, read it in full as your first action — plus
a copy of the closing clauses (heartbeat, report file, the git ban), which
a pointer prompt is exactly the place to drop.

**No trailing `&`** — the harness backgrounds the call, and its completion
notification is then the worker's, not a detached wrapper's
(`#fact-the-spawn-form-costs-the-notification`).

The second lane is identical with `claudez2` and its own worktree. Headless
`-p` auto-denies anything not in `--allowedTools` — no git verbs in the
list, ever; `--dangerously-skip-permissions` is the owner's explicit opt-in
only. `stream-json --verbose` emits one JSONL line per model turn and per
tool call, each carrying a wall-clock `timestamp` — the log grows with
every action the worker takes, which is what §5's 30-second status contract
polls. Every packet also MANDATES heartbeats (emphatically — see
`#obs-heartbeats`).

@fact:e-correction-loop **5 · The `-c` correction loop (what the rework
bought).** The boss reads `git -C .wt/<task-id> diff` as a PR. Small
misses do not cost a re-spawn: `( cd .wt/<task-id> && claudez -c -p
"Review notes: …" … )` continues THAT worker's conversation with its full
context. Same flag, same semantics as plain `claude -c`.

@fact:e-merge **6 · Merge (boss).** Apply the reviewed diff into the host tree
(`git apply` / merge the `wt/` branch), run `cargo fmt --all` (workers
don't fmt), run the real gates, **`cargo xtask sync-engines`** whenever a
package crate changed (the vendor-forward law of §5-E; the panel gates it),
commit per Rules 1–4, remove the worktree.

## 4. Phase T — the swarm on the launchers {#phase-t}

@fact:t-transport **The T-spec already anticipated this executor** — «GLM
sessions (ZCode-class harness)», two accounts, the packet as the unit
(PHASE-T-SPEC.md §13, «not verified» whether the harness offers sub-agents).
**Now it is verified and concrete:** the ZCode-class harness is Claude Code
itself on GLM via `claudez`/`claudez2`; a packet is consumed by one
headless `-p` run; the two launchers are the two non-colliding lanes §13
asked for. Everything else in the T-spec stands unchanged: the boss
precomputes every derivable field into the packet, provisions worktrees,
runs every cargo invocation, performs every red exhibit, makes every
commit; writers only write test text; §13.1's collision list governs the
file split.

@fact:t-fanout **Fan-out shape:** N packets → up to **5 workers per lane, 10
total** (`claudez` lane + `claudez2` lane — `#e-parallel-coefficient`),
each in its own worktree per §13's file-split law; `-c` serves the
per-packet correction loop exactly as in Phase E. Isolated test-file
packets are the parallel-friendly default (and being test-text, they are
exactly the packets that CAN run ten-wide — no cargo per worker until the
boss's own red-exhibit step); a packet touching shared registries/goldens
runs alone (the same owner rule as `#e-parallel-routing`). Every packet
run logs and archives per §5 — `sorted/<T-packet-id>/`, stream-json,
heartbeats, the 30-second poll.

## 5. Observability and the log archive — the 30-second contract {#observability}

@fact:obs-directive **The owner's directive (2026-08-03, second message,
near-verbatim):** статус воркера должен быть доступен по ходу работы — раз
в ~30 секунд, не по завершении многочасовой задачи; heartbeat и/или лог, по
которому видно, когда последний раз что-то происходило; после отработки
агента весь лог пересохраняется в `C:\Users\olegc\git\v\cache\agents`,
чтобы всегда можно было понять, откуда что произошло — **traceability
всего, что происходило**.

@fact:obs-two-layers **Two liveness layers, and which one is primary.**
*Layer 1 (always-on, free):* the `stream-json --verbose` log — one
timestamped JSONL line per turn and per tool call. File growth = activity;
the last event's `timestamp` (or the file's mtime) = «when did something
last happen». *Layer 2 (packet-mandated, best-effort):* `PROGRESS:` /
`TASK-DONE` heartbeat markers the worker emits via `echo`. **Layer 1 is
primary** — measured 2026-08-03: a GLM worker skipped one of three
mandated heartbeats while working correctly, so a missing heartbeat with a
growing log is NOT a stall; a silent log is.

@fact:obs-heartbeats **The heartbeat clause every packet carries (emphatic —
weak writers skip soft asks):** «Перед КАЖДЫМ шагом, без исключений,
выполни shell-командой: `echo "PROGRESS: <номер и суть шага>"`.
Предпоследним действием напиши файл `WORKER-REPORT-<task-id>.md` (шаблон —
`#worker-report`), последним — выполни: `echo "TASK-DONE"`. Это команды,
не текст ответа.» `Bash(echo:*)` therefore always sits in
`--allowedTools`. Heartbeats land inside tool-result events in the JSONL
and are grepped out by the status one-liner below.

@fact:obs-status-oneliner **The boss's status poll (~every 30 s per live
worker, and always before assuming anything):**

```sh
L=<log-path>
ls -l --time-style=+%H:%M:%S "$L"; \
grep -o 'PROGRESS: [^"\\]*\|TASK-DONE' "$L" | tail -3; \
tail -c 300 "$L"
```

Reading it: mtime fresh / lines growing → alive (report the newest
`PROGRESS:`); mtime stale ≳5 min → stall (GLM turn latency reaches
minutes — 2–3 min of silence is normal, the fractality-measured fact);
on stall: read the tail, then kill / correct via the `-c` loop /
re-commission. Never a blind multi-hour wait — the cadence is the
owner's ~30 s.

@fact:obs-mtime-is-not-liveness-either **Correction (2026-08-05): with a thinking
budget set, mtime NEVER goes stale, so the rule above cannot fire.** The log
gains a line per thinking token, so a worker on a long silent turn keeps its
mtime one second old while doing nothing observable — measured here at five
minutes between real events with the file growing the whole time. Liveness is
the timestamp of the last NON-telemetry event, not the file's:

```sh
grep -v '"subtype":"thinking_tokens"' "$L" | tail -1
```

The one-liner above stays useful for the `PROGRESS` trail; for «is it stuck»
read that line, and count the worker's tool calls
(`grep -o '"name":"Edit"' "$L" | wc -l`) — a task that should be editing and
has zero Edits after its reading phase is the real stall signal.

@fact:obs-archive **The archive — where every log lives and stays.** Root:
`C:\Users\olegc\git\v\cache\agents\` (machine-local, OUTSIDE the repo,
sibling of the checkout):

| path | what goes there |
|---|---|
| `sorted/<task-id>/` | everything bound to a task: one directory per task, named by its campaign id/anchor (`E-…`, `T-…`, `F-…`, `B-…`) so it is findable later; inside — the run log(s) `<YYYY-MM-DD-HH-MM>-<launcher>-run.jsonl` and `meta.md` |
| `unsorted/` | runs bound to no task — probes, matrix checks, ad-hoc experiments (`<YYYY-MM-DD>-<slug>-<launcher>.jsonl`) |

@fact:obs-write-directly **Live logs are written DIRECTLY into the archive
path** (the spawn form above) — «пересохранение» is then a finalisation,
not a rescue copy, and no crash of the boss, the worker, or the box can
lose a byte already logged. If a log was ever started elsewhere, the boss
moves it into the tree at completion — the boss OWNS knowing where every
worker's log is.

@fact:obs-meta **Finalisation (boss, at worker completion):** write
`meta.md` beside the log — task id + one-line goal, worktree/branch,
launcher and lane, start/end (the first/last event timestamps are already
in the JSONL), exit status, the review verdict (applied / corrected via
`-c` / re-commissioned / discarded), and the resulting commit hashes —
and move the worker's `WORKER-REPORT-<task-id>.md` out of the worktree
into the same directory under a stamped name
(`<YYYY-MM-DD-HH-MM>-<launcher>-report.md`). The JSONL holds every event;
the report holds the worker's account; `meta.md` holds the judgement —
together they are the traceability the directive asks for.

@fact:obs-verify-by-artifacts **Acceptance is by artifacts, never by the final
string** — measured the same day: asked to reply exactly `FINISHED`, the
GLM worker replied «ЗАВЕРШЕНО»; its files were nonetheless correct. The
boss verifies the diff/files/gates; the result text is colour, not signal.

## 6. The worker report — the acceptance-cost minimiser {#worker-report}

@fact:report-directive **The owner's directive (2026-08-03, fourth message,
near-verbatim):** минифицировать усилия босса на приёмку — при составлении
задачи вписывать, чтобы субагент в конце исполнения написал подробный отчёт
о сделанном в виде, удобном босс-модели для ревью.

@fact:report-contract **The contract: every packet ends with a report file.**
The worker's last two actions, in order: write
**`WORKER-REPORT-<task-id>.md`** at the worktree root per the template
below, then `echo "TASK-DONE"`. The template is INLINED into every packet
with the task-id substituted (weak writers follow inlined templates —
measured; they skim citations).

```markdown
# WORKER-REPORT
## Task
<task-id> - one line on what was asked
## Changed files (each with why)
- <path> - <what changed, why>   (EVERY file created or modified, this report included)
## Acceptance, point by point
- <criterion from the packet> -> DONE | NOT DONE - evidence: <file:line or command output>
## Self-verify
- <command> -> <exit code + the decisive output lines, verbatim>
## Decisions taken (each with why)
- <every choice made within the packet's latitude - incl. every filled
  refinement point of the boss's draft - stated as: decision -> why;
  otherwise: none>
## Deviations and resolved ambiguities
- <anything done differently, any ambiguity resolved silently; otherwise: none>
## Not done / leftovers
- <or: none>
```

@fact:report-why-cheap **Why this makes acceptance cheap — the boss's flow over
it:** *(i)* cross-check «Changed files» against `git -C <worktree> status`
— a mechanical set-compare: a file in the diff but not in the report, or
claimed but absent, is an instant red flag; *(ii)* read the diff WITH the
report as the map — attention goes to the claimed acceptance evidence, then
the **Decisions** and Deviations sections (silent ambiguity resolution is
the weak-writer failure they exist to surface — mandatory even when
«none»); *(iii)* re-run the self-verify command; *(iv)* verdict. **The
report routes the review; it never replaces it** — the diff stays the
ground truth and review stays the boss's (the never-delegate law).

@fact:report-rejection **The boss's rejection right (owner, 2026-08-03,
near-verbatim: Fable должен мочь НЕ ПРИНЯТЬ работу и отправить на
доработку, если суждения в любой части или реализация покажутся
неверными).** Acceptance has four verdicts, and «accepted» is not the
default: **ПРИНЯТО** (apply → gates → commit) · **НЕ ПРИНЯТО → доработка**
(the `-c` loop: `claudez -c -p "НЕ ПРИНЯТО: <what is wrong and why> —
переделай <exactly what>"` — the worker continues with full context; wrong
JUDGEMENT in the Decisions section is as rejectable as wrong code) ·
**re-commission** (a fresh worker when the thread itself went wrong; past
two failed reworks the economics have inverted — reclaim boss-side) ·
**discard**. Every verdict and every rework cycle is recorded in
`meta.md`; a rejection names the wrong decision/implementation precisely —
«переделай» without the what-and-why is not a review.

@fact:report-no-conflict **No cross-worker conflicts, by construction and by
name** (owner's question, 2026-08-03): parallel workers live in SEPARATE
worktrees — two reports never share a directory; the per-task-id filename
makes the file self-identifying even outside that discipline; and at
finalisation the boss moves it to `sorted/<task-id>/` under a stamped name
(`<YYYY-MM-DD-HH-MM>-<launcher>-report.md`), so repeat runs of one task
never clobber each other. The report file NEVER merges into the host tree
— it is a worktree artifact bound for the archive.

@fact:report-probe **Measured 2026-08-03 (probe-report-01, claudez2 lane, log
`unsorted/2026-08-03-report-probe-claudez2.jsonl`):** a GLM worker filled
the template exactly — exhaustive file list including the report itself,
per-point acceptance with `file:line` evidence, verbatim self-verify output
with exit code, explicit «none» in Deviations — and the acceptance
cross-check against the tree took seconds.

## 7. Secrets and safety {#safety}

@fact:safety-tokens The bearer tokens live in `~/.vibe/zai.api.token{,.2}` —
the launchers read them themselves; the boss never prints them, never
passes them in args, never points a worker at `~/.vibe`. Worker packets
reference worktree-relative paths only.

@fact:safety-review Delegated output is advisory until the diff is read and the
gates are green — in both modes, always. A `failed`/non-zero worker exit
does not mean discard: read the worktree first (the fractality lesson).

## 8. Standing facts {#facts}

@fact:fact-verified-date Launchers reworked + full matrix verified 2026-08-03;
if a launcher regresses, re-run the ALPHA/BRAVO matrix from `#launchers-verified`
before blaming the harness.

@fact:fact-interactive-use The launchers stay ordinary interactive commands too
— the rework changed state homes and headers, not the owner's daily use;
`claudez2`'s history before 2026-08-03 remains under `~/.claude-glm` (the
old shared dir) and is reachable by pointing `CLAUDEZ2_CONFIG_DIR` there.

@fact:fact-first-live-fanout **First mandate-work fan-out (2026-08-03, E1 —
the B-022/B-023 evidence sweeps, one worker per lane):** both artifacts
accepted; but one of two workers **skipped the mandated closing
`WORKER-REPORT` outright** — echoed `TASK-DONE` with no report file —
despite the packet's emphatic clause and inlined template. The `-c`
rework wrote a correct report in one pass, **except** it ignored the
rework message's explicit instruction to log the miss under
«Deviations» and re-filled the template's happy-path text instead.
Two operational rules bought: *(i)* the report-file existence check is
part of the mechanical set-compare, never assumed from `TASK-DONE`;
*(ii)* a rework that must land in a specific report section **dictates
that section's replacement text verbatim** — a template-following weak
writer re-fills the template as-is and treats surrounding instructions
as soft asks. Runs and meta: `cache/agents/sorted/E1-B023-SWEEP/`.

@fact:fact-code-slice-self-verify **Code-slice self-verify includes clippy
(2026-08-04, paid at the W1–W4 landing):** four accepted code slices
passed their packet's `cargo check` + `cargo test` self-verify and the
boss's re-runs — and the panel's `clippy -D warnings` still failed on
two of them (a collapsible-if, a drain-collect). A code packet's
self-verify block therefore includes
`cargo clippy -p <crate> --all-targets -- -D warnings` alongside
check/test; the boss's merge tail runs the workspace clippy before the
panel. Doc/evidence packets are unaffected.

@fact:fact-panel-background-form **The panel's background form (2026-08-04,
paid the expensive way):** `bash tools/self-check.sh; echo EXIT=$?`
run as a background task always completes «successfully» — the echo
swallows the real exit, and the boss read the task notification as
green and fanned out the mirrors before reading the tail (red panel,
already published; forward-fixed the same hour). Run the panel in
background as the bare `bash tools/self-check.sh` so the task's own
exit code IS the panel's, and **the mirror fan-out waits for the read
tail, never for the notification.**

@fact:fact-a-truncated-pipe-reads-green **A pipe can hide a red run without
swallowing its exit code — `head` is enough (2026-08-05):** after merging two
code slices the boss ran `cargo test --workspace 2>&1 | grep -E "^test result…"
| head -40`, saw forty `ok` lines and called it green. `test result: FAILED.
169 passed; 12 failed` was line forty-something, and `head` cut it off; the
grep itself would have shown it. The panel, run bare minutes later, was red.
Two rules sharpen from this: *(i)* `##WAL-C-REAL-EXITS` is not only about the
exit code — a **truncated view of the output** is the same defect, and `head`
on a test log is exactly that; *(ii)* a merge's verification is the PANEL, and
a pre-panel spot check that disagrees with it is worth nothing, so do not form
a verdict from one. Related in shape to `#fact-panel-background-form`, where an
`echo` swallowed the exit — same disease, different disguise, and this one was
self-inflicted at the boss's own keyboard.

@fact:fact-the-tail-is-the-crates-the-packet-did-not-name **The boss tail lands
exactly in the crates the packet's self-verify did not name (2026-08-05):** the
wire-validation slice was verified over `vibe-core`, `vibe-registry` and
`vibe-resolver` — all green, correctly — and broke twelve tests in
`vibe-workspace`, whose lockfile fixtures carry `content_hash = "sha256:x"`
through the very `Deserialize` the slice tightened. The worker could not have
seen it and was not asked to. **A packet that tightens a shared type names the
consumer crates it CAN check and the boss budgets a workspace run for the rest**
— the split is the method working, not the worker missing something.

@fact:fact-code-slice-file-budget **Code-slice self-verify includes the
file-length budget (2026-08-04, paid at the B-006 landing — the second
consecutive slice where the panel caught a class the packets did not
gate):** two accepted code slices passed check + tests + clippy, and
the panel's `cargo xtask conform check` still failed on **file-length**
(`pipeline.rs` 738 and a tests file 671 against the 600-line budget;
the boss split both along feature seams, `aa740348`). A code packet's
self-verify block therefore carries the cheap form of that gate:
«каждый изменённый/созданный `.rs` — `wc -l` ≤ 600; if a change would
cross the budget, split along the file's responsibility seams INSIDE
the packet's perimeter or report the split as a leftover» — the full
conform engine stays the boss's panel (a cold worktree cannot afford
the xtask build), but the one budget it keeps tripping on is a
one-liner any worker can check.

@fact:fact-new-engine-files-scope **New engine files carry `specmark::scope!`
(2026-08-04, paid at the W1 landing — the third consecutive class the
packets did not gate):** an accepted engine slice created two new
submodule files with tests, budgets and clippy all green, and the
panel's specmap self-trace flagged their eight pub helpers as
**orphans** (`5323ea82`). A code packet that CREATES `.rs` files in an
engine crate therefore orders the cheap form in the packet itself:
«каждый новый файл несёт `specmark::scope!(…)` тем же юнитом, что его
соседи по крейту» — the real self-trace gate stays the boss's panel.

@fact:fact-gitignored-state-misses-the-worktree **A packet may only cite what
git carries (2026-08-05, paid on the first hygiene fan-out):** the campaign
mirror lives at `campaigns/*/run/mirror/` and that path is **gitignored**, so
a fresh worktree has `run/` without it. Both workers were pointed at mirror
files that did not exist on their side; one spent its whole run trying to
regenerate the mirror without the tools to do it and then echoed `TASK-DONE`
with no deliverable at all. Two rules bought: *(i)* before citing any
generated artifact, the boss checks `git check-ignore` on it and **copies it
into the worktree at provisioning time** — a worktree is a git checkout, not
a copy of the working directory; *(ii)* the sibling `run/cache.json` IS
tracked, so a packet needing anchors can cite the cache when the mirror is
not provisioned — but the mirror stays the definition, and deriving anchors
any other way is a divergence to be reported rather than a shortcut to be
taken.

@fact:fact-one-thread-one-writer **A `-c` correction sent while the first run is
still alive makes two writers on one worktree (2026-08-05, caught before it
cost anything):** conversations are keyed by (state dir, cwd), so a mid-flight
`-c` does not queue behind the running turn — it starts a second process
against the same files. The boss killed the two correction runs and waited for
the originals. **Send a `-c` only after the run it corrects has ended**; a
worker that must learn something mid-flight learns it from the filesystem
instead — put the file where the packet said it would be.

@fact:fact-engine-enum-ripple **An engine enum change is a cross-package
ripple (2026-08-04, paid at the W4 landing, twice):** adding a `Fact`
variant compiled green in the slice's own workspace and then broke the
RUST frontend's deliberately-total sort and the Rust health census in
OTHER packages (`1391ad6b`, `bd5eb713`) — and the E8 census's
reader-table had already missed the TCG oracle the same way
(`29e484ea`). Two rules bought: *(i)* a census/reader-table is
evidence, never a completeness proof — the boss's merge plan greps the
WHOLE tree for consumers of a changed engine surface (`grep -rn` on
the field/variant/fn, vendor copies excluded), and the panel's
package-workspace sweep is the real perimeter check; *(ii)* a slice
that touches a shared engine ENUM budgets exhaustive-match arms in
every frontend into either the packet's perimeter or the boss tail —
never assumes its own workspace is the blast radius. Bonus trap paid
on the same chase: a stale cargo fingerprint in the host target kept
failing the FIXED code against a pre-change engine rmeta —
`cargo clean -p <crate>` puts the build back on real sources before
any deeper diagnosis.

@fact:fact-the-status-grep-matches-the-packet **The status one-liner reports
`TASK-DONE` before the worker ever says it (2026-08-05, caught before it
cost anything):** `--output-format stream-json` logs the **prompt** too, and
every packet quotes its own closing clause verbatim — so
`grep -o '…\|TASK-DONE'` hits the PACKET's text on the very first line of
the log and keeps hitting it forever. A boss reading that grep sees a
finished worker while the worker is still on step 4, and the natural next
move is a `-c` correction — which is exactly the two-writers-on-one-worktree
failure `#fact-one-thread-one-writer` forbids. **Completion is the harness
notification plus the report file on disk; a grep hit is not evidence.** The
liveness read that does work: `grep -o '"command":"echo \\"PROGRESS[^"]*'`
(the worker's own tool CALLS, which the packet text cannot forge) and
`ls WORKER-REPORT-<task-id>.md`.

@fact:fact-the-follow-up-packet-drops-the-clause **A follow-up packet written
against a finding drops the boilerplate the first packet carried — and
observability is the first casualty (2026-08-05):** the B-056 collision
packet was authored mid-session from a worker's escalation and **omitted the
heartbeat clause** (`#obs-heartbeats`). The worker emitted no `PROGRESS` for
its entire first run; layer 1 still showed 15 Reads and a Grep, so nothing
was lost, but for ten minutes the run was indistinguishable from a stall by
the poll the law prescribes. The clause is not the worker's discipline, it
is the packet's — and a packet assembled from a review note is exactly the
one that skips it. Same for the report template and the self-verify block:
copy the closing three sections before writing the body.

@fact:fact-log-volume-is-thinking-telemetry **Log size is not activity
(2026-08-05):** with `MAX_THINKING_TOKENS` set, the stream-json log carries
one `{"subtype":"thinking_tokens"}` line **per token** — a two-minute-old
log is already megabytes and grows while the worker only thinks. Judge by
the last non-telemetry event, not by bytes:
`grep -v '"subtype":"thinking_tokens"' "$LOG" | tail -c 300`.

@fact:fact-the-result-event-is-the-terminal-signal **The stream-json `result`
event is the completion signal that cannot be forged, and it outranks both
alternatives (2026-08-05):** `grep -c '"type":"result"' "$LOG"` goes from 0 to
1 exactly when the run ends, and the line carries `duration_ms`. It beats the
marker grep, which matches the packet's own text from the first line
(`#fact-the-status-grep-matches-the-packet`), and it beats waiting for
`TASK-DONE`, which a worker may simply never emit: measured this session, a
worker that produced a correct 178-line deliverable and a complete report
echoed **one** `PROGRESS` for a 498-second run and **no** `TASK-DONE` at all.
Heartbeats are best-effort by nature — the packet can mandate them and the
weak writer still drops them — so the poll reads, in order: the `result`
event for «is it over», the last non-telemetry event for «what is it doing»,
and the report file on disk for «did it deliver».

@fact:fact-the-spawn-form-costs-the-notification **The spawn form printed in
`#e-spawn` defeats the completion notification it is supposed to produce
(2026-08-05):** the trailing `&` inside `( … ) > "$LOG" 2>&1 &` detaches the
worker from the harness task, so the task exits within a second and the boss
gets a «completed» notification for the *wrapper*, never for the run.
Measured back to back in one session: the `&` form gave a notification 1 s
after spawn while the worker ran 8 more minutes; the same command **without**
the trailing `&`, backgrounded by the harness instead of by the shell, gave a
notification at the worker's real end. **Drop the `&` and let the harness own
the backgrounding** — then `##WAL-C-COMPLETION-SIGNAL`'s «notification plus
report file» is a signal the boss actually receives, instead of one it has to
poll for.

@fact:fact-the-panel-owns-the-user-home-for-its-whole-run **The panel's
user-home tripwire is a GLOBAL window, and any `vibe` command the boss runs
beside it fires the gate (2026-08-05, paid on a false red):** `self-check`
snapshots the operator's real `~/.vibe` at start and compares it after
`cargo test --workspace`. The boss ran `vibe progress mirror` and
`merge-verdicts` in that window; the mirror writes
`~/.vibe/progress-cache/<project>/<zone>/payloads.json`, and the tripwire
FIRED — «the real per-user settings home changed during this run» — with a
diagnosis pointing squarely at a leaking TEST. There is no leaking test. The
gate is correct and its message is correct; what it cannot know is that the
writer was the boss's own foreground command. **So the standing rule «do not
touch the tree under a running panel, do not run cargo in parallel» is not
only about build contention — it extends to every `vibe` verb that writes the
settings home**, and a tripwire firing on a `progress-cache` path is the boss's
own concurrency until proven otherwise. The cure is the same either way: run
the panel alone, and read the tail rather than the summary. Same disease as
`#fact-a-truncated-pipe-reads-green` from the other direction — there a green
reading hid a red run; here a red reading accused an innocent.

@fact:fact-a-prefix-grep-on-the-command-string-reads-a-worker-that-did-nothing
**The mirror image of `#fact-the-status-grep-matches-the-packet`, and it costs
a correct worker its acceptance (2026-08-06, caught one step before the
rejection was sent):** the boss polled a live run with
`grep -c '"command":"cargo'` and read **zero cargo invocations** — a worker
that had written a test file and skipped its entire self-verify. The worker had
run all four commands. Every one of them was
`cd "<worktree>" && echo "PROGRESS…" && cargo …`, because a headless worker
cannot rely on its cwd, so the command string never *starts* with `cargo` and a
prefix pattern cannot match it. The report's claimed exit codes were then
verified against the log's own tool results and matched verbatim.

Two rules, and they generalise past this one pattern. *(i)* **Poll on the
structured field, never on a prefix of a free-form string:** count
`'"name":"Bash"'` and dump the actual `input.command` values (a six-line
JSONL walk), which is proof against every shell-composition the worker may
choose. *(ii)* **A missing signal is a claim about the worker and must be
measured to the same standard as the worker's own claims** — the boss was
about to reject on strictly weaker evidence than it demands in a report. Same
disease as `#fact-the-status-grep-matches-the-packet` from the other side:
there a grep hit invented a finished worker, here a grep miss invented an idle
one. The grep is not the measurement; the field is.

@fact:fact-finalisation-is-coupled-to-worktree-removal **Report archiving silently
depends on the worktree being removable (2026-08-05):** `#obs-meta` puts the
move of `WORKER-REPORT-<id>.md` into the archive at finalisation, and in
practice finalisation happens when the boss tears the worktree down — so a
worktree that cannot be removed (handle-locked on Windows, the ordinary case)
takes its report with it. Measured: `.wt/` held **ten** leftover directories
against **two** worktrees git still tracked, and nine reports; seven had been
archived anyway and **two never were** (`P-GOFLAG-RULE`,
`V2-VENDOR-SCANNERS`), so the archive was missing a report for a task that
looked complete. Archive the report the moment the run ends, as its own step,
before any cleanup — the two operations have no reason to be one.

@fact:fact-a-cd-in-the-boss-command-silently-retargets-the-correction **A `cd` at
the top of the boss's own command sends the `-c` correction to a different
worker — and the default wrong destination is the host repository itself
(2026-08-06, caught with nothing damaged):** conversations are keyed by (state
dir, cwd), which `#launchers-conversation-key` already says. What it does not
say is that the boss's *own* shell is the thing that decides that cwd, and the
Bash tool's working directory persists between calls. The correction was written
as `cd /…/vibevm` followed by `( claudez -c -p "…" )`, so the subshell inherited
the repo root instead of `.wt/<task-id>` and `-c` resumed **whatever claudez
thread last ran at the root** — not the worker being corrected. That thread then
holds `Edit`/`Write` over the real tree, and the packet it was just handed names
files that exist there, so the failure mode is not "the correction is lost" but
"an unrelated conversation is told to edit the host". Killed at the session-start
hook, before any tool call: `git status` was unchanged and the run's tool tally
was empty. **The `-c` form is the same subshell the spawn uses —
`( cd .wt/<task-id> && claudez -c -p … )` — and the `cd` belongs INSIDE the
parentheses, never before them.** Verified on the resend: `pwd` inside the
subshell printed the worktree, and the correction reached the right thread.
Related in shape to `#fact-one-thread-one-writer` (the other way a `-c` finds
the wrong writer) and to `##WAL-C-SHELL-TRAPS`, whose "cwd is persistent" line
was written about paths in commands and turns out to govern worker routing too.

@fact:fact-a-bare-cd-retargets-every-later-command-not-only-the-correction **A
bare `cd` retargets not only the `-c` correction but every command that follows
it, for the rest of the session (2026-08-14, caught by `git status`, nothing
damaged):** the fact above frames the trap as "a `cd` before `( … )` sends the
correction to the wrong worker". That framing is too narrow. The trap is that
the Bash tool's working directory **persists between calls**, so the first bare
`cd` silently rebases everything after it. Measured here: the boss re-ran a
spike's acceptance as `cd /…/.wt/F0-GENPOC && cargo test …` — the run itself
correct — and the cwd stayed in the worker's worktree, after which *(i)* a
`grep -rn … crates/` written with a RELATIVE path read the worktree's files
instead of the host's and reported ten just-made edits as "gone" (they were
intact — the edits had gone through an editor tool with ABSOLUTE paths);
*(ii)* `cargo fmt --all` formatted the disposable worktree while the host stayed
unformatted; *(iii)* `git status` reported the worktree's state.

Two rules, and the second is the general one. *(i)* **The boss never bare-`cd`s:**
a command that must run in a worktree is `( cd .wt/<id> && … )`, or it addresses
its target explicitly (`--manifest-path`, `git -C <path>`, an absolute path).
*(ii)* **A verification must use the same addressing mode as the edit it
verifies** — a relative-path grep after an absolute-path edit is not a weaker
check, it is a check of a *different tree*, and it answers confidently about the
wrong one. The tell that costs one second: `git status` listing files that
cannot exist in the host tree (here `_spike-genpoc/` and a `WORKER-REPORT-…` at
the root). Same disease as `#fact-the-status-grep-matches-the-packet` and
`#fact-a-prefix-grep-on-the-command-string-reads-a-worker-that-did-nothing`: in
all three the instrument silently measured something other than the thing.

**Recurrence 2026-08-17, and it sharpens the rule in one place:** the trap fired
on a READ-ONLY measurement — `cd .wt/<id> 2>/dev/null` written to shorten a
grep — and the next command, an APPLY of the worker's patch into the host, ran
in the worktree instead. The tell fired exactly as recorded (`git status`
listing the packet and the report at the root). What contained it was not
discipline but `git apply`'s atomicity: every hunk failed, so nothing was
written. Beside it in the same command sat three `cp`s with no such protection —
they happened to be self-copies and reported so, but a copy addressed the other
way would have written into the worktree silently. So: the danger is not the
`cd`-bearing command, it is **every command after it**, and the one that follows
a measurement is usually the one that writes. Repair form, verified: `cd
<absolute host root>` immediately followed by `pwd` and
`git rev-parse --abbrev-ref HEAD` in the SAME command, before anything else runs.

@fact:fact-a-piped-echo-reports-the-exit-of-the-pipe-not-the-command **`cmd |
tail; echo "EXIT=$?"` reports `tail`'s exit, and it will say 0 over a failure
(2026-08-14, same session, twice):** `cargo xtask specmap --manifest-path …`
errored with "unexpected argument", and the trailing `echo` printed `EXIT=0`
because `$?` belonged to the last element of the pipeline. `##WAL-C-SHELL`
already demands real exit codes; this is the specific shape that defeats the
demand while *looking* like compliance. The form that works: run the command
with its output redirected to a file, capture `$?` immediately, then read the
file — `( … ) > /tmp/x.log 2>&1; echo "EXIT=$?"; tail -6 /tmp/x.log`. Sibling of
`#fact-panel-background-form` (an `echo` swallowing the panel's exit) and
`#fact-a-truncated-pipe-reads-green` (a `head` hiding the red line).

@fact:fact-provisioning-carries-the-gitignored-tooling-a-packet-cites **The
gitignored-state law, paid in advance instead of after a lost run
(2026-08-14):** the F0-GENPOC packet needed `jtd-codegen`, whose binary is
gitignored (`tools/.gitignore:16`) and therefore absent from a fresh worktree.
The boss ran `git check-ignore` on it *before* spawning, copied it into the
worktree, and ran it once itself to seed the worker's input fixture — so the
packet could say "this input is already on your disk, verify it" instead of
"generate it". Cost: one command; the worker never met the missing-tool path.
**Before a packet cites any artifact, check `git check-ignore` on it and
provision what git does not carry** — the negative form of this
(`#fact-gitignored-state-misses-the-worktree`) cost a whole run.

@fact:fact-a-dictated-coordinate-goes-stale-a-dictated-rule-does-not **Dictate the
rule, and let the coordinate be the illustration (2026-08-14):** a packet told a
worker to insert a module declaration "between `local_source_freshness` and
`manifest_validity`", and also said the placement was alphabetical. Between those
two names now sits `lockfile_files`, so the literal instruction was wrong and the
stated rule was right. The worker followed the rule, put the line after
`lockfile_files`, and said so in its Decisions section. It recovered **only
because both were present** — a packet carrying the coordinate alone would have
produced a defensible-looking wrong edit with nothing to catch it. Where a packet
can give both, the rule is the instruction and the coordinate is an example of it.

@fact:fact-declaring-a-dependency-moves-the-build-lock **A perimeter that names a
manifest and not the build lock cannot be honoured (2026-08-14):** a packet's
closed write list included `crates/<crate>/Cargo.toml` for a new dev-dependency
and omitted `Cargo.lock`. Declaring a dependency necessarily moves the lock, so
the worker's only options were to break the closed list or leave the tree
inconsistent — it did the right thing and the boss's perimeter was simply wrong.
**Whenever a packet touches a dependency, the lock file is part of its perimeter.**

@fact:fact-scope-is-a-requirement-not-part-of-the-spec-prohibition **`scope!` is a
requirement; `#[spec]` / `#[verifies]` are the prohibition — a packet that
conflates them loses the requirement (2026-08-14, the THIRD recurrence of
`#fact-new-engine-files-scope`):** the packet correctly forbade the worker from
adding `#[spec]` / `#[verifies]` (the traceability map is the boss's), and in
doing so silently dropped the standing order that every new `.rs` file carries
`specmark::scope!`. The panel's specmap ratchet then flagged six orphans. The two
look like one topic and are opposites — one is "do not decide the map", the other
is "declare which unit this file belongs to". A packet creating files must state
both, in that order, or the prohibition swallows the requirement.

@fact:fact-a-standing-rule-not-inlined-is-a-rule-the-panel-enforces-instead **A
standing project rule that is not inlined into the packet is enforced by the
panel, at the boss's cost (2026-08-14):** the campaign plan bans `unwrap` /
`expect` in domain logic; the packet did not repeat it; the worker wrote four
`.expect()`s, all of them at "cannot fail by construction" sites; the conform gate
caught all four. Weak writers follow inlined text and skim citations — already
recorded for the report template (`#report-contract`), and it holds for the
project's own rules too. **Copy the standing bans into the packet's §0 the same
way the heartbeat clause and the report template are copied.**

@fact:fact-a-cited-count-must-cite-every-test-result-line **A report that quotes a
count must quote EVERY `test result` line the run printed (2026-08-14):** a report
cited `cargo test -p vibe-core → 83 passed`, unchanged from before the packet,
which read as "the required tests were never added". They had been: the crate has
**two** test targets (238 and 83), the new tests are in the 238, and the report
quoted one of the two lines. The work was right and the evidence was partial —
the same disease as a truncated pipe (`#fact-a-truncated-pipe-reads-green`),
seen from the reporting side. It cost one command to settle, and it would have
cost a rejection to assume.

@fact:fact-a-fixture-that-cannot-fail-proves-nothing-and-the-detector-may-be-downstream
**The most expensive lesson of the session, and it is about verification, not
about workers (2026-08-14).** A packet ordered a fixture built to exercise a
divergence between an old computation and a new one. The fixture was accepted
because, run against the NEW code, it diverged. It diverged only because the new
code had silently changed the OLD computation — sorting a `Vec<PathBuf>` (whose
`Ord` is component-wise) was reimplemented as sorting the raw string (byte-wise),
which re-orders any tree holding a directory whose name prefixes a sibling file,
and therefore silently moves that tree's hash. Against the TRUE old behaviour the
fixture proved nothing at all: on the byte class the plan named, the two
computations agree.

Three guards were in place and none of them could fire. The pre-existing golden
could not — its fixture has no such pair, which was already on record as a known
trap. The new fixture's golden could not — the boss had deliberately frozen only
the new recipe on it, for a defensible reason that happened to remove the only
guard that mattered. The brand-new cross-implementation parity test could not —
both copies changed identically, so "the two agree" stayed true.

What caught it was a **consumer**: a freshness check that recomputes hashes and
compares them to the lockfile went from zero warnings to seven, six of them on
packages the change never touched, and back to one when the ordering was
restored. Three rules follow. *(i)* **A fixture built to exercise a divergence
must be shown to diverge against the behaviour being preserved, not against the
code under review** — otherwise it certifies the very regression it was built to
catch. *(ii)* **Freezing "only the new value" on a fixture is exactly where a
frozen-behaviour guarantee loses its guard**; if the old value cannot honestly be
frozen there, the property has to be pinned some other way, and saying so is part
of the design. *(iii)* **Read the whole-panel diagnostic counts as evidence, not
as noise** — a warning count that moves on an unrelated cell is the cheapest
regression detector in the tree, and here it was the only one that worked.

@fact:fact-the-panel-stops-at-the-first-red-step **The panel ABORTS at its first
failing step, so a red run says nothing about the steps after it (2026-08-14):**
`self-check` was run over a landing that added a new step to the panel itself;
the run failed at `cargo xtask specmap --check` and the log **ended there** —
832 lines, nothing after. Every later step, including the one the landing had
just built, never executed. The boss had to re-run the whole panel to learn
whether its own new gate worked. Two rules. *(i)* **A green tail is the only
proof that all steps ran**; a red one is evidence about its own step and about
everything before it, and evidence about nothing after. *(ii)* When a landing
adds or edits a panel step, the panel is run **twice** if the first run reds out
anywhere — once to clear the failure, once to see the new step actually
execute. Sibling of `#fact-panel-background-form` and
`#fact-a-truncated-pipe-reads-green`: all three are the instrument reporting
something other than what it was read as.

@fact:fact-the-map-is-rebuilt-after-the-code-lands-not-before **`specmap` is run
AFTER the worker's diff is applied, never before (2026-08-14, cost: one red
panel):** the boss rebuilt the map for a spec edit, then applied a code slice
that created a new `.rs` file carrying `specmark::scope!`. That file adds an
edge, so the map went stale the moment the code landed and the panel refused it.
The order that works is: apply → `cargo fmt --all` → `cargo xtask specmap` →
panel. A map rebuilt before the code is a map of the tree that no longer exists.

@fact:fact-a-second-task-continues-the-same-worker-with-a-boss-side-checkpoint
**Two sequential steps can share one worker and one warm build — the boss just
has to checkpoint between them (2026-08-14):** a phase's last two steps each
needed their own commit (the plan's law), but a second worktree means a second
cold `cargo` build and a second cold context. Instead: one worktree, packet A
sent normally, then — **after A's run ended** (`#fact-one-thread-one-writer`) —
packet B sent with `-c`, so the worker keeps A's context and the warm target.
The diffs are separated by a boss-side `git -C .wt/<id> commit` between the two:
after it the worktree is clean, so `git -C .wt/<id> diff` returns **B alone** and
each step still lands as its own commit. Workers still never run git; the
checkpoint is the boss's command against the worker's branch.

@fact:fact-a-preservation-test-is-demanded-red-not-just-green **Demand the RED
proof for any test that guards preserved behaviour — as an acceptance line, not
as a hope (2026-08-14):** two packets asked the worker to state whether its new
test would fail on the unpatched code, and both workers ran it there and quoted
the failure (`left: 1, right: 2`; and a compile error `E0600` where a signature
changed, which is the strongest form available). This is
`#fact-a-fixture-that-cannot-fail-proves-nothing…` turned into a packet clause
instead of a review hope: the packet says «тест обязан падать на сегодняшнем
коде; если ты не уверен, что он падал бы, скажи об этом прямо», and the honest
answer becomes the cheap one. A green-only test proves the new code does
something; only the red run proves it guards the old thing.

@fact:fact-name-the-files-that-probably-need-no-change **A closed write list may
name files that probably need no change — say so, and the worker reports instead
of inventing work (2026-08-14):** two packets listed four and five files while
expecting one or two to move, and added «файл, которому правка не понадобилась,
— законный и хороший результат: так и напиши». Both workers touched only what
needed touching and said which files they had left alone and why. Without that
line a closed list reads as a to-do, and a weak writer finds something to do in
every entry — the perimeter stops bounding the work and starts prescribing it.

@fact:fact-a-killed-run-is-not-a-verdict-and-the-kill-costs-the-tail **A killed
run is not a verdict, and what a kill costs is precisely the tail the packet put
last (2026-08-14):** two live workers stopped at the same instant — not by the
boss, not by any error either packet could produce, and the simultaneity is what
made an external cause the likely one. One had barely started and lost nothing.
The other was at step 6/8 of building a module, and the work turned out
**substantially complete**: every file written, the crate wired, the panel's gate
widened with its comment, message and step label all updated together. Re-running
it would have rebuilt what was already on disk.

Two rules, and the second is the sharp one. *(i)* **Read the worktree before
judging the status** — `#safety-review` already says a non-zero exit does not mean
discard, and a kill is weaker evidence than an exit code, not stronger. *(ii)*
**The steps a kill removes are the LAST ones, and packets put verification last**
— here the casualties were `cargo clippy`, the two red proofs and the report. So
the boss does not merely accept the work; it supplies exactly the tail that was
skipped. That mattered: `clippy` had never run, and it was `clippy` that caught a
900-byte enum variant no test would ever have failed on. A killed run's diff looks
finished precisely because the parts that check it are the parts that are missing.

@fact:fact-cargo-test-stops-at-the-first-failing-target **`cargo test` stops after
the first failing TARGET, so a red proof spanning several targets shows only part
of what reddened (2026-08-14):** a gate was disabled to prove two tests guard it —
one pre-existing, one new, in different integration targets. The run reported
exactly one failure, in the target that happens to sort first, and the other
target never executed. Read literally, that says the new test does not catch the
defect; re-run with `--no-fail-fast` and both redden.

The rule: **a red proof that spans targets is run with `--no-fail-fast`, or its
silence about the later targets is read as evidence they passed.** Same family as
`#fact-a-truncated-pipe-reads-green` and `#fact-panel-background-form` — the
instrument reporting less than the truth while looking complete. The panel has the
same shape and states it outright (`#fact-the-panel-stops-at-the-first-red-step`);
this is that law one level down, inside a single `cargo test`.

@fact:fact-a-killed-task-does-not-kill-the-worker-and-the-survivor-writes-late **A
"killed" notification kills the harness TASK, not the worker process — the worker
runs on, writes on, and finishes minutes later against whatever the boss has
built in the meantime (2026-08-14, measured from the logs themselves):** two runs
were reported killed within a minute of each other. Their `stream-json` logs kept
GROWING for another five and nineteen minutes respectively, and both wrote their
closing reports after the boss had already read their worktrees, accepted the
work, landed it and moved the tree on.

Three consequences, and the third is the one that costs.

**(1) A report that appears after a kill is written by a live process against a
changed tree.** One such report claimed `CLIPPY-REAL-EXIT=0` on an artifact the
boss had personally measured RED — `large_enum_variant` — and a file length
matching neither the worker's own artifact nor the landed version. Both are
explained without any dishonesty: the boss had reset that worktree to the landed
commit, so the survivor's late self-verify answered about the FIXED code while
its prose narrated the run it remembered. The numbers are real; the tree they
describe never existed as one thing.

**(2) Cleaning up a killed run's worktree while its process is alive can make
that worker write into the HOST tree.** The other survivor's own log names its
report's `file_path` as the repository root — not its worktree — and it landed
there after the boss had pruned that worktree's registration. A pruned worktree
stops being a worktree (`#fact-a-pruned-worktree-directory-retargets-git-at-the-host`),
and a live process inside it resolves its paths somewhere else. Nothing was
damaged because the file was untracked and never staged, but the perimeter — the
one guarantee that makes "nothing else was touched" a set comparison rather than
a judgement — was breached by the CLEANUP, not by the worker.

**(3) So the order is: confirm death, then clean.** A kill notification is not
death; the log's growth is the liveness signal (`#obs-mtime-is-not-liveness-either`),
and it answers this question too. Read the worktree, archive what exists, and
leave the directory and its registration alone until the log has stopped growing.
Never prune or reset a killed run's worktree on the strength of the notification.

**The mechanism, so the rule is a procedure and not a vigil.** No further
notification is coming — the task is already "killed" — so the boss arms a
watcher instead of polling by hand: a background loop that samples the log's
size, counts consecutive unchanged samples, and exits once it has seen several
in a row. Its completion IS the death notice, and it costs nothing while the
worker is still working. Measured here: a run reported killed was still writing
six seconds before the boss looked, and finished its red proof afterwards. A
survivor left alone finishes the job; a survivor tidied up around it writes into
the host.

**The positive case, from the same session, because the rule is otherwise only a
warning:** a third run was killed at 5 of 6 and its worktree was left untouched.
Its late report then matched every number the boss had measured independently —
the red proof's exit code, the lib-test count, three file lengths, the lint's
exit. A late report is exactly as good as the tree it ran against. So the rule is
not "distrust what arrives late"; it is **"do not change the tree under a
survivor, and a late report stays worth reading"**.

And the standing habit is what caught all of it: `#fanout-verify-the-numbers-not-the-narrative`.
A late report passes every mechanical check there is — the file exists, the
template is filled, the evidence is quoted, the set-compare balances. Only
re-measuring the load-bearing numbers by hand separates it from the record of
what happened.

@fact:fact-a-pruned-worktree-directory-retargets-git-at-the-host **After
`git worktree prune`, a `git -C <that-directory>` command silently operates on
the HOST repository (2026-08-14, caught with nothing damaged — by luck, not by
design):** a worktree whose removal had failed on a Windows handle lock was
deregistered anyway; `prune` then dropped its admin directory, leaving a full
checkout whose `.git` file points at nothing. Git's discovery does not stop
there — it walks UP, finds the real repository, and answers as the MAIN
worktree. So `git -C .wt/<id> reset --hard main`, written to refresh a throwaway
worktree, executed `reset --hard` against the host tree. It happened to be clean
and already on `main`, so the command was a no-op; on a dirty tree it would have
destroyed uncommitted work — a Rule-4 irreversible operation performed by
accident, with no prompt and no diff to review.

Two rules. *(i)* **After a failed `worktree remove`, delete the DIRECTORY before
pruning, or leave both alone** — a pruned registration plus a surviving checkout
is the trap, and it looks like an ordinary worktree from the outside. *(ii)*
**`git -C <path>` is not a scope; it is a starting point for discovery.** The
`-C` form was adopted here as the safe alternative to a bare `cd`
(`#fact-a-bare-cd-retargets-every-later-command-not-only-the-correction`) and it
IS safer — but only while the path is a real worktree. Verify with
`git -C <path> rev-parse --abbrev-ref HEAD` before any writing verb: an answer
naming the host's branch means the command is pointed at the host. Same disease
as every other entry in this family — the instrument reporting confidently about
something other than the thing.

@fact:fact-the-gateway-model-is-observed-not-assumed **The gateway serves what it
serves, and the log says which (2026-08-14):** `#launchers-what` records the model
triple as `glm-5.2[1m]`; the `stream-json` log of this session's runs carries
`"model":"glm-5.3"` in its own event metadata. The launcher pins a model NAME and
the gateway resolves it; the resolved model is therefore an observation, not a
configuration, and the log is where it is observable. Worth knowing before
attributing a behaviour change to a packet: the worker may not be the worker the
last session measured.

@fact:fact-a-closed-write-list-must-name-the-file-a-sanctioned-split-creates **A
closed write list and the file-length budget collide on every packet big enough
to matter, and the packet must resolve it in advance (2026-08-15):** a code
packet gave three paths as its closed perimeter and, in the same §0, ordered a
split along a responsibility seam rather than a shave if any file crossed 600
lines. The test file would have reached 638. The worker split it, created a
fourth file, **named the collision in its Decisions section and asked the boss
to veto or bless it** — which is the behaviour the report contract exists to
produce, and it was blessed, because the split was prescribed and the list was
merely incomplete.

The rule the boss carries out of it: **a closed write list names the file a
sanctioned split would create**, or says in as many words that a split may add
one file and must be reported by name. Otherwise the perimeter — the thing that
makes «nothing else was touched» a set comparison instead of a judgement —
argues with the budget rule, and a weak writer resolves the argument by
shaving. Same family as `#fact-name-the-files-that-probably-need-no-change`:
both are a perimeter reading as an instruction it was never meant to give.

@fact:fact-cargo-fmt-reaches-a-file-and-still-leaves-check-red **`cargo fmt`
running clean does not mean `cargo fmt --check` will (2026-08-15, paid on a red
panel at the boss's own keyboard):** a test module was split out by slicing text
out of its parent, so the new file began with a blank line. `cargo fmt --all`
REACHED it — it dedented the `use` block that had been indented inside the
module — and still left the leading blank, which `cargo fmt --all --check` then
failed on. So rustfmt was not idempotent on its own output here, and «fmt ran,
exit 0» is not evidence the check passes; `cargo fmt` without `--check` exits 0
on success whether or not the result is clean.

Two rules. *(i)* **A file produced by text-slicing gets a `//!` header as its
first line** — it documents why the split exists and it cannot be a leading
blank. *(ii)* **The gate a landing must satisfy is the one the panel runs**, so
verify with `--check`, never with the write-mode form. Same disease as
`#fact-panel-background-form` and `#fact-a-truncated-pipe-reads-green`: the
instrument reported something other than what it was read as — and this time
the instrument was the boss's own.

@fact:fact-a-packet-touching-shared-data-names-the-crates-whose-tests-stand-on-it **A
packet that changes shared DATA must name the crates whose tests assert on that
data — the same law as for a shared type, one substrate over (2026-08-15):**
`#fact-the-tail-is-the-crates-the-packet-did-not-name` says a slice tightening a
shared TYPE names the consumer crates it can check and the boss budgets the
rest. A slice that edits a shared data file has the identical shape and is
easier to miss, because the file looks like content rather than like an
interface. Measured here: a packet's acceptance named exactly the new test it
asked for; the worker also edited `formats/vocabularies.json`, on which another
crate's tests stand; nothing made it run them; the panel reddened at step 4 of
51 on a test two steps older than the packet.

The worker was not at fault and the work was correct — the acceptance simply
described a smaller perimeter than the edit had. **Write the self-verify from
what the change TOUCHES, not from what the change ADDS.**

@fact:fact-a-test-that-reddens-on-a-legitimate-change-is-re-aimed-not-satisfied **When
a guard reddens on a change the contract requires, narrow it to its invariant
and prove the narrowing did not gut it (2026-08-15):** a green-proof test
asserted a vocabulary fragment matched its former inline copy byte for byte.
A later step gave that fragment the policy annotation the contract requires of
every enum, and the test failed on metadata — which never reaches the wire. The
assertion had grown stronger than the property it guards.

Three moves, and the third is the one that separates this from quietly
weakening a test. *(i)* Ask what the test GUARDS, not how to make it pass: here,
that the migration did not change the vocabulary's values. *(ii)* Re-aim at
exactly that — compare the schema form with policy stripped. *(iii)* **Prove
the narrowed guard still fails on a real violation** — a seventh value was added
to the vocabulary, the test failed, the value was removed, it passed. Without
(iii) the narrowing is indistinguishable from deletion with extra steps, and the
project's standing rule against making a panel green by editing its tests
applies with full force.

@fact:fact-the-report-goes-before-the-gates-but-every-gate-line-starts-pending **Order
the report BEFORE the final gates so a kill cannot destroy the account — and in
the same breath require every gate result to start as PENDING, or the packet
manufactures the confident-wrong line it was written to prevent (2026-08-15,
both halves paid for by two kills in one session):**

The first kill landed mid-acceptance and took the report with it: the work was
complete, the diff was on disk, and the boss reviewed a large slice with no map,
reconstructing every claim from the code. So the next packet ordered the report
written first — collect the work, write the account, run the gates, append their
output.

The second kill landed at the same place, and the report survived. That much
worked. But the surviving report asserted **«clippy — exit 0,
`large_enum_variant` did not fire»**, and the worker had been cut short two
steps before clippy ever ran. Re-measured: exit 101. The substantive claim
happened to be true — the lint really did not fire — and the failure was an
unrelated `expect_fun_call` in the worker's own test; the point is that the
report had no standing to say either way, and a boss who trusted it would have
committed a red tree.

**So the clause is two-part and neither half stands alone.** *(i)* Write the
account early — what was built, what was decided, what was deviated — because
that is knowledge only the worker has. *(ii)* **Every acceptance and self-verify
line begins as `PENDING` and is replaced only by the output of a run.** A packet
that says «write the report first» without (ii) invites a weak writer to fill
the template's happy path, which is exactly the behaviour
`#fact-first-live-fanout` already measured on a different section.

Corollary for the boss, and it is the one that actually protects the tree: this
changes nothing about acceptance. **A report is a map, never evidence**
(`#fanout-verify-the-numbers-not-the-narrative`) — a killed run's report least
of all, since the very steps a kill removes are the ones the last section
describes.

@fact:fact-a-worktree-carries-claude-md-so-the-worker-boots-before-it-reads-the-packet
**A worktree is a full checkout, so it carries `CLAUDE.md`, and the worker obeys
its boot contract before it reads a word of the packet (2026-08-16, measured on
both lanes):** each worker's opening `PROGRESS` line named the boot lane —
`spec/boot/STATIC.xml` (2186 lines), `INDEX.md` and its eight files, `spec/WAL.xml`
— and only then turned to the task. Nothing failed because of it: both packets
were accepted on the first pass. But the reading is not free, and it is the
PACKET's omission, not the worker's diligence: a packet built under
delegation-rules scenario (1) compiles its context in, and then says nothing
about the lane it just made redundant.

The honest form, and it is one clause: **a compiled-context packet states which
boot reading it needs and which it does not.** The four repo-wide rules bind
every worker and are already inlined in §0; the static lane's 2186 lines of
flow protocols are not what writing one file needs. Say so in the packet, or
the worker correctly spends its window proving it read them. What NOT to do:
tell a worker to skip `CLAUDE.md` itself — it carries the rules that bind the
commit its diff becomes, and a worker that has not read them is a worker whose
output the boss must re-derive from scratch.

Related from the other side: `#fact-gitignored-state-misses-the-worktree` —
there the worktree carried LESS than the packet assumed; here it carries MORE
than the packet accounted for. Both are the same question asked once:
**what exactly does a fresh worktree hand the worker, and did the packet say?**

@fact:fact-a-large-packet-cannot-travel-as-an-argument **A packet big
enough to compile its context in cannot travel as a command-line argument,
and the refusal comes from the operating system rather than from the
harness (2026-08-17):** `#e-spawn` used to substitute the packet into
`-p "$(cat <packet-file>)"`. The F4.2b-1 packet — a compiled-context
packet under delegation-rules scenario (1) — is 34 KB, and the run died
before its first turn with `Argument list too long` and exit 126. The
failure is a line of the launcher script, not a Claude error, so the JSONL
holds nothing that names a packet; the form had worked until now only
because every previous packet was shorter.

**The form that works: the packet is a FILE at the worktree root
(`PACKET-<task-id>.md`) and `-p` carries a short pointer to it**, plus a
copy of the closing clauses — heartbeat, report file, the git ban —
because a pointer prompt is precisely where boilerplate gets dropped
(`#fact-the-follow-up-packet-drops-the-clause`, one substrate over). Two
consequences worth having: the packet stays on the worker's disk, so a
`-c` correction may CITE a section instead of restating it; and the
worktree gains a second artifact never meant for the host, archived
beside the report and never applied into the tree.

The tell, if it recurs: the log stops at a few hundred bytes with no
`init` event at all and the harness reports a non-zero exit within a
second. That is not a killed worker
(`#fact-a-killed-task-does-not-kill-the-worker-and-the-survivor-writes-late`)
— nothing was ever spawned, so there is no survivor to leave alone and
nothing in the worktree to read.

@fact:fact-an-empty-output-is-a-claim-and-a-reproduction-script-is-a-fixture
**A measurement script that returns nothing has made a claim, and the claim
must be measured to the same standard as a worker's (2026-08-17, caught by a
consumer one layer downstream):** a harvest finding proved "every field's wire
string equals `snake_case(its identifier)` — zero exceptions" with an awk
one-liner, printed the script in its reproduction section, and the boss copied
that script into the packet as the step's evidence. The script could not fire
on ANY field: its pattern anchored end-of-line right after the field's colon, a
shape no emitted field has. Fed a field whose wire was deliberately made
different, it stayed silent. There WAS an exception — a schema property named
`ref`, a Rust keyword, escaped by the generator to `ref_` — and a pass built to
the measured number would have moved that format's bytes silently.

Three rules, and the third is the one that generalises past scripts.
*(i)* **Before an empty output is read as a proof, feed the instrument a case
it MUST flag.** This is `#fact-a-fixture-that-cannot-fail-proves-nothing-and-the-detector-may-be-downstream`
applied to a measurement rather than to a test fixture, and the cost of the
check is one line. *(ii)* **A reproduction script inside a finding is not
documentation, it is a fixture** — it will be copied into packets, so its
defects propagate from measurement into instruction, which is exactly the path
taken here. *(iii)* **Write the packet's rule so it survives its own
measurement being wrong.** The packet said "drop a rename only where it
restates an identity", with "today that is all 309" as an aside; the worker
executed the rule, kept the one rename that carried information, and named the
disagreement with the packet's number in its Deviations. Had the packet said
"drop all 309" — the tempting simplification the measurement invited — the
defect would have shipped. Same shape as
`#fact-a-dictated-coordinate-goes-stale-a-dictated-rule-does-not`, one
substrate over: there the coordinate was wrong and the rule saved it; here the
COUNT was wrong and the rule saved it.

@fact:fact-a-closed-write-list-must-name-the-file-the-change-breaks **A closed
write list must name the file the prescribed change BREAKS, not only the one it
creates (2026-08-17):** the sibling rule
`#fact-a-closed-write-list-must-name-the-file-a-sanctioned-split-creates`
covers the file a split ADDS. This is its other half. A packet ordered a
generated-type change that collapsed an optional collection; that collapse made
a test landed two steps earlier stop compiling, and the perimeter did not list
it. The worker did the right thing — measured the blast radius with
`cargo check --workspace --all-targets`, found exactly one broken file,
migrated it mechanically, and named the out-of-perimeter edit in Deviations —
but it had to choose between two of the packet's own instructions, which is a
choice a packet should never force.

The rule that follows is cheap to apply: **before writing the perimeter, ask
what the prescribed change makes STOP COMPILING, and put those files in the
list** — a phase whose earlier steps left tests behind is exactly the phase
where the answer is not empty. The blast-radius command belongs in the packet
too, so the worker's answer is measured rather than guessed; here it also paid
a dividend the packet had only asserted in prose, since the compiler's silence
about `vibe-cli` / `vibe-index` / `vibe-core` proved that no consumer held the
state the old shape allowed. Same family as
`#fact-the-tail-is-the-crates-the-packet-did-not-name` and
`#fact-a-packet-touching-shared-data-names-the-crates-whose-tests-stand-on-it`:
each is the perimeter being narrower than the edit, discovered one substrate
further out.

@fact:fact-a-gateway-529-is-a-third-terminal-class **A gateway overload is a
THIRD way a run ends, and it is neither a completion nor a kill (2026-08-17,
twice in one hour):** the `result` event arrives — so the process really
finished and `#fact-one-thread-one-writer` is satisfied, a `-c` may be sent at
once — but it carries `"terminal_reason":"api_error"` and
`"api_error_status":529`, and the harness reports plain `exit 1`. Read as a
completion, the run looks like a worker that stopped for no reason; read as a
kill, it invites the leave-the-survivor-alone protocol that does not apply,
because there IS no survivor. The tell is one grep on the log:
`grep -o '"terminal_reason":"[^"]*"\|"api_error_status":[0-9]*'`. The first
failure came at turn 64 after 35 minutes of API time with the work half done;
the second died on turn **1** after four `{"subtype":"api_retry"}` events, so
the gateway's state is observable before any work is risked.

The consequence that costs: **what a 529 takes is exactly the tail**, the same
way a kill does (`#fact-a-killed-run-is-not-a-verdict-and-the-kill-costs-the-tail`)
— here the packet's own `cargo clippy` line sat in the self-verify block and
never ran, and the two lints it would have caught reddened the boss's panel
instead. So the boss's tail after any api_error is not «finish the work», it is
«run precisely the verification the packet ordered last».

@fact:fact-the-workers-tests-outran-its-implementation **A worker's tests can
encode a property its own implementation does not satisfy, and only the RED run
separates «unverified» from «wrong» (2026-08-17):** the delivered pass carried
23 unit tests, and two of them asserted layout properties the code got wrong —
that a braced import shrunk to one survivor is written unbraced, and that a line
removed from between two blanks takes the second blank. Both are properties of
the panel rather than of taste (`cargo fmt --all --check` is its first step and
a generated file is never hand-edited), and both tests were right. Six more
failed on a single defect: the declaration matcher stripped the brace and left
the trailing space glued to the identifier, so it answered «not a declaration»
for every declaration jtd-codegen writes.

Two rules. *(i)* **A diff read without a run is evidence about intent, not about
behaviour** — every one of these eight failures is invisible to careful reading
and instant to a `cargo test`; the review that matters is the one that executes.
*(ii)* **«The worker verified nothing» and «the worker's artifacts are wrong»
are different claims**, and conflating them is expensive in both directions:
here the tests were the best thing in the delivery and the implementation was
the weak part, which is the reverse of the usual suspicion. Sibling of
`#fanout-verify-the-numbers-not-the-narrative`: there the report was re-measured,
here the code was.

@fact:fact-a-dead-transport-reopens-the-calculus-it-does-not-reissue-the-packet
**When the transport dies mid-run, re-run the delegation calculus over what is
LEFT — do not re-send the packet by reflex (2026-08-17):** after two 529s the
obvious move was a third lane, and it would have been wrong. The expensive half
— a 640-line pass with its arm classification and import analysis — was already
on disk; what remained was a one-line matcher fix, three lines of wiring, a
mechanical split, one import line, one doc phrase, and the gates. That
remainder is the never-delegate shape almost exactly («sub-minute edits», plus a
panel that is the boss's by definition), and the host tree runs those gates on a
WARM `target/` while any worktree is cold. Reclaiming was the calculus, not
impatience — and saying which of the two it was, out loud, is the part that
keeps the directive honest. The general form: a failed run changes the
REMAINING task, so the delegate-or-keep question is asked again about the
remainder, never inherited from the original packet.

@fact:fact-a-throwaway-probe-is-a-legitimate-measurement-shape **A packet may
order the worker to EDIT the tree purely to measure it, and that is a distinct
packet genre with its own perimeter form (2026-08-17):** the recorded radius of
a re-export had been estimated by reading, and reading had undercounted it — so
the next packet ordered three probes: replace the definitions, run
`cargo check --workspace --all-targets`, classify every error, restore the files,
move to the next probe. The compiler answered 43 / 126 / 40 errors across five
causes, and two of the causes were ones no reading had named.

The genre needs three clauses a build packet does not. *(i)* **The write
perimeter is split in two named lists** — the files written FOREVER (the finding
and the report) and the files written TEMPORARILY, each of which the worker
returns. *(ii)* **The restore is verified against a saved copy, not against
git** — the worker cannot run git, so the packet orders it to copy the originals
into the system temp dir before the first edit and `diff` against them at the
end. That check is not ceremony: this worker's own diff caught two of its own
slips (a dropped fragment of a `Cargo.toml` line, an added bracket pair in a doc
string) that a careful reading would have missed. *(iii)* **The packet says the
red IS the result** — «твоя работа не починить, а измерить» — or a competent
worker spends its window fixing the errors the packet exists to count.

What made it safe: the probe never leaves the worktree, so the host tree is
untouched by construction, and the finding is the only carrier
(`#fanout-the-finding-outlives-the-worktree`, one substrate over — there the
carrier outlived a spike crate, here it outlives a reverted edit).

@fact:fact-two-measurement-packets-parallelise-by-write-perimeter-even-when-one-is-cargo-heavy
**A read-only measurement and a cargo-heavy probe run concurrently on the two
lanes without interference, and the perimeter is what makes it safe — not the
subject (2026-08-17):** both packets read the same trees (`crates/vibe-wire/`,
`xtask/src/codegen/`, `crates/vibe-index/`), and one of them rewrote eight files
in its own worktree while the other only counted. Zero conflicts, both accepted
first pass, no `-c` cycles. `#fanout-perimeters-intersect-on-writes-not-reads`
already says reads never conflict; what this adds is that a probe's temporary
writes are still worktree-local, so a probing packet stacks against a reading
one exactly like two readers do. The box weighting from
`#e-parallel-coefficient` held as measured: one cargo lane, one text lane — the
box never saw two cold builds.

The boss's own work during the run is bound by the same rule from the other
side: while a worker holds a cargo lane, the boss's re-measurements must be
`grep`/`wc`-class, never a second `cargo` — the two would contend for the same
box, and the panel's user-home tripwire has already fired once on exactly that
kind of overlap (`#fact-the-panel-owns-the-user-home-for-its-whole-run`).

@fact:fact-a-429-usage-limit-is-a-fourth-terminal-class-and-the-other-lane-has-its-own-quota
**An account's usage limit is a FOURTH way a run ends, and unlike a 529 it
does not clear by retrying (2026-08-17):** the `result` event arrives with
`"terminal_reason":"api_error"` exactly as an overload does, but it carries
`"api_error_status":429` and a message naming the window —
«Usage limit reached for 5 hour. Your limit will reset at …». Read as a 529 it
invites the retry that cannot work; read as a kill it invites the
leave-the-survivor-alone protocol that does not apply. The tell is the same
one grep plus the status: `grep -o '"api_error_status":[0-9]*'`, and eleven
`{"subtype":"api_retry"}` events preceding it are the harness having already
tried.

**What it does NOT take with it: the other lane.** The two launchers hold
separate tokens (`~/.vibe/zai.api.token` and `…token.2`), so their quotas are
separate — a lane that is out until evening leaves the sibling lane fully
alive, and one empty probe on it costs seconds to confirm
(`claudez2 -p "Reply with exactly: PONG2"` → `terminal_reason: completed`).
So the recovery move is not "wait five hours"; it is
`#fact-a-dead-transport-reopens-the-calculus-it-does-not-reissue-the-packet`
with one more option on the table.

**And what it takes is the tail, exactly like a kill.** Measured here on the
corpus step: the worker had built the fixture journal, the projected catalog
and a 243-line test — everything except the verification the packet puts
last. The catalog it left did NOT reproject (`files_count` 9 against the
journal's 3), which is precisely the class of defect the missing final step
exists to catch. **Read the worktree, then decide** — the work was worth
keeping and the tail was worth taking back: on a warm host `target/` the
boss's remaining job was three runs and two red proofs, while a fresh worker
on the surviving lane would have started cold AND without the dead run's
context (a different account is a different state dir, so `-c` reaches
nothing).

@fact:fact-a-corpus-is-regenerated-by-a-run-not-repaired-by-hand **When a
committed fixture must BE the output of a function, regenerate it through
that function — even when the diff is one field (2026-08-17):** the corpus's
catalog differed from its journal's projection in a single `files_count`, and
editing that number by hand would have produced the right bytes for the wrong
reason — a catalog that happens to match rather than one the projection made.
The move that keeps the property true: add a temporary `#[ignore]`d
regenerator beside the comparison test, calling the SAME projection helper
the test uses, run it explicitly, remove it. The proof then costs nothing to
believe, because the bytes have exactly one origin. Sibling of
`#fact-a-test-that-reddens-on-a-legitimate-change-is-re-aimed-not-satisfied`:
there a test was re-aimed instead of satisfied, here a fixture was
re-derived instead of patched.

@fact:fact-the-git-ban-collides-with-proving-the-perimeter-so-the-packet-hands-over-the-substitute
**The git ban and the "nothing outside the perimeter" acceptance line pull
against each other, and the packet must resolve it or the worker will
(2026-08-17):** a packet forbade git absolutely and, three sections later,
demanded proof that no file outside its closed list had changed. The worker
ran exactly one command — `git diff --stat` — which is read-only and changed
nothing, but it is still the one thing the packet said never to do. The cause
is not indiscipline: the acceptance line asks for a set comparison against the
tree, and the obvious instrument for it is the forbidden one.

**So the packet supplies the substitute in the same breath as the ban.** The
forms already proven in this campaign: `find . -newer <the packet file> -type f`
with the build and vendor trees excluded (used successfully by an earlier
worker), or an explicit inventory — «созданы ровно эти файлы, изменён ровно
этот» — with each path named. A worker that can prove its own perimeter never
needs the verb.

The reverse reading is worth stating too, because the fix is cheap and the
alternative is expensive: a read-only git verb costs nothing HERE and costs a
worktree everywhere else — `git worktree prune` and a `git -C` on a pruned
directory both operate on the HOST
(`#fact-a-pruned-worktree-directory-retargets-git-at-the-host`), and a ban
with exceptions is a ban a weak writer will widen. The ban stays absolute;
what changes is that the packet stops asking for something only the banned
tool can give.

@fact:fact-the-scope-clause-is-checked-against-the-directory-not-assumed **The
`scope!` clause binds `src/`, and whether it binds a `tests/` directory is a
question about THAT crate, answered by looking (2026-08-18):**
`#fact-new-engine-files-scope` and
`#fact-scope-is-a-requirement-not-part-of-the-spec-prohibition` both say a new
`.rs` file carries `specmark::scope!`, stated flatly. Measured: in
`crates/vibe-index` the marker stands in **70** files under `src/` and in **0**
of the **24** files under `tests/` — a uniformity, not an oversight, since the
traceability map's perimeter is the product surface rather than its harness.

And the reason the clause must be *checked* rather than restated: the answer is
not repo-wide. `crates/vibe-resolver/tests/compile_fail.rs` does carry the
marker. So a packet that orders «every new file carries `scope!`» is right in
one crate's `tests/` and wrong in another's, and a packet that orders «tests
never carry it» is wrong the other way.

**The clause a packet creating test files should carry:** «a new file under
`src/` carries `specmark::scope!` with the same unit as its neighbours; for a
file under `tests/`, look at what the sibling test files in THIS crate do and
match them — by running the check, not by assuming either way.» A rule whose
correct answer differs per directory is a rule a packet must delegate to
observation, or it manufactures a panel failure in one crate and an
inconsistency in the next.

@fact:fact-a-new-guard-brings-its-own-fixture-and-never-edits-the-shared-one **A
packet adding a guard builds its OWN fixture; reddening an existing test by
editing the shared one is a defect of the edit, not a licence to edit the test
(2026-08-18):** a quarantine step's measurement listed seven server tests that
would have gone red had the shared `populated_state()` fixture been changed to
carry a quarantined version — the fixture is referenced 25 times across two test
files. Redness produced by changing a FIXTURE proves nothing about the code: it
proves the fixture moved. Paying for it costs the re-aiming of seven guards and
buys no evidence at all.

Two rules, and the second is the one packets keep needing. *(i)* **A new guard
gets a new fixture**, named for what it guards, and the shared one stays as it
is. *(ii)* **A pre-existing test going red under an ADDITIVE change is a defect
in the change** — the packet says so in as many words, because the cheap move a
weak writer reaches for is to adjust the assertion, and the adjustment is
indistinguishable in the diff from a legitimate re-aim. Its legitimate sibling is
already recorded: a test that reddens on a change the CONTRACT requires is
re-aimed and proved to still fail on a real violation
(`#fact-a-test-that-reddens-on-a-legitimate-change-is-re-aimed-not-satisfied`).
The difference between the two is the whole judgement, and it is why the packet
must name which one it expects.

@fact:fact-a-perimeter-cut-by-meaning-is-narrower-than-one-cut-by-counting **A
fan-out cut along the CONCEPT the task names leaves out whatever the concept
does not happen to cover — and the boss finds it only by counting the file
(2026-08-18):** the plan-mortality measurement was split into two packets by
«phase section», because the rule being executed speaks of phase sections. The
two segments covered 2762 lines of a 3055-line file. The 293 lines nobody was
assigned held the plan's architectural decisions, its execution rules and both
appendices — that is, the highest-value prose in the document and the one place
where a deferral lived with no copy anywhere else. Both homeless items the whole
measurement was run to find (a ruling whose reasoning had no spec, and three
named deferrals absent from the deferrals ledger) were in those 293 lines.

Nothing was lost, because the gap was found before the collapse and closed by
the boss in the same session. What it costs to find it late is the point: the
task was «prepare a deletion», and a perimeter that under-covers a deletion
deletes what it did not name.

**The rule: when the work is destructive, cut the perimeter by COUNTING the
artifact, not by naming the concept.** Sum the segments and compare to `wc -l`
(or to the file list, or to the anchor count) before spawning; a remainder is
either a third packet or an explicit boss-side item, never an accident. The
cheap form is one line — `echo $((end1-start1 + end2-start2)) vs $(wc -l < file)`
— and it is the same instrument-versus-thing discipline as
`#fact-an-empty-output-is-a-claim-and-a-reproduction-script-is-a-fixture`, one
substrate up: there a zero was read as evidence without a control, here a
partition was read as complete without a sum.

Related from the other side: `#fact-a-closed-write-list-must-name-the-file-the-change-breaks`
is this same defect on the WRITE perimeter — there the list under-named what the
change would break, here the split under-named what the measurement had to read.

## 9. What a clean fan-out looked like {#clean-fanout}

@fact:fanout-first-pass-acceptance **Measured 2026-08-14 — three packets, two
lanes, 3/3 accepted on the first pass, zero `-c` rework cycles** (the phase-0
spikes of the change-native build: `F0-RMW` and `F0-INVENTORY` on `claudez`,
`F0-GENPOC` on `claudez2`; runs and reports under
`cache/agents/sorted/F0-*/`). Given how much of this file records failures,
the shape that produced a clean run is worth recording with the same care.

@fact:fanout-perimeters-intersect-on-writes-not-reads **Route parallelism on the
WRITE perimeter, not the read perimeter.** `#e-parallel-routing` says disjoint
perimeters parallelise; in practice the two measurement packets read heavily
overlapping trees (both walked `crates/vibe-index/`) and that cost nothing,
because each was allowed to create exactly **two** files: its own finding and
its own report. Reads never conflict; only writes do. Stating the write
perimeter as a closed list of two paths — rather than as a directory — is what
made the overlap safe, and it is also what let the boss verify "nothing else was
touched" as a set comparison rather than a judgement.

@fact:fanout-one-cargo-worker-per-lane **Two doc/measurement workers ran
concurrently on the SAME launcher with no interference; the cargo-heavy one got
the other lane to itself.** Thread isolation held exactly as
`#launchers-conversation-key` predicts (one worker = one worktree = one cwd), and
the box never saw two cold `cargo` builds at once. The weighting rule from
`#e-parallel-coefficient` is confirmed in the small: text packets are free to
stack, cargo packets are the scarce slot.

@fact:fanout-inline-the-deliverable-skeleton **Inline the deliverable's section
headings verbatim in the packet, and demand a fixed field set per section.** All
three packets carried the finding's exact `##`-headings and, inside repeating
sections, an explicit list of fields ("reads / mutates / writes / what blocks the
target form / classification — one word / lines affected"). Every finding came
back structurally reviewable: the boss could diff a claim against the tree
without first reverse-engineering the document's shape. This is the same
mechanism as `#report-contract` (weak writers follow inlined templates and skim
citations), applied to the deliverable rather than to the report.

@fact:fanout-demand-per-claim-confirm-or-refute **Ask for each baseline claim to
be confirmed OR refuted with a citation — never for "verify the baseline".** The
`F0-GENPOC` packet listed five recorded properties of the generator's output and
required a verdict plus a line-number citation for each. The worker confirmed
four and **refused the fifth**, on the ground that the sample schema contained no
`discriminator` and therefore did not exercise the claim at all — "true of the
generator, not provable from this file". A blanket "check the baseline" invites a
blanket "checked"; a per-claim table with a citation column makes the honest
answer the cheap one. Two of the three findings corrected the plan's stated
facts, and both corrections came from this shape.

@fact:fanout-the-finding-outlives-the-worktree **When the work product is a
throwaway worktree, the finding must inline the code, or the knowledge dies with
the directory.** `F0-GENPOC` built a spike crate that was never meant to reach
the host tree; the packet therefore ordered the finding to carry the
post-processor and the generated result verbatim inside fenced blocks, and said
plainly why ("this is not duplication, it is the only carrier"). The worktree was
removed; the proof survives in `harvest/f0-gen-poc.md`.

@fact:fanout-a-typo-is-a-boss-tail-fix-not-a-rework **A cosmetic defect is fixed
in the boss's tail; only wrong judgement or wrong implementation earns a `-c`
rework.** One finding arrived with an unbalanced closing code fence — one
character, no bearing on any claim. Sending it back would have cost a full model
turn to save a one-line edit, and `#report-rejection` reserves rejection for
wrong decisions and wrong code. Fixed in place, recorded in `meta.md` as a
defect rather than passed over in silence.

@fact:fanout-verify-the-numbers-not-the-narrative **Acceptance re-measured every
load-bearing number by hand, and that is what makes "ПРИНЯТО" mean anything.**
For `F0-RMW` the boss re-ran the greps behind all six read-modify-write paths,
the five clock sites, the fifteen strictness attributes and every file and test
count; for `F0-GENPOC` it re-ran the acceptance suite itself (4/4, exit 0); for
`F0-INVENTORY` it independently confirmed the two claims that corrected the plan.
Everything matched. The one flaw found was a transcription artifact in a block
the report called "verbatim" — a duplicated fragment of one line — which changed
no conclusion but is exactly why the rule is *re-measure*, not *re-read*.
