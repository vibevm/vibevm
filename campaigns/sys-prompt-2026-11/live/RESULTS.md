# P9 — live check results, Claude Code (plus the P10 OpenCode and Qwen Code rows)

Run 2026-11-10 by the central session, as `RUNBOOK.md` orders. The owner
authorised it; it covers Claude Code only.

**Environment.** Claude Code 2.1.322, model `claude-opus-5-5` on every run.
`vibe` was built from `daaec4637`. The step 4 re-run and the C7 note used
`6323e8a36`. The launcher was `claude2`, declared in a scratch
`$VIBE_SETTINGS/agents.toml` with `config_dir = "~/.claude2"`, so the
owner's own settings were not touched. Linux only.

**One deviation from the runbook.** Every session ran in print mode:
`vibe agent claude2 -p "<prompt>" --output-format json`. An agent cannot
drive the interactive terminal UI. Print mode passes the same
`--append-system-prompt-file` slot. Usage numbers come from the JSON
`usage` record of each run, not from `read-usage.sh`. An interactive run was
not measured here; row C9 measured it on 2026-11-12 by driving the TUI through tmux.

Every answer verbatim, with its counters, is in `ANSWERS.md`.

**OpenCode rows (P10, 2026-11-11).** OpenCode **2.2.7**, Linux, through the
built-in `opencode` launcher. **Two models, at the owner's instruction.**
`home/gpt-6.1-sol`, whose provider entry declares a **512,000**-token context
(output 8,192), carries the real-size legs; `homelocal/qwen3.8-27b`, with a
**56,000**-token context, is the small/medium second data point and is what
forced the samples to be sized at all.

