# PROP-015 — MCP server and agent integration {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: IMPLEMENTED; retroactive spec home for vibe-mcp and the vibe mcp family"/>

##milestone-line **Milestone:** M1.7 ([`ROADMAP.md`](../../../ROADMAP.md)). The server slice
shipped first; the agent-integration surface (`vibe mcp install` /
`status` / `upgrade` / `uninstall`) followed. @impl/done

##status-line **Status:** IMPLEMENTED — this PROP is the retroactive spec home the
CONVERT-PLAN v0.1 §7 endgame opened for the `vibe-mcp` crate and the
`vibe mcp` command family. Units typed at REQ grain; the code carries the
matching `scope!` / `#[spec(implements)]` edges. @impl/done

##related **Related:** [PROP-004 §5 / §6](../../common/PROP-004-tessl-research.md)
(the comparative research that motivated treating agent context as a
managed, distributable artefact), [PROP-002 §2.1](../vibe-registry/PROP-002-decentralized-registry.md#identity)
(`content_hash` identity the `query_package` tool surfaces),
[PROP-003 §2.5](../vibe-resolver/PROP-003-dep-evolution.md) (the subskill
delivery modes the `read_subskill` / `materialise_subskill` tools read),
[`VIBEVM-SPEC.md` §5](../../../VIBEVM-SPEC.md) (the product's AI-integration
scope), and [PROP-023](../vibe-registry/PROP-023-bridge-packages.md) (the
bridge-packages design that added the [#skill-include](#skill-include) req on
2026-06-24 — additive; §2.6 `#skill` is unchanged). @spec/done

---

## 1. Motivation {#motivation}

##close-the-loop vibevm installs spec-and-discipline packages into a project; the consuming
agent then has to *find and read* what landed. Two integration surfaces
close that loop: @impl/done

1. ##SURFACE-SERVER A **Model Context Protocol server** (`vibe mcp serve`) that exposes the
   project's lockfile-derived state to any MCP-speaking agent as callable
   tools — so the agent queries package identity and pulls subskill content
   on demand instead of guessing from the file tree. @impl/done
2. ##SURFACE-INSTALL An **agent-integration command family** (`vibe mcp install` and friends)
   that wires that server into each agent's own configuration and writes a
   per-agent skill manifest, so an operator runs one command instead of
   hand-editing five different config files. @impl/done

- ##PRODUCT-SCOPE Both are product scope (`VIBEVM-SPEC.md` §5). @impl/done
- ##READ-MOSTLY Neither changes wire
  formats, the lockfile schema, or install behaviour — they are read-mostly
  surfaces over state the rest of vibevm already owns. @impl/done

## 2. Decisions {#decisions}

### 2.1 The server: JSON-RPC 2.0 over stdio {#server}

##req-server `req r1` @impl/done

##SERVER-TRANSPORT-AGNOSTIC **Decision.** `vibe-mcp` is a transport-agnostic MCP server. @impl/done

- ##SERVER-JSONRPC It speaks
  JSON-RPC 2.0 over line-delimited stdin/stdout (the MCP stdio form),
  handling the `initialize` handshake, `tools/list`, `tools/call`, and
  `ping`. @impl/done
- ##SERVER-HANDSHAKE-IDENTITY The protocol version is a one-line `const` (`PROTOCOL_VERSION`);
  the server name/version surface in the handshake. @impl/done
- ##SERVER-TRANSPORT-TRAIT Transport is a trait (`Transport`) — production uses `StdioTransport`,
  tests inject `MemoryTransport` for deterministic round-trips. @impl/done
- ##SERVER-FRESH-LOCKFILE Each `tools/call` reloads the project lockfile fresh, so a concurrent
  `vibe install` surfaces on the next call without a server restart. @impl/done
- ##SERVER-MISSING-LOCK A missing `vibe.lock` is an empty lockfile, not an error — the agent
  sees empty state through the normal tool response. @impl/done

### 2.2 The tool surface: one seam, three tools {#tools}

##req-tools `req r1` @impl/done

##TOOL-SEAM **Decision.** Every tool implements one seam (`McpTool`): it `describe`s
itself (name, human description, JSON-Schema input shape) and `run`s
against parsed arguments plus the read-only `ServerContext`. @impl/done

- ##TOOL-REGISTRATION Tools
  register at one point; the dispatcher routes by registered name and does
  not know a tool's identity beyond it. @impl/done
- ##TOOL-CELL-BOUNDARY The seam is the cell boundary — a
  new tool is a new cell, not an edit to the dispatcher. @impl/done

##TOOLS-IO-CONVENTION The shipped tools (all group-qualified `<group>/<name>` pkgrefs in;
structured JSON + a text rendering out): @impl/done

- ##TOOL-QUERY-PACKAGE **`query_package`** — the full lockfile entry for an installed package
  (kind, version, `content_hash`, registry, source, `files_written`,
  features, active subskills, `describes` PURL, language). Read-only. @impl/done
- ##TOOL-READ-SUBSKILL **`read_subskill`** — the concatenated content of an active subskill's
  files. `eager` / `lazy-push` subskills read from the project tree;
  `lazy-pull` subskills read from the package cache (PROP-003 §2.5.0), so
  the agent gets bytes regardless of delivery mode. Read-only. @impl/done
- ##TOOL-MATERIALISE-SUBSKILL **`materialise_subskill`** — copy a `lazy-pull` subskill's content into
  the project tree. No-op for `eager` / `lazy-push` (already on disk);
  refuses to overwrite without `force`. The one writing tool. @impl/done

##TOOL-FAILURE-RENDERING A tool failure renders as `isError: true` in the result payload (a
tool-level failure), distinct from a transport-level JSON-RPC error. @impl/done

