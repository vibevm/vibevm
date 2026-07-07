# MCP-SOVEREIGNTY-PLAN v0.1 — standalone discipline MCP servers, package-declared MCP delivery, and the vibevm demontage

_Status: **DRAFT — awaiting owner review (2026-07-07).** Written against
tree `5185bda` (the discipline-core mini-fix close: floor green,
self-check 13 steps exit 0, conform 0 with 11 gated, specmap
592/578/590 0/0, corpus 9/9, both live chains green, mirrors synced).
Authored on the owner's commission of the same day; the four owner
answers that shape the scope are quoted verbatim in §0. Cold-executable:
every wave is a safe stop; the floor must be green at every phase
boundary. Registry publish stays HELD for the owner's word throughout._

Mandate (owner, 2026-07-07, four resolutions on the architecture
discussion):

1. «Все команды должны быть доступны как MCP tools (не только четвёрка
   tcg)» — the per-language MCP server exposes the WHOLE discipline
   toolchain, not only the type oracle.
2. Abstraction-level discipline: «если tcg и mcp это разные сущности …
   не обязательно их лепить в один файл. Но если это одна сущность —
   склей воедино» — entities stay separate cells; one entity never
   splits cosmetically.
3. «сделать возможность установки mcp серверов … пакеты с mcp серверами
   должны поддерживать все остальные применимые к ним фичи пакетов» —
   package-declared MCP servers become a first-class vibe feature with
   its own PROP, composing with every existing package feature.
4. vibe-tcg «по обстоятельствам»; every MCP-server function must ALSO
   be reachable as a plain command-line tool call («аналогично как и в
   vibevm»), name/home/composition of that CLI to be thought through.

Production-grade quality bar applies (standing owner directive,
`spec/boot/90-user.md`): no MVP framing, no stub subcommands as shipped
surface, full implementations.

## 0. Why this exists (one screen)

Today the discipline's agentic layer is delivered ONLY through
`vibe mcp serve`: vibevm's product MCP mounts the four `tcg_*` tools and
dispatches per-language binaries through the consumer's lockfile. That
was the right prototype topology — and it created two real problems the
owner has now named:

1. **An operational cycle.** vibevm itself is developed under AI-Native
   Rust; the discipline's agentic tools require vibevm's MCP at runtime;
   therefore vibevm-the-project depends on vibevm-the-binary in its own
   development loop, and every discipline consumer inherits that
   dependency («размотка компилятора» where none is needed).
2. **Lost package self-sufficiency.** The stacks already ship their
   engines, gates, CLIs, oracles, and skills — everything EXCEPT the MCP
   transport. A project that wants AI-Native Rust with the oracle must
   carry the full vibevm product in its runtime path, although vibe's
   actual job there is install-time wiring, not serving.

The fix is topological, and the code inventory says it is cheap on the
product side: after the Self-Sufficiency and Traceability-Relocation
campaigns, the ONLY vibevm-side discipline code left is `crates/vibe-tcg`
(~1 082 lines: OracleRegistry, lockfile→slot→artifact dispatch,
consent-gated builds, respawn-once, per-language recipe tables) and the
83-line `crates/vibe-mcp/src/tcg.rs` adapter. Almost all of `vibe-tcg`
exists ONLY because the MCP server lives outside the package slot and
must find/build/spawn a foreign binary. A stack-resident MCP server
links `tcg-oracle-bridge-rust` and `conform_cli_rust::build_rules`
directly — the dispatch machinery evaporates, and «the gate and the
oracle answer from one engine» becomes true by construction (one
workspace), not by protocol.

After this campaign: the stacks serve themselves (one MCP server binary
per language, full command surface, zero vibe in the runtime path);
vibe remains the installer/wirer (`vibe mcp install` learns
package-declared servers, with the PROP-025 consent posture); vibe-mcp
keeps only its four product tools; `vibe-tcg` is deleted.

## 1. Current-state facts, verified at authoring (do not re-discover)

Numbered so later phases can cite them; re-verify only what a phase
touches.

