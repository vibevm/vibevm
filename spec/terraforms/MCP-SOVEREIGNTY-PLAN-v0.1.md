# MCP-SOVEREIGNTY-PLAN v0.1 — the `mcp` package kind, standalone discipline MCP servers, package-declared MCP delivery, and the vibevm demontage

_Status: **ACCEPTED WITH OWNER AMENDMENT — EXECUTION COMMISSIONED
2026-07-07 («выполни план до конца»).** The owner reviewed the draft and
REVERSED its D1: the four-kinds set is a terminology snapshot, not an
architectural freeze — «расширь и сделай mcp чем-то отдельным (в
дальнейшем возможно появится еще один kind — app, для запускаемых
графических приложений)». This revision makes `mcp` a first-class
package KIND, ships the discipline servers as SEPARATE `mcp:`-kind
packages (exact-pinned to their stacks — the skew analysis in D1a is
what the pin answers), and folds the executor's recorded
recommendations for the remaining §12 points (resolutions ledger in
§12). The VIBEVM-SPEC §4 amendment this requires is owner-sanctioned by
the same directive (the 00-core.md escape hatch: «edits require the
user»). Originally DRAFT the same day, written against tree `5185bda`
(mini-fix close: floor green, corpus 9/9, mirrors synced).
Cold-executable: every wave is a safe stop; the floor must be green at
every phase boundary. Registry publish stays HELD for the owner's word
throughout._

Mandate (owner, 2026-07-07, four resolutions on the architecture
discussion + the kind amendment):

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
   vibevm»), name/home/composition of that CLI thought through.
5. (Amendment, post-draft) «расширь [kinds] и сделай mcp чем-то
   отдельным (в дальнейшем возможно появится еще один kind — app, для
   запускаемых графических приложений)» — the `mcp` KIND, with the
   taxonomy left open for `app`.

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
must find/build/spawn a foreign binary. A server built in the same
dependency closure as the gate links `tcg-oracle-bridge-rust` and
`conform_cli_rust::build_rules` directly — the dispatch machinery
evaporates, and «the gate and the oracle answer from one engine» holds
by construction of the build, not by protocol.

After this campaign: `mcp` is an installable package KIND; the
discipline servers ship as `mcp:org.vibevm/discipline-rust` /
`mcp:org.vibevm/discipline-typescript`, exact-pinned to their stacks,
serving the full command surface with zero vibe in the runtime path;
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
- **F7 — the vendor-sync mechanism scales, but is single-source today.**
  `sync-engines.toml` names ONE `source_root` (discipline-core) ×
  4 crates × 2 targets; `cargo xtask sync-engines --check` is
  self-check step 6. This campaign needs MULTI-SOURCE sync (stack →
  mcp-package projections) — a bounded tool extension (D3a).
- **F8 — the kind set and its mechanics.** `VIBEVM-SPEC.md` §4 defines
  `flow`, `feat`, `stack`, `tool`; `spec/boot/00-core.md` repeats it as
  terminology discipline. Kind mechanics in code: manifest `kind`
  parsing (vibe-core), slot naming `vibedeps/<kind>-<name>/<version>`,
  boot-snippet categories/ordering, `requires_kinds` compatibility,
  `vibe check` offender checks, registry naming (PROP-008 Fqdn).
  Extending the enum touches each of these seams — enumerable, all
  in-product. THE OWNER HAS SANCTIONED the §4 amendment (mandate 5);
  the amendment text must leave the taxonomy explicitly open («the set
  grows by owner amendment; `app` is anticipated») so the next kind
  does not repeat this archaeology.
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
  chains currently drive THROUGH vibe-mcp — Wave 6 re-homes them.
- **F11 — next free PROP number: 027** (026 is the last; checked
  `spec/common` + `spec/modules/*`).
- **F12 — known machine/platform lessons that bind here.** node and
  cmd-shims refuse `\\?\`-verbatim paths (three homes of the lesson);
  Claude Code MCP discovery reads `.mcp.json` / `~/.claude.json`, NOT
  `settings.json`; bare non-exe commands need `cmd /c` wrap on Windows;
  `.mcp.json` merges must preserve key order. The servers this plan
  ships are real `.exe` artifacts — direct absolute (verbatim-free)
  paths avoid the shim class entirely.
- **F13 — cross-slot Cargo path-deps are IMPOSSIBLE** (PROP-024 §2.4;
  proven twice: the vendor-sync decision at deferrals-closeout, the
  slot-layout disagreement). Any package whose crates need another
  package's crates VENDORS them. This fact is what shapes D1a — a
  standalone `mcp` package cannot path-dep into its stack's slot;
  manifest rewriting that could change this is PROP-025 v2,
  specified-only, NOT this campaign.

## 2. Target end-state (what done looks like)

```
flow:org.vibevm/discipline-core (0.6.0)
├─ crates/mcp-core                             ← AUTHORED here (neutral transport)
└─ spec/mechanisms/MCP-CORE-v0.1.md

stack:org.vibevm/rust-ai-native (0.6.0)        ← report seams + tcg session lib
stack:org.vibevm/typescript-ai-native (0.5.0)     surface; NO server crates here

