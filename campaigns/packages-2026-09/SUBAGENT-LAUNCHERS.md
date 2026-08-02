# Subagent launchers — claudez/claudez2 as the E/T worker transport {#root}

<status stage="impl" state="done" comment="owner directive 2026-08-03; launchers reworked and verified the same day (the ALPHA/BRAVO matrix below); the mode switch is the owner's lever"/>

##the-directive **The owner's directive (2026-08-03, chat, near-verbatim):**
доработать запускаторы `claudez` / `claudez2`, чтобы они работали с `-c` как
обычная `claude` и годились как субагенты; написать инструкцию, как фазы E и T
используют их вместо нативных агентов; **переключение native ↔ claudez остаётся
в ведении владельца — он в любой момент может сменить способ вызова**; effort —
всегда max; по возможности воркеры работают **параллельно на обоих запускаторах
в worktree** и потом мерджатся — но правки, требующие изменений во многих местах
(конфликтоопасные), идут **одним потоком**; более-менее изолированные — **сразу
двумя**.

## 1. The switch — owned by the owner {#switch}

##switch-file The switch is one line in
[`SUBAGENT-MODE.toml`](SUBAGENT-MODE.toml) beside this file:
`mode = "claudez"` or `mode = "native"`. The boss re-reads it **before every
fan-out**, so an edit takes effect immediately — mid-phase, mid-batch, any
time. Saying it in chat works too (the boss updates the file so the state
stays durable).

##switch-native `native` means exactly what Phase D ran: the harness's
built-in `opus5` subagents through the Agent tool. `claudez` means workers
are **Claude Code processes on GLM-5.2** spawned through the launchers below.
Fractality stays out of this campaign either way (plan §6 `#delegation`).

##switch-does-not-change **What the mode never changes:** verdicts, anchor
routing, review of delegated output, spec/plan authoring, commits and
pushes stay the boss's in both modes (the never-delegate set); briefs cite
durable files only; Rules 1–4 bind identically; the presentation format to
the owner is unaffected.

## 2. The transport — what the launchers are {#transport}

##launchers-what Machine facts (this box; the launchers live OUTSIDE the
repository): `C:\Users\olegc\opt\bin\{claudez,claudez2}` (bash) and
`{claudez.ps1,claudez2.ps1}` (PowerShell). Each sets the z.ai
Anthropic-compatible gateway env (`ANTHROPIC_BASE_URL`, bearer from a token
file, model triple `glm-5.2[1m]` for opus/sonnet + `glm-5-turbo` for haiku)
and hands off to `claude`, **passing every argument through** — so `-p`,
`-c`, `--resume`, `--allowedTools` behave exactly as with plain `claude`.

##launchers-state-contract **The state contract that makes them
subagent-grade (reworked 2026-08-03):** the two launchers keep SEPARATE
Claude state dirs, so in one and the same cwd each launcher's `-c` continues
**its own** latest conversation and never steals the sibling's thread:

| launcher | account/token | `CLAUDE_CONFIG_DIR` | overrides |
|---|---|---|---|
| `claudez` | `~/.vibe/zai.api.token` | `~/.claude-glm` | `CLAUDEZ_CONFIG_DIR`, `ZAI_API_TOKEN_FILE` |
| `claudez2` | `~/.vibe/zai.api.token.2` | `~/.claude-glm2` | `CLAUDEZ2_CONFIG_DIR`, `ZAI_API_TOKEN_FILE_2` |

##launchers-effort **Effort is max by construction** (owner, 2026-08-03):
both launchers export `MAX_THINKING_TOKENS=32000` — Claude Code's
thinking-budget lever; harmless if the gateway model ignores it. Override:
`CLAUDEZ_MAX_THINKING`.

##launchers-verified **Verified 2026-08-03, the ALPHA/BRAVO matrix:** in one
scratch cwd, `claudez -p` seeded codeword ALPHA, `claudez2 -p` seeded BRAVO
(fresh `~/.claude-glm2` bootstrapped headless, second token live); then
`claudez -c -p` answered **ALPHA** and `claudez2 -c -p` answered **BRAVO** —
from bash AND from PowerShell (`Get-Command` resolves both to the `.ps1`
scripts; cross-shell continuation hits the same per-launcher thread). Eight
runs, exit 0 each.

