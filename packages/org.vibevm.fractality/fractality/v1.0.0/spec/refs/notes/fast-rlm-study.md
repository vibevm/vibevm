# Study note — fast-rlm (the engineering-complete RLM runtime) {#root}

_T2 note (boss-authored) for INVENTORY S11 — repo pin `f25f310`
(MIT, avbiswas, v0.4.1, ~7.1k LOC: Deno/TS engine + Python facade
+ React TUI). First-pass survey delegated to GLM-5.2 over a
sandboxed copy; boss spot-checked the load-bearing claims verbatim
(depth throw + leaf prompt swap, dollar-budget enforcement, ACP
tool-stripping deny-lists) — all held. Decisions and facts only._

## What it is {#what}

An independent, feature-complete RLM runtime: Pyodide (WASM
Python) inside Deno; each agent turn is one fenced repl block
executed against a per-agent REPL; `llm_query` re-enters the same
engine at depth+1; `batch_llm_query` fans out via asyncio.gather;
the child's `FINAL(x)` returns to the parent **as a native Python
object** ("the return value is the actual Python object", the
model is told not to re-parse it) — symbols, never transcripts.

## Facts that matter to us {#facts}

- **The most complete budget lattice in the field**, all enforced
  in the engine loop, not the prompt: `max_depth` (3; at the cap
  the agent gets a LEAF system prompt with no `llm_query` docs AND
  the call throws "MAXIMUM DEPTH REACHED — solve on your own"),
  `max_calls_per_subagent` (20), `max_money_spent` ($0.20 default,
  cumulative cost check per call), `max_completion_tokens`
  (cumulative), `max_prompt_tokens` (per-call ceiling),
  `max_global_calls` (∞; 50 under ACP). Exhaustion = structured
  fatal error propagated up the recursion into the run's output
  JSON.
- **Nothing is inherited implicitly:** a child gets a fresh REPL,
  the passed context (rehydrated to a real object), optional
  schema/tools/instruction/MCP allow-list — each grant re-passed
  explicitly per call. (I1's spirit, discovered independently.)
- **Structured IO end-to-end:** output schemas (Pydantic / JSON
  Schema) validated by Ajv against FINAL, with per-path errors fed
  back for retry within budget; parents impose schemas on children
  the same way; dict contexts get a step-0 top-level schema print
  so the model indexes instead of reading.
- **Observability quartet:** Pino JSONL per run (`run_id`,
  `parent_run_id`, `depth`, step, code, output, usage — the run
  tree reconstructs from the log), depth-indented terminal panels,
  a live `on_step` NDJSON tail callback, and a React TUI that
  renders the RunTree forest with per-step drill-down. Plus
  resumable on-disk sessions.
- **ACP backend (the surprise):** drives Claude Code / Codex /
  opencode *as models* via the Agent Client Protocol — launched in
  a throwaway temp cwd with injected config that STRIPS their own
  tools (CC: `.claude/settings.json` permissions.deny on
  Bash/Read/Write/Edit/WebFetch/WebSearch; opencode: per-tool
  "deny"; codex: empty sandbox permissions), forcing all
  computation through the observable REPL. ACP agents report zero
  token usage, so only call-count budgets bite.
- The delegation "compression guard": a self-check
  (`confirmDelegation`) reviews sub-spawn requests; batch calls
  get one batch-level review.

## Decisions we take {#decisions}

1. **The budget lattice is the checklist for packet quotas
   (Stage B):** depth / per-agent calls / per-call token ceiling /
   cumulative tokens / currency / global calls — six distinct
   axes, each with its own failure message. MC quotas should name
   the same axes (plus wall-clock, which fast-rlm lacks and MC
   already owns).
2. **Leaf-mode = prompt + capability swap at the cap** — the
   fourth boundary behavior (with ReDel's tool removal,
   recursive-llm's soft refusal, ROMA's force-execute): change
   BOTH the surface (no spawn docs) and the enforcement (throw).
   Belt and suspenders; adopt for worker profiles at depth ceiling.
3. **Schema-validated returns with retry-on-error** is the
   structured-outcome contract our packets want: validate the
   worker's result against the packet's output schema at the seam,
   feed violations back once within budget before failing the run.
4. **Explicit re-granting per spawn** (no ambient inheritance)
   confirms I1 as market practice; extend the same rule to MCP
   grants in worker profiles.
5. **Config-injected tool-stripping on our own substrate** (their
   ACP deny-lists) is exactly the mechanics for V1
   promotion/demotion on Claude Code — the same settings surface
   our initiative adapter already writes. Promotion = rewriting
   the child's settings/permissions at spawn; no new machinery.
6. **run_id/parent_run_id in every log line** — I3's journal
   already has run identity; ensure depth + parent ids ride every
   event so the tree view is a pure log fold (their TUI proves the
   payoff).

**Non-adoptions:** Pyodide/Deno substrate (ours is OS processes);
prompt-fenced repl-block protocol; zero-usage ACP accounting (our
workers meter for real — a fabric advantage worth stating in the
savings methodology).
