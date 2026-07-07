# CONTINUE.md — cold-resume checkpoint

_Written 2026-07-07 (night; third campaign of the day). **The AGENTIC-TCG
CAMPAIGN (`spec/terraforms/AGENTIC-TCG-TS-PLAN-v0.1.md`) is COMPLETE —
Phases 0–7, EXECUTED status in the plan, floor green at close.** The
agentic type oracle is real end to end: the node LanguageService oracle,
the persistent Rust bridge, the `tcg-typescript` slot binary with
discipline enrichment, the portable `vibe-tcg` crate mounted by vibe-mcp
as four `tcg_*` tools, full spec parity, and the automated two-arm
battery with its honest null result. typescript-ai-native is **0.4.0**.
Local on `main`, **~90 commits ahead of origin `c3fcf63`, NONE mirrored —
the mirror stays HELD for the owner's explicit word.**_

> **`spec/WAL.md` is the canonical living state**; if this snapshot and
> the WAL disagree, the WAL wins. The **git log is the authoritative
> per-item record**. Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its
> files → `spec/WAL.md`), then read this.

---

## TL;DR

The owner asked why token-level type-constrained decoding needs an LLM at
all when we live inside agents — the answer (only the GUARANTEE needs
logits; information, feedback latency, and generation-time discipline
ship today as tools) became AGENTIC-TCG-TS-PLAN v0.1, owner-amended
(portability: the tool family must lift into a standalone MCP server;
battery: automated via opencode) and executed the same day. Everything
shipped and everything is green; the battery's §4.3 prediction was
FALSIFIED in its opt-in form and recorded honestly — a weak model never
spontaneously calls a tool it is merely offered. Token-level TCG is
re-dispositioned VERY-FAR-FUTURE in its own brief.

## Where work stands

- **Branch `main`**, working tree clean after the checkpoint commits;
  ~90 ahead of `origin/main` (`c3fcf63`), NOT mirrored (policy hold).
- Versions: discipline-core 0.4.0, rust-ai-native 0.4.0,
  typescript-ai-native **0.4.0** (+2 crates `tcg-oracle-bridge` /
  `tcg-cli-typescript`, +1 tool `tools/ts-oracle`, +3 spec docs, 4th
  `[[binary]]`). vibevm gains product crate `crates/vibe-tcg` +
  `vibe_workspace::bins` + the vibe-mcp adapter cell. Registry publish
  of all three packages: owner call, not done.
- Floor at close: `self-check.sh` 13 steps exit 0; conform 0 (11 gated /
  4 exempt); specmap 592/578/590, 0 orphans/0 warnings; ts-demo floor
  7/7; `fresh_ts_project` green; `live_chain_on_ts_demo` (ignored-by-
  default, real chain) green; oracle node tests 11/11; corpus agreement
  100% @ p50 19.3 ms.

## The open items (owner-court)

1. **Mirror ~90 commits** — `cargo xtask mirror --check` then
   `cargo xtask mirror`, on the owner's word only.
