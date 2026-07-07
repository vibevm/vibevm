# TCG-STAGE-B-DELIVERY-PLAN v0.1 — delivery experiments for the agentic oracle

_Status: **BACKLOGGED (owner, 2026-07-07, same day as authoring)** —
the owner deferred commissioning; the §14 review points stay OPEN for
whenever this campaign is picked up, and the §1 facts must be
re-verified then (they age). Successor focus the same day: the Rust
agentic twin ([AGENTIC-TCG-RUST-PLAN-v0.1](AGENTIC-TCG-RUST-PLAN-v0.1.md)).
Originally DRAFT — awaiting owner review (2026-07-07); written against
tree `40eaca6` (the agentic-tcg session-end checkpoint; floor green,
local == origin). Commissioned by the owner as the realisation plan for
owner-court item 2 of that checkpoint: the Stage-B delivery backlog
recorded in
[`research/tcg-bench/reports/REPORT-2026-07-07-with-tools.md`](../../research/tcg-bench/reports/REPORT-2026-07-07-with-tools.md)
— (1) forced-loop write-path delivery, (2) an MCP-mounted battery arm,
(3) an uptake metric. Cold-executable after the §14 review points
resolve: any phase is a safe stop; the floor is untouched by
construction (this campaign lives entirely in `research/tcg-bench/` +
this file — zero product or package code, D5). Prior art:
[AGENTIC-TCG-TS-PLAN-v0.1](AGENTIC-TCG-TS-PLAN-v0.1.md) (EXECUTED) built
the oracle and the battery; its §4.3 prediction was falsified in the
opt-in delivery form, which is exactly the question this plan
operationalises._

Mandate (owner, 2026-07-07): plan the realisation of the Stage-B
delivery experiments — the null's follow-up. Stage A proved the
oracle's mechanics (differential corpus 7/7 @ p50 19.3 ms; the live MCP
chain answers enriched) and proved that the INFORMATION is right (both
battery FAILs are verbatim what `tcg_validate` reports, with advice).
What it falsified is the weakest delivery form: an opt-in CLI named in
the prompt is a tool a weak model never spontaneously calls. Stage B
varies the DELIVERY MECHANISM while holding everything else fixed, and
instruments uptake so future arms can separate "consulted and ignored"
from "never consulted".

## 0. Why this exists (one screen)

The Stage-A result was a clean null: control 10/2 vs with-tools 10/2,
the same two `ts-unsafe-in-domain` regressions (tasks 04/07), zero
observable tool consultation. The honest reading in the with-tools
REPORT: the gap is delivery, not information — the answer exists, the
agent never asks. That splits the remaining question into two
falsifiable halves:

- **Does a stronger delivery form produce consultation at all?** The
  MCP-mounted arm (D2) makes the oracle a first-class tool with the
  skill's teaching in context — the affinity question: tool-call
  affinity vs shell-command affinity.
- **Does forced consultation change outcomes?** The write-path hook arm
  (D3) runs `tcg_validate` on the hypothetical content automatically
  and refuses violating writes with the findings as the tool result —
  consultation as a gate, not an offer. This is the cheap agentic
  approximation of the token-level sibling (the mask is the extreme of
  the same idea: generation physically cannot proceed through an
  invalid state).

Both halves need the third deliverable first: the **uptake metric**
(D1), because Stage A could not even measure consultation — the raw
event streams died with the throwaway work copies, and the surviving
`tool_calls` counter does not distinguish an oracle call from an `ls`.
The metric is the instrument; the arms are the experiments.

Outcome-neutrality is part of the mandate: another null is a valid,
publishable result (it would re-disposition the agentic line's battery
claims and strengthen the case that only gate-grade delivery — the
floor, or the far-future mask — moves weak models). The campaign is an
experiment, not a promotion.

## 1. Current-state facts, verified at authoring (do not re-discover)

