# PROP-015 — MCP server and agent integration {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: IMPLEMENTED; retroactive spec home for vibe-mcp and the vibe mcp family"/>

@fact:milestone-line **Milestone:** M1.7 ([`ROADMAP.md`](../../../ROADMAP.md)). The server slice
shipped first; the agent-integration surface (`vibe mcp install` /
`status` / `upgrade` / `uninstall`) followed. @status:impl/done

@fact:status-line **Status:** IMPLEMENTED — this PROP is the retroactive spec home the
CONVERT-PLAN v0.1 §7 endgame opened for the `vibe-mcp` crate and the
`vibe mcp` command family. Units typed at REQ grain; the code carries the
matching `scope!` / `#[spec(implements)]` edges. @status:impl/done

@fact:related **Related:** [PROP-004 §5 / §6](../../common/PROP-004-tessl-research.md)
(the comparative research that motivated treating agent context as a
managed, distributable artefact), [PROP-002 §2.1](../vibe-registry/PROP-002-decentralized-registry.md#identity)
(`content_hash` identity the `query_package` tool surfaces),
[PROP-003 §2.5](../vibe-resolver/PROP-003-dep-evolution.md) (the subskill
delivery modes the `read_subskill` / `materialise_subskill` tools read),
[`VIBEVM-SPEC.md` §5](../../../VIBEVM-SPEC.md) (the product's AI-integration
scope), and [PROP-023](../vibe-registry/PROP-023-bridge-packages.md) (the
bridge-packages design that added the [#skill-include](#skill-include) req on
2026-06-24 — additive; §2.6 `#skill` is unchanged). @status:spec/done

---

## 1. Motivation {#motivation}

@fact:close-the-loop vibevm installs spec-and-discipline packages into a project; the consuming
agent then has to *find and read* what landed. Two integration surfaces
close that loop: @status:impl/done

1. @fact:SURFACE-SERVER A **Model Context Protocol server** (`vibe mcp serve`) that exposes the
   project's lockfile-derived state to any MCP-speaking agent as callable
   tools — so the agent queries package identity and pulls subskill content
   on demand instead of guessing from the file tree. @status:impl/done
2. @fact:SURFACE-INSTALL An **agent-integration command family** (`vibe mcp install` and friends)
   that wires that server into each agent's own configuration and writes a
   per-agent skill manifest, so an operator runs one command instead of
   hand-editing five different config files. @status:impl/done

- @fact:PRODUCT-SCOPE Both are product scope (`VIBEVM-SPEC.md` §5). @status:impl/done
- @fact:READ-MOSTLY Neither changes wire
  formats, the lockfile schema, or install behaviour — they are read-mostly
  surfaces over state the rest of vibevm already owns. @status:impl/done

## 2. Decisions {#decisions}

### 2.1 The server: JSON-RPC 2.0 over stdio {#server}

@fact:req-server `req r1` @status:impl/done

@fact:SERVER-TRANSPORT-AGNOSTIC **Decision.** `vibe-mcp` is a transport-agnostic MCP server. @status:impl/done

- @fact:SERVER-JSONRPC It speaks
  JSON-RPC 2.0 over line-delimited stdin/stdout (the MCP stdio form),
  handling the `initialize` handshake, `tools/list`, `tools/call`, and
  `ping`. @status:impl/done
- @fact:SERVER-HANDSHAKE-IDENTITY The protocol version is a one-line `const` (`PROTOCOL_VERSION`);
  the server name/version surface in the handshake. @status:impl/done
- @fact:SERVER-TRANSPORT-TRAIT Transport is a trait (`Transport`) — production uses `StdioTransport`,
  tests inject `MemoryTransport` for deterministic round-trips. @status:impl/done
- @fact:SERVER-FRESH-LOCKFILE Each `tools/call` reloads the project lockfile fresh, so a concurrent
  `vibe install` surfaces on the next call without a server restart. @status:impl/done
- @fact:SERVER-MISSING-LOCK A missing `vibe.lock` is an empty lockfile, not an error — the agent
  sees empty state through the normal tool response. @status:impl/done

### 2.2 The tool surface: one seam, three tools {#tools}

@fact:req-tools `req r1` @status:impl/done

@fact:TOOL-SEAM **Decision.** Every tool implements one seam (`McpTool`): it `describe`s
itself (name, human description, JSON-Schema input shape) and `run`s
against parsed arguments plus the read-only `ServerContext`. @status:impl/done

- @fact:TOOL-REGISTRATION Tools
  register at one point; the dispatcher routes by registered name and does
  not know a tool's identity beyond it. @status:impl/done
- @fact:TOOL-CELL-BOUNDARY The seam is the cell boundary — a
  new tool is a new cell, not an edit to the dispatcher. @status:impl/done

@fact:TOOLS-IO-CONVENTION The shipped tools (all group-qualified `<group>/<name>` pkgrefs in;
structured JSON + a text rendering out): @status:impl/done

- @fact:TOOL-QUERY-PACKAGE **`query_package`** — the full lockfile entry for an installed package
  (kind, version, `content_hash`, registry, source, `files_written`,
  features, active subskills, `describes` PURL, language). Read-only. @status:impl/done
- @fact:TOOL-READ-SUBSKILL **`read_subskill`** — the concatenated content of an active subskill's
  files. `eager` / `lazy-push` subskills read from the project tree;
  `lazy-pull` subskills read from the package cache (PROP-003 §2.5.0), so
  the agent gets bytes regardless of delivery mode. Read-only. @status:impl/done
- @fact:TOOL-MATERIALISE-SUBSKILL **`materialise_subskill`** — copy a `lazy-pull` subskill's content into
  the project tree. No-op for `eager` / `lazy-push` (already on disk);
  refuses to overwrite without `force`. The one writing tool. @status:impl/done

@fact:TOOL-FAILURE-RENDERING A tool failure renders as `isError: true` in the result payload (a
tool-level failure), distinct from a transport-level JSON-RPC error. @status:impl/done

#### 2.2.1 Searching the map — the set-returning twin of `explain` {#map-query}

@fact:MAP-QUERY-ANSWERS-A-DIFFERENT-QUESTION **`query` finds nodes; `explain`
looks at one.** Until 2026-08-06 the map could only be asked about a target
already known by name, so *«which of these exist?»* was unanswerable — not in
the host, not in any language stack, not in the engine. `query` is that
question, and the two are deliberately separate verbs rather than one verb with
a mode: a point lookup and a set filter render differently, cap differently, and
fail differently. @status:impl/done

@fact:MAP-QUERY-THE-SIMPLE-LEVEL-IS-PERMANENT **The filter level is a permanent
level, not a first version to be replaced** *(owner ruling, 2026-08-06)*. An
agent accustomed to grep reaches for filters; a query language demands a form it
will not build without need. So the filters must work on their own and must
never become a degenerate case of a grammar — in the library they are their own
entry point, so a broken parser could not take them down with it. @status:impl/done

@fact:MAP-QUERY-THREE-FILTERS-AND-A-CEILING **Three filters, combined with AND,
under a hard ceiling:** exact spec address, substring of a code symbol, element
kind. None is required; those given narrow. The ceiling is not a convenience and
cannot be removed — the answer is read by an agent with a bounded context, and
an unbounded one is useless rather than generous. When it truncates it says so,
with the total, in both renderings. @status:impl/done

@fact:MAP-QUERY-RESULTS-ARE-NODES **Results are nodes, never edges.** «Find me an
edge» is not a question anyone asks; «find me what has, or lacks, an edge of this
kind» is — so edges are a filter dimension for the query level above, and the
result set stays spec units and code items. @status:impl/done

@fact:MAP-QUERY-A-HIT-CARRIES-ITS-SOURCE **Every hit records where it came
from**, because a second producer is already designed: the code-quality engine's
findings join these results **at query time**, by the owner's ruling that two
engines must not merge their data. Nothing of that engine is built here; the
result shape simply does not close the door — a discriminated field rather than a
closed variant set, so a third source adds a value instead of breaking every
reader. @status:spec/plan

@fact:MAP-QUERY-IS-ONE-CAPABILITY-WITH-TWO-SURFACES **One library, two thin
surfaces**, per the omnichannel floor: the filtering lives in the host's trace
crate, `vibe query` renders it for a person and the `query` tool renders it for
an agent, and both call the same function. The MCP tool's description says when
to reach for it instead of `explain` — that description is the surface's own,
which is exactly what the agent-facing class owns and all it owns. @status:impl/done

@fact:MAP-QUERY-THE-KIND-VOCABULARY-IS-MEASURED-NOT-INVENTED **`kind` filters over
a measured vocabulary.** The committed map carries nine code kinds, and a spec
unit answers on its own kind rather than borrowing the code one; the two
vocabularies are disjoint, so one filter serves both without ambiguity. **Caveat
worth carrying:** every spec unit in this tree is legacy-unmarked, so a spec kind
matches nothing here today. The path is open and exercised only by fixtures —
stated because a filter that silently returns nothing is indistinguishable from
one that is broken. @status:impl/done

@fact:MAP-QUERY-BUILT-FRESH-LIKE-EXPLAIN The map is built fresh in memory per
call, never read from the committed artefact — the same posture `explain` takes,
for the same reason: a query answers for the tree as it is. @status:impl/done

@fact:MAP-QUERY-THE-LANGUAGE-LEVEL-IS-NOT-BUILT **The query language is designed
and not built.** Its shape is the filters plus graph traversal — depth, and
«has no edge of kind X», which is what answers *«which rules does nothing
verify»* — and it introduces a grammar that will need versioning. It stands on
this level rather than replacing it. **SUPERSEDED 2026-08-06 — it is built; the
contract is §2.2.2 below.** @status:impl/done

#### 2.2.2 The query language — traversal on top of the filters {#map-select}

@fact:SELECT-IS-A-THIRD-VERB-NOT-A-MODE **`select` is a third verb, for the same
reason `query` and `explain` are two.** `explain` looks at one target, `query`
filters a set, and `select` walks the graph from a set. Three questions, three
caps, three failure modes — a mode flag on one verb would make the result type
depend on the argument, and every consumer would carry three readers. @status:impl/done

@fact:SELECT-STANDS-ON-THE-FILTER-LEVEL-AND-CANNOT-TAKE-IT-DOWN **It stands on
the filter level and cannot take it down with it.** The three filter predicates
are the ones §2.2.1 ships, reached rather than redefined; the parser lives in its
own module behind its own entry point, so a broken grammar leaves `query`
answering. That separation is the owner's ruling in
`##MAP-QUERY-THE-SIMPLE-LEVEL-IS-PERMANENT` made structural instead of promised. @status:impl/done

@fact:SELECT-SEVEN-PREDICATES-JOINED-BY-AND **Seven predicates, whitespace
separated, joined by AND, and no operators at all:** `uri:`, `symbol:`, `kind:`
(the filter level's own), `scope:` (prefix of a spec address), `has:` / `lacks:`
(an edge verb), `depth:` (0..3). No disjunction, no parentheses, no precedence —
each of those is a permanent versioning liability, and a predicate can be added
without renumbering a language that has no operator layer to renumber. @status:impl/done

@fact:SELECT-SCOPE-EXISTS-BECAUSE-A-MEASUREMENT-PUT-IT-THERE **`scope:` is in the
set because the level's own canonical question is unanswerable without it.**
Measured before the build: 5 742 of 5 825 spec units carry no `verifies` edge,
and nothing that shipped could narrow that — `kind` is carried by **0** units in
this tree and `uri` is exact, so composing the negative predicate with the filter
level yields 5 742 or 1. A document prefix brings 67 of the corpus's 72 documents
inside the ceiling; the five that do not are named in the design record rather
than left to be rediscovered. @status:impl/done

@fact:SELECT-HAS-AND-LACKS-READ-THE-EDGE-FROM-THE-NODES-SIDE **`has:` and
`lacks:` select seeds, and «touches» reads from each family's own side** —
incoming for a spec unit, outgoing for a code item. On a directed bipartite
graph that is the only reading under which one predicate serves both families,
and applying them after the walk instead would answer a question nobody asked. @status:impl/done

@fact:SELECT-DEPTH-EXPANDS-AND-ZERO-IS-THE-IDENTITY **`depth:N` expands the seed
set along edges, undirected, and the seeds stay in the answer.** `depth:0` is the
default and the identity, so a query without it is exactly the seed selection —
which is what keeps this level a strict superset of the one below rather than a
different thing wearing its name. Every hit carries the hop count it was reached
at, so a caller can tell what it asked for from what the walk brought with it. @status:impl/done

@fact:SELECT-THE-BOUND-IS-CHOSEN-AGAINST-A-MEASUREMENT-NOT-A-FEELING **The depth
bound is 3, and the number came from the graph.** Exhaustively over all 1 205
edge-bearing nodes, 71.7 % reach more at depth 2 than at depth 1 and only 5.9 %
reach more at depth 3; the largest connected component is 44 nodes. So depth is a
precision control on this map rather than a safety one — and the result ceiling
stays hard regardless, because it protects against a future map, not this one. @status:impl/done

@fact:SELECT-AN-UNKNOWN-PREDICATE-IS-AN-ERROR **An unknown predicate, an unknown
verb, a repeated predicate, an out-of-range depth or an empty query is an ERROR
that names the offending token and lists what was expected** — never a silently
ignored clause. Same law as the markup's typed fences, for the same reason: a
grammar that ignores what it does not understand promises everything and checks
nothing, and whoever trusted the promise is the one who finds out. An empty query
is refused rather than read as «everything», because that answer already exists
one verb away. @status:impl/done

@fact:SELECT-THE-GRAMMAR-VERSION-TRAVELS-IN-THE-ANSWER **The grammar carries a
version and reports it in every answer rather than demanding it in every query.**
A query string stays free of ceremony; the structured answer states the version
it was parsed under, so a consumer that cares can branch and one that does not is
unaffected. Requiring a prefix would tax every caller forever to buy nothing
until the first breaking change. @status:impl/done

@fact:SELECT-THE-REASONING-LIVES-BESIDE-THE-CONTRACT The rejected shapes, the
measurements above with their commands, and the correction of a sampled reading
that was wrong about depth are in
[`spec/design/map-query-language.md`](../../design/map-query-language.md) — the
lore this contract is the short form of. @status:impl/done

### 2.3 Tool and server errors cite their REQ {#errors}

@fact:req-errors `req r1` @status:impl/done

- @fact:ERROR-LAYERS **Decision.** `ToolError` and `ServerError` are the crate's two error
  layers. @status:impl/done
- @fact:ERROR-CITES-REQ Each variant's Display text carries the violated `spec://` unit
  and a fix surface (the Class-F product-error grammar), so a failing tool
  call is navigable back to this PROP without source access. @status:impl/done

### 2.4 Agent detection {#agent-detection}

@fact:req-agent-detection `req r1` @status:impl/done

@fact:AGENT-SET **Decision.** The integration surface supports a fixed set of MCP-capable
coding agents (Claude Code, Claude Code Desktop, Cursor, OpenCode, Codex). @status:impl/done

- @fact:AGENT-PRESENCE An agent is *present* in a project when its project-level markers exist
  (e.g. `.claude` / `CLAUDE.md`, `.cursor` / `.cursorrules`) or its
  user-level host config directory exists. @status:impl/done
- @fact:AGENT-DETECTION-DEFAULT Detection drives the default
  target set for `vibe mcp install`; the operator can always override with
  an explicit agent filter. @status:impl/done

### 2.5 Per-agent configuration {#agent-config}

@fact:req-agent-config `req r2` @status:impl/done

@fact:CONFIG-SHAPE-DECL **Decision.** Each agent declares its config shape, and the writer is
agent-aware but format-generic: @status:impl/done

- @fact:CONFIG-FORMAT **Format** — JSON or TOML per agent (Codex is TOML-only). @status:impl/done
- @fact:CONFIG-SECTION-KEY **Section key** — the agent's MCP-servers table name (`mcpServers`,
  `mcp`, `mcp_servers`). @status:impl/done
- @fact:CONFIG-SCOPE **Scope** — project (`.<agent>/…` in the repo) and/or user (the host
  config dir). Some agents are user-only (Claude Code Desktop, Codex). @status:impl/done
- @fact:CONFIG-PATH **Config path** — resolved per (agent, scope), cross-platform. The
  path must be the file the agent actually reads for MCP *discovery*,
  not merely a settings file it happens to own. For Claude Code that is
  `<project>/.mcp.json` (project) and the top-level `mcpServers` of
  `~/.claude.json` (user) — **never `settings.json`**, which only
  *gates* `.mcp.json` servers (`enabledMcpjsonServers`) and does not
  define them. @status:impl/done
- @fact:CONFIG-MERGE **Merge discipline** — installing upserts vibevm's one entry under the
  section key and **preserves every foreign key, and their order**: the
  JSON writer round-trips order-preserving (`serde_json/preserve_order`),
  so a merge into a large `~/.claude.json` appends rather than
  re-alphabetising the operator's whole file. Uninstalling strips only
  vibevm's entry and leaves the rest. The operator's other MCP servers
  and unrelated config survive every operation. @status:impl/done

- @fact:CONFIG-SCOPE-INDEPENDENT The vibevm entry is **scope-independent**: `vibe mcp serve` with no
  `--path`, resolving its project root from the launcher's CWD (an MCP
  client sets CWD to the project directory for a project-scope server),
  so one shape serves every scope and a committed `.mcp.json` stays
  portable. @status:impl/done
- @fact:CONFIG-WINDOWS-SHIM On Windows the launcher is wrapped as `cmd /c vibe …` because
  `vibe` is a `vibe.cmd` shim that an MCP client's bare process-spawn
  cannot exec directly. @status:impl/done

### 2.6 Skill materialisation {#skill}

@fact:req-skill `req r1` @status:impl/done

@fact:SKILL-MANIFEST **Decision.** For agents that support a skill manifest (Claude Code,
OpenCode, Codex — not the JSON-config-only Cursor / Claude Code Desktop),
`vibe mcp install` also writes a `SKILL.md` describing how to use vibevm
through the MCP tools. @status:impl/done

- @fact:SKILL-BODY-DATA The skill body is **data, not code** — a vendored
  template (`include_str!`) rendered into each agent's skill directory
  (`.<agent>/skills/vibevm/SKILL.md`). @status:impl/done
- @fact:SKILL-IDEMPOTENT Writing is idempotent: identical
  content is left untouched (`unchanged`); a divergent file is updated. @status:impl/done

### 2.7 The integration lifecycle {#lifecycle}

@fact:req-lifecycle `req r1` @status:impl/done

@fact:LIFECYCLE-MATRIX **Decision.** The agent-integration command family is a coherent
lifecycle over the (agent × scope) matrix, every verb idempotent and
every mutating verb offering `--dry-run` and a confirmation: @status:impl/done

- @fact:VERB-INSTALL **`install`** — detect (or accept a filter), preview, confirm, write
  MCP entries and skills. @status:impl/done
- @fact:VERB-STATUS **`status`** — read-only: what would install / upgrade / uninstall do. @status:impl/done
- @fact:VERB-UPGRADE **`upgrade`** — refresh stale MCP blocks and `SKILL.md` files in place. @status:impl/done
- @fact:VERB-UNINSTALL **`uninstall`** — strip vibevm's MCP entries and skill files, preserving
  foreign config. @status:impl/done

@fact:LIFECYCLE-REPORTS Per-(agent, scope) outcomes are reported as structured records
(`AgentInstallReport` / `SkillInstallReport`) the CLI renders or emits as
JSON. @status:impl/done

### 2.8 Selective skill projection {#skill-include}

@fact:req-skill-include `req r1` @status:impl/done

@fact:INCLUDE-DECL **Decision.** `SkillDecl` gains an optional `include` — a list of glob
patterns relative to the skill's `path`. @status:impl/done

- @fact:INCLUDE-SELECTIVE When present, only matching files are
  projected into the agent's skill directory, preserving their relative
  structure; when absent or empty, the whole `path` tree is projected — the
  existing §2.6 behaviour, unchanged. @status:impl/done
- @fact:INCLUDE-COMPOSES Selection composes with the
  already-working nested `path`: a skill can point at a subdirectory **and** pick
  specific files out of it. @status:impl/done
- @fact:INCLUDE-BRIDGE-USE This is available to any skill but is load-bearing for bridge packages
  ([PROP-023](../vibe-registry/PROP-023-bridge-packages.md)): a bridged upstream
  tree is full of unrelated files, and the maintainer projects just the
  `SKILL.md` and whatever it references without vendoring the noise. @status:impl/done
- @fact:INCLUDE-DETERMINISTIC Glob
  matching is deterministic; a pattern that matches nothing is a
  declared-but-empty selection (surfaced, not a silent no-op). @status:impl/done

```toml
[[skill]]
name = "vim"
path = "upstream/skills/vim"
include = ["SKILL.md", "references/**/*.md"]   # omit → whole tree (§2.6)
```

## 3. Out of scope {#out-of-scope}

- @fact:OOS-NO-WIRE-CHANGES **No new wire formats or lockfile changes.** The server reads the
  existing lockfile schema; the tools surface existing fields. @status:spec/done
- @fact:OOS-AGENT-SEMANTICS **No agent-specific behaviour beyond config shape.** The integration
  knows each agent's *file format and paths*, not its runtime semantics. @status:spec/done
- @fact:OOS-HTTP-SSE **HTTP / SSE transports.** Stdio is the shipped transport; the
  `Transport` trait leaves room without committing to more today. @status:spec/done
- @fact:OOS-LLM-TOOLS **LLM-provider tools.** PROP-003 §F virtual-capability emission waits on
  a real `vibe-llm` (`VIBEVM-SPEC.md` §10.4). @status:spec/done

## 4. Acceptance {#acceptance}

- @fact:ACC-SERVER-ROUNDTRIP The server answers `initialize` / `tools/list` / `tools/call` over a
  `MemoryTransport` round-trip; each tool has a behavioural oracle. @status:impl/done
- @fact:ACC-TOOL-CONTRACTS `query_package` / `read_subskill` / `materialise_subskill` each behave
  per §2.2 against a lockfile fixture (found / not-found / invalid-pkgref;
  the delivery-mode split; the force / no-overwrite contract). @status:impl/done
- @fact:ACC-INSTALL-LIFECYCLE `vibe mcp install` writes the expected entry under each agent's section
  key, preserves foreign keys, and is idempotent; `uninstall` is its
  inverse; `status` writes nothing. @status:impl/done
