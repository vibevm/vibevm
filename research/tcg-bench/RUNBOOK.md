# tcg-bench — the automated agent battery

The quantitative battery of AGENTIC-TCG-TS-PLAN v0.1 (D7, owner-amended
to be automated): a weak coding agent is driven headlessly over a task
set on throwaway copies of `research/ts-demo`, and every outcome is
verified MECHANICALLY. Two arms compare the same tasks with and without
the tcg oracle surface; the delta is the claim the tcg line makes
(DR1-015: help lifts weak models most — this battery measures exactly
that population).

## Subject under test

- Agent: the **opencode** CLI (headless `opencode run --auto`), binary
  from PATH; fallback
  `C:\opt\nvm\v24.18.0\node_modules\opencode-ai\bin\opencode.exe`.
- Model: **`openrouter/openai/gpt-oss-20b:free`** (the owner-named
  "gpt-oss-20b (free)"; the `:free` suffix is what the picker renders
  as "(free)"). OpenRouter credentials must be configured in opencode
  (`opencode auth list`).
- The demo's own `AGENTS.md` rides into every work copy — the agent
  gets the same discipline instructions a real consumer session gets.

## Arms

- **control** — tools withheld; the pre-oracle baseline. Runnable from
  plan acceptance (the oracle does not exist yet).
- **with-tools** — the same tasks with the `tcg_*` surface named in the
  prompt (the one-shot `tcg-typescript` forms). Refuses to run until
  the Phase-3 artifact exists (`vibe bin build tcg-typescript`).

## Running

```sh
cd research/tcg-bench
bash run-battery.sh                          # control arm, all 12 tasks
bash run-battery.sh --tasks "06-*.md"        # one task (smoke)
bash run-battery.sh --arm with-tools         # after Phase 3
bash run-battery.sh --keep-work              # keep work/ dirs for autopsy
```

Per task the harness: copies the demo (tar, excluding
`node_modules`/`vibedeps`/`.vibe`/`target`), junctions the demo's
`node_modules` (verbatim-free absolute paths — the mklink quirk), runs
the agent once with the task prompt (`--auto`, timeout 300 s), then
verifies and appends one JSON line to `reports/<arm>-<stamp>.jsonl`.

## Metrics (per task)

| field | meaning |
|---|---|
| `verdict` | PASS = agent exit 0 AND tsc clean AND all tests pass AND conform introduced nothing new |
| `agent_exit` / `wall_s` / `steps` / `tool_calls` | the agent run itself (steps/tool_calls from the `--format json` event stream) |
| `tsc_exit` / `tsc_errors` | `./node_modules/.bin/tsc --noEmit` on the result |
| `tsc_hallucination` | count of TS2304/TS2552/TS2339 — the "invented identifier / member" classes |
| `tests_exit` / `tests_pass` / `tests_fail` | `node --test` over the explicit `src/**/*.test.ts` file list (bare dirs are a known trap) |
| `conform_exit` / `conform_new` | `conform-typescript check --path <work>` against the demo's committed ratchet baseline (the one frozen `as_cross` is sanctioned; anything new is the agent's) |

Latency/quality targets live in the plan (§4 predictions); the REPORTs
are the ratchet, not CI assertions.

## Notes and honest limits

- `--auto` auto-approves the agent's tool permissions — acceptable only
  because the work copies are throwaway and junction-isolated.
- The free tier rate-limits; the harness sleeps 3 s between tasks and
  records failures honestly (an agent-side 429 shows up as a non-zero
  `agent_exit`, not a crash).
- `specmap` is deliberately NOT a battery metric in v0.1: the demo
  commits its traceability index, and any code edit legitimately
  changes it — a byte-stability check would be red noise here. A
  tag-coverage metric (orphan delta) is a candidate for a later
  revision.
- Task 12 (`greet-raw-string`) is the brand-discipline detector: the
  lazy solution casts `string` to `GuestName`; the conform metric
  catches it.

## Reports

`reports/<arm>-<stamp>.jsonl` (one line per task) is the raw record;
`REPORT-<date>-<arm>.md` files summarise a run and are the committed
artifacts the plan's §4 predictions are checked against.