- **The harness**: `research/tcg-bench/run-battery.sh` (238 lines) —
  arms `control`/`with-tools`; per task: tar-copy of `research/ts-demo`
  excluding `node_modules`/`vibedeps`/`.vibe`/`target`/
  `discipline/health`; `node_modules` junctioned via PowerShell
  `New-Item -ItemType Junction`; **`remove_work` unlinks ONLY the
  node_modules junction** before `rm -rf` — a second junction (D2 adds
  `vibedeps`) would today be `rm -rf`'d THROUGH into the demo's real
  tree. Mechanical verdicts: agent exit ∧ task_done (`.check` grep
  lines) ∧ tsc ∧ node --test ∧ conform-no-new. The conform binary is
  toolcached (`.toolcache/`) after a mid-run slot refresh yanked it
  (three conform=127 rows — the standing lesson).
- **The row schema** (JSONL per task): task/arm/model/verdict/
  task_done/agent_exit/wall_s/steps/tool_calls/tsc_*/tests_*/
  conform_*. `tool_calls` counts `"type":"tool"` events
  undifferentiated. **Raw `agent.jsonl` streams are NOT archived** —
  Stage-A uptake is unrecoverable (recorded honestly in the REPORT).
- **Baselines that stand**: control 10/2 and with-tools 10/2, both
  2026-07-07, both `openrouter/z-ai/glm-5-turbo`, rows in
  `reports/{control,with-tools}-2026-07-07-*.jsonl`. Arms compared
  against each other must share one model (RUNBOOK rule; the
  gpt-oss:free degradation is why).
- **opencode**: PATH resolves to 1.17.14
  (`/c/nvm4w/nodejs/opencode`, npm-global package
  `C:\nvm4w\nodejs\node_modules\opencode-ai`); the RUNBOOK's fallback
  copy at `C:\opt\nvm\v24.18.0\...` also exists. The shipped exe is a
  bun bundle (no readable dist JS), but the literal string
  `tool.execute.before` IS present in the 1.17.14 binary — the plugin
  hook exists in this version; its load path and block semantics are
  Phase-0 spike matter, not assumption. MCP servers are configured via
  a project-local `opencode.json` (`mcp.<name>` with a local
  command) — exact key shape is spike matter for the same reason.
- **The one-shot surface the hook needs exists**: `typescript-ai-native-tcg
  validate <file> [--content-from -|<path>] [--json]`
  (tcg-cli-typescript/src/main.rs:39 `content_from`; also on
  `complete`), exits 1 on an error diagnostic OR a non-baselined
  finding. Artifact: slot-resident
  `vibedeps/stack-typescript-ai-native-lang/0.6.0/target/release/
  typescript-ai-native-tcg.exe` (repo slot, built).
- **The MCP surface the mounted arm needs exists**: `vibe mcp serve
  --path <dir>` (commands/mcp/mod.rs:98) lists the four `tcg_*` tools;
  the OracleRegistry resolves lockfile → slot → artifact from the
  `--path` root, builds org.vibevm binaries silently, refuses
  third-party with a recipe (PROP-026). A work copy carries `vibe.toml`
  + `vibe.lock` (the tar includes them) but NO `vibedeps/` — without
  D2's junction the registry would answer not-installed.
- **ts-demo has its own materialised `vibedeps/`**
  (`research/ts-demo/vibedeps`, stack 0.4.0 by content identical to
  the repo slot) — the junction source for D2. Whether ITS
  `target/release/typescript-ai-native-tcg.exe` is pre-built is a Phase-0 check
  (one `vibe bin build` from the demo root warms it; org.vibevm is
  consent-allow-listed).
- **MCP-held `vibe.exe` blocks workspace rebuilds** (standing finding)
  — and, inverted, this box runs the OWNER's live `vibe mcp serve`
  sessions for the vibevm root. Any battery process sweep must match
  on the battery's work-root path in the command line, never on the
  image name alone.
- **The teaching text exists**: both package skills
  (`packages/org.vibevm/typescript-ai-native/v0.4.0/spec/skills/*/
  SKILL.md`) carry the Stage-A "generation-time assistant" section
  (consult `tcg_validate` before writing; the floor stays the truth) —
  the with-mcp arm quotes that posture instead of inventing new words.
- **The demo's `AGENTS.md` rides into every work copy** — all arms
  keep the same standing discipline instructions; arms differ only in
  the appended block + mounted surface.
- Node v24.18.0; junctions need verbatim-free absolute Windows paths
  (cygpath -w); real exit codes captured per step — the standing
  machine quirks all hold unchanged.

