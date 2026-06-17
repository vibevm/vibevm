# Quickstart: opencode + vibevm hello-world

End-to-end walkthrough that takes a fresh machine with `opencode` already installed, sets up vibevm globally (one-time bootstrap), and demonstrates that opencode can **create a vibevm-managed hello-world project from scratch** by being told what to do — without the user re-explaining vibevm or running `vibe init` by hand.

This document is **dual-purpose by design**:

- **As a tutorial** — copy-paste each command, follow the prompts, you should arrive at a working setup within 5 minutes.
- **As an integration test** — the [Acceptance checklist](#acceptance-checklist) below is a machine-readable list of facts that must be true after a successful run. Run this guide before tagging a vibevm release; if any checkbox fails, slice 5 has regressed.

For per-command reference rather than walkthrough, see [`docs/commands/mcp-install.md`](../commands/mcp-install.md), [`mcp-upgrade.md`](../commands/mcp-upgrade.md), [`mcp-uninstall.md`](../commands/mcp-uninstall.md), [`mcp-status.md`](../commands/mcp-status.md), [`mcp-serve.md`](../commands/mcp-serve.md).

Tested on: Windows 11 Pro for Workstations, PowerShell 5.1, opencode 1.14.x, GLM-4.7-Flash via LM Studio (any tool-use-capable model works). Vibevm `HEAD` ≥ slice-5.

---

## 0. Prerequisites

- Working `opencode` in PATH (`where.exe opencode` returns a path).
- A model with tool-use support configured in opencode (Claude / GPT-4o / Llama-3.1 / GLM-4 / etc.).
- Internet access to `github.com` (for `vibe install flow:wal` later, when the agent does it).
- Rust toolchain installed (only if you want the persistent-PATH variant).

---

## 1. Make `vibe` discoverable to opencode

opencode launches `vibe mcp serve` as a subprocess; `vibe` must resolve in the PATH the opencode process inherits.

**Variant A — persistent (recommended).** `cargo install` puts `vibe.exe` into `~/.cargo/bin/`:

```powershell
cd C:\Users\olegc\gits\vibevm
cargo install --path crates/vibe-cli --locked
where.exe vibe        # should print C:\Users\olegc\.cargo\bin\vibe.exe
vibe --version        # vibe 0.1.0-dev
```

**Variant B — one-session.** Add the debug build dir to PATH:

```powershell
$env:PATH = "C:\Users\olegc\gits\vibevm\target\debug;$env:PATH"
where.exe vibe
vibe --version
```

With variant B, run **everything else** (including `opencode`) in the same PowerShell window.

---

## 2. Bootstrap vibevm globally (one-time, no project needed)

This is the slice-5 difference from slice-4. Slice 4 required you to first create a vibevm project, then install MCP into the project. Slice 5 lets you install MCP + skill at user-level once, and from then on the agent itself knows how to create new vibevm projects.

```powershell
# Run this from ANY directory (no vibe.toml needed).
cd C:\Users\olegc          # or wherever, doesn't matter
vibe mcp install --auto --scope user --invoked-by manual-bootstrap
```

You'll be prompted to confirm the apply. On approval, this writes:

- `~/.config/opencode/opencode.json` — MCP server entry under `mcp.vibevm` (no `--path`, server resolves CWD per call).
- `~/.config/opencode/skills/vibevm/SKILL.md` — vibevm skill loaded by opencode globally.
- Equivalent files for any other detected agent (Claude Code, Codex, etc.).

Verify what landed:

```powershell
type ~/.config/opencode/opencode.json
# expect: { "$schema": ..., "mcp": { "vibevm": { "type": "local",
#          "command": ["vibe","mcp","serve"], "enabled": true } } }
type ~/.config/opencode/skills/vibevm/SKILL.md
# expect: YAML frontmatter + "Section A — bootstrap mode" body
```

**Sanity probe** — drive the MCP server by hand to confirm vibe is reachable:

```powershell
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | vibe mcp serve
```

Expected:
```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"vibe-mcp","version":"0.1.0-dev"},"capabilities":{"tools":{"listChanged":false}}}}
```

---

## 3. Open a fresh empty directory and start opencode

```powershell
mkdir C:\Users\olegc\hello-from-agent
cd C:\Users\olegc\hello-from-agent
opencode
```

The directory is empty — **no `vibe.toml`, no spec/, nothing**. The agent loads the global opencode config (incl. `mcp.vibevm`) plus the user-level `vibevm` skill — that gives it everything it needs to know about vibevm without anything in the project.

---

## 4. Demo prompts — let the agent do the work

These prompts demonstrate the slice-5 contract: the agent creates the vibevm project itself, using the skill instructions and the MCP tools, with the user only stating intent.

### Prompt A — minimum (just probe MCP wiring)

```
Use the vibevm MCP server to call query_package("nonexistent"). I expect
an error since this directory has no vibevm project — confirm that's
what you get and tell me what the SKILL.md instructs you to do next
when you see no vibe.toml here.
```

**Pass criterion:** opencode visibly issues a `query_package` tool call (TUI shows it), gets an error/empty response, then summarises Section A of the SKILL.md (run `vibe init`, install starter packages, etc.) without you having to tell it.

### Prompt B — bootstrap a hello-world project (full demo)

```
Create a vibevm-managed Hello World project in this directory. Steps:

1. Run `vibe init` to scaffold the project.
2. Build a minimal hello-world: README.md with one line about the
   project, docs/hello.md saying "Hello, world!".
3. Inspect the resulting project — `vibe list`, `vibe show config`,
   and `vibe outdated` so I see what's tracked.
4. Pass --invoked-by opencode on every vibe CLI call you issue.
5. Before suggesting any vibe flag, run `vibe <subcmd> --help` and
   read the actual current surface — do not guess from training data.

End with a one-paragraph summary of what you did and what command I
might want to run next (e.g. install a real package).
```

**Pass criteria:**

- `vibe init` runs, creates `vibe.toml`, `vibe.lock`, and the
  starter files: `spec/boot/` (the authored `00-core.md` /
  `90-user.md` plus the generated `INDEX.md`), `spec/WAL.md`, and
  the `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` instruction files,
  each carrying a managed `<vibevm>` block.
- `README.md` and `docs/hello.md` materialise with sensible content.
- `vibe list` / `vibe show config` / `vibe outdated` actually run
  (outputs visible in TUI), with `--invoked-by opencode` on each.
- Every `vibe …` line in the transcript carries `--invoked-by opencode`.

This prompt deliberately does NOT assume any project convention
(WAL discipline, PROP/FEAT spec corpus, etc.). The vibevm skill
covers only the package manager surface; project-specific
disciplines (such as the WAL protocol shipped by `flow:wal`) are
opt-in via `vibe install <pkgref>` and additional skills.

If you want the agent to also adopt a specific convention, install
the relevant package + its skill explicitly. For example, after
Prompt B:

```
Install flow:wal via vibevm. Then read the WAL protocol via the
vibevm MCP server (read_subskill) and apply it: update spec/WAL.md
to record what we did in this session per the protocol.
```

### Prompt C — operator wants project-scope skill committed too

After Prompt B completes:

```
Now this project will be shared with my team. Commit the vibevm setup
to the repo so they don't need to run `vibe mcp install` themselves —
land a project-scope vibevm skill file in this directory.
```

**Pass criterion:** opencode runs `vibe mcp install --scope project --what skill --agent opencode --invoked-by opencode`. New `.opencode/skills/vibevm/SKILL.md` lands inside the repo.

---

## Acceptance checklist

Use this list as both an integration-test gate and a triage tool. Copy into a PR description when shipping changes that touch slice-5 surface.

- [ ] `where.exe vibe` resolves to a built-and-installed binary.
- [ ] `vibe --version` exits 0.
- [ ] `vibe mcp install --auto --scope user` succeeds without `vibe.toml` in CWD.
- [ ] `~/.config/opencode/opencode.json` contains `mcp.vibevm` with `command: ["vibe", "mcp", "serve"]` (NO `--path` argument).
- [ ] `~/.config/opencode/skills/vibevm/SKILL.md` exists, starts with `---`, contains `name: vibevm`, has `Section A` (bootstrap) and `Section B` (inside-project) headers, references `query_package` / `read_subskill` / `materialise_subskill` / `--invoked-by` / `VIBE_INVOKED_BY`.
- [ ] Hand test in step 2 returns a JSON envelope with `protocolVersion: "2024-11-05"`.
- [ ] After launching opencode in an EMPTY directory and running Prompt A, the TUI shows a tool call to `query_package` AND the agent's response references Section A without prompting.
- [ ] After Prompt B, all of: `vibe.toml`, `vibe.lock` (empty), `spec/boot/` (the authored `00-core.md` / `90-user.md` plus the generated `INDEX.md`), `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` (each with a `<vibevm>` block), `README.md`, `docs/hello.md` exist with sensible content. (No package install required by Prompt B; the lockfile is empty and no `vibedeps/` tree exists until the user opts in to a `vibe install`.)
- [ ] Every `vibe …` invocation in the opencode transcript includes `--invoked-by opencode`.
- [ ] `vibe --json mcp status` after Prompt B reports `unchanged` for the user-scope opencode entries (since the user-scope install hasn't drifted).
- [ ] After Prompt C, `<project>/.opencode/skills/vibevm/SKILL.md` exists.

If you check every box on a clean machine, slice-5 (bootstrap mode + two-state skill + scope/what axes) is healthy end-to-end.

---

## Lifecycle commands

After bootstrap, the four mcp lifecycle commands cover everything:

| Command | When to use |
| --- | --- |
| [`vibe mcp install`](../commands/mcp-install.md) | First-time wiring of an agent. Flexible scope/what/agent matrix. |
| [`vibe mcp upgrade`](../commands/mcp-upgrade.md) | After `cargo install --path crates/vibe-cli` — pulls the new SKILL.md / wire shape into agents that already had vibevm wired. **Does not create new installs.** |
| [`vibe mcp uninstall`](../commands/mcp-uninstall.md) | Wipe vibevm from one or more agents (foreign keys preserved). |
| [`vibe mcp status`](../commands/mcp-status.md) | Read-only drift report for both MCP-config and SKILL.md across all (agent × scope) pairs. |

Typical "I just upgraded vibe" sequence:

```powershell
cd C:\Users\olegc\gits\vibevm
git pull
cargo install --path crates/vibe-cli --locked
vibe mcp upgrade --invoked-by manual --yes     # refresh user-level installs
```

---

## Troubleshooting

**`opencode: command not found`** — opencode's installer didn't put it in PATH for the current shell. Use a terminal where opencode worked before.

**MCP server unreachable from opencode (`vibe not found` / spawn error).** opencode inherits PATH from its parent. With variant B (one-session PATH), launch opencode from the same PowerShell. Better: switch to variant A (`cargo install`).

**Model refuses to call tools.** Switch to a tool-use-capable model in opencode (`/models` or edit `~/.config/opencode/opencode.json`).

**Skill not auto-loading.** Verify the file path and frontmatter:
```powershell
type ~/.config/opencode/skills/vibevm/SKILL.md | findstr /i "name description Section"
# should print: name: vibevm
#                description: Use whenever ...
#                ## Section A — bootstrap mode (no `vibe.toml` here)
#                ## Section B — inside an existing vibevm project
```

Newer opencode versions activate skills via an explicit agent-side `skill` tool call, not auto by description match. In that case explicitly ask: *"Load the skill named `vibevm` and follow its instructions for the rest of this conversation."*

**`vibe install flow:wal` fails inside the agent's run.** Read-only clone needs no token. Probe with `git ls-remote https://github.com/vibespecs/org.vibevm.wal` (the canonical repo `flow:wal` resolves to under `naming = "fqdn"`). If that fails, the issue is in your network / DNS, not in vibevm.

**Reset and retry.** Bootstrap state lives at `~/.config/opencode/opencode.json` and `~/.config/opencode/skills/vibevm/`. To wipe:

```powershell
vibe mcp uninstall --auto-equivalent: --agent all --scope user --yes
```

The hello-world sandbox lives at `C:\Users\olegc\hello-from-agent\` — delete the directory to retry.

---

## Maintenance

This guide is a contract: when slice-5 surface changes, update accordingly.

- **CLI flag changes** (new `vibe mcp install` flag, renamed `--scope` knob) → update sections 2 + 4 + acceptance checklist.
- **MCP wire-shape changes** (new tool, renamed tool, new field) → update Prompt A and the hand-test.
- **New supported agent** → file a sibling `docs/guides/agent-mcp-quickstart-<agent>.md`. Keep per-agent guides focused.
- **Skill template changes** (`crates/vibe-cli/src/commands/skill_template.md`) → update the acceptance checklist's "SKILL.md must contain" line if a tested string changes; update the agent skill-section structure references if Section A/B structure shifts.
- **`flow:wal` content_hash drift** — the prompts don't pin a specific hash, only that one is present. No update needed unless `flow:wal@0.1.0` gets unpublished.

When a vibevm release ships, run this guide top-to-bottom against a clean sandbox before tagging. A failed acceptance checkbox is a release blocker.

## Related

- [`docs/commands/mcp-install.md`](../commands/mcp-install.md) — full reference for the install UX.
- [`docs/commands/mcp-upgrade.md`](../commands/mcp-upgrade.md) — refresh stale installs.
- [`docs/commands/mcp-uninstall.md`](../commands/mcp-uninstall.md) — remove vibevm from agents.
- [`docs/commands/mcp-status.md`](../commands/mcp-status.md) — read-only drift report.
- [`docs/commands/mcp-serve.md`](../commands/mcp-serve.md) — server wire format.
- [`crates/vibe-cli/src/commands/skill_template.md`](../../crates/vibe-cli/src/commands/skill_template.md) — the SKILL.md body the binary ships (two-state).
- [`spec/research/PROP-004-tessl-comparative-research.md`](../../spec/research/PROP-004-tessl-comparative-research.md) §5.1 — design rationale for MCP integration.
- [`spec/WAL.md`](../../spec/WAL.md) — slice-4 + slice-5 checkpoints.