### 2.3 Tool and server errors cite their REQ {#errors}

##req-errors `req r1` @impl/done

- ##ERROR-LAYERS **Decision.** `ToolError` and `ServerError` are the crate's two error
  layers. @impl/done
- ##ERROR-CITES-REQ Each variant's Display text carries the violated `spec://` unit
  and a fix surface (the Class-F product-error grammar), so a failing tool
  call is navigable back to this PROP without source access. @impl/done

### 2.4 Agent detection {#agent-detection}

##req-agent-detection `req r1` @impl/done

##AGENT-SET **Decision.** The integration surface supports a fixed set of MCP-capable
coding agents (Claude Code, Claude Code Desktop, Cursor, OpenCode, Codex). @impl/done

- ##AGENT-PRESENCE An agent is *present* in a project when its project-level markers exist
  (e.g. `.claude` / `CLAUDE.md`, `.cursor` / `.cursorrules`) or its
  user-level host config directory exists. @impl/done
- ##AGENT-DETECTION-DEFAULT Detection drives the default
  target set for `vibe mcp install`; the operator can always override with
  an explicit agent filter. @impl/done

### 2.5 Per-agent configuration {#agent-config}

##req-agent-config `req r2` @impl/done

##CONFIG-SHAPE-DECL **Decision.** Each agent declares its config shape, and the writer is
agent-aware but format-generic: @impl/done

- ##CONFIG-FORMAT **Format** — JSON or TOML per agent (Codex is TOML-only). @impl/done
- ##CONFIG-SECTION-KEY **Section key** — the agent's MCP-servers table name (`mcpServers`,
  `mcp`, `mcp_servers`). @impl/done
- ##CONFIG-SCOPE **Scope** — project (`.<agent>/…` in the repo) and/or user (the host
  config dir). Some agents are user-only (Claude Code Desktop, Codex). @impl/done
- ##CONFIG-PATH **Config path** — resolved per (agent, scope), cross-platform. The
  path must be the file the agent actually reads for MCP *discovery*,
  not merely a settings file it happens to own. For Claude Code that is
  `<project>/.mcp.json` (project) and the top-level `mcpServers` of
  `~/.claude.json` (user) — **never `settings.json`**, which only
  *gates* `.mcp.json` servers (`enabledMcpjsonServers`) and does not
  define them. @impl/done
- ##CONFIG-MERGE **Merge discipline** — installing upserts vibevm's one entry under the
  section key and **preserves every foreign key, and their order**: the
  JSON writer round-trips order-preserving (`serde_json/preserve_order`),
  so a merge into a large `~/.claude.json` appends rather than
  re-alphabetising the operator's whole file. Uninstalling strips only
  vibevm's entry and leaves the rest. The operator's other MCP servers
  and unrelated config survive every operation. @impl/done

- ##CONFIG-SCOPE-INDEPENDENT The vibevm entry is **scope-independent**: `vibe mcp serve` with no
  `--path`, resolving its project root from the launcher's CWD (an MCP
  client sets CWD to the project directory for a project-scope server),
  so one shape serves every scope and a committed `.mcp.json` stays
  portable. @impl/done
- ##CONFIG-WINDOWS-SHIM On Windows the launcher is wrapped as `cmd /c vibe …` because
  `vibe` is a `vibe.cmd` shim that an MCP client's bare process-spawn
  cannot exec directly. @impl/done

### 2.6 Skill materialisation {#skill}

##req-skill `req r1` @impl/done