##launchers-conversation-key **The `-c` scoping rule to build on:** claude
keys conversations by (config dir, cwd). One worker = one cwd (its
worktree) = one continuable thread per launcher. `-c` in a cwd with no
prior thread errors — expected, same as plain `claude`.

## 3. Phase E — the worker lifecycle {#phase-e}

##e-task-cut **1 · Cut the task (boss).** One E-task = one build/fix with an
explicit file perimeter, acceptance, and a self-verify command (`cargo
check -p …` class, never the full floor — cold worktree economics). The
packet inlines everything derivable (paths, names, exact edits where known)
and cites durable files only.

##e-parallel-routing **2 · Route the parallelism (boss, owner's rule
2026-08-03).** Intersect the candidate tasks' file perimeters BEFORE
spawning: **disjoint perimeters → two lanes in parallel** (`claudez` →
worktree A, `claudez2` → worktree B), merged after review; **a many-place,
cross-cutting edit (perimeters intersect or the edit sprawls) → one
thread**, no parallelism. When in doubt, one thread — a serialized hour is
cheaper than an interleaved conflict.

##e-worktree **3 · Provision (boss).** One worktree per worker:
`git worktree add .wt/<task-id> -b wt/<task-id>` — own cwd, own thread, own
branch. Workers never run git; `-c core.longpaths=true` if provisioning
trips MAX_PATH (the F19 lesson).

##e-spawn **4 · Spawn (boss, background, log captured).** Bash form:

```sh
( cd .wt/<task-id> && claudez -p "$(cat <packet-file>)" \
    --allowedTools "Read" "Glob" "Grep" "Edit" "Write" \
      "Bash(cargo check:*)" "Bash(cargo test:*)" "Bash(cargo fmt:*)" \
    --output-format text ) > run/wt-<task-id>.log 2>&1 &
```

The second lane is identical with `claudez2` and its own worktree. Headless
`-p` auto-denies anything not in `--allowedTools` — no git verbs in the
list, ever; `--dangerously-skip-permissions` is the owner's explicit opt-in
only. Liveness = log growth + `git -C .wt/<task-id> status`, never a blind
timeout; a worker silent for several times its sibling's runtime is stuck —
re-commission.

##e-correction-loop **5 · The `-c` correction loop (what the rework
bought).** The boss reads `git -C .wt/<task-id> diff` as a PR. Small
misses do not cost a re-spawn: `( cd .wt/<task-id> && claudez -c -p
"Review notes: …" … )` continues THAT worker's conversation with its full
context. Same flag, same semantics as plain `claude -c`.

##e-merge **6 · Merge (boss).** Apply the reviewed diff into the host tree
(`git apply` / merge the `wt/` branch), run `cargo fmt --all` (workers
don't fmt), run the real gates, **`cargo xtask sync-engines`** whenever a
package crate changed (the vendor-forward law of §5-E; the panel gates it),
commit per Rules 1–4, remove the worktree.

## 4. Phase T — the swarm on the launchers {#phase-t}

##t-transport **The T-spec already anticipated this executor** — «GLM
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

##t-fanout **Fan-out shape:** N packets → up to two lanes × one packet at a
time per lane (`claudez` lane + `claudez2` lane), each in its own worktree
per §13's file-split law; `-c` serves the per-packet correction loop
exactly as in Phase E. Isolated test-file packets are the parallel-friendly
default; a packet touching shared registries/goldens runs alone (the same
owner rule as `#e-parallel-routing`).

## 5. Secrets and safety {#safety}

##safety-tokens The bearer tokens live in `~/.vibe/zai.api.token{,.2}` —
the launchers read them themselves; the boss never prints them, never
passes them in args, never points a worker at `~/.vibe`. Worker packets
reference worktree-relative paths only.

##safety-review Delegated output is advisory until the diff is read and the
gates are green — in both modes, always. A `failed`/non-zero worker exit
does not mean discard: read the worktree first (the fractality lesson).

## 6. Standing facts {#facts}

##fact-verified-date Launchers reworked + full matrix verified 2026-08-03;
if a launcher regresses, re-run the ALPHA/BRAVO matrix from `#launchers-verified`
before blaming the harness.

##fact-interactive-use The launchers stay ordinary interactive commands too
— the rework changed state homes and headers, not the owner's daily use;
`claudez2`'s history before 2026-08-03 remains under `~/.claude-glm` (the
old shared dir) and is reachable by pointing `CLAUDEZ2_CONFIG_DIR` there.