mcp:org.vibevm/discipline-rust (0.6.0)         ← NEW PACKAGE, NEW KIND
├─ vibe.toml   kind = "mcp"
│              [requires] "stack:org.vibevm/rust-ai-native" = "=0.6.0"  (exact pin)
│              [[binary]] discipline-mcp-rust
│              [[mcp_server]] name="discipline-rust" binary="discipline-mcp-rust"
├─ crates/discipline-mcp-rust                  ← the stdio MCP server (authored)
│    src/main.rs / server.rs / tools_discipline.rs / tools_tcg.rs
├─ crates/vendor/…                             ← synced projections (D3a):
│    mcp-core, conform-core, specmap-core, specmark, specmark-grammar
│      (source: discipline-core), conform-frontend-rust, conform-cli-rust,
│    specmap-cli-rust, discipline-cli-rust, tcg-oracle-bridge-rust,
│    tcg-session-rust (source: rust-ai-native)
└─ specmap.toml (self-trace), README, spec/ (brief)

mcp:org.vibevm/discipline-typescript (0.5.0)   — the same shape, 15 tools

vibevm (product)
├─ vibe-core: Kind::Mcp (+ the seams of F8); McpServerDecl
├─ vibe mcp install — registers package-declared servers (managed block,
│                     consent-gated); status/uninstall lifecycle
├─ crates/vibe-mcp — product tools ONLY; tcg.rs GONE
├─ crates/vibe-tcg — DELETED
└─ .mcp.json — mounts discipline-rust + discipline-typescript (dogfood)
```

Runtime dependency directions after the campaign: mcp packages
exact-pin their stacks (resolver-enforced lockstep — the no-skew
property moves from «same workspace» to «same resolved version set»);
stacks depend on discipline-core (vendored, build-time only); vibevm
depends on packages (installed + registered servers); NOTHING depends
on vibe at serving time. A consumer without vibe can vendor an mcp
package and `cargo build` its server from the slot alone.

## 3. Decisions (D1–D15; §12 records the resolutions)

### D1 — `mcp` is a package KIND (owner amendment; reverses the draft)

The kind set grows: `flow`, `feat`, `stack`, `tool`, **`mcp`**. An
`mcp`-kind package's primary deliverable is one or more MCP servers; the
`[[mcp_server]]` declaration table is VALID ONLY in `mcp`-kind packages
(offender check — the taxonomy is enforced, not advisory). VIBEVM-SPEC
§4 is amended under the owner's sanction (mandate 5), with the
amendment text recording that the taxonomy grows by owner amendment and
naming `app` as anticipated. `spec/boot/00-core.md` is user-owned — the
owner updates its four-kinds line himself, or explicitly delegates the
edit (execution NOTE: ask at the Wave-1 boundary; do not edit
user-owned boot files silently).

### D1a — the discipline servers are SEPARATE `mcp:` packages,
exact-pinned to their stacks

Because cross-slot path-deps are impossible (F13), the server package
VENDORS its dependency closure (D3a) — and vendoring re-opens the
gate-vs-oracle version-skew the in-slot draft killed by construction.
The pin closes it: `[requires] "stack:org.vibevm/rust-ai-native" =
"=0.6.0"` (exact). Installing the mcp package forces the exact stack
version; the resolver — not a runtime handshake — guarantees «one
engine, one truth». Consequences accepted and priced: every stack
campaign now syncs + bumps its mcp sibling (mechanical, sync-engines
does the bytes); self-check grows two package gates; publish grows two
packages. The pin is a REQ in PROP-027 with an offender check: an
`mcp` package whose stack requirement is not exact is refused by
`vibe check`.

### D2 — one server binary per language, full command surface

Owner answer 1 fixes the surface: ALL commands. ONE server per language
mounting BOTH entities (discipline gates + tcg oracle) as separate tool
cells — operationally one process, structurally two cells (owner
answer 2: different entities, different files; one server, because
transport is not an entity boundary). Tool inventory:

- Rust (17): `init`, `floor`, `conform_check`, `conform_freeze`,
  `specmap_check`, `specmap_write`, `trace_explain`, `test_gate`,
  `tripwire`, `health`, `fast_loop`, `codemod_add_cell`,
  `ledger_render`, `tcg_validate`, `tcg_scope`, `tcg_complete`,
  `tcg_type`, plus `tcg_bench` (heavy-budget description) — see D9.
- TypeScript (16): the same minus `ledger_render`.

Naming: snake_case tool ids; hosts namespace by server
(`mcp__discipline-rust__floor`), so identical ids across the two
language servers are not a collision. The four tcg tools KEEP their
`tcg_` prefix — continuity with every skill and transcript that already
teaches them.

### D3 — `mcp-core`: the neutral transport, authored in discipline-core

A minimal MCP stdio server cell: Content-Length framing, `initialize`
handshake, `tools/list`, `tools/call`, error grammar, a `ToolSet`
registry seam (`name → schema + handler`). Authored in
`flow:org.vibevm/discipline-core/crates/mcp-core`, vendored wherever
needed by sync-engines. No async runtime, no third-party protocol
crates: a blocking stdio loop exactly like vibe-mcp's, sized to what a
discipline server needs. Protocol revision: the same MCP revision
vibe-mcp speaks today (proven against the five Agent enum hosts).
vibe-mcp is NOT rebased onto mcp-core in this campaign (named deferral,
§10).

### D3a — multi-source vendor sync

`sync-engines.toml` generalises from one `source_root` to `[[sync]]`
blocks (`source_root` × `crates` × `targets` each); the check stays one
command and one self-check step. Sync sets after this campaign:

1. discipline-core → {rust stack, ts stack, mcp-rust pkg, mcp-ts pkg}:
   the neutral five (conform-core, specmap-core, specmark,
   specmark-grammar, mcp-core).
2. rust-ai-native → {mcp-rust pkg}: conform-frontend-rust,
   conform-cli-rust, specmap-cli-rust, discipline-cli-rust,
   tcg-oracle-bridge-rust, tcg-session-rust.
3. typescript-ai-native → {mcp-ts pkg}: the TS analogues
   (conform-frontend-typescript, conform-cli-typescript,
   specmap-cli-typescript, discipline-cli-typescript, ts-extract-bridge,
   tcg-oracle-bridge, tcg-session-typescript — exact list verified at
   Wave 4 against the tool handlers' real dependency closure).

The fix surface is ALWAYS the authored copy; vendored trees are
write-throughs — unchanged law, more edges.

### D3b — the tcg session cell becomes a lib surface in each stack

The enriching persistent-session logic (oracle spawn, quiescence law,
respawn-once, enrichment through `build_rules`) lives inside
`tcg-cli-rust`/`tcg-cli-typescript` serve cells today. Wave 2/4 extract
it into a vendorable lib crate per stack (`tcg-session-rust`,
`tcg-session-typescript`) that BOTH the CLI relay and the MCP server
consume — one entity, one home (owner answer 2), two transports. The
NDJSON relay's behaviour is pinned by its existing tests and must not
move.

### D4 — tools are thin adapters over the SAME lib fns the CLIs call

Parity by construction (F5), pinned by test (§4 P2). The known gap F5a
(runners print to stderr, return `()`) is resolved by the S3 spike
finding (§13): a **process-level stderr capture guard** in mcp-core's
`capture` cell wraps each tool dispatch — child-process output (floor's
cargo/prettier/node) is captured too, which writer-threading could
never do. The stacks' lib and CLI signatures are NOT touched; CLI
output stays byte-identical because nothing CLI-side changes. The
guard is Drop-restoring (panic-safe) and legal because the server
dispatches tools sequentially.

### D5 — the CLI story: the two entity-CLIs already ARE the parity
surface; vibe-tcg is deleted

Owner answer 4 asks that every MCP function be reachable as a plain CLI
call. Verified inventory (F4): it already is — `discipline-rust` covers
the 11 discipline families, `tcg-rust` the oracle ops; the two binaries
map to the two ENTITIES (gates vs oracle) — exactly owner answer 2's
separation. This plan ships NO new CLI utility and RENAMES nothing; the
deliverable is the pinned parity MAP (tool id ↔ CLI invocation, one
table in each brief + one enumeration test per server) and the F5a
report seams. The CLIs stay in the STACKS (a stack without agents is
still fully operable); the servers live in the mcp packages and vendor
the same libs — parity holds across packages because the pin (D1a)
holds the versions together. `vibe-tcg` is deleted whole.

### D6 — registration: direct artifact path in a vibevm-managed block

`vibe mcp install` (extended, Wave 5) writes package servers into agent
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

### D7 — lockfile and resolution: mirror PROP-025's model; the kind
rides the existing schema

`[[mcp_server]]` declarations are read from slot manifests at
`vibe mcp install` time (the way `vibe bin list/exec` reads
`[[binary]]`); the `kind` field already travels manifest→lockfile→slot
naming, so `mcp` needs enum + offender work, not schema work. The
executing phase verifies against `vibe-core`/`vibe-workspace`'s actual
model and follows it byte-for-byte. Offender checks ride `vibe check`
(incl. D1's «[[mcp_server]] only in mcp-kind» and D1a's exact-pin
rule).

### D8 — long-running and destructive tools: run-to-completion, honest
reports, no hidden prompts

`floor`, `test_gate`, `fast_loop`, `tcg_bench` run minutes; MCP calls
run to completion and return the full step report (the same Class-F
REQ-grammar text the CLI prints) plus a structured
`{ok, steps?, findings?, …}` head. No interactive prompts exist in any
mounted runner (verified F4/F5 — `codemod add-cell` has rollback,
`init` never overwrites without `--force`); tools expose `force`-like
flags as explicit parameters so the no-prompts-in-server rule holds.
Tool descriptions carry budget hints («runs the full verification
floor; expect minutes»).

### D9 — tcg tool semantics in-process; `serve` and the NDJSON protocol
remain CLI-side

Inside the server, tcg tools hold the SAME persistent oracle session
the relay holds today, through the extracted session lib (D3b) — no
subprocess, no NDJSON hop. The `tcg-* serve` NDJSON relay REMAINS
SHIPPED as the non-MCP embedding form (F9's protocol keeps its
consumers). `tcg_bench` is mounted as a tool (it is a command — owner
answer 1) with the heavy-budget description. `language` parameter
(PROP-026 continuity): each server ACCEPTS the param, refuses a
mismatch with its own language, treats absence as «this server's
language» — the enum-value bet re-reads as «a new language is a new
mcp package shipping the SAME tool grammar» (D12).

### D10 — version bumps at each wave's open (the standing ritual)

discipline-core 0.5.0→**0.6.0** (Wave 2, mcp-core lands),
rust-ai-native 0.5.0→**0.6.0** (Wave 3, seams + session lib),
typescript-ai-native 0.4.0→**0.5.0** (Wave 4). The NEW mcp packages are
BORN at their stack's pinned version (0.6.0 / 0.5.0) — birth, not bump.
Each bump follows the mini-fix campaign's exact move list. vibevm
product stays 0.1.0-dev.

### D11 — Stage-B synergy, recorded not scoped

TCG-STAGE-B-DELIVERY-PLAN's «MCP-mounted arm» (backlogged) becomes
runnable for free after Wave 4: the with-tools arm mounts
`discipline-typescript` instead of prompt-naming a CLI. This plan does
NOT run the experiment; the pointer lands in the Stage-B plan's §1
re-verification notes during Wave 6.

### D12 — PROP-026 amendment: the grammar is the invariant, the
topology re-dispositions

The four-op grammar (validate/scope/complete/type + params + answer
shapes) is unchanged — the bet that actually cashed. The TOPOLOGY half
(«one multiplexed server, language as a parameter») re-dispositions to
«one `mcp:` package per language, same grammar, `language` param
validated». PROP-026 gains a §(next) recording the re-disposition, WHY
(§0), and the transition map. TCG-PROTOCOL-RUST/TS gain a scope note:
the NDJSON protocol serves non-MCP embedders; MCP hosts speak MCP-CORE.

### D13 — naming, per the standing language-suffix policy

Packages `mcp:org.vibevm/discipline-rust` /
`mcp:org.vibevm/discipline-typescript` (slots `vibedeps/mcp-discipline-rust/…`
— no kind/name stutter). Server crates + binaries `discipline-mcp-rust`
/ `discipline-mcp-typescript`; agent-visible server names
`discipline-rust` / `discipline-typescript` — matching the umbrella CLI
names agents already know. mcp-core and the session libs follow the
suffix policy (`tcg-session-rust` / `tcg-session-typescript`; mcp-core
is language-neutral, no suffix).

### D14 — trust posture, stated once

Registering + spawning a package MCP server executes package code at
agent-session start. The consent gate (D6) inherits PROP-025's model
verbatim; the scope-discipline rule (90-user.md) applies to the
REGISTRATION writes (only the target agent's config files, only managed
entries); server processes inherit the project root as cwd and receive
NO secrets from vibe. The refusal recipe names the exact command to
consent (`vibe mcp install --assume-yes …`).

### D15 — dogfood is the acceptance

Wave 6 registers both mcp packages' servers into vibevm's own
`.mcp.json` (managed block), the live chains re-home onto the mcp
packages' suites, and vibe-mcp's suite shrinks to product tools. The
operational cycle is broken when: an agent session on vibevm reaches
every discipline tool without `vibe mcp serve` running, and `vibe-mcp`
builds with zero references to tcg or the stacks.

## 4. Predictions (falsifiable; checked per wave and at close)

- **P1** — mcp-core's replay suite (scripted transport, no real agent)
  covers handshake, tools/list, tools/call, error grammar, and
  malformed-frame refusal; green without node/r-a anywhere near it.
- **P2** — parity: for each server, an enumeration test pins tools/list
  == the declared inventory (D2) AND each tool's answer on a fixed
  fixture equals the corresponding CLI invocation's report
  (existence-grain: same findings/fingerprints/exit class).
- **P3** — live chain per server on the demos: `initialize` →
  `tools/list` (17/16) → `tcg_validate` clean 0/0 → seeded E0308 (rust)
  / TS2322 (ts) through a pure overlay → `floor` tool green. Disk
  byte-identical after.
- **P4** — the vibe-absent invariant: each server passes its live chain
  with `vibe` scrubbed from PATH and no vibevm process running.
- **P5** — the corpus stays 9/9 (bench CLI form), and `tcg_bench`
  through the server reports the same agreement on the same corpus.
- **P6** — after Wave 6, vibe-mcp's four product tools pass their suite
  unchanged; `grep -r "tcg" crates/vibe-mcp/src` is empty;
  `crates/vibe-tcg` does not exist; root workspace builds green.
- **P7** — the fresh-consumer walk: `vibe install` (mcp package pulls
  its stack via the exact pin) + `vibe mcp install --agent claude-code`
  on rust-demo yields a `.mcp.json` whose managed entry launches the
  slot artifact directly; a scripted MCP handshake over that exact
  command line lists 17 tools.
- **P8** — the exact-pin law holds mechanically: `vibe check` refuses
  an mcp package with a non-exact stack requirement; resolving an mcp
  package installs the pinned stack version even when a newer stack
  exists in the registry (fixture-proven).
- **P9** — no behaviour change outside the campaign's surfaces: vibevm's
  conform/specmap counts move only by the new crates' own gated/tagged
  items; demo baselines unchanged except where a wave explicitly
  re-teaches them.

## 5. Wave 0 — spikes (no commits; gates for everything after)

- **S1 — bare-binary MCP handshake.** A throwaway binary using the
  planned mcp-core loop shape answers a scripted initialize/tools-list
  exchange AND a real Claude Code stdio probe on this box. Proves the
  protocol revision + framing choices before mcp-core is authored.
- **S2 — long-call behaviour.** The spike binary exposes a sleep-like
  tool (~90 s); observe the host's patience/timeout — calibrates D8's
  budget notes (run-to-completion stands regardless).
- **S3 — report-capture seam shape.** Thread the write seam (F5a)
  through ONE runner (`conform_cli_rust::run_check`) in a scratch
  branch; confirm CLI byte-identity and buffer capture. Fixes the idiom
  Wave 3 applies everywhere.
- **S4 — kind-mechanics inventory.** Enumerate every `kind` seam in
  vibe-core/vibe-workspace/vibe-check/vibe-mcp (F8's list verified
  against code, with file:line), so Wave 1 lands as a sweep, not a
  hunt. Also verify: the resolver honours `=X.Y.Z` exact requirements
  (a unit-level probe against resolvo's semver handling).
- Exit gate: S1–S4 findings recorded in §11; no tree changes survive
  the spikes.

## 6. Wave 1 — the `mcp` kind in the product (+ VIBEVM-SPEC §4)

1. **VIBEVM-SPEC §4 amendment** (owner-sanctioned, mandate 5): the
   `mcp` kind defined (deliverable = MCP servers; [[mcp_server]] valid
   only here; exact-pin law D1a), the taxonomy recorded as
   owner-extensible, `app` named as anticipated. Boot-snippet
   category/ordering for `mcp` defined (no boot snippet by default —
   agents learn servers via registration, not boot text; a
   `[boot_snippet]` remains legal).
2. **vibe-core**: `Kind::Mcp` through every S4-inventoried seam
   (parse, display, slot naming, requires_kinds, registry naming);
   `McpServerDecl` next to `BinaryDecl` (D1 offender checks: table only
   in mcp-kind, binary reference resolves, names unique, args from the
   closed substitution set; D1a exact-pin check).
3. **vibe-check**: the new offender diagnostics (REQ-citing, Class-F
   grammar).
4. **Fixtures**: a hermetic `mcp`-kind fixture package (stub server
   binary) in `fixtures/registry/` — install/resolve/check/slot-naming
   e2e; the P8 exact-pin fixture pair (mcp@X pinning stack@X with
   stack@Y present).
5. Gates: full panel + WAL. Commit shape: `docs(spec): VIBEVM-SPEC §4 —
   the mcp kind (owner-sanctioned)`, `feat(core): Kind::Mcp + [[mcp_server]]`,
   `test(install): mcp-kind fixtures`, `build(deps)` if slots move.
   NOTE (D1): ask the owner about the `00-core.md` four→five kinds line
   at this wave's close — user-owned file, not edited silently.

## 7. Wave 2 — mcp-core in discipline-core (0.6.0)

1. **Bump** discipline-core 0.5.0→0.6.0 (D10 move list).
2. **Author `crates/mcp-core`** (cells: `wire` — line-delimited
   JSON-RPC request/response/error types + grammar, protocol
   "2024-11-05" — the S1-proven shape; `server` — the blocking loop,
   initialize, tools/list, tools/call, ping; `toolset` — the registry
   seam; `capture` — the S3 process-level stderr guard, dup2-based,
   Windows CRT-fd aware, Drop-restoring). Replay tests per P1 incl.
   capture-guard tests (child-process output captured; nested guard
   refused; restore-on-panic); doctests on every pub seam; scope!
   tags; the package's conform/specmap self-gates extended to the new
   crate.
3. **Mechanism spec** `spec/mechanisms/MCP-CORE-v0.1.md` (REQ-grain
   units: framing, handshake, tool grammar, error grammar, the
   no-prompts rule).
4. **Vendor**: sync-engines gains the D3a multi-source shape (xtask
   change rides this wave); set 1 grows mcp-core; both stacks receive
   it (their workspaces reference it only when needed — verify cargo
   tolerates an unreferenced vendored dir, else membership waits for
   its consumer).
5. Gates: package fmt/test/clippy, sync-engines --check, full
   self-check, WAL. Commits: `build(packages)`, `feat(discipline-core):
   mcp-core …`, `docs(spec): MCP-CORE-v0.1`, `refactor(xtask):
   multi-source sync-engines`, `build(deps)`.

## 8. Wave 3 — rust: seams, session lib, and `mcp:org.vibevm/discipline-rust`

1. **Bump** rust-ai-native 0.5.0→0.6.0 (D10).
2. **Extract `tcg-session-rust`** (D3b) from the relay's session cell;
   the relay consumes it; relay tests unchanged. (The draft's «report
   seams» step is DELETED per the S3 spike finding — fd-capture in the
   server replaces it; no stack lib signatures move.)
4. **Birth `mcp:org.vibevm/discipline-rust` (0.6.0)**: package skeleton
   (vibe.toml with kind/pin/[[binary]]/[[mcp_server]], LICENSE, README,
   spec/ brief with the D5 parity map), `crates/discipline-mcp-rust`
   per §2's cell map, vendored closure per D3a set 2 (+ set 1), own
   specmap.toml self-trace, package Cargo workspace green.
5. **Tests**: P2 parity enumeration + fixture parity
   (server-vs-gate fingerprint-for-fingerprint); live chain P3 on
   rust-demo over real stdio against the built artifact; P4
   vibe-absent run; P5 bench-tool agreement.
6. **Wiring**: root vibe.toml requires the mcp package (vibevm is its
   first consumer); re-materialise; self-check grows the package gate
   (fmt/test/clippy + self-trace).
7. Gates: full panel + demo floor + corpus + WAL. Commits:
   `build(packages)` (bump), `refactor(rust-ai-native): report seams`,
   `refactor(rust-ai-native): tcg-session-rust`, `feat(packages):
   mcp:discipline-rust — the standalone server`, `test(...)`,
   `build(deps)`.

## 9. Wave 4 — typescript: the mirror (`mcp:org.vibevm/discipline-typescript`)

Mirror of Wave 3 phase-for-phase: ts stack 0.4.0→0.5.0,
`tcg-session-typescript` extraction (no report seams — S3), the mcp
package birth (0.5.0, exact-pinned), 16 tools, ts-demo live chain,
vendored closure per D3a set 3 (verified against the real dependency
closure at execution).
Explicit asymmetries in the brief: no ledger tool; the TS oracle IS the
compiler (no approximation caveat); node-dependent tools keep the
hard-fail-with-recipe posture. Self-check grows the ts-side package
gates it never had (closing the mini-fix campaign's D-e finding for the
new package at least; the ts STACK gate remains a §10 deferral item
unless trivially cheap here).

## 10. Wave 5 — PROP-027: MCP delivery through vibe

1. **PROP-027** (`spec/modules/vibe-mcp/PROP-027-mcp-packages.md`): the
   kind's normative spec — D1/D1a laws, the `[[mcp_server]]` schema,
   consent (D6/D14), lifecycle (install/refresh/uninstall/status), the
   managed-block convention, agent matrix (five hosts × two scopes),
   and the COMPOSITION TABLE with every package feature (PROP-020
   hooks, PROP-021 submodules, PROP-022 materialization modes, PROP-023
   bridges, PROP-024 code-bearing, PROP-025 binaries, PROP-015 §2.8
   skill-include) — owner answer 3 made normative, each row with its
   test.
2. **vibe-mcp/vibe-workspace**: discovery from slot manifests,
   registration through the EXISTING agents.rs machinery with the
   managed-block extension, `vibe mcp status` staleness, uninstall
   paths.
3. **Tests**: the fixture package (Wave 1) exercised through every
   agent-format writer; the REAL walk P7 on rust-demo; the composition
   table's rows.
4. Gates: full panel + WAL. Commits: `docs(spec): PROP-027 …`,
   `feat(mcp): install package-declared servers`, `test(mcp): …`.

## 11. Wave 6 — the demontage, the re-teach, the dogfood

1. **vibe-mcp**: delete `tcg.rs`, the `vibe-tcg` dep, the
   skill_template tcg section (replaced by a pointer to the mcp
   packages' briefs); product tools untouched (P6).
2. **Delete `crates/vibe-tcg`** (root members/deps, conform.toml
   de-gate, specmap de-list — the WAL records the count move).
3. **Live chains re-home** into the mcp packages' own suites; vibe-mcp
   keeps a product-tools smoke.
4. **Specs**: PROP-026 amendment (D12); TCG-PROTOCOL scope notes;
   GUIDE-AI-NATIVE-RUST §12/§13 + TS §14/§15 re-teach; boot snippets'
   toolchain blocks; the sweep/terraform skill pairs; ROADMAP milestone
   (M1.26 «MCP sovereignty»); Stage-B pointer (D11).
5. **Dogfood (D15)**: vibevm's `.mcp.json` + the demos' — managed
   entries via `vibe mcp install`; READMEs teach it.
6. Full close panel: self-check (grown), both demo floors, corpus,
   re-homed live chains, the fresh-consumer walk, mirrors, WAL,
   CONTINUE refresh if the owner winds down.

Named deferrals (visible, not silent):
- **D-a** vibe-mcp rebasing onto mcp-core — after the topology settles.
- **D-b** stable artifact shims (PROP-025 v2) — would make managed
  entries version-stable; today re-install rewrites them.
- **D-c** registry publish of the grown package set — owner call.
- **D-d** the Stage-B MCP-mounted arm (D11) — separate commission.
- **D-e** the ts STACK self-check gate + colon-free fact-store slot
  names — the mini-fix campaign's hygiene candidates, still open.
- **D-f** MCP progress notifications for long tools — v1 is
  run-to-completion (D8).
- **D-g** the `app` kind — anticipated by the §4 amendment text, not
  designed here.

## 12. Review points — RESOLVED (owner amendment + recorded executor defaults, 2026-07-07)

1. **Naming** — packages `mcp:org.vibevm/discipline-rust` /
   `-typescript`; binaries `discipline-mcp-rust` / `-typescript`;
   agent-visible names `discipline-rust` / `discipline-typescript`
   (executor default, D13).
2. **Topology** — one server per language, both entities mounted
   (executor default, D2).
3. **Kind** — **OWNER RESOLUTION, reverses the draft**: `mcp` is a
   package KIND; servers ship as separate packages; VIBEVM-SPEC §4
   amendment sanctioned; `app` anticipated. (D1/D1a; the exact-pin +
   vendor-projection consequences were surfaced to the owner in the
   amendment discussion.)
4. **`language` param** kept grammar-compatible; `tcg_bench` mounted
   (executor default, D9).
5. **vibe-tcg** deleted, no deprecation delegate (executor default,
   D5).
6. **Agent matrix v1** — all five hosts × both scopes (executor
   default, D6; the machinery exists).
7. **Wave order** — kind → transport → rust → ts → delivery →
   demontage (restructured by resolution 3; servers still precede the
   install feature).
8. **Latency** — no new targets for MCP-served tools; corpus
   regressions report, never cancel (executor default; §17.7
   precedent).

## 13. Execution ledger (filled by the executing session)

### Wave 4 — EXECUTED (2026-07-07); commit map

- `07af178` refactor(typescript-ai-native): `pub mod bench` (in-place,
  the rust build_rules-export precedent).
- `e19d57d` feat(packages): **mcp:org.vibevm/discipline-typescript
  v0.4.0** — 17 tools (no ledger in the TS umbrella), pin `=0.4.0`,
  the closure + the stack's embedded-source `tools/` dir (six [[sync]]
  sets). TWO TOOLCHAIN FIXES surfaced by the mirror: sync-engines'
  walker now speaks PROP-024 §2.2's FULL denylist (it had copied
  ts-extract's local node_modules wholesale — strays purged; note the
  filter HIDES denylisted files from the differ, so pre-existing
  strays need a manual purge once), and the TS API differences
  (u64 Position, root-level bridge re-exports, 3-arg codemod,
  Display-only bridge errors) are spoken natively.
- `e81b882` build(deps): `vibedeps/mcp-discipline-typescript/0.4.0`.
- Proof: the hermetic e2e pins the ABSENT-TOOLCHAIN posture through
  MCP (a bare project's conform_check answers isError WITH the install
  recipe); the live chain on ts-demo with vibe scrubbed from PATH —
  17 tools, clean validate, seeded TS2322 via pure overlay (disk
  byte-identical), conform green with the frozen brand-cast finding —
  **0.85 s**. Both language servers now serve vibe-free.
- Panel: full self-check exit 0 with the TS mcp package's four gate
  steps (22 steps total).

### Wave 3 — EXECUTED (2026-07-07); commit map

- `fdd6baf` feat(packages): **mcp:org.vibevm/discipline-rust v0.5.0** —
  the kind's first real inhabitant. TWO PLAN SIMPLIFICATIONS recorded:
  D3b's session-lib extraction proved unnecessary (`tcg-cli-rust`
  already IS the lib; the respawn-once law lives in the server's
  shared `TcgSession` cell), and the STACK BUMP FELL AWAY with it (no
  stack content changed — the package is born at 0.5.0
  version-mirroring the `=0.5.0` pin; D10 amended by execution:
  bump only what changes).
- `cf2e64c` build(deps): vibevm is the first consumer —
  `vibedeps/mcp-discipline-rust/0.5.0` (kind-prefixed slot naming live).
- The server: 18 tools (13 discipline adapters over the SAME lib fns
  the CLIs call, capture-guarded so reports carry child-process
  output; 5 tcg tools over ONE persistent r-a session, per-call policy
  reload, language guard with the recipe). The vendored closure
  mirrors the stack layout (6 stack crates + 5 neutral, 3 [[sync]]
  sets, 19 pairs); the whole 35-suite closure builds and tests inside
  the package.
- Proof: the hermetic e2e drives the BUILT binary on a bare temp
  project (init → conform green → the mini-fix vacuity warning seen
  THROUGH MCP → a seeded unwrap turns the gate red as an isError
  result → language refusal → protocol errors in-protocol; a fresh
  project's untagged pub fn is an ORPHAN refusal — parity includes the
  refusals, so the fixture carries no pub surface). The live chain:
  the binary on rust-demo with **vibe scrubbed from PATH** — 18 tools,
  clean validate, seeded E0308 via pure overlay (disk byte-identical),
  conform green — **2.58 s**. P2/P3/P4 CONFIRMED; PROP-027 §2.6 live.
- Panel: full self-check exit 0 with the mcp package's four new gate
  steps (fmt / crate tests / workspace clippy / self-trace).

### Wave 2 — EXECUTED (2026-07-07); commit map

- `28e6481` build(packages): discipline-core 0.5.0→0.6.0 (the ritual
  move list; stacks widened `^0.6` in place).
- `f922437` build(deps): the 0.6.0 slot + the three in-repo reference
  repoints; demos deliberately deferred to the waves that touch them.
- `ef018ee` feat(discipline-core): **mcp-core** — wire (line-delimited
  2024-11-05), server loop (replayable, bad frames never kill it),
  toolset (isError-result law, no prompts), capture (dup2 /
  SetStdHandle into a FILE; child-process output captured — proven
  live on this box; one sequential test because the redirect is
  process-global and libtest diverts in-process eprintln — the rustdoc
  example pins that path libtest-free).
- `183bbf9` docs(spec): MCP-CORE-v0.1 (five REQ units).
- `69fc129` refactor(xtask): multi-source `[[sync]]` sets; the ledger
  law recorded in the manifest itself: an mcp package mirrors its
  stack's crates/ LAYOUT (relative path-deps must hold) and mcp-core
  targets only mcp packages.
- `044ae86` build(deps): the flow slot gains mcp-core + the spec.
- Panel at the boundary: self-check 13 steps exit 0, vibe check clean,
  core-package suite 11 green incl. the capture end-to-end, self-trace
  0 orphans with mcp-core covered.

### Wave 1 — EXECUTED (2026-07-07); commit map

- `4943ad4` docs(spec): VIBEVM-SPEC §4.1 (owner-sanctioned five-kind
  register, `app` anticipated) + PROP-027 (kind law, manifest law,
  exact-pin law normative; registration/consent/composition specified
  for Waves 3–5).
- `01280bd` feat(core): `Kind::Mcp` in both enums (ONE non-exhaustive
  match total — the S4 prediction held), `McpServerDecl` +
  `MCP_ARG_VARS` + `VersionSpec::is_exact_pin`, `validate_mcp_kind`
  enforcing the five laws, six law tests, the pin-server/pin-stack
  fixture pair, and the P8 e2e — the exact pin selected 0.1.0 with
  0.2.0 deliberately on offer, slot `vibedeps/mcp-pin-server/0.1.0`,
  lockfile kind recorded, `vibe check` green. First run.
- `f2e9e51` docs: the five-kind wording sweep (owner-authorised
  «правь всё что реально нужно по смыслу» — includes the user-owned
  00-core.md terminology line and the PROP-018 §2.4 supersession note:
  its MCP-as-section half is superseded by PROP-027, the skill law
  untouched).
- Panel at wave close: workspace tests green, full self-check 13 steps
  exit 0, specmap 604/586/599 0 suspects/0 warnings, vibe check clean.
- P8 CONFIRMED at wave grain. The Wave-1 D1 NOTE (00-core.md) resolved
  by the same owner follow-up.

### Wave 0 — spike findings (2026-07-07, no tree changes)

- **S1 — protocol shape.** The proven contract is vibe-mcp's own
  production shape: JSON-RPC 2.0, **line-delimited** stdio (NOT
  Content-Length framing — the draft's D3 said LSP-style framing and
  was WRONG; only the tcg bridge speaks LSP), `PROTOCOL_VERSION =
  "2024-11-05"` (`vibe-mcp/src/lib.rs:61`), methods
  initialize/tools-list/tools-call/ping, notifications absorbed
  without response, `McpTool { descriptor(), run() }` + `Transport
  { read_line, write_line }` seams (`lib.rs:188-311`). mcp-core mirrors
  this exactly; the real-host probe rides Wave 3's live chain (the
  shape is already proven daily against Claude Code by the tcg tools).
- **S2 — long-call behaviour**: deferred to Wave 3's live chain
  against the real host (no scripted probe can answer host patience);
  run-to-completion stands regardless, budget hints in descriptions.
- **S3 — F5a RESOLVED BY ANALYSIS, plan amended.** Writer-threading is
  REJECTED: `floor`'s child processes (cargo, prettier, node) write to
  fd 2 directly — a threaded `&mut dyn Write` captures NONE of their
  output, so the reports would be hollow exactly for the heaviest
  tools. F5a resolves as a **process-level stderr capture guard**
  (dup2 dance, Windows via the CRT fd layer; Drop restores on unwind;
  legal because tool dispatch is sequential) living in mcp-core as its
  own `capture` cell. Consequence: the «report seams» sweeps vanish
  from Waves 3/4 — the stacks' lib/CLI signatures are NOT touched at
  all, and CLI byte-identity is trivially preserved. D4 is amended
  accordingly.
- **S4 — kind-mechanics inventory.** Canonical enum:
  `vibe-core/src/package_ref.rs:30` (`PackageKind` + `ALL: [_; 4]` +
  `FromStr` closed set + `Error::BadPackageKind`; its own doc already
  says extension is «a spec change, not a code change»). A DELIBERATE
  duplicate lives in `vibe-index/src/types/kinds.rs` (PROP-005 §3.2
  standalone-redistribution trade-off; parity-tested against
  vibe-core; carries `clap::ValueEnum` + its own FromStr error text
  naming the four kinds). `requires_kinds: Vec<PackageKind>`
  (`manifest/package.rs:280`) extends automatically with the enum.
  `NamingConvention::{KindName, KindSlashName}` compose via `Display`
  — auto-extend. **Exact `=` pins are ALREADY first-class**: structural
  pin construction at `manifest/package.rs:126` and
  `manifest/lockfile.rs:469`, resolver-side `={version}` parsing at
  `vibe-resolver/src/lib.rs:540` — D1a's law rides existing machinery;
  the Wave-1 fixture proves it end-to-end. 38 files reference
  `PackageKind::` (the sweep list is `grep -rl "PackageKind::"
  crates/`); the compiler drives the sweep once the variant lands
  (match exhaustiveness).
