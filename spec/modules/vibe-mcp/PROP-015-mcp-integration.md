# PROP-015 — MCP server and agent integration {#root}

**Milestone:** M1.7 ([`ROADMAP.md`](../../../ROADMAP.md)). The server slice
shipped first; the agent-integration surface (`vibe mcp install` /
`status` / `upgrade` / `uninstall`) followed.
**Status:** IMPLEMENTED — this PROP is the retroactive spec home the
CONVERT-PLAN v0.1 §7 endgame opened for the `vibe-mcp` crate and the
`vibe mcp` command family. Units typed at REQ grain; the code carries the
matching `scope!` / `#[spec(implements)]` edges.
**Related:** [PROP-004 §5 / §6](../../common/PROP-004-tessl-research.md)
(the comparative research that motivated treating agent context as a
managed, distributable artefact), [PROP-002 §2.1](../vibe-registry/PROP-002-decentralized-registry.md#identity)
(`content_hash` identity the `query_package` tool surfaces),
[PROP-003 §2.5](../vibe-resolver/PROP-003-dep-evolution.md) (the subskill
delivery modes the `read_subskill` / `materialise_subskill` tools read),
[`VIBEVM-SPEC.md` §5](../../../VIBEVM-SPEC.md) (the product's AI-integration
scope).

---

## 1. Motivation {#motivation}

vibevm installs spec-and-discipline packages into a project; the consuming
agent then has to *find and read* what landed. Two integration surfaces
close that loop:

1. A **Model Context Protocol server** (`vibe mcp serve`) that exposes the
   project's lockfile-derived state to any MCP-speaking agent as callable
   tools — so the agent queries package identity and pulls subskill content
   on demand instead of guessing from the file tree.
2. An **agent-integration command family** (`vibe mcp install` and friends)
   that wires that server into each agent's own configuration and writes a
   per-agent skill manifest, so an operator runs one command instead of
   hand-editing five different config files.

Both are product scope (`VIBEVM-SPEC.md` §5) and neither changes wire
formats, the lockfile schema, or install behaviour — they are read-mostly
surfaces over state the rest of vibevm already owns.

## 2. Decisions {#decisions}

### 2.1 The server: JSON-RPC 2.0 over stdio {#server}

`req r1`

**Decision.** `vibe-mcp` is a transport-agnostic MCP server. It speaks
JSON-RPC 2.0 over line-delimited stdin/stdout (the MCP stdio form),
handling the `initialize` handshake, `tools/list`, `tools/call`, and
`ping`. The protocol version is a one-line `const` (`PROTOCOL_VERSION`);
the server name/version surface in the handshake.

- Transport is a trait (`Transport`) — production uses `StdioTransport`,
  tests inject `MemoryTransport` for deterministic round-trips.
- Each `tools/call` reloads the project lockfile fresh, so a concurrent
  `vibe install` surfaces on the next call without a server restart.
- A missing `vibe.lock` is an empty lockfile, not an error — the agent
  sees empty state through the normal tool response.

### 2.2 The tool surface: one seam, three tools {#tools}

`req r1`

**Decision.** Every tool implements one seam (`McpTool`): it `describe`s
itself (name, human description, JSON-Schema input shape) and `run`s
against parsed arguments plus the read-only `ServerContext`. Tools
register at one point; the dispatcher routes by registered name and does
not know a tool's identity beyond it. The seam is the cell boundary — a
new tool is a new cell, not an edit to the dispatcher.

The shipped tools (all group-qualified `<group>/<name>` pkgrefs in;
structured JSON + a text rendering out):

- **`query_package`** — the full lockfile entry for an installed package
  (kind, version, `content_hash`, registry, source, `files_written`,
  features, active subskills, `describes` PURL, language). Read-only.
- **`read_subskill`** — the concatenated content of an active subskill's
  files. `eager` / `lazy-push` subskills read from the project tree;
  `lazy-pull` subskills read from the package cache (PROP-003 §2.5.0), so
  the agent gets bytes regardless of delivery mode. Read-only.
- **`materialise_subskill`** — copy a `lazy-pull` subskill's content into
  the project tree. No-op for `eager` / `lazy-push` (already on disk);
  refuses to overwrite without `force`. The one writing tool.

A tool failure renders as `isError: true` in the result payload (a
tool-level failure), distinct from a transport-level JSON-RPC error.

### 2.3 Tool and server errors cite their REQ {#errors}

`req r1`

**Decision.** `ToolError` and `ServerError` are the crate's two error
layers; each variant's Display text carries the violated `spec://` unit
and a fix surface (the Class-F product-error grammar), so a failing tool
call is navigable back to this PROP without source access.

### 2.4 Agent detection {#agent-detection}

`req r1`

**Decision.** The integration surface supports a fixed set of MCP-capable
coding agents (Claude Code, Claude Code Desktop, Cursor, OpenCode, Codex).
An agent is *present* in a project when its project-level markers exist
(e.g. `.claude` / `CLAUDE.md`, `.cursor` / `.cursorrules`) or its
user-level host config directory exists. Detection drives the default
target set for `vibe mcp install`; the operator can always override with
an explicit agent filter.

### 2.5 Per-agent configuration {#agent-config}

`req r2`

**Decision.** Each agent declares its config shape, and the writer is
agent-aware but format-generic:

- **Format** — JSON or TOML per agent (Codex is TOML-only).
- **Section key** — the agent's MCP-servers table name (`mcpServers`,
  `mcp`, `mcp_servers`).