##SKILL-MANIFEST **Decision.** For agents that support a skill manifest (Claude Code,
OpenCode, Codex — not the JSON-config-only Cursor / Claude Code Desktop),
`vibe mcp install` also writes a `SKILL.md` describing how to use vibevm
through the MCP tools. @impl/done

- ##SKILL-BODY-DATA The skill body is **data, not code** — a vendored
  template (`include_str!`) rendered into each agent's skill directory
  (`.<agent>/skills/vibevm/SKILL.md`). @impl/done
- ##SKILL-IDEMPOTENT Writing is idempotent: identical
  content is left untouched (`unchanged`); a divergent file is updated. @impl/done

### 2.7 The integration lifecycle {#lifecycle}

##req-lifecycle `req r1` @impl/done

##LIFECYCLE-MATRIX **Decision.** The agent-integration command family is a coherent
lifecycle over the (agent × scope) matrix, every verb idempotent and
every mutating verb offering `--dry-run` and a confirmation: @impl/done

- ##VERB-INSTALL **`install`** — detect (or accept a filter), preview, confirm, write
  MCP entries and skills. @impl/done
- ##VERB-STATUS **`status`** — read-only: what would install / upgrade / uninstall do. @impl/done
- ##VERB-UPGRADE **`upgrade`** — refresh stale MCP blocks and `SKILL.md` files in place. @impl/done
- ##VERB-UNINSTALL **`uninstall`** — strip vibevm's MCP entries and skill files, preserving
  foreign config. @impl/done

##LIFECYCLE-REPORTS Per-(agent, scope) outcomes are reported as structured records
(`AgentInstallReport` / `SkillInstallReport`) the CLI renders or emits as
JSON. @impl/done

### 2.8 Selective skill projection {#skill-include}

##req-skill-include `req r1` @impl/done

##INCLUDE-DECL **Decision.** `SkillDecl` gains an optional `include` — a list of glob
patterns relative to the skill's `path`. @impl/done

- ##INCLUDE-SELECTIVE When present, only matching files are
  projected into the agent's skill directory, preserving their relative
  structure; when absent or empty, the whole `path` tree is projected — the
  existing §2.6 behaviour, unchanged. @impl/done
- ##INCLUDE-COMPOSES Selection composes with the
  already-working nested `path`: a skill can point at a subdirectory **and** pick
  specific files out of it. @impl/done
- ##INCLUDE-BRIDGE-USE This is available to any skill but is load-bearing for bridge packages
  ([PROP-023](../vibe-registry/PROP-023-bridge-packages.md)): a bridged upstream
  tree is full of unrelated files, and the maintainer projects just the
  `SKILL.md` and whatever it references without vendoring the noise. @impl/done
- ##INCLUDE-DETERMINISTIC Glob
  matching is deterministic; a pattern that matches nothing is a
  declared-but-empty selection (surfaced, not a silent no-op). @impl/done

```toml
[[skill]]
name = "vim"
path = "upstream/skills/vim"
include = ["SKILL.md", "references/**/*.md"]   # omit → whole tree (§2.6)
```

## 3. Out of scope {#out-of-scope}

- ##OOS-NO-WIRE-CHANGES **No new wire formats or lockfile changes.** The server reads the
  existing lockfile schema; the tools surface existing fields. @spec/done
- ##OOS-AGENT-SEMANTICS **No agent-specific behaviour beyond config shape.** The integration
  knows each agent's *file format and paths*, not its runtime semantics. @spec/done
- ##OOS-HTTP-SSE **HTTP / SSE transports.** Stdio is the shipped transport; the
  `Transport` trait leaves room without committing to more today. @spec/done
- ##OOS-LLM-TOOLS **LLM-provider tools.** PROP-003 §F virtual-capability emission waits on
  a real `vibe-llm` (`VIBEVM-SPEC.md` §10.4). @spec/done

## 4. Acceptance {#acceptance}

- ##ACC-SERVER-ROUNDTRIP The server answers `initialize` / `tools/list` / `tools/call` over a
  `MemoryTransport` round-trip; each tool has a behavioural oracle. @impl/done
- ##ACC-TOOL-CONTRACTS `query_package` / `read_subskill` / `materialise_subskill` each behave
  per §2.2 against a lockfile fixture (found / not-found / invalid-pkgref;
  the delivery-mode split; the force / no-overwrite contract). @impl/done
- ##ACC-INSTALL-LIFECYCLE `vibe mcp install` writes the expected entry under each agent's section
  key, preserves foreign keys, and is idempotent; `uninstall` is its
  inverse; `status` writes nothing. @impl/done
