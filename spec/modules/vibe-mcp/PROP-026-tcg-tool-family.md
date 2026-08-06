# PROP-026 — the tcg tool family (the agentic type oracle's product seam) {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: proposed and implemented 2026-07-07 (TCG-TS plan phase 4); per MCP-SOVEREIGNTY wave 6 superseded in topology, grammar kept normative; fact grain 2026-07-24"/>

@fact:status-line **Status: proposed 2026-07-07 with AGENTIC-TCG-TS-PLAN v0.1 (owner-
accepted the same day, with the §3 portability amendment); implemented
by its Phase 4. History: 2026-07-07 (same day, AGENTIC-TCG-RUST-PLAN
v0.1) — the §2 promise cashed: `language: "rust"` lands as an enum
value dispatching to the rust stack's `rust-ai-native-tcg` relay; no new tools,
no schema shape change. **Superseded in topology** (MCP-SOVEREIGNTY wave 6):
the standalone `vibe-tcg` crate was deleted whole and the tool grammar below
stays **normative**, now served by the per-family MCP servers of
[PROP-027](PROP-027-mcp-packages.md).** Module: `vibe-mcp`
(adapter) + `vibe-workspace` (binary resolution); the `vibe-tcg` crate this
contract was written against no longer exists. @status:impl/done

@fact:related **Related:** [PROP-015](PROP-015-mcp-integration.md) (the MCP server
this family is first mounted on), [PROP-018](../../common/PROP-018-agentic-standalone-modes.md)
(these tools are ALGORITHMIC — the `query_package` path; no affinity, no
relay, no `Intent`), [PROP-025](../vibe-workspace/PROP-025-binary-delivery.md)
(the slot-dispatch model the registry reuses), and the package-side
mechanism specs `TCG-ORACLE-v0.1` / `TCG-PROTOCOL-v0.1` in
`stack:org.vibevm.ai-native/typescript-ai-native-lang` plus `TCG-ORACLE-RUST-v0.1` /
`TCG-PROTOCOL-RUST-v0.1` in `stack:org.vibevm.ai-native/rust-ai-native-lang` (the
oracles ship THERE; vibevm ships only this thin product seam). @status:spec/done

- @fact:SUPERSEDED-TOPOLOGY **SUPERSEDED IN TOPOLOGY, 2026-07-07 (MCP-SOVEREIGNTY-PLAN v0.1, the
  owner's `mcp`-kind resolution):** the tool GRAMMAR this PROP defines —
  the four ops, their params, the answer shapes, the no-prompt rule —
  is unchanged and remains normative; the TOPOLOGY half (one multiplexed
  product server, `language` as the dispatch parameter, the `vibe-tcg`
  registry crate) is retired. @status:impl/done
- @fact:TOOLS-NEW-HOME The tools now ship in the per-language
  `mcp`-kind packages ([PROP-027](PROP-027-mcp-packages.md):
  `mcp:org.vibevm.ai-native/rust-ai-native-mcp`, `…/typescript-ai-native-mcp`), each
  serving its own language over the vendored `mcp-core` transport. @status:impl/done
- @fact:ENUM-BET-REREAD The §2 enum-value bet re-reads as «a new language is a new mcp package
  shipping the SAME tool grammar». @status:impl/done
- @fact:LANGUAGE-COMPAT-PARAM `language` survives as a validated
  compatibility parameter (a mismatch refuses with the recipe naming the
  right server). @status:impl/done
- @fact:TCG-CRATE-DELETED `vibe-tcg` and the vibe-mcp `tcg_*` adapters are
  DELETED. @status:impl/done
- @fact:RETIRED-SECTIONS-KEPT §3–§5 below describe the retired topology and stay as the
  design record. @status:spec/done

## 1. Problem {#problem}

@fact:req-problem `req r1` @status:impl/done

- @fact:oracle-degraded-path The typescript-ai-native stack ships a type oracle (`typescript-ai-native-tcg
  serve`) that answers validate/scope/complete/type queries over in-memory
  overlays at millisecond latency. An agent can already reach it the
  degraded way (`vibe bin exec typescript-ai-native-tcg -- <op> …`, one cold spawn
  per question). @status:impl/done
- @fact:warm-path-missing What is missing is the WARM path: a persistent oracle
  per language behind the agent's MCP session, so consultation costs
  milliseconds, not process startups. @status:impl/done
- @fact:PRODUCT-SURFACE-BOUNDARY That is product surface — lockfile
  resolution, slot dispatch, child lifecycle, consent — and it must not
  leak into either the package (which knows nothing of MCP) or vibe-mcp's
  core (which stays a generic JSON-RPC tool host). @status:impl/done

## 2. The four tools {#tools}

@fact:req-tools `req r3` @status:impl/done

@fact:FOUR-TOOLS `tcg_validate`, `tcg_scope`, `tcg_complete`, `tcg_type` — thin schema
adapters over the TCG-PROTOCOL ops of the same names, each with: @status:impl/done

- @fact:PARAM-LANGUAGE `language` (required; accepts `"typescript"` and — since the Rust
  twin landed, exactly as this clause promised — `"rust"`; any other
  value is a ToolError NAMING the supported set, so the next language
  adds an enum value, not new tools); @status:impl/done
- @fact:PARAMS-PASSTHROUGH the op's own params per TCG-PROTOCOL §2 (`file`, `position`,
  `content`, …), passed through verbatim. @status:impl/done

- @fact:ENRICHED-RESPONSES Responses return the ENRICHED protocol result (TCG-PROTOCOL §3) as
  `structuredContent`, plus a compact human text rendering (findings
  first). @status:impl/done
- @fact:NO-AFFINITY These tools are deterministic queries — per PROP-018 §2.3 they
  carry NO affinity machinery and never park intents. @status:impl/done

