# Reference-source inventory (clean-room register) {#root}

_The single committed record of every external source this project studies.
Clones and downloads live under the host `/refs/` tree, which is gitignored
wholesale — nothing third-party is ever committed. The host clean-room
directive (2026-07-07, extended to this workspace 2026-07-09) binds every
row: sources are **inspiration-only**; the working method is study → a
decisions-only study note under `notes/` → implementation from the note.
No line porting, no file-by-file adaptation, no borrowed expression.
License and commit pin are recorded **before** any study deeper than
LICENSE + README (IGNITION plan, Phase 0 s6)._

## Rules

1. A source may be read only after its row carries a license verdict.
2. Study notes record *what the source achieves and which decisions we
   take*, never its text or code shapes.
3. Implementation sessions open the study note, not the source.
4. A row's `class` is `inspiration-only` unless the owner explicitly
   clears something stronger.
5. Methods described in papers are implementable freely (methods are not
   copyrightable); their *reference code* stays inspiration-only.

## Sources

| id | source | local path (host /refs/) | pin | license | class | study note | status |
|---|---|---|---|---|---|---|---|
| S1 | github.com/steipete/agent-scripts — `skills/codex-first/SKILL.md` (delegation-first rules) | src/agent-scripts/ | Ф0.s6 | Ф0.s6 | inspiration-only | notes/codex-first-study.md (Ф0.s6) | pending intake |
| S2 | github.com/barkain/claude-code-workflow-orchestration (initiative/orchestration prototype; owner: early prototype, do not imitate the implementation) | src/claude-code-workflow-orchestration/ | Ф0.s6 | Ф0.s6 | inspiration-only | notes/barkain-study.md (Campaign 2) | pending intake |
| S3 | github.com/alexzhang13/rlm (RLM reference implementation, Python) | src/rlm/ | Ф0.s6 | Ф0.s6 | inspiration-only | notes/rlm-study.md (Campaign 3) | pending intake |
| S4 | arXiv 2512.24601 — Recursive Language Models (paper, open access) | papers/2512.24601.pdf | n/a | arXiv license (record exact variant at download) | method: free to implement; text: cite, never copy | notes/rlm-study.md (Campaign 3) | pending intake |
| S5 | z.ai GLM coding-plan + Claude Code integration docs (base URL, model ids, env vars, quotas, pricing) | src/zai-docs/ | snapshot date | vendor docs — facts only, no text reuse | facts source | folded into plan §5 / D6 / D12 (Ф0.s3) | pending intake |
| S6 | Anthropic Claude Code docs — headless mode, settings/env, hooks (paved-road surface) | src/cc-docs/ | snapshot date | vendor docs — facts only, no text reuse | facts source | folded into plan §5 / D6 (Ф0.s3) | pending intake |
| S7 | Landscape scan: claude-code-router, claude_swarm, claude-squad, and whatever Ф0.s3 surfaces | src/landscape/ (shallow, optional) | Ф0.s7 | per repo | positioning only — never studied for implementation | notes/landscape.md (Ф0.s7) | pending intake |
| S8 | Entire.io — Checkpoints (agent execution checkpointing: scope + history control, rewind/audit) | src/entire-checkpoints/ (future) | future campaign | future | inspiration-only | notes/checkpoints-study.md (future campaign) | named, not intaken — DEF-12: owner ruling 2026-07-09, too young to adopt now |

## Log

- 2026-07-09 — inventory created at ignition; all rows pending intake
  (IGNITION Phase 0 s6/s7 fills pins, licenses, and the S1 study note).