## 2. Target end-state (what done looks like)

```
research/tcg-bench/
├─ run-battery.sh              extended: arms with-mcp / with-hook;
│                               uptake fields in every new row;
│                               raw-stream archival; vibedeps junction
│                               + junction-SET-safe remove_work;
│                               per-copy opencode.json writer;
│                               plugin copy-in; scoped process sweep
├─ plugins/
│   └─ tcg-guard.js            NEW: the write-path hook (forced loop)
├─ configs/
│   └─ opencode.mcp.json       NEW: the MCP-arm config template
├─ RUNBOOK.md                  arms + uptake metrics documented
└─ reports/
    ├─ raw/<arm>-<stamp>/<task>/   NEW: archived agent.jsonl + step outs
    ├─ REPORT-<date>-with-mcp.md   NEW
    ├─ REPORT-<date>-with-hook.md  NEW
    └─ REPORT-<date>-stage-b-synthesis.md  NEW: the four-arm table
spec/terraforms/TCG-STAGE-B-DELIVERY-PLAN-v0.1.md   this file
```

Nothing else moves. No package bump, no vendor sync, no product crate
edits, no ts-demo edits; `git status` outside the two trees above stays
clean for the whole campaign (D5, posted as prediction P4).

The four-arm comparison at close:

| arm | delivery form | teaching | binding |
|---|---|---|---|
| control (Stage A) | none | AGENTS.md only | none |
| with-tools (Stage A) | one-shot CLI named in prompt | prompt block | opt-in |
| with-mcp (Stage B) | first-class MCP tools | prompt block + skill posture | opt-in, low-friction |
| with-hook (Stage B) | write-path gate | the refusal text itself | forced |

## 3. Decisions (D1–D6)

### D1 — the uptake metric is the instrument, and it lands first

Every future run archives the raw event stream BEFORE the work copy
dies: `agent.jsonl` + `tsc.out`/`tests.out`/`conform.out` copied to
`reports/raw/<arm>-<stamp>/<task>/` (small text; the Stage-A streams
were lost with the copies and the REPORT says so). The runner then
parses the stream into new row fields, all zero-defaulted so old rows
stay comparable:

- `oracle_mcp_calls` — MCP tool invocations matching `tcg_` (the
  server-prefixed name form is spike matter; the grep matches the
  suffix either way);
- `oracle_bash_calls` — bash/tool invocations whose text contains
  `typescript-ai-native-tcg`;
- `oracle_ops` — per-op breakdown string (`validate:3,scope:1`);
- `first_oracle_step` — when in the run the first consultation
  happened (0 = never);
- `bash_writes` — shell-redirect file writes (the hook arm's escape
  hatch, counted honestly);
- `hook_blocks` / `hook_overrides` / `hook_ms_total` — reserved, zero
  outside the hook arm;
- `opencode_version` — per-row, because R8 is real.

"Consulted and ignored" vs "never consulted" then falls out of
`oracle_*_calls` × verdict, per task, mechanically.
*Rejected:* re-running the Stage-A with-tools arm solely to backfill
uptake — spends quota to instrument a null we already trust; the
with-mcp arm subsumes the question with a stronger delivery form.

### D2 — the with-mcp arm: first-class mounting, hermetic per copy

The runner writes a per-copy `opencode.json` into the work root (from
`configs/opencode.mcp.json`, paths substituted absolute + verbatim-free)
registering ONE local MCP server: the battery-toolcached `vibe.exe`
running `mcp serve --path <work>`. Consequences, each deliberate:

- **Toolcache `vibe.exe`, never `target/debug/vibe.exe`** — a battery
  must not hold the workspace artifact for 25 minutes (the MCP-held-exe
  rebuild-block finding). Copied once in the probe section; hard-fail
  with a build recipe if absent. Stage B needs no new product code
  (D5), so any exe carrying PROP-026 suffices; the probe logs its
  mtime.
- **`--path <work>`** — the project root must be the work copy, so
  `tcg_validate` answers against the copy's files, `conform.toml`, and
  frozen baseline. One shared server would cross-contaminate roots.
  *Rejected* for the same reason: a global/user-level opencode config
  (leaks the arm into unrelated runs on this box; per-copy config keeps
  the arm hermetic and vanishes with the copy).