- **F1 — the vibevm-side coupling surface.** `crates/vibe-tcg` = 360
  (lib.rs: TcgHost seam, LANGUAGES, per-language recipe/dispatch
  tables) + 351 (registry.rs: OracleRegistry, ProcessLink, consent,
  respawn-once) + 371 (tests) lines. `crates/vibe-mcp/src/tcg.rs` = 83
  lines (the four `tcg_*` tool adapters). `vibe-mcp/src/skill_template.md`
  carries the tcg teaching section. Nothing else product-side serves the
  discipline at runtime (`vibe trace` is a delegating alias and stays).
- **F2 — vibe-mcp module map** (`crates/vibe-mcp/src/lib.rs:42-49`):
  `agent_config`, `agentic`, `agents`, `install`, `jsonrpc`, `pkgskill`,
  `tcg`, `tools`, `transport`. The MCP plumbing (`jsonrpc`, `transport`)
  also carries PROP-018 agentic/standalone-mode machinery — it is NOT a
  minimal neutral server loop.
- **F3 — agent-integration machinery exists and is multi-agent.**
  `vibe-mcp/src/agents.rs`: `enum Agent { ClaudeCode, ClaudeCodeDesktop,
  Cursor, OpenCode, Codex }`, `Scope { Project, User, Both }`, JSON and
  TOML config formats. `vibe mcp install` already writes vibevm's OWN
  server into these agents' configs (PROP-015; the 2026-06-22 fixes:
  Claude Code project scope = `.mcp.json`, user scope = top-level
  `mcpServers` of `~/.claude.json`, `preserve_order` merge, Windows
  spawn quirks).
- **F4 — the CLI surfaces that must become tools.**
  `discipline-rust` (rust stack): init, floor, conform (check|freeze),
  specmap, trace (explain), test-gate, tripwire, health, fast-loop,
  codemod (add-cell), ledger (render) — 11 subcommand families
  (`discipline-cli-rust/src/main.rs:31-116`). `tcg-rust`: serve,
  validate, scope, complete, type, bench — 6
  (`tcg-cli-rust/src/main.rs:31-83`). TS twins: `discipline-typescript`
  (10 families — no ledger), `tcg-typescript` (6).
- **F5 — the libraries already expose the entry points.** The CLIs are
  thin `clap` mains over lib fns (`conform_cli_rust::run_check/run_freeze`,
  `specmap_cli_rust::run_specmap/run_gate`, `discipline_cli_rust::*`
  per-module runners, the tcg one-shot ops in `tcg-cli-rust/src/lib.rs`).
  Tool handlers can delegate to the SAME fns — parity by construction.
  Known caveat: several runners print to stderr and return
  `Result<()>`; tool adapters need report text as a VALUE (F5a: an
  output-capture or writer-parameter seam will be needed — see D8).
- **F6 — binaries are a solved delivery.** PROP-025 `[[binary]]`
  (model: `vibe-core/src/manifest/package/binary.rs:38` `BinaryDecl`),
  slot-resident artifacts, consent-gated `vibe bin build/exec`
  (org.vibevm allow-listed, else `--assume-yes-or-refuse`), lockfile
  dispatch. Both stacks declare 4 binaries each today.
- **F7 — the vendor-sync mechanism scales.** `sync-engines.toml` names
  4 authored crates × 2 targets; `cargo xtask sync-engines --check` is
  self-check step 6. Adding a crate = one manifest line + a mirror run.
- **F8 — the four-kinds terminology rule.** `VIBEVM-SPEC.md` §4 /
  `spec/boot/00-core.md`: only `flow`, `feat`, `stack`, `tool` are
  installable kinds. PROP-024 (code-bearing) and PROP-025 (binaries)
  both added SURFACES, not kinds — the precedent this plan follows.
- **F9 — the wire contracts in force.** TCG-PROTOCOL-RUST-v0.1 /
  TCG-PROTOCOL-v0.1 (TS): the NDJSON serve-relay protocol + one-shot
  exit contract (validate exits 1 on error diagnostic OR non-baselined
  finding). PROP-026: the four-tool family, `language` as an enum value
  («a new language is an enum value, not new tools») — the topology
  half of that bet is what this campaign re-dispositions (D12).
- **F10 — proven acceptance instruments.** The differential corpus
  (9/9, cold 2 534 ms / warm p95 63 ms), both live chains
  (`vibe-mcp/tests/tcg_tools.rs --ignored`), demo floors (rust ALL
  green; ts 7/7), the single-crate walk, self-check 13 steps. The live
  chains currently drive THROUGH vibe-mcp — Wave 5 re-homes them.
