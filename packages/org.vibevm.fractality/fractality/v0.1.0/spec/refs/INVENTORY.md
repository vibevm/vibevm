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
| S1 | github.com/steipete/agent-scripts — `skills/codex-first/SKILL.md` (delegation-first rules) | src/agent-scripts/ | `d6ed98c` (2026-07-09) | **MIT** (Peter Steinberger) | inspiration-only | notes/codex-first-study.md ✅ | **studied; note on file; clean** |
| S2 | github.com/barkain/claude-code-workflow-orchestration (initiative/orchestration prototype; owner: early prototype, do not imitate the implementation) | src/claude-code-workflow-orchestration/ | `175311b` (2026-06-20) | **MIT** (Nadav Barkai) | inspiration-only | notes/barkain-study.md ✅ | **studied 2026-07-10 (Campaign 2 open); note on file; clean** |
| S3 | github.com/alexzhang13/rlm (RLM reference implementation, Python) | src/rlm/ | `72d6940` (2026-06-25) | **MIT** (Alex Zhang) | inspiration-only | notes/rlm-study.md (Campaign 3) | cloned, license cleared; deep study deferred to Campaign 3 |
| S4 | arXiv 2512.24601 — Recursive Language Models (paper, open access) | papers/2512.24601.pdf (9.9 MB) | downloaded 2026-07-09 | arXiv (record exact variant at read) | method: free to implement; text: cite, never copy | notes/rlm-study.md (Campaign 3) | downloaded; read deferred to Campaign 3 |
| S5 | z.ai GLM coding-plan + Claude Code integration docs (base URL, model ids, env vars, quotas, pricing) | src/zai-docs/ | snapshot 2026-07-09 | vendor docs — facts only, no text reuse | facts source | folded into plan §5 / D6 / D12 ✅ | **facts extracted (Ф0.s3)** |
| S6 | Anthropic Claude Code docs — headless mode, settings/env, hooks (paved-road surface) | src/cc-docs/ | snapshot 2026-07-09 | vendor docs — facts only, no text reuse | facts source | folded into plan §5 / D6 / D18 ✅ | **facts extracted (Ф0.s3)** |
| S7 | Landscape scan: claude-code-router, claude_swarm, claude-squad | src/landscape/ (shallow) | snapshot 2026-07-09 | per repo | positioning only — never studied for implementation | notes/landscape.md ✅ | **scanned (Ф0.s7)** |
| S8 | Entire.io — Checkpoints (agent execution checkpointing: scope + history control, rewind/audit) | src/entire-checkpoints/ (future) | future campaign | future | inspiration-only | notes/checkpoints-study.md (future campaign) | named, not intaken — DEF-12: owner ruling 2026-07-09, too young to adopt now |
| S9 | Amazon S3 ranged-read & presign semantics + RFC 7233 (byte ranges, suffix ranges, ETag/If-Match, presigned URLs, part-aligned parallel GETs) | public standards/docs — no clone needed | n/a | public API semantics / IETF standard | facts source — API *shapes* adopted, no code involved | folded into plan D19 | adopted 2026-07-09 (owner directive, fifth message) |
| S10 | github.com/sentient-agi/ROMA (recursive plan/execute meta-agent; the Atomizer need-gate) | src/roma/ | at Ф3 clone | **Apache-2.0** (API-verified 2026-07-10) | inspiration-only | notes/roma-study.md (T2) | selected R2 (rlm-source-selection.md) |
| S11 | github.com/avbiswas/fast-rlm (symbol-returning sub-agents; depth/call/$/token budgets) | src/fast-rlm/ | at Ф3 clone | **MIT** (API-verified 2026-07-10) | inspiration-only | notes/fast-rlm-study.md (T2) | selected R3 |
| S12 | github.com/zhudotexe/redel + arXiv 2408.02248 (recursive delegation toolkit, EMNLP 2024 demo) | src/redel/ | at Ф3 clone | **MIT + Commons Clause** (LICENSE-verified 2026-07-10; study-only — irrelevant to us, no code adopted from any source) | inspiration-only | notes/redel-study.md (T2) | selected R4 |
| S13 | github.com/grishahq/recursive-llm (minimal RLM, enforced max_depth) | src/recursive-llm/ | at Ф3 clone | **MIT** (API-verified 2026-07-10) | inspiration-only | notes/recursive-llm-study.md (T2) | selected R5 |
| S14 | arXiv 2506.16411 — When Does Divide and Conquer Work for Long Context LLM? (ICLR 2026) | papers/2506.16411.pdf | v2 2026-02-28 | arXiv — method free to implement, text cite-only | method source | notes/dnc-noise-study.md (T2) | selected P2 |
| S15 | arXiv 2510.11967 — Context-Folding (ByteDance/CMU); official repo sunnweiwei/FoldAgent NOT intaken (license unverified) | papers/2510.11967.pdf | — | arXiv — method free, cite-only | method source | notes/context-folding-study.md (T2) | selected P3 |
| S16 | arXiv 2603.15653 — SRLM (Apple; recursion-vs-REPL ablation critique) | papers/2603.15653.pdf | — | arXiv — method free, cite-only | method source | notes/srlm-study.md (T2) | selected P4 |
| S17 | arXiv 2605.06639 — Recursive Agent Optimization (learned delegation policy) | papers/2605.06639.pdf | — | arXiv — method free, cite-only | method source | notes/rao-study.md (T2) | selected P5 |
| S18 | alexzhang13.github.io/blog/2025/rlm/ (anchor project's blog face) | articles/zhang-rlm-blog.html | snapshot at Ф3 | web article — cite-only, no text reuse | facts source | notes/rlm-study.md (T1, shared with S3/S4 per D-R7) | selected A1 |
| S19 | primeintellect.ai/blog/rlm (ecosystem state mid-2026) | articles/primeintellect-rlm.html | snapshot at Ф3 | web article — cite-only | facts source | notes/rlm-articles-t3.md (grouped T3) | selected A2 |
| S20 | cognition.com/blog/dont-build-multi-agents + /multi-agents-working (the counterpoint arc) | articles/cognition-dont-build-multi-agents.html, articles/cognition-multi-agents-working.html | snapshot at Ф3 | web articles — cite-only | facts source | notes/rlm-articles-t3.md | selected A3 |
| S21 | anthropic.com/engineering/multi-agent-research-system (production orchestrator-worker) | articles/anthropic-multi-agent-research.html | snapshot at Ф3 | web article — cite-only | facts source | notes/rlm-articles-t3.md | selected A4 |
| S22 | avilum.github.io/minrlm practical guide + benchmark (repo avilum/minrlm MIT, 72★) | articles/minrlm-guide.html | snapshot at Ф3 | web article cite-only; repo MIT | facts source | notes/rlm-articles-t3.md | selected A5 |
| S23 | arXiv 2405.17402 — THREAD: Thinking Deeper with Recursive Spawning (NAACL 2025) | papers/2405.17402.pdf | — | arXiv — method free, cite-only | method source | notes/rlm-runners-up-t3.md (grouped T3) | runner-up (filtered-return contract) |
| S24 | arXiv 2603.02615 — Think, But Don't Overthink (RLM reproduction; counterpoint numbers) | papers/2603.02615.pdf | — | arXiv — method free, cite-only | method source | notes/rlm-runners-up-t3.md | runner-up (T3 paragraph mandatory) |
| S25 | github.com/brainqub3/claude_code_RLM (RLM scaffold on Claude Code) | — (web README level only, no clone) | n/a | **MIT** (API-verified 2026-07-10) | inspiration-only | notes/rlm-runners-up-t3.md | runner-up |
| S26 | github.com/tinyhumansai/tinyagents (Rust RLM harness; run-tree/cost-rollup telemetry) | — (web README level only, no clone) | n/a | **GPL-3.0** (API-verified 2026-07-10; study-only, code never adopted) | inspiration-only | notes/rlm-runners-up-t3.md | runner-up |

## Log

- 2026-07-10 (late) — RLM research Ф2: S10–S26 registered from the
  three-wave selection (`notes/rlm-source-selection.md`); every row
  carries its license verdict BEFORE study per rule 1. S4 note: the
  local PDF is v1; the paper is at **v3 (2026-05-11)** — re-fetch at
  Ф3 and record the version. S15 note: FoldAgent repo deliberately
  not intaken (license unverified); the paper carries the idea.
- 2026-07-09 — inventory created at ignition; all rows pending intake
  (IGNITION Phase 0 s6/s7 fills pins, licenses, and the S1 study note).
- 2026-07-10 — Campaign 2 open: S2 deep-studied → `barkain-study.md`
  (BD1–BD6 keeps + named non-adoptions). Survey delegated to the big
  slot over a sandboxed copy; boss spot-checked the load-bearing claims
  against the source. Clean-room posture held (concept-level note, no
  text/code carried).
- 2026-07-09 — Ф0 intake: S1–S4 cloned/downloaded at pinned commits,
  **all three repos MIT** (clean-room posture holds regardless — no
  code ported). S1 codex-first fully studied → `codex-first-study.md`
  (decisions DC1–DC6 + the mandated improvements). S5/S6 facts
  extracted (see the plan's Ф0 findings). S7 landscape scanned →
  `landscape.md`. S2/S3/S4 deep study deliberately deferred to
  Campaigns 2/3 per the plan; licenses are cleared so those campaigns
  may open without a legal gate.