- **Scope** — project (`.<agent>/…` in the repo) and/or user (the host
  config dir). Some agents are user-only (Claude Code Desktop, Codex).
- **Config path** — resolved per (agent, scope), cross-platform. The
  path must be the file the agent actually reads for MCP *discovery*,
  not merely a settings file it happens to own. For Claude Code that is
  `<project>/.mcp.json` (project) and the top-level `mcpServers` of
  `~/.claude.json` (user) — **never `settings.json`**, which only
  *gates* `.mcp.json` servers (`enabledMcpjsonServers`) and does not
  define them.
- **Merge discipline** — installing upserts vibevm's one entry under the
  section key and **preserves every foreign key, and their order**: the
  JSON writer round-trips order-preserving (`serde_json/preserve_order`),
  so a merge into a large `~/.claude.json` appends rather than
  re-alphabetising the operator's whole file. Uninstalling strips only
  vibevm's entry and leaves the rest. The operator's other MCP servers
  and unrelated config survive every operation.

The vibevm entry is **scope-independent**: `vibe mcp serve` with no
`--path`, resolving its project root from the launcher's CWD (an MCP
client sets CWD to the project directory for a project-scope server),
so one shape serves every scope and a committed `.mcp.json` stays
portable. On Windows the launcher is wrapped as `cmd /c vibe …` because
`vibe` is a `vibe.cmd` shim that an MCP client's bare process-spawn
cannot exec directly.

### 2.6 Skill materialisation {#skill}

`req r1`

**Decision.** For agents that support a skill manifest (Claude Code,
OpenCode, Codex — not the JSON-config-only Cursor / Claude Code Desktop),
`vibe mcp install` also writes a `SKILL.md` describing how to use vibevm
through the MCP tools. The skill body is **data, not code** — a vendored
template (`include_str!`) rendered into each agent's skill directory
(`.<agent>/skills/vibevm/SKILL.md`). Writing is idempotent: identical
content is left untouched (`unchanged`); a divergent file is updated.

### 2.7 The integration lifecycle {#lifecycle}

`req r1`

**Decision.** The agent-integration command family is a coherent
lifecycle over the (agent × scope) matrix, every verb idempotent and
every mutating verb offering `--dry-run` and a confirmation:

- **`install`** — detect (or accept a filter), preview, confirm, write
  MCP entries and skills.
- **`status`** — read-only: what would install / upgrade / uninstall do.
- **`upgrade`** — refresh stale MCP blocks and `SKILL.md` files in place.
- **`uninstall`** — strip vibevm's MCP entries and skill files, preserving
  foreign config.

Per-(agent, scope) outcomes are reported as structured records
(`AgentInstallReport` / `SkillInstallReport`) the CLI renders or emits as
JSON.

## 3. Out of scope {#out-of-scope}

- **No new wire formats or lockfile changes.** The server reads the
  existing lockfile schema; the tools surface existing fields.
- **No agent-specific behaviour beyond config shape.** The integration
  knows each agent's *file format and paths*, not its runtime semantics.
- **HTTP / SSE transports.** Stdio is the shipped transport; the
  `Transport` trait leaves room without committing to more today.
- **LLM-provider tools.** PROP-003 §F virtual-capability emission waits on
  a real `vibe-llm` (`VIBEVM-SPEC.md` §10.4).

## 4. Acceptance {#acceptance}

- The server answers `initialize` / `tools/list` / `tools/call` over a
  `MemoryTransport` round-trip; each tool has a behavioural oracle.
- `query_package` / `read_subskill` / `materialise_subskill` each behave
  per §2.2 against a lockfile fixture (found / not-found / invalid-pkgref;
  the delivery-mode split; the force / no-overwrite contract).
- `vibe mcp install` writes the expected entry under each agent's section
  key, preserves foreign keys, and is idempotent; `uninstall` is its
  inverse; `status` writes nothing.