- **F11 — next free PROP number: 027** (026 is the last; checked
  `spec/common` + `spec/modules/*`).
- **F12 — known machine/platform lessons that bind here.** node and
  cmd-shims refuse `\\?\`-verbatim paths (three homes of the lesson);
  Claude Code MCP discovery reads `.mcp.json` / `~/.claude.json`, NOT
  `settings.json`; bare non-exe commands need `cmd /c` wrap on Windows;
  `.mcp.json` merges must preserve key order. The servers this plan
  ships are real `.exe` artifacts — direct absolute (verbatim-free)
  paths avoid the shim class entirely.

## 2. Target end-state (what done looks like)

```
stack:org.vibevm/rust-ai-native (0.6.0)
├─ crates/vendor/{conform-core, specmap-core, specmark, specmark-grammar,
│                 mcp-core}                    ← NEW neutral cell, vendored
├─ crates/{conform,specmap,discipline,tcg}-cli-rust   (unchanged entities)
├─ crates/discipline-mcp-rust                  ← NEW: the stdio MCP server
│    src/main.rs        (clap: --path, serve-on-stdio)
│    src/server.rs      (mcp-core wiring: initialize, tools/list, tools/call)
│    src/tools_discipline.rs  (11 tools → discipline_cli_rust lib fns)
│    src/tools_tcg.rs         (5 tools → tcg oracle session + one-shots)
└─ vibe.toml: [[binary]] discipline-mcp-rust + [[mcp_server]] entry

stack:org.vibevm/typescript-ai-native (0.5.0)  — the same shape, 15 tools

flow:org.vibevm/discipline-core (0.6.0)
├─ crates/mcp-core                             ← AUTHORED here
└─ spec/mechanisms/MCP-CORE-v0.1.md            ← the transport contract

vibevm (product)
├─ crates/vibe-mcp      — product tools ONLY (query_package, agentic_explain,
│                         materialise_subskill, read_subskill); tcg.rs GONE
├─ crates/vibe-tcg      — DELETED
├─ vibe mcp install     — also registers package-declared [[mcp_server]]s
│                         into agent configs (managed block, consent-gated)
└─ .mcp.json            — mounts discipline-mcp-rust + discipline-mcp-typescript
                          (dogfood: the operational cycle demonstrably broken)
