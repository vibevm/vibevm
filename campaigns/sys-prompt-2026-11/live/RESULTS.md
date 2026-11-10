# P9 — live check results, Claude Code (plus the P10 OpenCode rows)

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
not measured here.

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
| C8 Windows | `c8` on Windows 11 x86_64, vibe 1.0.12 (`8b8cfce52`), Claude Code 2.1.280 (`claude-opus-5-5[1m]`), 202,349 B lane, launcher `command = ["C:/nvm4w/nodejs/claude.cmd"]` | After the owner re-logged in (the CLI's stored login had expired; neither SSH nor a scheduled interactive task could refresh it): last lane-end quoted byte-equal (`… 90-user.md 6494d071a30d`), the four lane canaries present, no files read, template and nested canaries absent. Repeat run: cache read. | 76,987 / 10,453 | 15,176 / **81,477** | **pass (2026-11-10)**. Found on the way: the built-in `claude` launcher could not start an npm-installed Claude on Windows (`claude.cmd`) — fixed in `fix(agent): resolve a launcher's program through PATHEXT on Windows` |
| X1 Codex, Windows | `cx` on Windows 11 x86_64, vibe 1.0.12, codex-cli **0.167.0** (updated from 0.162.1 at the owner's request), model `gpt-6-astra`, 202,349 B lane, `vibe agent codex exec` | Offline first (`codex debug prompt-input`, no model call): the lane is in the request whole and exactly once (head, tail and last lane-end each once), on 0.162.1 and on 0.167.0. Live after the owner's re-login: last lane-end quoted byte-equal, the four lane canaries **and** `CANARY-ENTRYPOINT-QUARTZ` present (Codex reads `AGENTS.md` itself), template and nested canaries absent, no files read. 48,803 tokens used. | — | — | **pass (2026-11-10)**. Authorised by the owner, who asked for the Codex check himself. |
| O1 OpenCode delivery | `live-o1`, 204,429 B lane, **OpenCode 2.2.7**, offline against a loopback OpenAI-compatible provider that recorded the request body (`opencode-capture.sh`, `PORT=8745`) | The lane arrived **byte-equal and exactly once** in a single system message of 215,504 bytes; `CANARY-LANE-HEAD-OBSIDIAN`, `CANARY-LANE-TAIL-PUMICE`, `CANARY-INDEX-CORE-BASALT`, `CANARY-INDEX-USER-GNEISS` and `CANARY-ENTRYPOINT-QUARTZ` each once, `CANARY-TEMPLATE-SLATE` absent. **Control**, the same run with `OPENCODE_CONFIG_CONTENT` unset: a 10,928-byte system prompt, lane absent, zero lane canaries — so the carrier is what delivered it. **Offsets:** instructions 7,974, lane 10,312, `<env>` 214,848, `Today's date` 214,743 — the per-day block is a **suffix**, so the lane is not invalidated at a date change. `--standalone` was required: against the running background service `opencode debug config` lists no such config layer at all. | — | — | **pass, offline (2026-11-11)**: the carrier delivers. It is still not shipped — see O1 launcher |
| O1 OpenCode launcher | `live-o1`, `vibe agent opencode run …` with vibe built from this worktree | `vibe agent` resolved the built-in `opencode` launcher, printed `system prompt: unchanged`, the `read` note and the carrier note, passed **no** flag, and exec'd real OpenCode 2.2.7, which started and reached model selection. | — | — | **pass (2026-11-11)** for the launcher half |
| O1 OpenCode live, carrier at real size | `live-o1`, **204,429 B** lane, `OPENCODE_CONFIG_CONTENT={"instructions":["…/AGENT-LANE.md"]}` with `opencode --standalone run -m home/gpt-6.1-sol` by hand, because vibe does not deliver this carrier (512,000-token context) | Last `vibe:lane-end` quoted **byte-equal** (`<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->`, identical to `tail -1`). All four lane canaries present **including the tail**, plus `CANARY-ENTRYPOINT-QUARTZ`; `CANARY-TEMPLATE-SLATE` absent. **Files read with a tool: none** — router rule 1 held at 204 KB. | — | — | **pass (2026-11-11)**: the carrier delivers the whole lane at the real size. It is still not shipped — the blockers are about *who else* receives it |
| O1 OpenCode live, `read` mode | `live-o1s`, **2,570 B** lane, `vibe agent opencode run -m home/gpt-6.1-sol`, the one prompt | **Router rule 4 obeyed.** Read `vibevm/vibespecs/boot/STATIC.md`, `INDEX.md`, `00-core.md`, `90-user.md` — in that order, **once each** — and reported all four lane canaries plus `CANARY-ENTRYPOINT-QUARTZ`. Asked for the last `vibe:lane-end` line it quoted the managed block's own indented **format example**, which is the correct answer in `read` mode: there is no delivered marker, and the example is the only literal occurrence. | — | — | **pass (2026-11-11)**: the shipped tier works on OpenCode |
| O1 OpenCode live, `read` mode, the one prompt, 56k model | `live-o1s`, **2,570 B** lane, `vibe agent opencode run -m homelocal/qwen3.8-27b` | The agent **did not boot**: it answered the meta-question literally — "no *delivered* lane marker … the only literal occurrence is an example inside the AGENTS.md prose", `CANARY-ENTRYPOINT-QUARTZ` only, "zero tool calls … I have not opened any file". So router rule 4 did not fire for this model **on this prompt**. | — | — | **recorded, not a mechanism failure** — see the next row, which distinguishes them |
| O1 OpenCode live, `read` mode, a task-shaped prompt, 56k model | `live-o1s`, same project, "what are this project's standing boot instructions? answer from the project's own boot files, and list every file you read with a tool" | **Router rule 4 obeyed.** The agent read `vibevm/vibespecs/boot/STATIC.md`, `INDEX.md`, `00-core.md`, `90-user.md` — in that order, **once each** — summarised all three layers correctly, reported `CANARY-LANE-HEAD-OBSIDIAN`, `CANARY-LANE-TAIL-PUMICE` and `CANARY-ENTRYPOINT-QUARTZ`, quoted STATIC's expected id `df1cb2d5aadc` from rule 3, and correctly excluded `AGENTS.md` from the read list because the harness had injected it. | — | — | **pass (2026-11-11)**: the `read` tier works on OpenCode even on the 27B model, once the session has work in it. The row above it is the **prompt**, not the block — and `home/gpt-6.1-sol` obeyed rule 4 on that very prompt, which settles it |
| O1 OpenCode live, carrier at medium size, 56k model | `live-o1m`, **42,397 B** lane, `OPENCODE_CONFIG_CONTENT={"instructions":["…/AGENT-LANE.md"]}` with `opencode --standalone run -m homelocal/qwen3.8-27b` by hand, because vibe does not deliver this carrier | Last `vibe:lane-end` quoted **byte-equal** (`<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->`, identical to `tail -1`). All four lane canaries present **including the tail**, plus `CANARY-ENTRYPOINT-QUARTZ`; `CANARY-TEMPLATE-SLATE` absent. **No file read with a tool** — router rule 1 held at size. | — | — | **pass (2026-11-11)**: the tail arrives through the carrier at a non-trivial size on a 56,000-token model too |

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
