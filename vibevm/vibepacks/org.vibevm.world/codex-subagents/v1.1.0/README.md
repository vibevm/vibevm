# Codex subagents

This tool package provides POSIX, PowerShell and CMD launchers deployable with receipt ownership
for bounded Codex CLI workers, plus a complete parent/worker contract. Its
default model request is `gpt-6.1-sol` with reasoning effort `medium`.

The core boundary is instruction loading: unattended execution suppresses
automatic project documents and ignores user configuration. The parent reads
the governing rules and names the exact relevant subset in a worker packet.
This is not a filesystem read sandbox. A worker follows named scope
cooperatively; the CLI sandbox limits permitted operations separately.

## Installation

Declare `tool:org.vibevm.world/codex-subagents@1.1.0` in the consumer, then run
its ordinary `vibe install`. From the consumer root, deploy the resolved slot
with an explicit host profile:

```text
vibe deploy --profile posix --path vibevm/vibedeps/org.vibevm.world.codex-subagents/1.1.0
vibe deploy --profile windows --path vibevm/vibedeps/org.vibevm.world.codex-subagents/1.1.0
```

Use the actual resolved slot if different. There is no default profile.
Installation materialises the package; deployment publishes `codex-worker`
for POSIX, or `codex-worker.ps1` and `codex-worker.cmd` for Windows, under
`<settings-root>/opt/bin` (default `~/.vibe/opt/bin`). The deploy engine owns
receipts, collision checks and reversal. The package does not modify PATH.
Codex CLI must already be installed; `CODEX_WORKER_BIN` may select its executable.

## Isolated account and state

Set `CODEX_WORKER_HOME` to an explicit absolute isolated state directory before
every command. No default falls back to the ordinary `~/.codex` home. Login
uses ChatGPT authentication stored as a file in this isolated home. The
launcher clears inherited `CODEX_` and `OPENAI_` authentication/session routing
variables before constructing its child environment; TLS CA settings remain
available. Login does not create a daemon or share the parent's conversation.

```text
codex-worker login [login options]
codex-worker status
codex-worker interactive [CLI options]
codex-worker exec [exec options and prompt]
codex-worker exec -
```

The final form reads a prompt from stdin. Authenticate through `login`; do not
put account files, account-specific paths or tokens in the package, packets or
reports. `status` reports CLI login state, not model entitlement or remaining
quota. `CODEX_WORKER_MODEL` and `CODEX_WORKER_EFFORT` override the default
requests. Requests are not verified execution metadata.

Use stdin for packet prompts on Windows: Windows PowerShell 5.1 and the CMD
shim can reparse empty or unusually quoted native arguments. The PowerShell
launcher refuses linked/reparse ancestry for its account-state directory.

## Bounded execution

Read `vibevm/vibespecs/flows/codex-subagents/CODEX-SUBAGENTS-PROTOCOL.xml` and
its sibling `packet-template.xml` before commissioning a worker. Copy the
template to an ordinary packet file, replace all placeholders and put
`##subagent-quiet-clause` on its first line. Put the same marker on the first
line of the short launch pointer. Name exact standing rules, source inputs,
write paths, meaningful narrow checks, dependencies, budget, report path and
accepting parent. A missing required rule is a packet defect: the parent must
amend it before the worker takes dependent action.

`exec` requests `--strict-config --ignore-user-config`,
`project_doc_max_bytes=0`, sandbox `workspace-write` and approval `never`.
For a review, request `-s read-only`. Use normal task-capable tools within this
scope. Boot suppression does not restrict what files a shell can read.
Native collaboration workers require the same explicit packet and host support
for boot suppression; CLI flags do not establish a native harness guarantee.

A bounded task has one observable outcome and a closed input/write contract.
Width is semantic scope, not a line or token count. Split independent outcomes
when useful; do not reduce requirements to make a task look small.

Record the exact cwd, isolated home and process/thread handles. Wait for real
process exit before corrections. Resume only the explicitly identified thread
in the same cwd/home; an observation timeout is not permission to restart a
still-running writer. Preserve visible unstaged artifacts and an early report
on crash or quota exhaustion. The parent reads the actual result, reproduces
decisive checks and alone accepts or rejects it.

## Evidence limits

Local Codex CLI `0.159.3` accepted the launcher options. POSIX fake-child
contract tests passed 10/10, including explicit model/sandbox overrides without
repeated flags. On Linux, the isolated account completed a live read-only
probe: quiet exec returned `QUIET_PROBE_OK`, while the same conditional prompt
with automatic project loading enabled returned the token from a sentinel
`AGENTS.md`. Both exited 0 and emitted no tool calls. The requested model was
`gpt-6.1-sol` at medium; the JSONL stream did not independently expose resolved
model/effort metadata. Those runtime identities remain unverified.

Linux offline installation and the POSIX receipt deployment passed with VibeVM
1.1.0; the deployment receipt is verified. A real bounded CLI worker audited
the installed package and completed all five checks without following full boot.
Native Windows and macOS remain UNRUN. The package is not
activated by creating its source. Historical GLM worker practice is retained
only in `provenance.xml`; this package has no GLM subscription, model routing
or campaign dependency. A sanitized record of the executed checks is in
`tests/evidence/2026-10-01-posix-quiet.json`.

Installation/use evidence: `tests/evidence/2026-10-01-linux-install.json`.

A Linux CLI workspace-write run exposed an extra ancestor Git metadata marker
to a no-Git fixture. That family's worker gate failed before changed code while
the parent's identical-tree run passed. Bind such environment-sensitive oracles
explicitly to the parent; retain the worker sandbox and report missing evidence.
See `tests/evidence/2026-10-01-worker-git-fixture.json`.