```

Runtime dependency directions after the campaign: stacks depend on
discipline-core (vendored, build-time only); vibevm depends on stacks
(as installed packages + registered servers); NOTHING depends on vibe at
serving time. A consumer without vibe can vendor a stack package and
`cargo build` its MCP server from the slot alone.

## 3. Decisions (D1–D15; ★ marks review points for §12)

### D1 — `[[mcp_server]]` is a manifest SURFACE, not a fifth kind ★

An MCP server is a property a package HAS, not what a package IS — the
same judgement PROP-024 made for code and PROP-025 for binaries. The
four-kinds rule (F8) stands. `[[mcp_server]]` references a `[[binary]]`
by name (the server IS a binary; delivery, consent, staleness, and slot
residence come from PROP-025 wholesale):

```toml
[[mcp_server]]
name = "discipline-rust"          # the agent-visible server name
binary = "discipline-mcp-rust"    # must match a [[binary]] in this manifest
description = "AI-Native Rust discipline + type oracle over MCP"
args = ["--path", "{project_root}"]   # substitution vars, small closed set
```

Offender checks at manifest parse: `binary` must resolve, names unique,
args substitution vars from the closed set only.

### D2 — one server binary per language, full command surface ★

Owner answer 1 fixes the surface: ALL commands. That makes the honest
name `discipline-mcp-rust` / `discipline-mcp-typescript` (D13 naming
policy: cross-language analogs carry the language suffix; the
`-mcp-` infix keeps it grep-distinct from the umbrella CLI). ONE server
per language mounting BOTH entities (discipline gates + tcg oracle) as
separate tool cells — operationally one process, structurally two cells
(owner answer 2: different entities, different files; one server,
because transport is not an entity boundary). Tool inventory:

- Rust (16): `init`, `floor`, `conform_check`, `conform_freeze`,
  `specmap_check`, `specmap_write`, `trace_explain`, `test_gate`,
  `tripwire`, `health`, `fast_loop`, `codemod_add_cell`,
  `ledger_render`, and `tcg_validate`, `tcg_scope`, `tcg_complete`,
  `tcg_type` (+ `tcg_bench` — see D9 for the count nuance).
- TypeScript (15): the same minus `ledger_render`.

Naming: snake_case tool ids; hosts namespace by server
(`mcp__discipline-rust__floor`), so identical ids across the two
language servers are not a collision. The four tcg tools KEEP their
`tcg_` prefix — continuity with every skill and transcript that already
teaches them.

### D3 — `mcp-core`: the neutral transport, authored in discipline-core

A minimal MCP stdio server cell: Content-Length framing (reuse the
framing grammar the rust bridge already implements — but authored
fresh and neutrally), `initialize` handshake, `tools/list`,
`tools/call`, error grammar, a `ToolSet` registry seam
(`name → schema + handler`). Authored in
`flow:org.vibevm/discipline-core/crates/mcp-core`, vendored into both
stacks by the EXISTING sync-engines mechanism (F7 — one manifest line).
No async runtime, no third-party protocol crates: a blocking stdio loop
exactly like vibe-mcp's, sized to what a discipline server needs.
Protocol revision: the same MCP revision vibe-mcp speaks today (proven
against the five Agent enum hosts). vibe-mcp is NOT rebased onto
mcp-core in this campaign (named deferral, §10) — its transport carries
PROP-018 mode machinery that must not be destabilised in a topology
campaign.

### D4 — tools are thin adapters over the SAME lib fns the CLIs call

Parity by construction (F5), pinned by test (§4 P2). The known gap F5a
(runners print to stderr, return `()`): each runner that a tool mounts
gains a report-capturing form — the house pattern is a
`&mut dyn io::Write` (or returned `Report` value) threaded through the
existing fn, with the CLI passing stderr and the tool passing a buffer.
This is a SEAM ADDITION to stack lib crates, not a behaviour change;
CLI output stays byte-identical (gated by the existing suites).

### D5 — the CLI story: the two entity-CLIs already ARE the parity
surface; vibe-tcg is deleted ★

Owner answer 4 asks that every MCP function be reachable as a plain CLI
call. Verified inventory (F4): it already is — `discipline-rust` covers
the 11 discipline families, `tcg-rust` the oracle ops. The two binaries
map to the two ENTITIES (gates vs oracle), which is exactly the
abstraction-level separation of owner answer 2 — so this plan ships NO
new CLI utility and RENAMES nothing; the deliverable is the pinned
parity MAP (tool id ↔ CLI invocation, one table in the brief + one
enumeration test per stack) and the F5a report seams. `vibe-tcg` is
deleted whole («по обстоятельствам» resolved: with in-slot serving
there is no cross-package dispatch left for it to do); the shared
`vibe_workspace::bins` cell stays — `vibe bin exec` needs it regardless.

### D6 — registration: direct artifact path in a vibevm-managed block

`vibe mcp install` (extended, Wave 4) writes package servers into agent
configs the same way it writes vibevm's own server today (F3), with:

- command = the ABSOLUTE, verbatim-free path to the slot-resident
  artifact (a real `.exe` — no cmd-shim class, F12), args from the
  manifest with substitutions resolved;
- a `vibevm-managed` marker per entry (JSON: a `"_vibevm"` sidecar key;
  TOML: a comment fence) so re-installs rewrite ONLY managed entries
  and operator-owned servers are never touched — the CLAUDE.md
  `<vibevm>` block convention, applied to agent configs;
- staleness co-managed with the slot: `vibe install` re-materialising a
  slot re-resolves registered paths (the artifact path embeds the
  version dir); `vibe mcp install` re-run is the documented refresh;
  `vibe mcp uninstall` / package uninstall removes managed entries;
- `vibe mcp status` reports managed entries whose artifact is missing
  or stale (not yet built → the recipe says `vibe bin build <name>`).

Building the artifact stays consent-gated by PROP-025 (org.vibevm
allow-listed; third-party requires `--assume-yes-or-refuse`);
REGISTERING a server is the same trust act as building (it schedules
code execution at agent start), so registration inherits exactly the
same consent gate and refusal recipe — one trust model, two verbs.

### D7 — lockfile and resolution: mirror PROP-025's model exactly

`[[mcp_server]]` declarations are read from slot manifests at
`vibe mcp install` time (the way `vibe bin list/exec` reads
`[[binary]]`); no new lockfile schema field unless `[[binary]]`
already records one — the executing phase verifies against
`vibe-workspace`'s actual model and follows it byte-for-byte. Offender
checks ride `vibe check`.

### D8 — long-running and destructive tools: run-to-completion, honest
reports, no hidden prompts

`floor`, `test_gate`, `fast_loop`, `tcg_bench` run minutes; MCP calls
run to completion and return the full step report (the same Class-F
REQ-grammar text the CLI prints) plus a structured
`{ok, steps?, findings?, …}` head. No interactive prompts exist in any
mounted runner (verified F4/F5 — `codemod add-cell` has rollback,
`init` never overwrites without `--force`); tools expose `force`-like
flags as explicit parameters so the no-prompts-in-server rule (the
PROP-026 posture) holds. Tool descriptions carry budget hints
(«runs the full verification floor; expect minutes»).

### D9 — tcg tool semantics move in-process; `serve` and the NDJSON
protocol remain CLI-side ★

Inside the server, tcg tools hold the SAME persistent oracle session
the `tcg-rust serve` relay holds today (bridge + enrichment linked
directly — no subprocess, no NDJSON hop; respawn-once and the
quiescence/deadline law port from the relay cell). The `tcg-rust serve`
NDJSON relay REMAINS SHIPPED as the non-MCP embedding form (F9's
protocol keeps its consumers: the one-shot exit contract and the bench
harness). `tcg_bench` is mounted as a tool (it is a command — owner
answer 1) with the heavy-budget description; the tool count is
therefore 17/16, with `tcg_bench` the one tool whose primary home
remains the CLI.
`language` parameter (PROP-026 continuity): each server ACCEPTS the
param, refuses a mismatch with its own language (grammar-compatible
with every existing skill/transcript), and treats absence as «this
server's language» — the enum-value bet re-reads as «a new language is
a new stack shipping the SAME tool grammar» (D12).

### D10 — version bumps at each wave's open (the standing ritual)

discipline-core 0.5.0→**0.6.0** (Wave 1, mcp-core lands),
rust-ai-native 0.5.0→**0.6.0** (Wave 2), typescript-ai-native
0.4.0→**0.5.0** (Wave 3). Each bump follows the mini-fix campaign's
exact move list (dir, workspace version, sync-engines.toml, self-check
paths where applicable, requires lines, re-materialise, external-specs
repoints in vibevm + demos). vibevm product stays 0.1.0-dev.

### D11 — Stage-B synergy, recorded not scoped

TCG-STAGE-B-DELIVERY-PLAN's «MCP-mounted arm» (backlogged) becomes
runnable for free after Wave 3: the with-tools arm mounts
`discipline-mcp-typescript` instead of prompt-naming a CLI. This plan
does NOT run the experiment; the pointer lands in the Stage-B plan's §1
re-verification notes during Wave 5.

### D12 — PROP-026 amendment: the grammar is the invariant, the
topology re-dispositions

The four-op grammar (validate/scope/complete/type + their params and
answer shapes) is unchanged — that is the bet that actually cashed. The
TOPOLOGY half («one multiplexed server, language as a parameter»)
re-dispositions to «one server per language stack, same grammar,
`language` param validated». PROP-026 gains a §(next) recording the
re-disposition, WHY (this plan's §0), and the transition map (which
tool ids moved where). TCG-PROTOCOL-RUST/TS gain a scope note: the
NDJSON protocol serves non-MCP embedders; MCP hosts speak MCP-CORE.

### D13 — naming, per the standing language-suffix policy

Crates `discipline-mcp-rust` / `discipline-mcp-typescript`; binaries
the same; server names (agent-visible) `discipline-rust` /
`discipline-typescript` — matching the umbrella CLI names agents
already know from the skills. mcp-core is language-neutral (no suffix,
like conform-core/specmap-core). ★ (the agent-visible name is taste —
flag for review.)

### D14 — trust posture, stated once

Registering + spawning a package MCP server executes package code at
agent-session start. The consent gate (D6) inherits PROP-025's model
verbatim; the scope-discipline rule (90-user.md) applies to the
REGISTRATION writes (only the target agent's config files, only managed
entries); server processes inherit the project root as cwd and receive
NO secrets from vibe. The refusal recipe names the exact command to
consent (`vibe mcp install --assume-yes …`).

### D15 — dogfood is the acceptance

Wave 5 registers both stack servers into vibevm's own `.mcp.json`
(managed block), the live chains re-home onto the stack servers, and
vibe-mcp's suite shrinks to product tools. The operational cycle is
broken when: an agent session on vibevm reaches every discipline tool
without `vibe mcp serve` running, and `vibe-mcp` builds with zero
references to tcg or the stacks.

## 4. Predictions (falsifiable; checked per wave and at close)

- **P1** — mcp-core's replay suite (scripted transport, no real agent)
  covers handshake, tools/list, tools/call, error grammar, and
  malformed-frame refusal; green without node/r-a anywhere near it.
- **P2** — parity: for each stack, an enumeration test pins
  tools/list == the declared inventory (D2) AND each tool's answer on a
  fixed fixture equals the corresponding CLI invocation's report
  (existence-grain: same findings/fingerprints/exit class).
- **P3** — live chain per server on the demos: `initialize` →
  `tools/list` (16/15) → `tcg_validate` clean 0/0 → seeded E0308 (rust)
  / TS2322 (ts) through a pure overlay → `floor` tool green. Disk
  byte-identical after.
- **P4** — the vibe-absent invariant: each server passes its live chain
  with `vibe` scrubbed from PATH and no vibevm process running.
- **P5** — the corpus stays 9/9 (bench CLI form), and `tcg_bench`
  through the server reports the same agreement on the same corpus.
- **P6** — after Wave 5, vibe-mcp's four product tools pass their suite
  unchanged; `grep -r "tcg" crates/vibe-mcp/src` is empty;
  `crates/vibe-tcg` does not exist; root workspace builds green.
- **P7** — the fresh-consumer walk: `vibe install` + `vibe mcp install
  --agent claude-code` on rust-demo yields a `.mcp.json` whose managed
  entry launches the slot artifact directly; a scripted MCP handshake
  over that exact command line lists 16 tools.
- **P8** — no behaviour change outside the campaign's surfaces: vibevm's
  conform/specmap counts move only by the new crates' own gated/tagged
  items; demo baselines unchanged except where a wave explicitly
  re-teaches them.

## 5. Wave 0 — spikes (no commits; gates for everything after)

- **S1 — bare-binary MCP handshake.** A 50-line throwaway binary using
  the planned mcp-core loop shape answers a scripted
  initialize/tools-list exchange AND a real `claude mcp`-style stdio
  probe on this box. Proves the protocol revision + framing choices
  before mcp-core is authored. (Claude Code is the probe host; the
  other four agents' configs are write-only surfaces here — their
  formats are already exercised by `vibe mcp install` today, F3.)
- **S2 — long-call behaviour.** The spike binary exposes a `sleep`-like
  tool (~90 s) and we observe the host's patience/timeout behaviour —
  calibrates D8's budget notes (NOT a design gate: run-to-completion
  stands regardless; this measures what to write in descriptions).
- **S3 — report-capture seam shape.** Pick ONE runner
  (`conform_cli_rust::run_check`) and thread the write seam (F5a) in a
  scratch branch; confirm CLI byte-identity and buffer capture. This
  fixes the idiom the Wave-2 sweep applies everywhere.
- Exit gate: S1–S3 findings recorded in this plan's §11 ledger; no
  tree changes survive the spike.

## 6. Wave 1 — mcp-core in discipline-core (0.6.0)

1. **Bump** discipline-core 0.5.0→0.6.0 (D10 move list;
   `build(packages)` + `build(deps)` pair).
2. **Author `crates/mcp-core`** (cells: `frame` — Content-Length IO;
   `wire` — request/response/error types + grammar; `server` — the
   blocking loop, initialize, tools/list, tools/call dispatch;
   `toolset` — the registry seam). Replay tests per P1; doctests on
   every pub seam; scope! tags; conform/specmap self-gates of the
   package extended to the new crate.
3. **Mechanism spec** `spec/mechanisms/MCP-CORE-v0.1.md` (REQ-grain
   units: framing, handshake, tool grammar, error grammar, the
   no-prompts rule) — the `spec://discipline-core/mechanisms/MCP-CORE-v0.1#…`
   units the code cites.
