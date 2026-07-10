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

## Log

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
