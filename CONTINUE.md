# CONTINUE.md — cold-resume checkpoint

_Written 2026-07-07, session close. This session ran TWO campaigns to
completion: (1) the **discipline-core mini-fix** (CONTINUE item 1 of
the previous checkpoint — the single-crate defect turned out
three-faced, plus a bonus LSP-retrigger fix the bench surfaced), and
(2) the **MCP-SOVEREIGNTY campaign whole**
(`spec/terraforms/MCP-SOVEREIGNTY-PLAN-v0.1.md`, Waves 0–6, EXECUTED
on the owner's «план должен быть выполнен до конца, все волны»). The
`mcp` package KIND exists end to end; the discipline serves itself
over MCP with no vibe in the runtime path; `crates/vibe-tcg` is
deleted; vibevm dogfoods its own `.mcp.json`. Everything is PUSHED to
both mirrors (`d7c3fe2`); the tree is clean; the full panel was green
at close._

> **`spec/WAL.md` is the canonical living state**; if this snapshot
> and the WAL disagree, the WAL wins. The **git log is the
> authoritative per-item record**; the plan's §13 carries per-wave
> commit maps. Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its
> files → `spec/WAL.md`), then read this.

---

## TL;DR

The session opened on `восстанови сессию`, executed the commissioned
discipline-core mini-fix (validator/scanner naming unified through
`store::crate_dir_name`; the path-scope predicates `in_src`/`in_tests`
/`is_lib_root` — the live walk caught that naming alone left the gate
silent; init's single-crate label switched to the directory basename;
scan-vacuity warnings in both engines; and the acceptance bench caught
rust-analyzer's ServerCancelled/-32802 race — the bridge now
retriggers under the same deadline). Then the owner opened the
architecture discussion that became MCP-SOVEREIGNTY: kinds are
owner-extensible, `mcp` is the fifth KIND (`app` anticipated), the
discipline servers ship as SEPARATE exact-pinned packages, all
commands (not just tcg) serve over MCP, and vibe delivers but never
serves. Six waves later that is all real, tested, and dogfooded.

## Where work stands

- **Branch `main`**, tree clean, local == origin == github @
  `d7c3fe2` (mirrored at campaign close per Rule 4 — the work was
  owner-commissioned).
- Package versions: discipline-core **0.6.0** (+ `mcp-core`),
  rust-ai-native **0.5.0** (bench module exported; boot snippet
  re-taught), typescript-ai-native **0.4.0** (same),
  **mcp:org.vibevm/discipline-rust 0.5.0** (NEW: 18 tools),
  **mcp:org.vibevm/discipline-typescript 0.4.0** (NEW: 17 tools).
  vibevm product: `Kind::Mcp` + `McpServerDecl` + the delivery surface
  in vibe-mcp/vibe-workspace; `vibe-tcg` DELETED; vibe-mcp back to its
  four product tools.
- Close panel: self-check **22 steps exit 0** (grew both mcp-package
  gates); conform 0 findings (**10 gated** after vibe-tcg's departure
  / 4 exempt); specmap **604/583/597, 0 suspects/0 warnings**; `vibe
  check` clean; `vibe bin list` = **10 binaries**; corpus **9/9**
  (cold 2 538 ms, warm p95 60 ms); live chains **2.55 s (rust) /
  0.82 s (ts), both with vibe scrubbed from PATH**; both demos
  repinned at the 0.6.0 flow; `.mcp.json` answered live (18+17 tools)
  over the registered command lines.

## The open items (owner-court)

1. **Registry publish** — the set grew to FIVE packages:
   discipline-core 0.6.0, rust-ai-native 0.5.0, typescript-ai-native
   0.4.0, discipline-rust 0.5.0, discipline-typescript 0.4.0. Owner
   call, unchanged posture (never published without the word).
2. **Stage-B delivery experiments** — still BACKLOGGED
   (`spec/terraforms/TCG-STAGE-B-DELIVERY-PLAN-v0.1.md`); note its
   «MCP-mounted arm» is now FREE to run — mount
   `discipline-typescript` instead of prompt-naming a CLI. Re-verify
   its §1 facts at pickup; they predate two campaigns.
3. **vibe-mcp rebase onto mcp-core** (named deferral D-a) — one MCP
   implementation ecosystem-wide; only after the topology has settled
   in use.
4. **PROP-025 v2 shims** (D-b) — would make `.mcp.json` managed
   entries survive version bumps without a re-register.
5. **Hygiene pair from the mini-fix campaign** (still open): a
   TS-STACK step in self-check (the 1.93.1 toolchain drift sat latent
   there; the mcp-TS package IS gated now, the stack itself is not),
   and colon-free fact-store slot names (today `sha256:<hex>.json`
   lands as an NTFS alternate data stream — works by accident).