4. **Vendor**: `sync-engines.toml` crates += `mcp-core`; mirror; both
   stacks' vendor trees gain the crate (their workspaces list it as a
   member only when Wave 2/3 consume it — verify cargo tolerates an
   unreferenced vendored dir; if not, membership lands with this wave
   behind a no-op).
5. Gates: package fmt/test/clippy, sync-engines --check, full
   self-check, WAL.
   Commit shape: `build(packages)`, `feat(discipline-core): mcp-core …`,
   `docs(spec): MCP-CORE-v0.1 …`, `build(deps)`.

## 7. Wave 2 — `discipline-mcp-rust` (rust-ai-native 0.6.0)

1. **Bump** rust stack 0.5.0→0.6.0 (D10).
2. **Report seams (F5a)** across the mounted runners in
   `conform-cli-rust`, `specmap-cli-rust`, `discipline-cli-rust`,
   `tcg-cli-rust` — the S3 idiom, CLI output byte-identical (suites
   green unchanged).
3. **New crate `discipline-mcp-rust`** per §2's cell map: `server.rs`
   wires mcp-core's ToolSet; `tools_discipline.rs` (11 tools → lib
   fns); `tools_tcg.rs` (5 tools → an in-process oracle session cell
   ported from the relay: same quiescence law, respawn-once, enrichment
   through `build_rules`; the NDJSON relay in `tcg-cli-rust` is
   untouched). `[[binary]]` declared; `[[mcp_server]]` entry AUTHORED
   in vibe.toml now but consumed only from Wave 4 (dead manifest data
   is refused by `vibe check` → gate the entry behind Wave 4's schema
   landing — the executing session orders these two waves' manifest
   edits accordingly; if Wave 4 executes later, the entry lands there
   instead. Named ordering hazard, not a design hole).
