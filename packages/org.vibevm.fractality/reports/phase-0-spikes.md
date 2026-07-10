# Phase 0 report — spikes and probes (retrospective)

_Campaign: FRACTALITY-IGNITION v0.1 · Phase 0 · executed 2026-07-09 ·
this report is written retroactively at campaign close from the §14
ledger and commit `6317cff`; the ledger stays canonical._

## What the phase proved (no commits, findings only)

Nine spikes (s1–s9), all green, every §5/§6 VERIFY resolved:

- **Nested headless spawn works** (P1 confirmed): `claude -p` under a
  parent CC session, clean-slate env included — with the Windows gap
  that `APPDATA`/`LOCALAPPDATA` must join the D5 whitelist (F2).
- **Provider facts pinned live** (F3): z.ai base URL, the
  `ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU}_MODEL` mapping (NOT the
  legacy pair), `glm-5.2[1m]` / `glm-5-turbo` ids, quota tiers (the
  owner's "4000 MCP" = the Max tier), recommended env knobs.
- **GLM smoke + the first golden transcript** (F4, P2 early check):
  a fresh `CLAUDE_CONFIG_DIR` onboards headless with NO interactive
  step (R5 resolved green); stream-json carries usage fields — the
  metering premise holds.
- **The kill-tree mechanism chosen by proof** (F5): win32job +
  `KILL_ON_JOB_CLOSE` reaps the whole tree even when the parent exits
  with no cleanup — the pod's core safety property is an OS guarantee.
  `taskkill /T /F` demoted to fallback.
- **The D18 permission surface confirmed on CC 2.1.202** (F6):
  `--permission-prompt-tool` and PreToolUse `permissionDecision`
  (incl. the native park-and-resume `defer`).
- **Clean-room intake** (F7): three reference repos pinned, all MIT,
  codex-first fully studied into a decisions-only note — the Phase 5
  source.
- **Landscape** (F8): fractality is the scheduler layer the
  router/orchestrator neighbors lack.
- **MSRV reality check** (F9): this box runs rustc 1.93.1 → `sysinfo`
  pinned `=0.37.2`, `rust-version = "1.93"` floor — caught exactly
  where Phase 0 is meant to catch it.

## Strange things worth remembering

- The provider's model mapping surface differed from the
  authoring-model's knowledge (legacy env names would have silently
  misrouted) — the "download the docs, never trust the cutoff" rule
  earned its keep on day one.
- First delegation field data arrived before any fractality code
  existed: two GLM-5.2 one-shots (fact extraction; the kill-tree spike
  draft) — the spike draft needed one boss fix (MSRV blind spot), then
  passed on the real machine.

## Decisions rewritten in place

D3 (pod topology validated), D5 (Windows whitelist), D6/D12/D18
(provider facts), D11 (crate pins + MSRV floor). No prediction
falsified.
