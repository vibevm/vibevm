# tcg-bench — the WITH-TOOLS arm vs the control baseline

_Recorded 2026-07-07, same box, same model
(`openrouter/z-ai/glm-5-turbo`), same twelve tasks and harness as
[REPORT-2026-07-07-control.md](REPORT-2026-07-07-control.md). Arm:
**with-tools** — the task prompt named the one-shot oracle forms
(`tcg-typescript validate/scope/complete/type`, full artifact path)
and asked the agent to consult them before and after each edit. Raw
rows: `with-tools-2026-07-07-0723.jsonl`._

## Result: 10 PASS / 2 FAIL — NO DELTA against control

| task | control | with-tools | Δ |
|---|---|---|---|
| 01-farewell-variant | PASS | PASS | — |
| 02-greet-many | PASS | PASS | — |
| 03-new-cell-announce | PASS | PASS | — |
| 04-reserved-name | FAIL (conform 1) | **FAIL (conform 1)** | none |
| 05-truncate-core | PASS | PASS | — |
| 06-greet-warmly | PASS | PASS | — |
| 07-digits-only | FAIL (conform 1) | **FAIL (conform 1)** | none |
| 08-farewell-all | PASS | PASS | — |
| 09-try-greet | PASS | PASS | — |
| 10-farewell-count | PASS | PASS | — |
| 11-polite-farewell | PASS | PASS | — |
| 12-greet-raw-string | PASS | PASS | — |

Aggregates: completion 12/12 both arms; mean wall 67.1 s with-tools vs
56.6 s control (noise-range slower); tsc/hallucination/test metrics
identical (all zero); **discipline regressions 2 → 2, the same two
tasks, the same rule**.

## The honest reading (plan §4.3: the prediction did NOT hold as stated)

The §4.3 prediction — tools available lowers the discipline-regression
count — was tested in its weakest delivery form: **opt-in one-shot CLI
named in the prompt**, with nothing forcing the consultation. At that
strength the effect is ZERO on this model and task set. The plausible
mechanism for the null: a weak model under a task prompt does not
spontaneously spend a tool call on verification it was not required to
perform — the same reason weak models skip running tests unless told
per-step. (The arm could not mount the MCP tools: the battery's
opencode runner has no vibevm MCP server configured; the CLI path was
the available surface.)

What this does NOT falsify: the oracle's mechanics (the differential
corpus agrees 7/7 at p50 19 ms; the live MCP chain answers enriched),
or the diagnosis that the failures live exactly where the oracle's
enrichment points (both FAILs are non-baselined `ts-unsafe-in-domain`
findings `tcg_validate` reports verbatim, with the guide-citing
advice). The gap is DELIVERY, not information: the answer exists, the
agent never asks.

## What follows (Stage-B backlog, owner's call)

1. **Forced-loop delivery**: a harness/hook where the write path runs
   `tcg_validate` on the hypothetical content automatically and feeds
   findings back — consultation as a gate, not an offer. (The token-
   level sibling is the extreme of the same idea; a write-hook is its
   cheap agentic approximation.)
2. **MCP-mounted arm**: register the vibevm MCP server in the battery
   runner's config so `tcg_*` are first-class tools with the skill's
   "consult before you write" teaching in context; measure whether
   tool-affinity (vs shell-command-affinity) changes uptake.
3. **Uptake metric**: count actual oracle invocations per run (parse
   the agent event stream for the tool/bash calls) so future arms
   separate "consulted and ignored" from "never consulted".

n=12, one model — direction-grade evidence only, as posted in the
plan's predictions section.