4. **Tests**: P2 parity enumeration + fixture parity; the finding-parity
   posture (relay-vs-gate) extends to server-vs-gate
   fingerprint-for-fingerprint; live chain P3 on rust-demo driven over
   real stdio against the built artifact; P4 vibe-absent run; P5 bench
   tool answer == bench CLI on the committed corpus.
5. **Brief**: `spec/rust/tools/discipline-mcp-rust.md` (seven-section
   house shape) incl. the parity MAP table (D5).
6. Gates: stack fmt/test/clippy, self-check, demo floor, corpus 9/9,
   WAL. Commit shape: `build(packages)`, `refactor(rust-ai-native):
   report seams …`, `feat(rust-ai-native): discipline-mcp-rust …`,
   `docs(rust-ai-native): brief …`, `build(deps)`.

## 8. Wave 3 — `discipline-mcp-typescript` (typescript-ai-native 0.5.0)

Mirror of Wave 2 phase-for-phase (15 tools; the oracle session ports
from `tcg-cli-typescript`'s relay; node-dependent tools keep the
hard-fail-with-recipe posture on absent toolchain). Explicit
asymmetries to state in the brief: no ledger tool; the TS oracle IS the
compiler (no approximation caveat); `ts-demo` is the live-chain bed.
The TS package gains a self-check presence question — see §10
deferral D-e (the mini-fix campaign's standing finding).

## 9. Wave 4 — PROP-027: package-declared MCP servers in vibe

1. **PROP-027** (`spec/modules/vibe-mcp/PROP-027-package-mcp-servers.md`):
   the `[[mcp_server]]` schema (D1), consent (D6/D14), lifecycle
   (install/refresh/uninstall/status), the managed-block convention,
   agent matrix (the five Agent hosts × Project/User scopes),
   composition clauses with PROP-020 hooks, PROP-022 materialization
   modes, PROP-023 bridges, PROP-024 code-bearing, PROP-025 binaries
   («all other applicable package features» — owner answer 3 — becomes
   a NORMATIVE composition table, each row with its test).
2. **vibe-core**: `McpServerDecl` next to `BinaryDecl` (offender checks
   per D1); **vibe-workspace/vibe-mcp**: discovery from slot manifests,
   registration writes through the EXISTING agents.rs machinery with
   the managed-block extension; `vibe mcp status` staleness reporting;
   uninstall paths.
3. **Tests**: schema offender tests; a hermetic fixture package with a
   stub server binary exercising install/refresh/uninstall/status on
   every agent-format writer; the REAL walk P7 on rust-demo; the
   composition table's rows (one test each; reuse existing fixtures
   where a feature pair is already proven).