2. **Publish 0.4.0/0.4.0/0.4.0** to the registry — owner call.
3. **Stage-B delivery experiments** (the null's follow-up, in
   `research/tcg-bench/reports/REPORT-2026-07-07-with-tools.md`):
   forced-loop write-path hook; an MCP-mounted battery arm (register
   vibevm's MCP in the opencode runner config); an uptake metric
   (count actual oracle calls per run). Commission separately.
4. **The Rust agentic twin** (`tcg_rust` over rust-analyzer) — the
   language parameter and PROP-026 are cut to admit it; separate plan.
5. Token-level TCG — VERY-FAR-FUTURE (owner disposition in
   `…/tools/vibe-tcg-ts.md`); waits on `vibe-llm` + local inference.
6. PROP-025 v2 surfaces unchanged (shims, §6 rewriting, GC).

## Quick-start (verify the tree)

```sh
bash tools/self-check.sh; echo "EXIT=$?"        # 13 steps, must be 0
cargo run -q -p vibe-cli -- bin list             # 7 binaries; tcg-typescript listed
cargo run -q -p vibe-cli -- bin exec tcg-typescript -- \
    validate src/cells/greeting/index.ts --root research/ts-demo
                                # 0 diagnostics; 1 finding [baselined]; exit 0
cargo run -q -p vibe-cli -- bin exec tcg-typescript -- \
    bench --corpus research/tcg-bench/corpus \
    --report /tmp/r.json --root research/ts-demo   # agreement 100%
cargo test -p vibe-mcp --test tcg_tools -- --ignored  # the live chain
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n{"jsonrpc":"2.0","id":2,"method":"tools/list"}\n' \
  | cargo run -q -p vibe-cli -- mcp serve --path research/ts-demo
                                # tools/list carries tcg_validate/scope/complete/type
cd research/tcg-bench && bash run-battery.sh     # control arm (GLM-5-Turbo)
```

## Non-obvious findings (this campaign; full list in the WAL section)

- The LanguageService serves CACHED programs for reused script-version
  numbers: ephemeral overlays must draw from a session-monotonic
  counter, and disk files must version by mtime. The differential
  corpus caught it on its first run (five 1.2 ms answers from the
  clean-disk cache); the e2e test could not (one overlay per file).
- node dies instantly on `\\?\`-verbatim entry paths —
  `canonicalize()` output is verbatim-stripped before node argv
  (bridge `verbatim_free`; the lesson's third home).
- The serve relay owns session init (a host's first frame is validate,
  not init).
- **An opt-in tool is a tool a weak model does not call**: with-tools
  10/2 == control 10/2, same two `ts-unsafe-in-domain` regressions
  (parseGuestName-extension tasks 04/07). Delivery, not information,
  binds; the oracle's information is exactly right (the failures are
  verbatim what tcg_validate reports with advice).
- gpt-oss-20b:free is not battery-grade (do-nothing runs, truncated
  streams at exit 0); GLM-5-Turbo via OpenRouter is the pinned
  fallback per the owner's directive.
- MCP-held `vibe.exe` blocks the workspace test rebuild; terminate,
  sessions respawn.

## Repository map (delta over the deferrals-closeout map)

```
vibevm/
├─ crates/vibe-tcg/               NEW product crate: tcg tool family behind
│                                  TcgHost; OracleRegistry; zero vibe-mcp deps
├─ crates/vibe-workspace/src/bins.rs   NEW shared cell: DeclaredBinary,
│                                  collect/find/consent/build (CLI + registry)
├─ crates/vibe-mcp/src/tcg.rs     NEW thin adapter: 4 tcg_* McpTool cells
├─ spec/modules/vibe-mcp/PROP-026-tcg-tool-family.md   NEW product seam spec
├─ spec/terraforms/AGENTIC-TCG-TS-PLAN-v0.1.md         EXECUTED
├─ research/tcg-bench/            NEW: run-battery.sh + 12 tasks + .check
│   ├─ corpus/                    7 differential cases + overlay contents
│   └─ reports/                   control + with-tools + bench baselines
└─ packages/org.vibevm/typescript-ai-native/v0.4.0/
    ├─ tools/ts-oracle/           NEW: oracle.ts + node:test suite (11)
    ├─ crates/tcg-oracle-bridge/  NEW: embed+materialise, transport, taxonomy
    ├─ crates/tcg-cli-typescript/ NEW: bin tcg-typescript (serve/one-shot/bench)
    └─ spec/typescript/
        ├─ tools/vibe-agentic-tcg-ts.md    NEW seven-section brief
        ├─ tools/vibe-tcg-ts.md            token-level: VERY-FAR-FUTURE
        └─ mechanisms/TCG-ORACLE-v0.1.md, TCG-PROTOCOL-v0.1.md   NEW req units
```

## Recent commit chain (campaign, newest first — see git log for all)

```
docs(wal)/docs(continue)      this checkpoint
docs(plan)+feat(research)     EXECUTED status, s4.3 outcome, with-tools report
build(deps)                   re-materialise with the oracle version fix
test(research)                differential corpus + bench baseline (100%)
fix(typescript-ai-native)     session-monotonic script versions (corpus catch)
docs(typescript-ai-native)    skills wiring for the agentic tcg
style(tcg)                    pay the discipline: 24 findings for real
feat(mcp)                     mount the tcg family as MCP tools
feat(tcg)                     the portable tcg tool family crate (PROP-026)
fix(typescript-ai-native)     verbatim-free node paths + relay self-init
refactor(workspace)           extract declared-binary resolution (bins cell)
feat(research)                completion checks + clean control baseline
build(deps)                   re-materialise with the tcg toolchain
docs(packages)                declare the tcg binary + boot row + protocol params
feat(typescript-ai-native)    tcg-typescript - serve, one-shot ops, bench
feat(typescript-ai-native)    tcg-oracle-bridge - persistent oracle client
refactor(conform)             export the assembly + vocabulary seams
feat(typescript-ai-native)    ship the ts oracle (LanguageService NDJSON server)
build(packages)+docs(...)     0.4.0 bump, brief+mechanisms, PROP-026, M1.24
docs(plan)                    write + owner-amend the agentic-tcg campaign
```

The WAL supersedes this snapshot wherever they diverge. Session-resume
phrase: `восстанови сессию` (boots into a status report and waits — the
open items above are the owner's call, not a standing mandate).