- **`vibedeps` junctioned** from `research/ts-demo/vibedeps` (read-
  mostly, artifact pre-warmed once in the probe section) so the
  OracleRegistry's lockfile → slot → artifact dispatch works inside the
  copy. `remove_work` is extended to unlink the junction SET
  (`node_modules`, `vibedeps`) before `rm -rf` — proven on a scratch
  copy in Phase 0 before the real loop ever runs (the demo's real trees
  are one missed unlink from deletion).
- **Prompt**: the task text verbatim + an `MCP_BLOCK` naming the four
  `tcg_*` tools and quoting the skills' consult-before-write posture —
  the same teaching a real consumer session gets, in parallel shape to
  Stage A's `TOOLS_BLOCK` so the measured delta is the delivery form,
  not the wording.
- **Lifecycle**: opencode owns the MCP child; the Phase-0 spike
  verifies no orphan `vibe.exe`/`node` survives `opencode run` exit.
  If leaks show, the runner sweeps between tasks — matching processes
  by the battery work-root substring in the command line, NEVER by
  image name (the owner's live vibevm MCP sessions run on this box).

### D3 — the with-hook arm: consultation as a gate (the forced loop)

An opencode plugin (`plugins/tcg-guard.js`, copied into
`<work>/.opencode/plugin/`) intercepts the file-writing tools (`write`,
`edit`) in `tool.execute.before`: it runs the toolcached one-shot
`typescript-ai-native-tcg validate <file> --content-from - --json` on the
HYPOTHETICAL content (the overlay path — the file need not be written),
and when the result carries an error-grade diagnostic or a
non-baselined finding it THROWS — the write is refused and the findings
text (verbatim, advice lines included) returns to the model as the tool
result. The model cannot land a violating write without reading the
findings: consultation as a gate, not an offer — the report's words,
and the cheap agentic approximation of the token-level mask.

Bounded honestly:

- **Livelock cap**: at most 2 blocks per file per run; the third
  attempt passes with a warning appended, `hook_overrides`
  incremented. A weak model that cannot act on the finding must not
  burn the 300 s timeout ping-ponging — and an override that then
  FAILs conform is itself a Stage-B finding (information insufficiency
  at fixed delivery, the exact complement of Stage A).
- **Scope**: only `.ts` writes under `src/` are validated (10 s
  per-call timeout; one-shot spawn per call — cold ~600 ms is
  acceptable at write grain; a persistent serve relay behind the hook
  is named Stage-C, not built here).
- **`bash` escapes are not gated** — redirect writes bypass the hook
  by construction; `bash_writes` counts them and the REPORT reads the
  count. A model that routes around the gate is a result, not a bug.
- **Prompt stays bare** (task verbatim, no tools block): the gate is
  invisible until it fires, and the refusal text itself teaches. This
  isolates one variable — with-hook compares against CONTROL; naming
  the tools too would blend D2's variable in (§14.4 offers the owner
  the alternative).
- **Fallback** (if the spike shows 1.17.14 plugins cannot block or do
  not load headless from the project dir): the harness-loop form —
  after the agent run the RUNNER validates changed files; on findings
  it re-invokes the agent once with the findings + diff in the prompt
  (headless continuation if `opencode run` supports it, else a fresh
  run). Run-grain instead of write-grain; arm renamed `with-loop`; the
  §4.2 claim weakens accordingly and the REPORT says so.

