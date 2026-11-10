# P9 — live check results, Claude Code

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
| C8 Windows | — | — | — | — | **not run**: no Windows host in this session |

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
