# PROP-026 — the tcg tool family (the agentic type oracle's product seam) {#root}

**Status: proposed 2026-07-07 with AGENTIC-TCG-TS-PLAN v0.1 (owner-
accepted the same day, with the §3 portability amendment); implemented
by its Phase 4. History: 2026-07-07 (same day, AGENTIC-TCG-RUST-PLAN
v0.1) — the §2 promise cashed: `language: "rust"` lands as an enum
value dispatching to the rust stack's `tcg-rust` relay; no new tools,
no schema shape change.** Module: `vibe-tcg` (new) + `vibe-mcp`
(adapter) + `vibe-workspace` (binary resolution).
**Related:** [PROP-015](PROP-015-mcp-integration.md) (the MCP server
this family is first mounted on), [PROP-018](../../common/PROP-018-agentic-standalone-modes.md)
(these tools are ALGORITHMIC — the `query_package` path; no affinity, no
relay, no `Intent`), [PROP-025](../vibe-workspace/PROP-025-binary-delivery.md)
(the slot-dispatch model the registry reuses), and the package-side
mechanism specs `TCG-ORACLE-v0.1` / `TCG-PROTOCOL-v0.1` in
`stack:org.vibevm/typescript-ai-native` plus `TCG-ORACLE-RUST-v0.1` /
`TCG-PROTOCOL-RUST-v0.1` in `stack:org.vibevm/rust-ai-native` (the
oracles ship THERE; vibevm ships only this thin product seam).

## 1. Problem {#problem}

`req r1`

The typescript-ai-native stack ships a type oracle (`tcg-typescript
serve`) that answers validate/scope/complete/type queries over in-memory
overlays at millisecond latency. An agent can already reach it the
degraded way (`vibe bin exec tcg-typescript -- <op> …`, one cold spawn
per question). What is missing is the WARM path: a persistent oracle
per language behind the agent's MCP session, so consultation costs
milliseconds, not process startups. That is product surface — lockfile
resolution, slot dispatch, child lifecycle, consent — and it must not
leak into either the package (which knows nothing of MCP) or vibe-mcp's
core (which stays a generic JSON-RPC tool host).

## 2. The four tools {#tools}

`req r3`

`tcg_validate`, `tcg_scope`, `tcg_complete`, `tcg_type` — thin schema
adapters over the TCG-PROTOCOL ops of the same names, each with:

- `language` (required; accepts `"typescript"` and — since the Rust
  twin landed, exactly as this clause promised — `"rust"`; any other
  value is a ToolError NAMING the supported set, so the next language
  adds an enum value, not new tools);
- the op's own params per TCG-PROTOCOL §2 (`file`, `position`,
  `content`, …), passed through verbatim.

Responses return the ENRICHED protocol result (TCG-PROTOCOL §3) as
`structuredContent`, plus a compact human text rendering (findings
first). These tools are deterministic queries — per PROP-018 §2.3 they
carry NO affinity machinery and never park intents.

## 3. The portable family crate — the owner amendment {#portability}

`req r3`

The family lives in a dedicated product crate **`vibe-tcg`**, NOT
inside vibe-mcp:

- `vibe-tcg` defines the tool descriptors/JSON schemas, the run logic,
  the `OracleRegistry` (§4), and a NARROW host abstraction —
  `trait TcgHost` exposing the project root and the no-prompt consent
  policy (§5). Dependencies: `vibe-core`, `vibe-workspace`, serde.
  **Zero vibe-mcp imports, by construction.**
- vibe-mcp mounts it through one thin adapter cell: newtype wrappers
  implementing `McpTool` by delegation, mapping `vibe-tcg`'s typed
  errors into `ToolError`. The adapter is the ONLY place the two crates
  meet.
- Consequence (the amendment's point): extracting a STANDALONE tcg MCP
  server later is one new binary crate — a JSON-RPC loop (vibe-mcp's
  `Server<T: Transport>` is already transport-generic) mounting the
  same `vibe-tcg` tools — with zero changes inside the family. That
  extraction is a named follow-up, not a redesign.

## 4. The oracle registry and child lifecycle {#registry}

`req r5`

`OracleRegistry`: interior-mutable (the MCP tool seam hands out shared
refs), lazily populated per language on first use, dropped with the
host session:

1. resolve the CURRENT project's lockfile → the stack slot that
   declares the language's oracle binary (`tcg-typescript` for
   TypeScript, `tcg-rust` for Rust; the per-language table also
   carries the requires-line and one-shot recipes so every refusal
   names ITS language's fix surface, not another's) — the PROP-025
   `[[binary]]` walk, through the SHARED `vibe-workspace`
   binary-resolution cell (extracted from vibe-cli by the TS campaign
   so CLI and registry cannot drift);
2. artifact present → spawn `<artifact> serve --root <project>` with
   piped stdio and hold the handle across calls; artifact absent →
   build per §5;
3. a dead child (`oracle-crashed`) → ONE transparent respawn attempt,
   then a recipe-carrying ToolError;
4. registry drop kills every child (kill-on-drop; the no-zombie
   property is test-asserted).

Failure surfaces are recipes, not dead ends: stack not installed → the
`[requires]` line + `vibe install`; language unsupported → the
supported set; node missing / typescript unresolvable → the
TCG-PROTOCOL §4 recipes passed through.

## 5. Consent: the no-prompt rule {#consent}

`req r5`

Building a slot binary executes package build scripts (PROP-025 §3).
An MCP server must NEVER prompt. So: `org.vibevm`-group slots build
on demand silently (the standing allow-list); any OTHER group is
refused with the exact recipe (`vibe bin build <name> --assume-yes`)
for the human to run in a terminal, where consent can actually be
given. The registry records nothing PROP-025 does not already record.

## 6. Non-goals {#non-goals}

`req r6`

No LSP relay (rename/code-actions/references are out; the surface is
the four queries + lifecycle, full stop). No reasoning ops, no
PROP-018 relay involvement. No per-call child spawn (that is what
`vibe bin exec` is for). No language autodetection — the agent says
what it is editing. Token-level TCG is the package brief's
very-far-future sibling and touches this PROP only as a future
consumer of the same oracle.

## 7. Acceptance {#acceptance}

`req r7`

- `tools/list` on a project with the TS stack installed carries the
  four tools; `tcg_validate` on a demo file returns diagnostics +
  `conform_findings` + `advice` in `structuredContent`.
- The same call on a project WITHOUT the stack returns the
  not-installed recipe naming THAT language's requires line;
  an unsupported `language` (e.g. `"go"`) returns the supported-set
  error listing both shipped languages.
- A killed oracle child is respawned once, transparently; a second
  failure surfaces `oracle-crashed` with its recipe; no node process
  survives the server.
- `vibe-tcg` compiles with no vibe-mcp dependency (the portability
  amendment, mechanically checkable in its Cargo.toml).
- vibe-cli's `bin` commands and the registry resolve binaries through
  the same `vibe-workspace` cell (one implementation, two consumers).