*Rejected:* gating via an AGENTS.md instruction ("always validate
before writing") — still opt-in; Stage A falsified that whole class.
Wrapper-scripting opencode's write path from outside — no such seam
exists headlessly.

### D4 — comparability: reuse the Stage-A baselines, change one thing per arm

The Stage-A control (10/2) and with-tools (10/2) rows ARE the
baselines; Stage B re-runs neither (default; §14.2 offers the
same-window quadruple at ~2× quota). Same 12 tasks verbatim, same
`.check` files, same verdict logic, same model id
`openrouter/z-ai/glm-5-turbo`, same timeout/sleep. New arms differ
ONLY in delivery mechanics. Cross-day alias drift at the provider is a
named validity limit — the synthesis table carries run dates and
`opencode_version` per row; if GLM-5-Turbo answers degenerate into the
gpt-oss:free failure shape (do-nothing runs, truncated streams), the
campaign STOPS and the owner picks the model — no silent swap (the
Stage-A pinned-fallback rule, carried over).

### D5 — zero product surface

The campaign touches `research/tcg-bench/` and this file. No package
edits, no version bumps, no vendor sync, no vibe-tcg/vibe-mcp/CLI
changes, no ts-demo edits — the floor is untouched by construction and
`self-check.sh` stays green trivially. Any spike finding that appears
to need product code (a missing flag, an MCP schema gap) STOPS the
phase and goes to the owner as a named decision — it does not get
"just slipped in".

### D6 — reporting form

One REPORT per executed arm (verdict table against both baselines +
the uptake table + wall-time deltas + honest notes), then one synthesis
REPORT: the four-arm table, predictions-vs-facts (§4), the delivery-
mechanism reading, and Stage-C candidates NAMED, not commissioned
(persistent relay behind the hook; k-repeat variance; a stronger-model
replication; write-grain vs run-grain if the fallback ran). The
direction-grade honesty rule carries over verbatim: n=12, one model —
direction, not magnitude.

## 4. Predictions (falsifiable, checked by the synthesis REPORT)

1. **with-mcp uptake > 0**: mounted first-class with the teaching in
   context, the model calls at least one `tcg_*` tool in ≥ half the
   tasks. Falsified → even first-class mounting does not move a weak
   model to voluntary verification; the agentic line's battery claim
   narrows to gate-grade delivery only.
2. **with-hook kills both regressions**: tasks 04 and 07 go PASS
   (conform regressions 2 → 0) within the standing 300 s walls. The
   informative partial failure: `hook_overrides > 0` on those tasks —
   delivery forced, model still can't act on the finding → the
   information-sufficiency claim itself is what breaks, and the REPORT
   must say exactly that.
3. **uptake ≠ outcome**: with-mcp may show uptake > 0 with verdicts
   unchanged — "consulted and ignored" becomes measurable for the
   first time. (This is the metric's reason to exist, not a failure of
   the arm.)
4. **Zero product-code changes** end to end (D5 holds; falsified →
   stop + owner, recorded here).
5. **Budget holds**: each arm ≤ ~25 min wall (12 × (~70 s + overhead +
   3 s)); the hook's validate calls add ≤ ~15 s per task worst-case;
   the Z.AI quota survives two arms.

## 5. Phase 0 — spikes (no commits; a red spike rewrites the decision here first)

1. **Plugin semantics on the installed 1.17.14** (presence is proven —
   the hook name is in the binary; semantics are not): scratch dir,
   `.opencode/plugin/probe.js` with log-only `tool.execute.before`/
   `.after`, one trivial `opencode run` — verify headless load from
   the project dir, that the hook fires for `write`/`edit`, that a
   THROW refuses the call with the message surfaced to the model, and
   the exact input shape (tool name + args carrying path/content).
   Red → D3 falls back to `with-loop` (harness grain).
2. **MCP config shape + lifecycle**: scratch `opencode.json` mounting
   `vibe.exe mcp serve --path <scratch ts-demo copy>` — verify the
   config key path, tool visibility headless (exact prefixed names of
   `tcg_*`), one `tcg_validate` answering enriched through opencode,
   and child lifecycle at exit (`tasklist` clean of the scratch-rooted
   `vibe.exe`/`node`). Leaks → the D2 scoped sweep goes in.
3. **Junction-set drill**: scratch copy + BOTH junctions in, extended
   `remove_work` out — prove the demo's real `node_modules` and
   `vibedeps` survive (`git -C research/ts-demo status` clean, dirs
   intact) BEFORE the real loop ever runs.
4. **Event-stream anatomy for D1**: from spikes 1–2's `agent.jsonl`,
   pin the JSON shapes the uptake parser greps (tool events with
   names/args; bash text) and confirm `tcg_`/`typescript-ai-native-tcg` are
   recoverable.
5. **Demo-slot artifact check**: does `research/ts-demo/vibedeps/
   stack-typescript-ai-native-lang/0.6.0/target/release/typescript-ai-native-tcg.exe`
   exist? If not: one `vibe bin build typescript-ai-native-tcg` from the demo
   root (org.vibevm, consented), record the build time — that is the
   probe-section budget. Also: toolcache `typescript-ai-native-tcg.exe` next to
   the conform copy (the slot-refresh 127 lesson covers the hook arm's
   one-shot too).
6. Findings land in the WAL session section.

## 6. Phase 1 — the uptake metric + archival (D1)

1. `run-battery.sh`: archive step before `remove_work` (raw dir per
   task); the uptake parser (grep/sed over the archived stream — the
   spike-4 shapes); the new row fields, zero-defaulted;
   `opencode_version` captured once per run into every row.
2. `RUNBOOK.md`: the metric table gains the new fields + the
   raw-archive note.
3. Acceptance: one-task smoke (`--tasks "01-*.md"` control arm) — the
   row carries the new fields (zeros for control), `reports/raw/…`
   populated, old-row comparability intact (fields absent there,
   documented).
4. Commit: `feat(research): tcg-bench uptake metric + raw-stream
   archival`.

## 7. Phase 2 — the with-mcp arm (D2)

1. Runner: the `with-mcp` branch — toolcache `vibe.exe`; per-copy
   `opencode.json` from the template (cygpath -w substitution);
   `vibedeps` junction; junction-set-safe `remove_work`; `MCP_BLOCK`
   prompt; the scoped sweep if spike 2 demanded it.
2. Run all 12 tasks; write `REPORT-<date>-with-mcp.md`: verdicts vs
   both baselines, the uptake table (calls/ops/first-step per task),
   wall deltas, honest notes (esp. tasks 04/07: consulted? ignored?
   never asked?).
3. Acceptance: 12 rows with real uptake fields; `tasklist` clean of
   battery-rooted processes after the run; `git -C research/ts-demo
   status` clean; the demo's vibedeps intact.
4. Commit: `feat(research): the with-mcp battery arm + report`.

## 8. Phase 3 — the with-hook arm (D3)

1. `plugins/tcg-guard.js`: self-contained (node child_process spawn of
   the toolcached one-shot; `--content-from -` stdin feed — a temp
   file if the spike showed stdin friction; JSON parse; block/cap/
   override per D3; stderr log lines so the archive shows every gate
   event).
2. Runner: the `with-hook` branch — plugin copy-in to
   `<work>/.opencode/plugin/`; bare prompt; hook fields
   (`hook_blocks`/`hook_overrides`/`hook_ms_total`) parsed from the
   guard's log lines in the archive.
3. Run all 12; write `REPORT-<date>-with-hook.md` — with a per-event
   autopsy of tasks 04/07: block → the model's next move → final
   state (the entire point of the arm).
4. Acceptance: 12 rows; the 04/07 question answered one way or the
   other; no product code touched; demo intact.
5. Commit: `feat(research): the with-hook forced-loop arm + report`
   (or `with-loop`, if the D3 fallback engaged — the REPORT and the
   commit body name the downgrade explicitly).

## 9. Phase 4 — synthesis + close

1. `REPORT-<date>-stage-b-synthesis.md`: the four-arm table (control /
   with-tools / with-mcp / with-hook), §4 predictions vs facts
   (falsified ones in the Stage-A honesty voice), the delivery-
   mechanism reading, Stage-C candidates named-not-commissioned.
2. `RUNBOOK.md` final pass (four arms documented); this plan's status
   line flips to EXECUTED with the outcome one-liner; WAL session
   note.
3. Commits: `docs(research): the stage-b synthesis - <outcome>`,
   `docs(plan): flip stage-b delivery to executed`.
4. Mirror and registry publish stay owner-held (standing policy);
   session-end checkpointing follows the standing wind-down contract,
   not this plan.

## 10. Risks & fallbacks

- **R1 — plugin can't block / doesn't load headless.** Detection:
  spike 1. Fallback: the D3 harness-loop (`with-loop`), claim
  reworded; recorded in plan + REPORT.
- **R2 — process leaks** (MCP `vibe.exe`, oracle `node`, opencode
  children). Detection: spike 2 + per-run tasklist acceptance.
  Mitigation: the scoped sweep — command-line work-root match ONLY
  (the owner's live vibevm MCP sessions must never match).
- **R3 — the junction hazard**: a missed unlink deletes the demo's
  real trees through the link. Mitigation: junction-SET `remove_work`
  proven on scratch (spike 3) before any real loop; the drill is a
  hard gate for Phases 2–3.
- **R4 — quota / 429 mid-arm.** Per-task resumability is already in
  the harness (`--tasks` glob re-runs the remainder into a new stamp);
  a two-stamp arm is recorded in its REPORT, not hidden.
- **R5 — model alias drift** vs the Stage-A window. Dates +
  `opencode_version` in every row; degeneration to the gpt-oss shape →
  STOP + owner (D4).
- **R6 — stale toolcache `vibe.exe`.** Stage B needs only PROP-026-
  era behaviour (D5); the probe logs the exe mtime; if the exe
  predates the Stage-A close commit, the probe hard-fails with the
  rebuild recipe rather than running a wrong-era server.
- **R7 — hook latency blows task walls.** 10 s per-validate timeout;
  `src/`-scoped `.ts`-only gating; `hook_ms_total` measured; if
  cold-spawn dominates, the persistent-relay refinement is Stage-C —
  the wall target moves only WITH a recorded reason (the Stage-A R1
  discipline).
- **R8 — opencode auto-update mid-campaign.** `opencode_version` per
  row; arms compared within one version or the synthesis says
  otherwise.

## 11. Non-goals (named, so they stay visible)

- Re-running the Stage-A arms (default; §14.2 is the owner's widening
  lever).
- New tasks or task edits — comparability outranks coverage in v0.1;
  a task-set revision is Stage-C matter.
- Any product or package code, any version bump (D5).
- A persistent oracle behind the hook (Stage-C candidate, named).
- Strong-model studies, k-repeat variance runs (Stage-C candidates).
- The token-level line (VERY-FAR-FUTURE, owner disposition stands).
- Registry publish, mirror — standing separate items.

## 12. Quick-start for the executing session

```sh
bash tools/self-check.sh; echo "EXIT=$?"     # 13 steps, 0 — before anything
opencode --version                            # 1.17.14 at authoring; note drift
ls vibedeps/stack-typescript-ai-native-lang/0.6.0/target/release/typescript-ai-native-tcg.exe
ls research/ts-demo/vibedeps                  # the D2 junction source
ls target/debug/vibe.exe                      # the D2 toolcache source
# then Phase 0 in order; findings → WAL; a red spike rewrites the
# affected decision HERE before Phase 1 commits anything
```

## 13. Whole-campaign acceptance (what "done" looks like)

- `reports/`: REPORT-with-mcp + REPORT-with-hook (or -with-loop, named
  as the fallback) + the synthesis, all with real numbers;
  `reports/raw/` archives for every new run.
- `run-battery.sh`: four arms selectable; every new row carries the
  D1 fields; `remove_work` junction-set-safe.
- `git -C research/ts-demo status` clean; `git status` clean outside
  `research/tcg-bench/` + `spec/terraforms/`; `self-check.sh` exit 0
  unchanged (by construction).
- §4 predictions checked in the synthesis; falsified ones carry the
  honest note in the Stage-A voice.
- All commits local; mirror + publish stay held for the owner's word.

## 14. Review points — OPEN for the owner

1. **Arm scope.** Run BOTH new arms (default, ~25 min + quota each),
   or with-mcp first and decide on the hook after its numbers land?
2. **Baseline freshness.** Reuse the 2026-07-07 control/with-tools
   rows (default), or re-run all four arms in one window for a clean
   same-window table at ~2× quota?
3. **The hook's posture.** Block-with-findings (gate; default — the
   report's "consultation as a gate" reading) vs feedback-after-write
   (advisory: the write lands, findings arrive as a follow-up tool
   result; zero livelock risk, weaker claim)?
4. **The hook arm's prompt.** Bare (default — isolates the gate as
   the only delta vs control) or also naming the tools (making it
   with-tools + gate, a different comparison)?
5. **Naming.** Plan file `TCG-STAGE-B-DELIVERY-PLAN-v0.1.md`, arms
   `with-mcp` / `with-hook` (/ `with-loop` fallback) — fine as
   proposed?