## 3. The portable family crate — the owner amendment {#portability}

@fact:req-portability `req r3` @status:spec/done

@fact:tcg-crate-home The family lives in a dedicated product crate **`vibe-tcg`**, NOT
inside vibe-mcp: @status:spec/done

- @fact:TCG-CRATE-CONTENTS `vibe-tcg` defines the tool descriptors/JSON schemas, the run logic,
  the `OracleRegistry` (§4), and a NARROW host abstraction —
  `trait TcgHost` exposing the project root and the no-prompt consent
  policy (§5). Dependencies: `vibe-core`, `vibe-workspace`, serde.
  **Zero vibe-mcp imports, by construction.** @status:spec/done
- @fact:MCP-ADAPTER-CELL vibe-mcp mounts it through one thin adapter cell: newtype wrappers
  implementing `McpTool` by delegation, mapping `vibe-tcg`'s typed
  errors into `ToolError`. The adapter is the ONLY place the two crates
  meet. @status:spec/done
- @fact:EXTRACTION-CONSEQUENCE Consequence (the amendment's point): extracting a STANDALONE tcg MCP
  server later is one new binary crate — a JSON-RPC loop (vibe-mcp's
  `Server<T: Transport>` is already transport-generic) mounting the
  same `vibe-tcg` tools — with zero changes inside the family. That
  extraction is a named follow-up, not a redesign. @status:spec/done

## 4. The oracle registry and child lifecycle {#registry}

@fact:req-registry `req r5` @status:spec/done

@fact:ORACLE-REGISTRY `OracleRegistry`: interior-mutable (the MCP tool seam hands out shared
refs), lazily populated per language on first use, dropped with the
host session: @status:spec/done

1. @fact:REG-RESOLVE resolve the CURRENT project's lockfile → the stack slot that
   declares the language's oracle binary (`typescript-ai-native-tcg` for
   TypeScript, `rust-ai-native-tcg` for Rust; the per-language table also
   carries the requires-line and one-shot recipes so every refusal
   names ITS language's fix surface, not another's) — the PROP-025
   `[[binary]]` walk, through the SHARED `vibe-workspace`
   binary-resolution cell (extracted from vibe-cli by the TS campaign
   so CLI and registry cannot drift); @status:spec/done
2. @fact:REG-SPAWN artifact present → spawn `<artifact> serve --root <project>` with
   piped stdio and hold the handle across calls; artifact absent →
   build per §5; @status:spec/done
3. @fact:REG-RESPAWN a dead child (`oracle-crashed`) → ONE transparent respawn attempt,
   then a recipe-carrying ToolError; @status:spec/done
4. @fact:REG-KILL-ON-DROP registry drop kills every child (kill-on-drop; the no-zombie
   property is test-asserted). @status:spec/done

@fact:FAILURES-ARE-RECIPES Failure surfaces are recipes, not dead ends: stack not installed → the
`[requires]` line + `vibe install`; language unsupported → the
supported set; node missing / typescript unresolvable → the
TCG-PROTOCOL §4 recipes passed through. @status:spec/done

## 5. Consent: the no-prompt rule {#consent}

@fact:req-consent `req r5` @status:impl/done

- @fact:NO-PROMPT-RULE Building a slot binary executes package build scripts (PROP-025 §3).
  An MCP server must NEVER prompt. @status:impl/done
- @fact:CONSENT-SPLIT So: `org.vibevm`-group slots build
  on demand silently (the standing allow-list); any OTHER group is
  refused with the exact recipe (`vibe bin build <name> --assume-yes`)
  for the human to run in a terminal, where consent can actually be
  given. @status:impl/done
- @fact:NO-NEW-RECORDS The registry records nothing PROP-025 does not already record. @status:spec/done

## 6. Non-goals {#non-goals}

@fact:req-non-goals `req r6` @status:spec/done

- @fact:NG-NO-LSP No LSP relay (rename/code-actions/references are out; the surface is
  the four queries + lifecycle, full stop). @status:spec/done
- @fact:NG-NO-REASONING No reasoning ops, no PROP-018 relay involvement. @status:spec/done
- @fact:NG-NO-PER-CALL-SPAWN No per-call child spawn (that is what `vibe bin exec` is for). @status:spec/done
- @fact:NG-NO-AUTODETECT No language autodetection — the agent says what it is editing. @status:spec/done
- @fact:NG-TOKEN-TCG-FUTURE Token-level TCG is the package brief's
  very-far-future sibling and touches this PROP only as a future
  consumer of the same oracle. @status:spec/done

## 7. Acceptance {#acceptance}

@fact:req-acceptance `req r7` @status:spec/done

- @fact:ACC-TOOLS-LIST `tools/list` on a project with the TS stack installed carries the
  four tools; `tcg_validate` on a demo file returns diagnostics +
  `conform_findings` + `advice` in `structuredContent`. @status:spec/done
- @fact:ACC-RECIPES The same call on a project WITHOUT the stack returns the
  not-installed recipe naming THAT language's requires line;
  an unsupported `language` (e.g. `"go"`) returns the supported-set
  error listing both shipped languages. @status:spec/done
- @fact:ACC-RESPAWN A killed oracle child is respawned once, transparently; a second
  failure surfaces `oracle-crashed` with its recipe; no node process
  survives the server. @status:spec/done
- @fact:ACC-NO-MCP-DEP `vibe-tcg` compiles with no vibe-mcp dependency (the portability
  amendment, mechanically checkable in its Cargo.toml). @status:spec/done
- @fact:ACC-SHARED-CELL vibe-cli's `bin` commands and the registry resolve binaries through
  the same `vibe-workspace` cell (one implementation, two consumers). @status:spec/done
