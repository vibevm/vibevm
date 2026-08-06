# Subagent launchers — claudez/claudez2 as the E/T worker transport {#root}

<status stage="impl" state="done" comment="owner directive 2026-08-03; launchers reworked and verified the same day (the ALPHA/BRAVO matrix below); the mode switch is the owner's lever"/>

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
archive).** Bash form — note `--output-format stream-json --verbose` and
the log path (§5's contract: the live log is written DIRECTLY into the
durable archive, so a crash of anything can lose nothing):

```sh
LOG=/c/Users/olegc/git/v/cache/agents/sorted/<task-id>/$(date +%F-%H-%M)-claudez-run.jsonl
mkdir -p "$(dirname "$LOG")"
( cd .wt/<task-id> && claudez -p "$(cat <packet-file>)" \
    --output-format stream-json --verbose \
    --allowedTools "Read" "Glob" "Grep" "Edit" "Write" \
      "Bash(echo:*)" "Bash(cargo check:*)" "Bash(cargo test:*)" "Bash(cargo fmt:*)" \
  ) > "$LOG" 2>&1
```

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