**How the samples were sized.** OpenCode's own fixed overhead was measured
from the offline capture *first*: a **10,928**-byte base system prompt
(including the project's injected `AGENTS.md`) plus **14,043** bytes of tool
definitions over 12 tools — **24,971** bytes before any lane. Assuming a
pessimistic **3 bytes per token** for this marker- and hash-heavy text, that
is ~8,300 tokens of overhead, so a 56,000-token model has room for roughly
100 KB of lane inside a 45,000-token working budget. The qwen legs stay far
inside it: the **unpadded 2,570**-byte lane for the `read` tier and a
**42,397**-byte one for the carrier (~22,500 tokens all told), with the tail
canary still the last line. On the 512,000-token model the full
**204,429**-byte lane is ~77,000 tokens and needs no special sample.

These rows report no cache counters: OpenCode's `run` prints none, and a
number nobody measured is not reported as zero.

**Qwen Code rows (P10, 2026-11-11).** Qwen Code **0.27.2**, Linux, through
the built-in `qwen` launcher. The first legs ran on `gpt-6.1-sol` through an
OpenAI-compatible endpoint with a 512,000-token context.

**Cost rule, owner 2026-11-11, applied from here on** («для экономии токенов
… запускай qwen на супер маленьких проектах и на локальном инференсе»): live
sessions use the **local** `qwen3.8-27b` (56,000 tokens) and the **tiny**
sample, and every real-size proof is **offline** against the fake provider,
where a 202 KB lane costs nothing. The `gpt-6.1-sol` rows below were taken
before the rule; they are kept as evidence and are not to be repeated. The
rule costs this campaign nothing, because the two questions that need a real
size — does the lane arrive whole, and exactly once — are questions about
bytes in a request, and `qwen-capture.sh` answers them from the recorder. The offline capture ran inside
`bwrap --unshare-net`, so `127.0.0.1` was the only reachable endpoint by
construction rather than by configuration. **Qwen Code's measured fixed
overhead:** a 21,708-byte system message whose base prompt runs
to byte 13,863, where the project's own context files begin, plus 9,129 bytes
of tool definitions over six tools, with a further 12,212-byte first user turn carrying the skills roster,
the deferred-tool list and the date/cwd reminder. These rows report no cache
counters: `qwen -p` prints none, and a number nobody measured is not reported
as zero.

The usage columns are the provider's own counters: `cache_creation_input_tokens`
and `cache_read_input_tokens`. Every run is a single request unless "turns"
says otherwise.

| Row | Project / lane | Answer, in short | creation | read | Verdict |
|---|---|---|---|---|---|
| C1 step 1 | `live-1`, 2,570 B | Last lane-end quoted byte-equal (`… 90-user.md 6494d071a30d`). All four lane canaries present, no files read, `CANARY-TEMPLATE-SLATE` absent. | 13,221 | 11,275 | **pass** |
| C2 step 2 | `live-1`, repeat | Same answer. | 7,418 | 15,381 | pass, but the lane is ~0.7k tokens, so the signal is weak; see step 3 |
| C3 (small) | `live-1b`, other path, git repo, dirty tree | Same answer. | 9,576 | 13,338 | inconclusive at this size; see C3 (size) |
| C1 step 3 | `live-3`, 202,349 B | Last lane-end quoted byte-equal. All four canaries including the tail, no files read. | 73,987 | 11,275 | **pass**: no truncation at 202 KB |
| C2 at size | `live-3`, repeat | Same answer. | **0** | **85,262** | **pass**: the whole prefix including the lane is a cache read |
| C3 (size) | `live-3b`: same lane bytes, other path, git repo, dirty tree | Same answer. | 9,574 | **75,803** | **pass**: the lane is cached across checkout path and git state |
| C1 this repo | worktree at `daaec4637`, opted in locally (not committed), 312,345 B lane | Last lane-end quoted byte-equal (`… 68-flow-omnichannel.xml 49d8aae57569`). No files read. No `@spec://` token resolved as a path. | 180,811 | 11,275 | **pass** |
| C2 this repo | repeat | Same answer. | 8,109 | **183,981** | **pass** |
| C4 classic, plain `claude2` | `live-4`, template and `[system_prompt]` removed | Read `STATIC.md`, `INDEX.md`, `00-core.md`, `90-user.md` in that order, once each. All four canaries. 5 turns. | 14,146 | 58,153 | **pass** |
| C4 via `vibe agent` | `live-4` | At `daaec4637`: **refused**, exit 1, "has not opted in … no flag to pass". Fixed in `6323e8a36`, which starts the harness unchanged with a one-line note. Re-run: same reads as the plain session, once each. | 3,635 | 68,782 | **fail → fixed → pass** |
| C5 subagent | `live-1` | The subagent saw no `CANARY-…` word and no `<!-- vibe:lane-begin` line. It saw only the block's format example. | 11,882 | 36,151 | **pass**: subagents do not inherit the slot |
| N1 nested | `live-1` | Read only `nested/AGENTS.md`. `CANARY-NESTED-TUFF` present, `CANARY-NESTED-LANE-SCORIA` absent. It ignored the nested block as another project's. Claude Code also auto-loaded `nested/CLAUDE.md`, and the agent ignored that block too. | 12,707 | 36,108 | **pass** |
| C6 repeated flag | `claude2` by hand, two `--append-system-prompt-file` | Only the **second** file's canary arrived. | — | — | recorded: **last flag wins**, the first file is dropped |
| C7 resume | `live-1`: `00-core.md` edited and installed, then `--resume` of the step 2 session | The agent saw the **old** `00-core.md` id `cc8a3fa2cf16` (current: `2d1b478e28d7`) and not the new canary. It reported "no sign the lane is out of date": the block carries only `STATIC`'s expected id, and that id was unchanged. | 709 | 22,799 | **neither runbook pass branch** (old lane, mismatch not noticed) — recorded: **the old lane comes back, silently**. `6323e8a36` makes `vibe agent` say so on `-c/--continue/-r/--resume` |
| C9 interactive | `live-int`, 2,570 B lane, vibe built from `5b9873ce4`, Claude Code 2.1.322 (`claude-opus-5-5`), `vibe agent claude2` in the **interactive terminal UI**, driven through tmux (2026-11-12) | The one prompt, typed into the TUI. Last lane-end quoted byte-equal (`<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->`), and the agent noted on its own that the managed block's placeholder example comes later in the text but closes no real file. The four lane canaries present, including the tail. «Files I've read with a tool this session: none.» A second interactive session with the same prompt: cache creation 14,699 → **10,199**, cache read 27,525 → **32,029** (session transcripts' first `usage` record). | 14,699 / 10,199 | 27,525 / **32,029** | **pass (2026-11-12)**: the interactive mode takes the same slot and caches it. This closes the runbook's one unmeasured mode |
| C8 Windows | `c8` on Windows 11 x86_64, vibe 1.0.12 (`8b8cfce52`), Claude Code 2.1.280 (`claude-opus-5-5[1m]`), 202,349 B lane, launcher `command = ["C:/nvm4w/nodejs/claude.cmd"]` | After the owner re-logged in (the CLI's stored login had expired; neither SSH nor a scheduled interactive task could refresh it): last lane-end quoted byte-equal (`… 90-user.md 6494d071a30d`), the four lane canaries present, no files read, template and nested canaries absent. Repeat run: cache read. | 76,987 / 10,453 | 15,176 / **81,477** | **pass (2026-11-10)**. Found on the way: the built-in `claude` launcher could not start an npm-installed Claude on Windows (`claude.cmd`) — fixed in `fix(agent): resolve a launcher's program through PATHEXT on Windows` |
| X1 Codex, Windows | `cx` on Windows 11 x86_64, vibe 1.0.12, codex-cli **0.167.0** (updated from 0.162.1 at the owner's request), model `gpt-6-astra`, 202,349 B lane, `vibe agent codex exec` | Offline first (`codex debug prompt-input`, no model call): the lane is in the request whole and exactly once (head, tail and last lane-end each once), on 0.162.1 and on 0.167.0. Live after the owner's re-login: last lane-end quoted byte-equal, the four lane canaries **and** `CANARY-ENTRYPOINT-QUARTZ` present (Codex reads `AGENTS.md` itself), template and nested canaries absent, no files read. 48,803 tokens used. | — | — | **pass (2026-11-10)**. Authorised by the owner, who asked for the Codex check himself. |
| O1 OpenCode delivery | `live-o1`, 204,429 B lane, **OpenCode 2.2.7**, offline against a loopback OpenAI-compatible provider that recorded the request body (`opencode-capture.sh`, `PORT=8745`) | The lane arrived **byte-equal and exactly once** in a single system message of 215,504 bytes; `CANARY-LANE-HEAD-OBSIDIAN`, `CANARY-LANE-TAIL-PUMICE`, `CANARY-INDEX-CORE-BASALT`, `CANARY-INDEX-USER-GNEISS` and `CANARY-ENTRYPOINT-QUARTZ` each once, `CANARY-TEMPLATE-SLATE` absent. **Control**, the same run with `OPENCODE_CONFIG_CONTENT` unset: a 10,928-byte system prompt, lane absent, zero lane canaries — so the carrier is what delivered it. **Offsets:** instructions 7,974, lane 10,312, `<env>` 214,848, `Today's date` 214,743 — the per-day block is a **suffix**, so the lane is not invalidated at a date change. `--standalone` was required: against the running background service `opencode debug config` lists no such config layer at all. | — | — | **pass, offline (2026-11-11)**: the carrier delivers. It is still not shipped — see O1 launcher |
| O1 OpenCode launcher | `live-o1`, `vibe agent opencode run …` with vibe built from this worktree | `vibe agent` resolved the built-in `opencode` launcher, printed `system prompt: unchanged`, the `read` note and the carrier note, passed **no** flag, and exec'd real OpenCode 2.2.7, which started and reached model selection. | — | — | **pass (2026-11-11)** for the launcher half |
| O1 OpenCode live, carrier at real size | `live-o1`, **204,429 B** lane, `OPENCODE_CONFIG_CONTENT={"instructions":["…/AGENT-LANE.md"]}` with `opencode --standalone run -m home/gpt-6.1-sol` by hand, because vibe does not deliver this carrier (512,000-token context) | Last `vibe:lane-end` quoted **byte-equal** (`<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->`, identical to `tail -1`). All four lane canaries present **including the tail**, plus `CANARY-ENTRYPOINT-QUARTZ`; `CANARY-TEMPLATE-SLATE` absent. **Files read with a tool: none** — router rule 1 held at 204 KB. | — | — | **pass (2026-11-11)**: the carrier delivers the whole lane at the real size. It is still not shipped — the blockers are about *who else* receives it |
| O1 OpenCode live, `read` mode | `live-o1s`, **2,570 B** lane, `vibe agent opencode run -m home/gpt-6.1-sol`, the one prompt | **Router rule 4 obeyed.** Read `vibevm/vibespecs/boot/STATIC.md`, `INDEX.md`, `00-core.md`, `90-user.md` — in that order, **once each** — and reported all four lane canaries plus `CANARY-ENTRYPOINT-QUARTZ`. Asked for the last `vibe:lane-end` line it quoted the managed block's own indented **format example**, which is the correct answer in `read` mode: there is no delivered marker, and the example is the only literal occurrence. | — | — | **pass (2026-11-11)**: the shipped tier works on OpenCode |
| O1 OpenCode live, `read` mode, the one prompt, 56k model | `live-o1s`, **2,570 B** lane, `vibe agent opencode run -m homelocal/qwen3.8-27b` | The agent **did not boot**: it answered the meta-question literally — "no *delivered* lane marker … the only literal occurrence is an example inside the AGENTS.md prose", `CANARY-ENTRYPOINT-QUARTZ` only, "zero tool calls … I have not opened any file". So router rule 4 did not fire for this model **on this prompt**. | — | — | **recorded, not a mechanism failure** — see the next row, which distinguishes them |
| O1 OpenCode live, `read` mode, a task-shaped prompt, 56k model | `live-o1s`, same project, "what are this project's standing boot instructions? answer from the project's own boot files, and list every file you read with a tool" | **Router rule 4 obeyed.** The agent read `vibevm/vibespecs/boot/STATIC.md`, `INDEX.md`, `00-core.md`, `90-user.md` — in that order, **once each** — summarised all three layers correctly, reported `CANARY-LANE-HEAD-OBSIDIAN`, `CANARY-LANE-TAIL-PUMICE` and `CANARY-ENTRYPOINT-QUARTZ`, quoted STATIC's expected id `df1cb2d5aadc` from rule 3, and correctly excluded `AGENTS.md` from the read list because the harness had injected it. | — | — | **pass (2026-11-11)**: the `read` tier works on OpenCode even on the 27B model, once the session has work in it. The row above it is the **prompt**, not the block — and `home/gpt-6.1-sol` obeyed rule 4 on that very prompt, which settles it |
| O1 OpenCode live, carrier at medium size, 56k model | `live-o1m`, **42,397 B** lane, `OPENCODE_CONFIG_CONTENT={"instructions":["…/AGENT-LANE.md"]}` with `opencode --standalone run -m homelocal/qwen3.8-27b` by hand, because vibe does not deliver this carrier | Last `vibe:lane-end` quoted **byte-equal** (`<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->`, identical to `tail -1`). All four lane canaries present **including the tail**, plus `CANARY-ENTRYPOINT-QUARTZ`; `CANARY-TEMPLATE-SLATE` absent. **No file read with a tool** — router rule 1 held at size. | — | — | **pass (2026-11-11)**: the tail arrives through the carrier at a non-trivial size on a 56,000-token model too |
| Q1 Qwen delivery, offline | `live-q1` / `live-q3`, **Qwen Code 0.27.2**, offline against a loopback OpenAI-compatible provider that recorded the request body (`qwen-capture.sh`, `PORT=8752`/`8753`, inside `bwrap --unshare-net`) | **Plain, the shipped tier:** one system message of 21,708 bytes carrying the managed router block at byte **14,046**, inside Qwen Code's own `--- Context from: AGENTS.md ---` delimiter — `CANARY-ENTRYPOINT-QUARTZ` once, **every lane canary 0**, `CANARY-TEMPLATE-SLATE` absent, lane x0. So the router arrives and nothing else of the lane does. **Through the slot:** `--append-system-prompt "$(cat AGENT-LANE.md)"` on the 2,570-byte lane put it at byte **16,985**, **byte-equal and exactly once**, all four lane canaries present including the tail, the template's absent. **At real size:** the 202,349-byte lane through the same slot is `E2BIG` — «Argument list too long», errno **7** — at `execve`, before Qwen Code starts; an environment string is capped by the same 128 KiB rule. **Offsets:** context files 13,863, router 14,046, lane 16,985, auto-memory 16,985 → 19,561 when the lane is appended, `Today's date` **absent from the system message** and at byte 7,079 of the 12,212-byte user turn — Qwen Code puts the per-day text in the **user turn**, so nothing per-day precedes the lane | — | — | **pass, offline (2026-11-11)**: the router reaches the model in the shipped tier; the slot delivers; the slot cannot carry a real lane |
| Q1 Qwen launcher | `live-q1`, `vibe agent qwen -p …` with vibe built from this worktree | `vibe agent` resolved the built-in `qwen` launcher, printed `system prompt: unchanged`, the `read` note and the slot note naming `--append-system-prompt`, the 128 KiB cap and `E2BIG`, passed **no** flag and exec'd real Qwen Code 0.27.2, which answered | — | — | **pass (2026-11-11)** for the launcher half |
| Q1 Qwen live, `read` mode, the one prompt | `live-q1`, **2,570 B** lane, `vibe agent qwen -p "<the one prompt>"`, `gpt-6.1-sol` | Quoted, verbatim: «` vibe:lane-end org.live/sample <path> <id>`» — the managed block's own indented **format example**, which is the correct answer in `read` mode: there is no delivered marker and the example is the only literal occurrence. Canaries: `CANARY-ENTRYPOINT-QUARTZ` alone. «Files read with a tool in this session: none.» | — | — | **pass (2026-11-11)**, and **the prompt cannot test router rule 4** — it ends «do not read any file to answer this», so not reading is obedience. See the next row |
| Q1 Qwen live, `read` mode, a task-shaped prompt | `live-q1`, same project, "what are this project's standing boot instructions? answer from the project's own boot files, and list every file you read with a tool" | **Router rule 4 obeyed.** Read `vibevm/vibespecs/boot/STATIC.md`, `INDEX.md`, `00-core.md`, `90-user.md` — in that order, **once each** — summarised all three layers correctly, quoted `STATIC`'s expected id `df1cb2d5aadc` from rule 3, reported `CANARY-LANE-HEAD-OBSIDIAN`, `CANARY-INDEX-CORE-BASALT`, `CANARY-INDEX-USER-GNEISS`, `CANARY-LANE-TAIL-PUMICE` and, separately, `CANARY-ENTRYPOINT-QUARTZ` as «the context-provided entrypoint», and **excluded `AGENTS.md` from the read list** because the harness had injected it | — | — | **pass (2026-11-11)**: the shipped tier works on Qwen Code |
| Q1 Qwen live, `read` mode, 56k local model | `live-q1`, **2,570 B** lane, `vibe agent qwen -p "<the task-shaped prompt>"`, local **`qwen3.8-27b`** (56,000 tokens) — the first row taken under the owner's cost rule | **Router rule 4 obeyed.** Read `AGENTS.md`, then `vibevm/vibespecs/boot/STATIC.md`, `INDEX.md`, `00-core.md`, `90-user.md` — once each — summarised all three layers correctly, quoted `STATIC`'s expected id `df1cb2d5aadc` from rule 3, named `root → org-live--carrier--root` from the renamed-anchor table, and reported `CANARY-LANE-TAIL-PUMICE` as `90-user.md`'s only content. It opened with «none arrived pre-delivered with lane markers, so each was read from disk», which is rule 1 reasoned out loud | — | — | **pass (2026-11-11)**: the shipped tier works on a 27B local model too. **One honest difference from the 512k row:** this model also read `AGENTS.md` **with a tool**, which the harness had already injected — so the entrypoint's bytes were paid for twice. Not a mechanism failure (the block says «this file» is part of the order) but worth keeping beside the stronger model's row, which excluded it |
| Q1 Qwen carrier, offline | `live-q1`, the **file-based** slot: `QWEN_WRITE_SYSTEM_MD` probe → base prompt → lane appended → `QWEN_SYSTEM_MD` (`qwen-capture.sh`'s `carrier` leg) | The request begins with Qwen Code's own base prompt **byte-exact**, carries the lane **exactly once** at byte **13,858**, and from the end of the lane onward is **byte-identical** to the plain run after its base. All four lane canaries plus the entrypoint's, `CANARY-TEMPLATE-SLATE` absent. The probe reaches no provider: with no prompt and stdin closed Qwen Code writes the dump and refuses for want of input, and under `--safe-mode` the dump is byte-identical to a plain run's base layer | — | — | **pass, offline (2026-11-11)**: the owner's composition works. Not shipped — see the row below |
| Q1 Qwen carrier, the blocker | `live-q1`, the same probe in both interaction modes under identical flags | The base prompt is **13,856** bytes headless and **18,451** interactive, differing by whole sections — the role sentence, the dedicated-tool and absolute-path rules, the background-process guidance, a block of worked examples. `--prompt-interactive` refuses piped input and a run with no tty is headless whatever is asked of it, so a probe without a terminal is **always** the headless variant | — | — | **recorded (2026-11-11)**, and it is what closes the tier: a launcher cannot know which copy a launch needs without allocating a pty and starting a second session |
| Q1 Qwen carrier, live at real size | `live-q3`, **202,349 B** lane, `QWEN_SYSTEM_MD=<base+lane>` with `qwen -p "<the one prompt>"` by hand, `gpt-6.1-sol`, combined file 216,207 bytes | Last `vibe:lane-end` quoted **byte-equal** (`<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->`). All four lane canaries **including the tail** plus `CANARY-ENTRYPOINT-QUARTZ`; template's absent. «Files read with a tool in this session: none.» | — | — | **pass (2026-11-11)**: a **file** has no command-line limit, so the real-size lane goes through. The 128 KiB cap is the additive slot's problem alone |
| Q1 Qwen context layer, import-inertness | `live-q1` copy with a hand-written `QWEN.md` carrying `@spec://org.vibevm.core/vibevm/common/PROP-076#SYS-QWEN` and `@./sibling.md` | The `@spec://` token arrived **verbatim**; the `@./sibling.md` beside it was **expanded in place**, inlined between `<!-- Imported from: ./sibling.md -->` and `<!-- End of import from: … -->`, carrying `CANARY-SIBLING-IMPORTED-ONYX` | — | — | **recorded (2026-11-11)**: C3's `@spec://` worry does **not** apply to Qwen Code, so the lane's 53 such tokens would survive the context layer. That layer still breaks `#SYS-WORKER-EXCLUSION`, which is why it is not the carrier |
| Q1 Qwen live, the slot by hand | `live-q1`, **2,570 B** lane, `qwen --append-system-prompt "$(cat …/AGENT-LANE.md)" -p "<the one prompt>"`, because vibe does not deliver this slot | Last `vibe:lane-end` quoted **byte-equal** (`<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->`, `diff` against `tail -1` empty). All four lane canaries present **including the tail**, plus `CANARY-ENTRYPOINT-QUARTZ`; `CANARY-TEMPLATE-SLATE` absent. **Files read with a tool: none** — router rule 1 held. The same command on the **202,349-byte** lane printed «Argument list too long» and never started Qwen Code | — | — | **pass (2026-11-11)** for the slot at a size it can take, **and the measurement that closes the tier** at the size that matters |
| O2 OpenCode whole-request proof, offline | `sample-big`, **204,845 B** lane, **OpenCode 2.2.7**, five legs against one loopback recorder (`opencode-prove.sh` + `opencode-prove.py`, `PORT=8741`), carrier composed by vibe's own interpreter through the sync tool | **Conditions (0) and (i)–(v), all PASS.** **(0)** the base composed from `prompt-bases/2.2.7/` is **1,470 bytes**, sha256 `3728fd87457f43145d42108048ce69a105a72bb9677d7b5d38fecdec15ce513e`, and the plain run's first 1,470 bytes are **byte-identical** to it. **(i)** `carried[:1472] + carried[1472 + 204845:] == plain` **exactly** — one contiguous insertion at byte **1,472** (= base 1,470 + the recorded `"\n\n"` separator) of length **204,845**, **zero** plain bytes in that window, `model` / `stream` / `tools` / `max_completion_tokens` / `stream_options` and every non-system message **equal**, both requests carrying the same six members. Plain system prompt **10,958 B**, carrier **215,803 B**. **(ii)** the inserted window is `AGENT-LANE.md` **byte for byte**, all 204,845 — `{file:}` trims the lane's one trailing newline and OpenCode's own part join restores one; both marker lines intact, lane ×1. **(iii)** seam `' work and investigate before deleting or overwriting them.\n\n'` before the first marker, `'\n# Your Model\n- Name: Fake 1…'` after the last. **(iv)** the two launches are byte-identical **throughout** (206,316-byte prefix, sha256 `da5ac530155cf454…` both), so no per-day byte is anywhere in the prefix. **(v)** a real `subagent` child — driven by a provider answering with the spawning tool call — carries the lane **×0** at **10,958 B** and is **byte-identical to its own plain counterpart**, every compared member included | — | — | **pass, offline (2026-11-11)**: proof `opencode-whole-request-0ca944df359f9d68`, and the only thing that wrote `verified = true` into the published set |
| O2 OpenCode live, condition (vi), 56k local model | `sample-tiny`, **2,570 B** lane, `vibe agent opencode run -m homelocal/qwen3.8-27b` — the launcher, not a hand-built command, because the claim is about what vibe delivers | **PASS, and the one condition a byte diff cannot make.** vibe's own notes first: «the 1470-byte base is OpenCode's own, composed from … for model `qwen3.8-27b` — this launch's own `homelocal/qwen3.8-27b` argument», so model discovery read the argument and chose the **default** base. The session then: **(1)** quoted `<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->` **byte-equal** to `tail -1`; **(2)** «none — I have made no tool calls in this session; the boot-lane content arrived pre-delivered in my context, not via a read», which is router rule 1 reasoned out loud; **(3)** stated the base-only rule: «By default, `oldString` must occur exactly once …; multiple matches fail unless `replaceAll` is true. The values of `oldString` and `newString` must also differ» — three facts that exist **only** in OpenCode's own base text and nowhere in the lane (checked: the lane does not contain `oldString`); **(4)** reported `CANARY-LANE-HEAD-OBSIDIAN`, `CANARY-INDEX-CORE-BASALT`, `CANARY-INDEX-USER-GNEISS`, `CANARY-LANE-TAIL-PUMICE` and `CANARY-ENTRYPOINT-QUARTZ` | — | — | **pass (2026-11-11)**: the base was **supplied, not displaced** |
| O2 OpenCode live, the question that had to be sharpened | the same project and model, first attempt | **A finding, not a failure.** Asked for «the LAST line in your context containing vibe:lane-end» the model answered the managed block's own indented **format example** — and it was **right**: in `launch` delivery the harness injects `AGENTS.md` *after* the agent's `system`, and that file's block contains such an example, so the example genuinely is the last one. The question now names the marker's own file (`90-user`), which only the delivered marker carries. Everything else in that first run already passed — canaries, no file read, the base-only rule | — | — | **recorded (2026-11-11)**: the prompt, not the delivery. A `launch`-tier question about "the last marker" must say which marker |
| O2 `vibe prompt flags --harness opencode` | `sample-tiny`, `VIBE_PROMPT_BASES_DIR` at the package checkout, no `-m` | One `OPENCODE_CONFIG_CONTENT=…` line then `--standalone`, and two notes: the carrier's own explanation, and «the 3286-byte base is OpenCode's own … for model `gpt-6.1-sol` — the effective OpenCode configuration this launch would use, as `opencode debug config` reports it». So with no argument of its own the command discovers the model from the harness's own answer and composes that family's base — `gpt`, 3,286 bytes, not the 1,470-byte default | — | — | **pass (2026-11-11)**: both halves of model discovery exercised, and the printed assignment is byte-identical to what the launch sets |
| Q2 Qwen identity carrier, offline whole-request proof | `sample-big`, **204,845 B** lane, **Qwen Code 0.27.2**, six legs in two session shapes: `plain` / `launch-a` / `launch-b` under `qwen-identity-prove.sh`, each a real session in its own network namespace (`bwrap --unshare-net`) against a loopback provider that recorded every request body, each driving one subagent spawn, and the **carrier legs launched by `vibe agent qwen` itself** so the bytes measured are the launcher's | **(i)** every main-session request is its plain counterpart plus **ONE** contiguous insertion and nothing else differs: interactive 208,787 raw bytes at raw offset **303** (system-message offset 260, 24,583 B → 229,429 B), headless the same at raw offset **306** (21,748 B → 226,594 B); tools array identical (15 tools interactive, 6 headless) and every non-system message and parameter identical. **(ii)** the inserted bytes are `AGENT-LANE.md` byte for byte plus one LF (204,846 B), the raw insertion JSON-decodes to the same bytes, the lane occurs **x1** in the system message and **x1** in the raw body, first and last marker lines intact. **(iii)** seam before `'…utilizing your available tools.\n\n'` \| `'<!-- vibe:lane-begin org.live/sample vibevm/vibespecs/boot/S'`; seam after `'…90-user.md 6494d071a30d -->\n\n'` \| `'# Core Mandates\n\n- **UserPromptSubmit Co'`. **(iv)** two consecutive launches: 275,603 B and 275,603 B, **the WHOLE request identical**, system message sha `35bd245f386c120d` both times, tools identical. **(v)** the general-purpose **subagent** (63,686 B, sha `5d9aa0182ddb891c`), the security classifier and the memory extractor are each **byte-identical** to their plain counterparts, with **zero** lane canaries and **zero** marker lines | — | — | **pass, offline (2026-11-11)**, `VERDICT: PASS` in both shapes, re-run from the committed tree with the built binary and these are that run's numbers. The subagent leg appears in the interactive run, where `agent` is reachable; the headless run's tool surface has 6 tools and spawned none. **One figure moves between runs and it is not the carrier:** the system message quotes the session's own home directory, and this harness gives each run its own, so the absolute sizes and the system-message digest shift by a few bytes when the output directory is renamed. Everything the proof asserts is invariant — the insertion's offset and length, the inserted bytes, the lane count, the two-launch identity and the subagent's byte-identity are all comparisons *within* one run |
| Q2 Qwen live, condition (vi), 56k local model | `sample-tiny`, **2,570 B** lane, `vibe agent qwen -p "<five questions>"`, local **`qwen3.8-27b`** from the operator's own `~/.qwen` (read, never written), under the owner's cost rule | Tail marker quoted **byte-equal** (`<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->`). «Files read with a tool: **none**» — router rule 1 held, the lane arrived as instructions. All four lane canaries **including the tail** plus `CANARY-ENTRYPOINT-QUARTZ`. **The base-only question**, whose answer exists nowhere in the lane and is the first sentence **after** the seam: «Text inside a `<qwen:user-prompt-submit-context>` tag is model context added by a configured `UserPromptSubmit` hook, not user input» — so the harness's own prompt survived the insertion. **Its own first sentence**, asked for directly: «You are Qwen Code, a non-interactive CLI agent developed by Alibaba Group, specializing in software engineering tasks.» — Qwen Code's own words, sampled from the installed package at launch and never stored by vibe | — | — | **pass (2026-11-11)**: the four byte-level conditions plus the two a byte diff cannot make |
| Q2 `vibe prompt flags --harness qwen` | `sample-big`, no arguments of its own | Printed `QWEN_SYSTEM_IDENTITY_MD=<settings>/agent/qwen/qwen-identity-headless-8d46a5e411b8bce3.md` on stdout — the assignment first, the empty flag line after — with three notes on stderr: the carrier and what is in the file (205,108 bytes, a 261-byte identity sentence), where the sentence was sampled from (the installed `@qwen-code/qwen-code` 0.27.2 package) and that the carrier's own check then passed on it, and the one that matters for a reader: the file was composed for the session shape **this command** has, because printing starts no session to classify | — | — | **pass (2026-11-11)** |
| Q2 Qwen soft launches, live | `sample-big`, `vibe agent qwen --safe-mode --version` and a launch under an unverified version | `--safe-mode`: «vibe cannot deliver this project's boot lane to `qwen` … reason: `--safe-mode` asks Qwen Code to disable every customization … delivering it anyway would be the opposite of what was asked», then real Qwen Code 0.27.2 started and answered. The warning is not suppressed by `--quiet`, and no carrier reached the child | — | — | **pass (2026-11-11)**: the launch honours the user's request and says what was not delivered |

## Findings

1. **Delivery works at the real size.** A 312 KB lane reaches Claude Code's
   slot whole: the tail marker is quoted byte-equal, and the router's rule 1
   holds, since no lane file is read with a tool. No size limit was hit up
   to 312,345 bytes.
2. **The cache claim is now measured, not argued.** At 202 KB, a repeat
   session in the same project had zero cache creation and an 85,262-token
   read. The same lane bytes at another path, with a different git state,
   gave a 75,803-token read. `--exclude-dynamic-system-prompt-sections`
   therefore does keep per-machine text out of the cached prefix. On this
   repository the repeat read was 183,981 tokens. This was measured on one
   account, one model and one Claude Code version within one hour. TTL,
   cost and latency were not measured.
3. **Classic mode is intact.** With no markers, the router's rule 4 produces
   the exact INDEX-order read, once each, both from a plain `claude2` and,
   after the fix, through `vibe agent`.
4. **Defect found and fixed:** `vibe agent` refused in a project that had not
   opted in (`6323e8a36`).
5. **Known limit, now documented in the product:** a resumed session keeps
   its starting lane, and a changed band-U file cannot be detected by the
   router. A new session is the remedy. `vibe agent` prints this on resume.
6. **Fixture note.** `CANARY-ENTRYPOINT-QUARTZ` lives only in the sample's
   `AGENTS.md`. Claude Code loads `CLAUDE.md`, so this canary never appears
   in a Claude session. That is expected and does not show a delivery gap.
7. **Qwen Code reads `AGENTS.md`, so the router needed no new file.** The
   intersection of what vibe writes (`CLAUDE.md`, `AGENTS.md`, `GEMINI.md`)
   and what Qwen Code reads (`QWEN.md`, `AGENTS.md`, both together) is
   `AGENTS.md`, and the block arrived through it in every Qwen leg. No
   `QWEN.md` is written: the same block in both names would be delivered
   twice.
8. **Qwen Code has four candidate carriers and each fails on a different
   axis.** `--append-system-prompt` is additive, cache-safe and worker-safe
   and takes **text**, so the lane may not travel in it. `QWEN_SYSTEM_MD`
   with `QWEN_WRITE_SYSTEM_MD` is file-based, worker-safe and carries 202 KB
   live, and it **replaces** a base prompt whose bytes depend on the
   session's shape, which a launcher cannot observe. The context-file layer —
   and the generated `QWEN.md` behind it — is additive, byte-stable and
   import-inert for this lane, and reaches every subagent. So `read` ships,
   and the two changes that would open `launch` are written down rather than
   waited for.

9. **OpenCode's `launch` tier ships, and the proof is a whole-request diff
   rather than a base comparison.** Conditions (0) and (i)–(v) are an
   equation over bytes — `carried[:offset] + carried[offset + length:] ==
   plain` — so "one contiguous insertion and nothing else changed" is a fact
   and not a reading of a diff tool's heuristics. Three things came out
   differently from the design: the inserted window is the lane file **byte
   for byte** (two named transformations cancel), the two launches are
   byte-identical **throughout** rather than merely up to the lane's end, and
   `--agent` turned out not to be a flag a launcher can pass at all — the
   document's own `default_agent` selects the agent instead, which is why a
   user's own `--agent` is now this kind's one conflicting flag.
10. **A `launch` default costs nothing where the data is absent.** Every
    refusal of the supplied-base path — no set, an unverified set, a digest
    mismatch, an unknown version, an unknown model, an inexpressible rule —
    is a soft launch naming which it was, so a machine with no prompt-base
    set for its installed OpenCode behaves exactly as it did before the tier
    existed.
11. **What the offline proof does NOT cover, stated rather than implied.** It
    runs under the loopback provider's own model, which matches no selection
    row, so what it proves about the base is the **default** base. The
    per-family files (`gpt`, `gpt-astra`, `kimi`, `meta`, `trinity`, and
    `anthropic` as an append) are held by their recorded digests and by the
    selection table's own `file:line` provenance, not by a capture. Proving
    those would need one provider per family.
12. **Qwen Code's fifth carrier is the one that ships, and it ships because
    the thing it replaces is knowable.** `QWEN_SYSTEM_IDENTITY_MD` replaces
    the base prompt's **first paragraph** and nothing else — upstream's own
    tests pin that — so the only fact a launcher has to supply is one
    sentence, and that sentence is a function of the interaction mode and
    the output style alone. vibe never stores it: it samples it at launch
    from the installed package's own `getCoreSystemPrompt`, and proves on
    that same package, in the same process, that the variable then yields
    the plain prompt with exactly one insertion. The contrast with
    `QWEN_SYSTEM_MD` is the whole lesson of both rows: the same mechanism
    is admissible or inadmissible depending on how much of someone else's
    prompt it makes you responsible for.
13. **The two conditions a byte diff cannot make are worth their cost
    every time.** The offline proof says the lane arrived and nothing else
    changed; it cannot say the harness's own instructions still work. The
    live leg asked for a rule that exists only in Qwen Code's own base and
    is the first sentence after the seam, and for the session's own first
    sentence — and both came back right, which is the only evidence that a
    byte-perfect insertion did not cost the harness its own voice.