4. **Docs**: PROP-015 cross-pointer (the family it extends), README/
   quick-start lines.
5. Gates: full panel + WAL. Commit shape: `docs(spec): PROP-027 …`,
   `feat(core): [[mcp_server]] manifest surface`, `feat(mcp): install
   package-declared servers …`, `test(mcp): …`, `build(deps)` if slots
   move.

## 10. Wave 5 — the demontage, the re-teach, the dogfood

1. **vibe-mcp**: delete `tcg.rs`, the `vibe-tcg` dep, the skill_template
   tcg section (replaced by a pointer to the stack servers' briefs);
   suite shrinks; product tools untouched (P6).
2. **Delete `crates/vibe-tcg`** (root members/deps, conform.toml
   de-gate, specmap de-list — index shrinks by its items; the WAL
   records the count move).
3. **Live chains re-home**: `tcg_tools.rs --ignored` tests move to the
   stacks' own integration suites (they already exercise stack code);
   vibe-mcp keeps a product-tools live smoke only.
4. **Specs**: PROP-026 amendment (D12); TCG-PROTOCOL-RUST/TS scope
   notes; GUIDE-AI-NATIVE-RUST §12/§13 + TS §14/§15 re-teach (server
   names, parity map, `vibe mcp install` flow); boot snippets' toolchain
   blocks; the two sweep/terraform skill pairs' generation-time
   sections; ROADMAP milestone (M1.26 candidate: «MCP sovereignty»).
5. **Dogfood (D15/P6)**: vibevm's `.mcp.json` mounts both stack servers
   via `vibe mcp install`; rust-demo/ts-demo get managed entries too
   (their READMEs teach it); Stage-B plan gets the D11 pointer.
6. Full close panel: self-check, both demo floors, corpus, re-homed
   live chains, the fresh-consumer walk, mirrors.

Named deferrals (visible, not silent):
- **D-a** vibe-mcp rebasing onto mcp-core (one MCP implementation
  ecosystem-wide) — after the topology settles.
- **D-b** stable artifact shims (PROP-025 v2) — would make `.mcp.json`
  entries version-stable; today re-install rewrites the managed block.
- **D-c** registry publish of 0.6.0/0.6.0/0.5.0 — owner call, as ever.
- **D-d** the Stage-B MCP-mounted arm (D11) — separate commission.
- **D-e** a TS-package step in self-check + colon-free fact-store slot
  names — the mini-fix campaign's two hygiene candidates, still open.
- **D-f** MCP progress notifications for long tools — v1 is
  run-to-completion (D8).

## 11. Execution ledger (filled by the executing session)

_Empty at authoring. Spike findings (S1–S3), per-wave commit maps, and
prediction outcomes land here._

## 12. Review points — the owner's court (unresolved at authoring)

1. **D2/D13 naming**: binaries `discipline-mcp-rust`/`-typescript`,
   agent-visible server names `discipline-rust`/`discipline-typescript`
   — approve or rename.
2. **D2 topology**: ONE server per language mounting both entities
   (recommended), vs two servers (gates / oracle) per language.
3. **D1**: `[[mcp_server]]` as a surface referencing `[[binary]]`, NOT
   a fifth package kind — confirm the four-kinds reading.
4. **D9**: `language` param kept grammar-compatible (validated, not
   multiplexing) — confirm; and `tcg_bench` mounted as a tool despite
   its weight — confirm or CLI-only.
5. **D5**: no new CLI utility (the two entity-CLIs are the parity
   surface); `vibe-tcg` deleted with NO deprecation delegate
   (pre-publish, no external consumers) — confirm.
6. **D6 agent matrix v1**: all five Agent hosts × both scopes
   (recommended — the machinery exists), or Claude Code first.
7. **Wave order**: servers before the install feature (Waves 2–3 before
   4) — engineering order; the owner listed installation first, so
   explicit sign-off requested.
8. **Bench/latency**: no new latency targets are set for MCP-served
   tools (the oracle budgets stay TCG-ORACLE's); a regression against
   the corpus baseline reports, never cancels (§17.7 precedent) —
   confirm.