6. **`vibe install --refresh <pkg>` ergonomics** — the demo
   rm-and-reinstall recipe was walked twice more this session; still
   live-able, friction is real.
7. **The `app` kind** — anticipated by VIBEVM-SPEC §4.1's amendment
   text; not designed.

## Quick-start (verify the tree)

```sh
bash tools/self-check.sh; echo "EXIT=$?"      # 22 steps, must be 0
cargo run -q -p vibe-cli -- bin list           # 10 binaries incl. discipline-mcp-*
cargo run -q -p vibe-cli -- mcp status --path . # both servers built + managed
# the registered servers answer over their exact command lines:
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n{"jsonrpc":"2.0","id":2,"method":"tools/list"}\n' \
  | ./vibedeps/mcp-discipline-rust/0.5.0/target/release/discipline-mcp-rust.exe --path .
# live chains (need rust-analyzer / node):
cargo test --manifest-path packages/org.vibevm/discipline-rust/v0.5.0/Cargo.toml \
  -p discipline-mcp-rust --test live_chain -- --ignored
cargo test --manifest-path packages/org.vibevm/discipline-typescript/v0.4.0/Cargo.toml \
  -p discipline-mcp-typescript --test live_chain -- --ignored
cargo run -q -p vibe-cli -- bin exec tcg-rust -- bench \
  --corpus research/tcg-bench/corpus-rust --report /tmp/r.json \
  --root research/rust-demo                   # agreement 9/9
```

## Non-obvious findings (this session; the WAL carries the full set)

- **rust-analyzer ServerCancelled (-32802 + `retriggerRequest:true`)
  is a retry instruction, not an error** — the diagnostics pull races
  r-a's own overlay revision bump nondeterministically (9/9 one day,
  deterministic-red the next). The bridge resends with a fresh id
  under the SAME deadline; replay-pinned.
- **A path-scope predicate inlined six times is six bugs** — the bare
  single-crate shape (`src/lib.rs`, no crate prefix) never matched
  `contains("/src/")`; the tree was scanned, attributed, validated,
  and every rule silently declined it. Only an end-to-end walk that
  EXPECTS a finding catches a rule-scope hole; vacuity warnings
  cannot (the files WERE attributed).
- **Writer-threading can never capture a tool run's whole story** —
  floor's cargo/node children write fd 2 directly. The mcp-core
  capture guard redirects the PROCESS stderr (dup2 / SetStdHandle)
  into a FILE (a pipe would deadlock a chatty floor). libtest diverts
  the test thread's own eprintln! before the std handle — so unit
  suites pin the child path and rustdoc examples (libtest-free) pin
  the in-process path.
- **Cross-slot Cargo path-deps stay impossible** (PROP-024 §2.4), so
  an mcp package VENDORS its closure and the **exact `=X.Y.Z` pin**
  (enforced by `Manifest::validate`) is what holds the vendored
  engines and the consumer's gates to one version set. The vendored
  layout must MIRROR the stack's crates/ layout or relative path-deps
  break; `mcp-core` targets only mcp packages.
- **sync-engines' walker now filters PROP-024's full denylist** —
  node_modules leaked into the first TS mirror; NOTE the filter hides
  denylisted strays from the differ, so pre-existing strays need one
  manual purge when adding a set.
- **The TS stack's crates embed `tools/` sources via include_str!
  from the package root** — any vendored copy must mirror `tools/`
  too (the sixth [[sync]] set).
- **Package servers register PROJECT-scope only** — `{project_root}`
  demands a project; every project-scope agent config is JSON, so the
  managed sidecar (top-level `"vibevm": {"managed": [...]}`, never a
  key inside a server entry — hosts validate entry shapes) has no
  TOML form.
- **Execution simplifications recorded in the plan ledger**: D3b's
  session-lib extraction proved unnecessary (tcg-cli crates already
  ARE the libs) and the stack bumps fell away (bump only what
  changes; the pin + version-mirroring carry the law).

## Repository map (delta over the previous checkpoint)

```
vibevm/
├─ .mcp.json                        NEW, COMMITTED: vibevm product entry +
│                                    both discipline servers (managed sidecar)
├─ crates/
│   ├─ vibe-tcg/                    DELETED (the whole dispatch layer)
│   ├─ vibe-mcp/                    product tools only; + pkg_servers cell
│   │                                (payloads, substitution, managed sidecar)
│   └─ vibe-workspace/src/bins.rs   + DeclaredMcpServer, collect_mcp_servers
├─ packages/org.vibevm/
│   ├─ discipline-core/v0.6.0/      + crates/mcp-core (wire/server/toolset/
│   │                                capture) + spec/mechanisms/MCP-CORE-v0.1
│   ├─ discipline-rust/v0.5.0/      NEW mcp-kind package: server crate +
│   │                                11-crate vendored closure, pin =0.5.0
│   ├─ discipline-typescript/v0.4.0/ NEW: server + 13-crate closure + tools/,
│   │                                pin =0.4.0
│   └─ (both stacks)                boot snippets re-taught; ts bench pub
├─ fixtures/registry/org.vibevm/
│   ├─ pin-server/ pin-stack/       the exact-pin + registration fixtures
├─ spec/
│   ├─ modules/vibe-mcp/PROP-027-mcp-packages.md   NEW, IMPLEMENTED
│   ├─ modules/vibe-mcp/PROP-026-…  superseded-in-topology block
│   └─ terraforms/MCP-SOVEREIGNTY-PLAN-v0.1.md     EXECUTED (+§13 maps)
├─ sync-engines.toml                multi-source [[sync]] sets (6)
└─ VIBEVM-SPEC.md §4.1              five kinds, owner-extensible register
```

## Standing policies in force (long form)

- **The five-kind register** (owner-sanctioned 2026-07-07): flow,
  feat, stack, tool, **mcp**; grows only by owner amendment to
  VIBEVM-SPEC §4.1; `app` anticipated. `[[mcp_server]]` is legal ONLY
  in mcp-kind packages and mandatory there.
- **The exact-pin law (PROP-027 §2.3)**: every package requirement of
  an mcp package is `=X.Y.Z`; mcp packages bump in lockstep with what
  they serve; version-mirroring is the naming convention.
- **Vibe-free serving (PROP-027 §2.6)**: agent hosts launch slot
  artifacts directly; vibe installs, builds, registers — never
  serves. The live chains enforce this with a scrubbed PATH.
- **One trust model, two verbs (PROP-027 §2.5)**: registering a
  server inherits PROP-025's consent gate (org.vibevm allow-listed;
  third parties need the explicit flag, refused with the recipe).
- **PROP-026's grammar survives its topology**: the four tcg ops'
  params/answers/no-prompt law are normative; `language` is a
  validated compatibility param (mismatch refuses naming the right
  server).
- Clean-room (PLDI'25 repo untouched), production-grade/no-MVP, the
  four CLAUDE.md rules, D13 language-suffix naming, publish held for
  the owner's word — all unchanged.

## Recent commit chain (this session, newest first)

```
d7c3fe2 docs(wal): the mcp sovereignty campaign complete - all six waves
1451954 build(deps): campaign close - slots, demos, and the dogfood config
7299e78 docs: the sovereignty re-teach - PROP-026 superseded in topology
36461ba refactor(mcp)!: retire the tcg adapters and delete vibe-tcg
cc6c3a4 docs(wal): mcp sovereignty wave 5 - vibe delivers, never serves
3caa986 feat(mcp): vibe delivers package-declared servers (PROP-027 s2.4-2.5)
d9e8c5e docs(wal): mcp sovereignty wave 4 - both language servers vibe-free
e81b882 build(deps): materialise the mcp:discipline-typescript slot
e19d57d feat(packages): mcp:discipline-typescript - the TS mirror
07af178 refactor(typescript-ai-native): export the bench module
fc848a8 docs(wal): mcp sovereignty wave 3 - the rust server ships standalone
cf2e64c build(deps): materialise the mcp:discipline-rust slot
fdd6baf feat(packages): mcp:discipline-rust - the discipline served standalone
7b7f4c6 docs(wal): mcp sovereignty wave 2 - the neutral transport ships
044ae86 build(deps): re-materialise the flow slot with mcp-core
69fc129 refactor(xtask): multi-source sync-engines
183bbf9 docs(spec): MCP-CORE-v0.1 - the transport mechanism
ef018ee feat(discipline-core): mcp-core - the neutral MCP transport
f922437 build(deps): re-materialise vibedeps at discipline-core 0.6.0
28e6481 build(packages): bump discipline-core to 0.6.0
7271dc9 docs(wal): mcp sovereignty waves 0-1 - the kind lands
f2e9e51 docs: sweep the four-kind wording to the five-kind register
01280bd feat(core): Kind::Mcp across the product + the [[mcp_server]] laws
4943ad4 docs(spec): the mcp kind - VIBEVM-SPEC s4.1 + PROP-027
257ae55 docs(plan): wave-0 spike findings - framing, capture, kind inventory
```

(Before these: the mini-fix campaign `0bce3b2`…`5185bda` — see the WAL
fifth-campaign section.)

The WAL supersedes this snapshot wherever they diverge. Session-resume
phrase: `восстанови сессию` (boots into a status report and waits —
the open items above are the owner's call, not a standing mandate).
