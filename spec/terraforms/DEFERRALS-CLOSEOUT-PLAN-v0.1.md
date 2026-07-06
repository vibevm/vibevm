# DEFERRALS-CLOSEOUT-PLAN v0.1 — close every §10 deferral of the Self-Sufficiency campaign

_Status: ACCEPTED with owner amendments, 2026-07-07. Written against tree
`57fa42e`; owner review resolved the three open questions: (1) editing
`spec/boot/90-user.md` is sanctioned ("меняй как хочешь"), (2) PROP-025 is
spec PLUS implementation, not spec-only (D6 rewritten), (3) the TypeScript
frontend is the full Compiler-API variant, not the lexical MVP (D2
rewritten). vibe-tcg-ts is explicitly OUT — a separate plan (see
non-goals). Cold-executable: any phase is a safe stop; the floor must be
green at every phase boundary._

Mandate (owner, 2026-07-07): take everything listed in
`SELF-SUFFICIENCY-PLAN-v0.1.md` §10 (plus the standing `vibe-registry`
line-budget note from CONTINUE.md) and plan its implementation. For
typescript-ai-native symmetry, do NOT wait for the full VibeVM TypeScript
pilot — build a small test demo project instead and implement the TS stack
symmetric to the Rust one.

## 0. The seven items this campaign closes

| # | §10 deferral | Closure shape |
|---|---|---|
| 1 | Engine-code consolidation into discipline-core | Phase 1 — authored home moves; vendor-sync mechanism |
| 2 | typescript-ai-native symmetry (conform/specmap/skills twins) | Phases 2–5 — engines, bins, skills, boot |
| 3 | (owner amendment) TS pilot → small demo project | Phase 6 — `research/ts-demo` + frozen test |
| 4 | DEBT.md / INTENT.md generated views | Phase 7 — `discipline-rust ledger render` |
| 5 | `vibe trace` product alias | Phase 8 — delegating subcommand |
| 6 | vibe-native binary delivery (future PROP) | Phase 9 — PROP-025 authored AND implemented (owner: "не только spec-only") |
| 7 | Owner-court: machine-quirks → `spec/boot/90-user.md` | Phase 10 — sanctioned 2026-07-07 |
| + | `crates/vibe-registry/src/lib.rs` at 599/600 lines | Phase 11 — module-grain split |

Non-goals (named, stay deferred):
- **`vibe-tcg-ts` — a SEPARATE plan** (owner asked the scope question
  directly; answered 2026-07-07). It is generation-time work — logit-level
  type-constrained decoding wired into an inference loop (GUIDE §14,
  the PLDI'25 lineage) — while everything in this campaign is
  deterministic post-generation gating. It depends on inference plumbing
  (`vibe-llm` is an M0 stub), needs research-style acceptance criteria
  (mask latency per token, language-service incrementality), and is not
  cold-executable today. This campaign BUILDS its prerequisites — the
  Compiler-API extractor infrastructure (D2) and the demo project as a
  testbed — so the follow-up plan starts from a real seam, not from zero.
  Same disposition for its Rust twin (`vibe-tcg`, carried as a conscious
  stub since the TS-stack session).
- `test-gate`/`tripwire`/`health`/`fast-loop`/`codemod` TS twins;
  prettier/eslint floor steps as gated defaults.
- The full VibeVM TypeScript surface (the demo is the pilot-lite, not the
  pilot).
- Cross-package path-dep rewriting at materialise time (PROP-025 §6 specs
  it as v2; this campaign implements §§2–5 only).
- Mirror/publish (owner-held; executable now that the network is back, but
  not part of this campaign's phases).

## 1. Standing constraints

- **Network: present but flaky.** The first probe of 2026-07-07 got
  connection-refused on both SSH endpoints; a re-probe the same day
  authenticated against BOTH gitverse.ru:22 and github.com:22, with
  api.github.com and registry.npmjs.org answering 200 (crates.io answers
  403 to a bare curl — the usual anti-bot response, not an outage;
  gitverse HTTPS timed out while its SSH works, and SSH is the push
  path). Treat network as available but re-verify at the moment of any
  network-dependent step. Consequences: the mirror and the registry
  publish are now EXECUTABLE and held purely on the owner's word; new
  crate dependencies are POSSIBLE but still minimised by policy (D2
  weighs this as a real choice, not a forced one); npm resolves
  `typescript` (also cached locally), and node is v24.18.0
  (type-stripping stable → `node --test` runs `.ts` natively).
- The four CLAUDE.md rules; floor green at every phase boundary
  (`bash tools/self-check.sh` exit 0, specmap `--check` clean 0 dangling,
  conform 0, package gates green).
- Machine quirks per DISCIPLINE-SWEEP-v0.2 §3 (Edit/Write only;
  `git commit -F - <<'MSG'`; Git Bash for self-check; real exit codes;
  no redirects into unset vars).
- Version moves: `discipline-core` 0.3.0 → **0.4.0** (gains a code-root),
  `rust-ai-native` 0.3.0 → **0.4.0** (engine authorship leaves, vendored
  copies arrive), `typescript-ai-native` 0.2.0 → **0.3.0** (gains a
  code-root + skills). vibevm's `specmap.toml` `[[external_specs]].root`
  hardcodes `vibedeps/flow-discipline-core/0.3.0/spec` — **must bump with
  the slot** or resolution silently dangles (checked by the 0-dangling
  floor). Registry publish of all three stays owner-held.

## 2. Decisions (D1–D8)

### D1 — consolidation mechanism: vendor-sync, not cross-slot path-deps

The blocker nobody wrote down: a stack crate cannot `path`-dep on a
discipline-core crate with ONE relative path, because the authored layout
(`packages/org.vibevm/<name>/v<ver>/`) and the materialised layout
(`vibedeps/<kind>-<name>/<ver>/`) differ in both directory naming and
version-prefix; a baked relative path satisfies one tree and breaks the
other, and each slot must stay a self-buildable workspace (PROP-024 §2.4 —
the property the fresh-project walk proved).

Options considered:
- (α) vibe rewrites cross-package path-deps at materialise time — real
  product surface, interacts with shippable-tree hashing (PROP-024 §2.2
  reproducibility), belongs in the PROP-025 family as future work. Rejected
  for this campaign, **named in PROP-025** (Phase 9).
- (β) align the two layouts — breaks PROP-011; rejected.
- (γ) **vendor-sync (chosen):** authored source of the language-neutral
  engines lives in `packages/org.vibevm/discipline-core/v0.4.0/crates/`
  (the Ф6 brief's option (a), "the principled end-state"); each stack ships
  a byte-identical **synced copy** under its own `crates/vendor/`, produced
  by a new `cargo xtask sync-engines` and gated by `sync-engines --check`
  as a new self-check step. Stacks stay self-contained workspaces; no vibe
  changes; divergence is mechanically impossible while the gate holds.

Crate disposition (corrected by the Phase 0 topology spike): the FOUR
neutral-engine crates — `conform-core`, `specmap-core`, `specmark`,
`specmark-grammar` — move to discipline-core (authored) and are vendored
into both stacks. `specmark` moves WITH the engines because conform-core
self-traces through its `scope!` markers (the Ф4b restoration) — leaving
it rust-stack-authored would make the flow package depend on a stack
package. (`specmark` is Rust-tagging machinery, but the engines are Rust
wherever they run; "checkers ship in each language stack" governs the
per-language FRONTENDS, not the neutral core.) `conform-frontend-rust`,
`env-audit`, `conform-cli`, `specmap-cli`, `discipline-cli` stay
rust-stack-authored. vibevm root `Cargo.toml` repoints BOTH its
`specmap-core` and `specmark` path-deps to the discipline-core authored
copies.
Vendored dirs are excluded from conform scanning (the `/vendor/` substring,
same mechanism as `/generated/`) and exempted in each stack's own
`specmap.toml` — authored copies carry the tags.

### D2 — TS fact source: the TypeScript Compiler API (owner-chosen, "полный нормальный вариант")

The Ф6 brief's own design, built as written: a **Node-side extractor on
the TypeScript Compiler API**, wrapped by a Rust `Frontend` whose `id()`
is `"ts-tsc"` — the exact identifier the brief names. The lexical and
swc variants were weighed and are dropped (owner decision 2026-07-07);
the AST gives what a lexer cannot promise: `as const` vs cross-type
`as`, `any` in type position vs inside a string, non-null assertion as
an AST node, JSDoc tags via `ts.getJSDocTags` instead of comment
regexes.

The shape:

- **The extractor ships in the TS stack** (`tools/ts-extract/` in the
  package): TypeScript source, erasable-syntax-only, run directly by
  `node` (type-stripping — stable on node ≥ 23, this box has v24; the
  stack's documented minimum). Its ONE dependency, `typescript`, is
  resolved from the CONSUMER's project (`require.resolve` from the
  project root) — the same `typescript` the consumer already needs for
  `tsc --noEmit`, so the gate adds no new install. Resolution failure =
  a hard error naming the recipe (`npm install -D typescript`), never a
  skip.
- **Protocol:** the Rust side spawns `node <extractor> --root <dir>`
  (batched, not per-file) and reads NDJSON — one record per file
  carrying (a) conform facts: imports, the `unsafe`-set occurrences
  with AST-accurate classification, `@ts-expect-error -- reason` /
  `@ts-ignore` from the comment stream, file metrics; and (b) spec
  markers: the §9 JSDoc tags with item names and spans. One extractor
  run feeds BOTH engines (D3). The protocol carries a
  `protocol_version`; the frontend `version()` is tied to it, so a fact-
  schema change retires conform's cache slots wholesale — the mechanism
  the brief already specifies.
- **A small bridge crate** in the TS stack (`ts-extract-bridge`: spawn,
  NDJSON parse into typed records, error taxonomy: node-missing /
  typescript-unresolvable / extractor-crash / protocol-mismatch — each
  a distinct actionable message). `conform-frontend-typescript` and the
  specmap TS scanner are its two consumers.
- **B5 preserved:** the Compiler API parses with recovery; a file it
  cannot make sense of yields a record with zero facts and a noted
  parse-degraded flag, never a gate error.
- **Consumer surface, honestly:** the structural gate for TS requires
  node + the project's `typescript` at gate time. That is the floor's
  existing reality for TS projects (D7's tsc step needs both anyway),
  so the extractor adds zero NEW requirements — it reuses them.

### D3 — TS traceability markers: JSDoc tags, per GUIDE §9

Markers are JSDoc block tags — `/** @implements spec://… */`,
`@verifies`, `@documents`, `@deviates spec://… reason`, `@informs` — the
erasure-clean form GUIDE §9 already prefers over decorators. Module-level
edge: a file-top `/** @scope spec://… */` block mirrors `scope!`. They are
read from the AST comment stream by the D2 extractor (the same run that
feeds conform), not by a second scanner pass.

Topology: specmap-core (discipline-core-authored, language-neutral) gains
a **scanner seam** — the specmap analog of conform's `Frontend` trait —
with rscan as the built-in Rust implementation; the TS implementation
lives in the TS stack (it owns the Node bridge) and is injected by
`specmap-cli-typescript`. The neutral core never learns about Node.
Dispatch is by extension (`.rs` → rscan, `.ts`/`.tsx`/`.mts` → the TS
scanner). URI grammar validation reuses `specmark-grammar` (one grammar,
both languages; grammar violations are findings, not panics). Edge budget
(≤3 per item), two-tier revisions, and asymmetric invalidation carry over
unchanged from PROP-014 — same index schema, same ratchet, so
`specmap.json` needs no format change.

### D4 — demo location and shape: `research/ts-demo`

New top-level `research/` (owner's suggestion) holding `ts-demo/` — a
minimal but REAL consumer: its own git-ignorable `vibedeps/` (bootstrap
documented, not committed — packages content is already in-repo twice),
its own `vibe.toml` (`[project]` + requires
`stack:org.vibevm/typescript-ai-native = "^0.3.0"`) resolved from the
in-repo `packages/` file:// registry (mutable per PROP-011 §2.6, so
package edits propagate on re-install), `tsconfig.json` at the GUIDE §1
floor (strict + noUncheckedIndexedAccess + exactOptionalPropertyTypes +
erasableSyntaxOnly), 2–3 cells (branded type at a seam, Result-shaped
error union, one runtime validator at an erasure boundary), `node:test`
tests runnable via bare `node --test` (strip-types — no build step),
a `package.json` whose one devDependency is `typescript` (serving BOTH
the tsc floor step and the D2 extractor's `require.resolve`;
`node_modules/` gitignored, `npm install` part of the bootstrap recipe),
`spec/PROP-001.md` with anchors, JSDoc markers citing them, and its own
`specmap.toml` (namespace `ts-demo`) + `conform.toml`. `research/` is
inert to vibevm's own gates (scan roots are explicit: `crates/*`, `xtask`)
— Phase 0 verifies.

### D5 — `vibe trace`: delegation, not embedding

`vibe trace <args…>` spawns `discipline-rust trace <args…>` from PATH and
passes the exit code through. On spawn failure it prints the recovery
recipe (`cargo install --path vibedeps/<slot>/crates/discipline-cli`, or
the in-place `cargo run` form) and exits non-zero. Embedding specmap-core
into vibe-cli is rejected: it couples the product binary to one engine
version while projects pin their own stack versions (skew), and delegation
keeps the product surface one function deep.

### D6 — binary delivery: PROP-025 authored AND implemented (owner-expanded)

Owner decision 2026-07-07: spec plus implementation in this campaign.
The PROP specifies the whole design; §§2–5 (manifest surface, consent-
gated install-time build, slot-resident artifacts, `vibe bin` dispatch +
shims) are implemented in Phase 9; §6 (cross-package path-dep rewriting
at materialise) stays specified-only as v2. Implementation choices made
here so Phase 9 executes cold:

- **Artifacts live in the slot** (`vibedeps/<slot>/<ver>/target/release/`),
  no separate store: the slot IS the unit — build output is outside the
  shippable tree (PROP-024 §2.2, the Ф4c filter), so hashes stay
  reproducible, and a slot refresh naturally invalidates its binaries
  (staleness for free).
- **Shims are dumb, `vibe` is the dispatcher:** `~/.vibevm/bin/<name>`
  (paired `.cmd` on Windows — the PROP-015 `cmd /c` lesson) execs
  `vibe bin exec <name> -- <args…>`; `bin exec` walks up from CWD to
  `vibe.lock`, resolves which slot pins that binary, builds it if the
  artifact is missing (consent rules below), and execs it with the exit
  code passed through. Outside any project: the newest installed
  version, from a user-level `~/.vibevm/bins.toml` registry updated at
  install time. Reconcile with PROP-019's existing shim dir at
  execution time — if the version manager already owns a PATH-prepended
  dir, reuse it rather than introduce a second one.
- **Consent per PROP-020's posture:** an install-time `cargo build` of
  package code executes build scripts and proc-macros; the first build
  of a (package, version) prompts exactly like hooks do (`--assume-yes`
  honoured, decision recorded alongside the hook-consent state, reused
  by later rebuilds of the same content hash).
- **Subcommands:** `vibe bin list` / `vibe bin path <name>` /
  `vibe bin exec <name> --` / `vibe bin sync` (create/refresh shims for
  everything the lockfile declares, the post-install idempotent step).
- `cargo install --path` remains the documented degraded path (no-vibe
  environments, CI).

### D7 — the TS floor v1 composition

`discipline-typescript floor` runs, in order: **typecheck** (`npx tsc
--noEmit`, resolved from the project's node_modules or the npm cache) →
**tests** (`node --test`, the built-in runner — zero deps) → **conform**
(`conform-typescript`) → **specmap** (`specmap-typescript --check`). Four
steps, each with the per-policy origin line (the defaulted-policy
announcement carried over from the Rust floor). prettier/eslint steps and
a TS test-gate (junit/TAP parsing) are NAMED deferrals — absent tooling is
a hard step failure, never a silent skip, so a consumer without
`typescript` installed sees a red floor with the install recipe, not a
green lie.

### D8 — skills twins: adapt, not symlink

`typescript-ai-native` ships `[[skill]]` entries `discipline-sweep-typescript`
and `terraform-typescript` — the Rust skills' procedure skeleton with the
toolchain swapped (`discipline-typescript floor`, tsc/node-test loops, TS
cards, TS quirks). They cite the same discipline-core playbooks (the method
is the flow package's; the skill is the stack's instantiation — the
established split).

## 3. Phase 0 — probes and spikes (no commits; gate for everything after)

1. Re-probe network at execution time (ssh gitverse/github; npm ping) —
   the 2026-07-07 re-probe already authenticated on both SSH endpoints,
   but the same morning saw both refuse connections, so treat
   reachability as a per-step fact, not a session constant.
2. `npm install typescript` in a scratch dir — REAL install, not
   dry-run; record whether `npx tsc --version` runs. (Resolves offline
   from the local npm cache too, probed 2026-07-07.)
3. `node --test` on a scratch `.ts` with type annotations + an
   `erasableSyntaxOnly`-clean feature set — confirm v24 strip-types runs it.
4. **Compiler-API extractor spike:** a ~40-line scratch script importing
   the freshly-installed `typescript`, parsing a fixture `.ts`, printing
   (a) JSDoc tags with names/spans via `ts.getJSDocTags`, (b) `any` in a
   type position vs `"any"` in a string, (c) a cross-type `as` vs
   `as const`, (d) the comment stream for `@ts-expect-error -- reason` —
   proves every fact class D2 promises before Phase 2 commits to the
   protocol. Also confirm the script itself runs as `.ts` under
   strip-types.
5. Vendor-sync build spike: copy `conform-core` into a scratch stack-shaped
   workspace under `crates/vendor/conform-core`, repoint one consumer,
   `cargo build` — validates D1's topology on Windows paths.
6. Confirm `research/` inertness: create the empty dir, run
   `cargo xtask specmap --check` + `conform check` — byte-stable.
7. Acceptance: findings recorded in the WAL session section; any red probe
   downgrades its dependent step per the notes above (nothing else blocks).

**EXECUTED 2026-07-07 — all six probes green.** Results, binding on the
later phases:

- Network: both SSH endpoints authenticate (same-day recovery from the
  morning's refusals — reconfirmed per-step posture).
- `npm install typescript` → **6.0.3** in ~1s; `tsc --version` runs.
  Note the MAJOR: the extractor targets the stable API and the 6.x
  surface used below is confirmed working.
- `node --test` executes annotated `.ts` under v24 strip-types (pass 1).
- Compiler-API spike: every D2 fact class proven on TS 6.0.3 —
  `any` in type position (and NOT inside a string literal), cross-type
  `as` vs `as const` discriminated, non-null assertion, imports, JSDoc
  tags with lines, `@ts-expect-error -- reason` from the comment stream
  (string-literal traps yield zero facts). **Finding:** `@implements`
  is a PARSED JSDoc tag (`JSDocImplementsTag` — its "class expression"
  eats `spec`, leaving `://…` in `.comment`); the extractor must read
  the spec-URI from the tag's RAW TEXT for parsed tags, not from
  `.comment`. Same caution for any tag name TypeScript recognises.
- Vendor topology: conform-core + specmark + specmark-grammar copied
  into a scratch stack-shaped workspace (`crates/vendor/*`,
  workspace-inherited fields redeclared) + a consumer — `cargo check`
  offline exit 0. **Finding:** conform-core depends on specmark (Ф4b
  self-trace), so the D1 move set is the FOUR neutral crates (D1
  corrected in place).
- `research/` is inert to both gates (specmap 573/566/578/0/0, conform
  0 — unchanged with the directory present).

## 4. Phase 1 — engine consolidation (discipline-core 0.4.0)

1. Version-bump dirs/manifests first (the 0.3.0→0.4.0 / 0.2.0→0.3.0 moves,
   `git mv`, requirements widened: stacks require core `^0.4`), so every
   later diff lands in final paths. vibevm `specmap.toml`
   `[[external_specs]].root` bumps with the slot on re-materialise.
2. `git mv` `conform-core`, `specmap-core`, `specmark-grammar` from
   `rust-ai-native/v0.4.0/crates/` → `discipline-core/v0.4.0/crates/`;
   discipline-core `Cargo.toml` becomes a workspace (its first code-root —
   PROP-024 applies as written; kind `flow` carries code the same way
   `stack` does).
3. New `cargo xtask sync-engines [--check]`: byte-copies the three crates
   into `rust-ai-native/v0.4.0/crates/vendor/` and
   `typescript-ai-native/v0.3.0/crates/vendor/` (`--check` = compare, exit
   1 on drift; a `.sync-manifest.toml` names source root + crate list so
   the tool is data-driven, not hardcoded).
4. Rust-stack crates repoint deps `../conform-core` → `../vendor/conform-core`
   etc.; vibevm root repoints `specmap-core` to the discipline-core
   authored path; conform/specmap configs gain the `/vendor/` exclusion +
   exempt entries (both stacks' own `specmap.toml`).
5. `self-check.sh` grows a step: `cargo xtask sync-engines --check`.
6. Re-materialise vibedeps (`vibe install`); regen specmap.
7. Acceptance: floor green end-to-end; vibevm index 0 dangling (the
   external_specs bump proven by the same 7 citations still resolving);
   rust-stack package tests + `specmap-rust --gate` green from the
   MATERIALISED slot; `sync-engines --check` green and RED when a vendored
   byte is touched (prove once, revert).
8. Commits: `build(packages): bump the discipline packages for the
   consolidation`, `refactor(discipline-core): take authorship of the
   neutral engine crates`, `feat(xtask): sync-engines vendoring gate`,
   `build(deps): re-materialise vibedeps at 0.4.0`.

## 5. Phase 2 — conform for TypeScript (engine rules + frontend + bin)

1. **The extractor** (`tools/ts-extract/` in the TS stack, per D2):
   TypeScript, erasable-only, Compiler API; batched NDJSON per-file
   records carrying conform facts + spec markers; `protocol_version`;
   parse-degraded flag for recovery cases (B5). Its own tests run under
   `node --test` (fixture files in, records out) — wired into the TS
   stack's package test suite behind a node-presence gate that FAILS
   (not skips) when node is absent, with the recipe.
2. **The bridge crate** `ts-extract-bridge` (spawn, typed NDJSON parse,
   the four-way error taxonomy). Unit-tested against recorded extractor
   output so the Rust side's protocol handling is testable without node.
3. conform-core (authored home): TS fact shapes + rules-as-queries —
   file budget (reuse), `ts-unsafe-in-domain` (the §8 ban set as Class-F
   findings, `@ts-expect-error -- reason` honoured as a recorded deviation
   the way `#[spec(deviates)]` is), `ts-cell-isolation` (imports cross
   seams only — config names the seam filename, default `index.ts`).
   Rules live once in core; the Rust path is untouched (frontends feed
   facts; rule sets keyed by frontend id).
4. New TS-stack crate `conform-frontend-typescript` (implements
   `Frontend`, `id() = "ts-tsc"`, delegating extraction to the bridge) +
   `conform-cli-typescript` (bin **`conform-typescript`**, mirroring
   conform-cli's run/check surface, reading the consumer's `conform.toml`
   with a `[typescript]` section: roots, seam name, domain/exempt dirs).
5. Fixture-driven tests in the TS stack: a dirty fixture tree (an `any`, an
   unchecked `as`, a `@ts-ignore`, an over-budget file, a sibling-cell
   import, an `as const` + a string-literal `"any"` that must NOT fire)
   → exact findings + exit 1; a clean fixture → 0. The Ф6 brief
   (`tools/conform-frontend-typescript.md`) status flips
   specified → shipped, its §3 open question answered by D1 and its §5
   honest note rewritten.
6. Acceptance: package tests green; vendored copies re-synced; floor green.
7. Commits: `feat(typescript-ai-native): ship the compiler-api fact
   extractor`, `feat(conform): typescript rule set in the neutral core`,
   `feat(typescript-ai-native): ship conform-typescript (ts-tsc frontend)`.

## 6. Phase 3 — specmap for TypeScript (tsscan + bin)

1. specmap-core (authored home): the **scanner seam** (D3) — a trait the
   index builder consumes, rscan refactored to be its built-in Rust
   implementation, dispatch by extension. Behaviourally identical for
   Rust trees: vibevm's `specmap --check` stays byte-stable (the Ph1
   acceptance re-run proves it).
2. TS-stack crate `specmap-scan-typescript`: the seam's TS
   implementation over `ts-extract-bridge` (the SAME extractor run
   already carries the §9 JSDoc markers). Plus
   `specmap-cli-typescript` (bin **`specmap-typescript`**:
   mint/`--check`/`--gate`, mirroring specmap-cli, injecting the TS
   scanner into the neutral core).
3. Tests: fixture TS tree with tagged/untagged exports → index golden,
   orphan ratchet fires, `@deviates` without reason = finding; mixed-tree
   test (one .rs + one .ts root) proves both scanners coexist in one
   index; bridge-replay test keeps the Rust side node-free.
4. Acceptance: floor green; vibevm index byte-stable; `specmap-typescript
   --check` reproduces its golden byte-for-byte.
5. Commits: `refactor(specmap): scanner seam in the neutral core`,
   `feat(typescript-ai-native): ship specmap-typescript (JSDoc via ts-tsc)`.

## 7. Phase 4 — the `discipline-typescript` umbrella

1. New TS-stack crate `discipline-cli-typescript` (bin
   **`discipline-typescript`**): `init` (generates the six artifacts in
   their TS shape — conform.toml `[typescript]`, specmap.toml with
   discovered `[[external_specs]]`, both baselines, both registries — the
   Rust init generalised, shared helpers vendored or duplicated-with-test,
   NOT cross-linked to discipline-cli), `floor` (D7's four steps),
   `conform` / `specmap` passthroughs, `trace` (same specmap-core explain —
   works over the mixed index).
2. `test-gate`/`tripwire`/`health`/`fast-loop`/`codemod` subcommands print
   a named not-yet-shipped message citing this plan (visible deferral, not
   absence).
3. Acceptance: `discipline-typescript init && floor` green on a scratch TS
   fixture (tsc step per Phase 0 probe result); package tests green.
4. Commit: `feat(typescript-ai-native): ship the discipline-typescript
   umbrella`.

## 8. Phase 5 — skills twins, boot snippet, card statuses

1. `[[skill]]` × 2 in the TS manifest (D8) + skill trees under
   `spec/skills/`; dogfood `vibe skill install` (projections land,
   gitignored as before).
2. `20-stack-typescript-ai-native.md` boot snippet gains the shipped-
   toolchain block (mirroring the Rust one: bins, install recipe, wiring
   pointer) — GUIDE gets §15-equivalent wiring/sweep sections (the §13/§14
   analog the Rust guide got).
3. Cards INDEX: statuses flip ONLY where a runnable checker now exists
   (F — conform-typescript bans; the file-budget/cell rows; E stays
   specified until the fast-loop twin ships; I stays WISH).
4. Acceptance: `vibe check` clean; skill projection verified; floor green.
5. Commit: `docs(typescript-ai-native): consumer front door - skills,
   boot toolchain block, card statuses`.

## 9. Phase 6 — the demo project + frozen acceptance

1. Author `research/ts-demo` per D4; README states purpose (pilot-lite for
   the TS stack; NOT the VibeVM TS pilot) + bootstrap recipe.
2. Manual walk, recorded in the WAL: `vibe install` from the in-repo
   registry → `discipline-typescript init --namespace ts-demo` → tag →
   `specmap-typescript` mint/check (discipline-core citations RESOLVE
   through the demo's vibedeps — the cross-package resolution proof, TS
   edition) → `floor` (tsc step per probe) → `trace explain`.
3. Freeze the walk as a hermetic package test in the TS stack
   (`crates/discipline-cli-typescript/tests/fresh_ts_project.rs`, engine
   calls, no npm/node dependency — the node-side steps are the manual
   walk's job, same split as the Rust twin).
4. Acceptance: demo floor green (tsc modulo the probe); hermetic test
   green; vibevm floor untouched (`research/` inert).
5. Commits: `feat(research): ts-demo - the typescript discipline walking
   skeleton`, `test(typescript-ai-native): freeze the fresh-ts-project
   bootstrap`.

## 10. Phase 7 — DEBT.md / INTENT.md generated views

1. `discipline-rust ledger render [--check]`: reads
   `discipline/registry/{debt,intent}.json`, writes `discipline/DEBT.md` +
   `discipline/INTENT.md` — deterministic (stable ordering, a "generated
   by; do not edit" header naming the source + regen command); `--check` =
   regenerate-and-compare, exit 1 on drift. debt.json's own header already
   promises "Human view: DEBT.md, generated from this file" — this makes
   the promise true. Grouping: by disposition/state then severity then id;
   each entry renders id, kind, severity, one-line, spec refs.
2. Generate both views for vibevm, commit them; add `ledger render --check`
   to the sweep skill's item list (health-adjacent, not floor — the floor
   stays fast).
3. TS twin: NOT duplicated — the subcommand lives in discipline-cli (Rust
   umbrella) but operates on any project root; the TS umbrella's deferral
   message for `ledger` points at it. (Registries are language-neutral
   JSON; one renderer is enough until the TS umbrella grows its own.)
4. Acceptance: views committed + `--check` green; hand-edit → red (prove
   once, revert); floor green.
5. Commit: `feat(discipline-cli): ledger render - the DEBT/INTENT views`.

## 11. Phase 8 — `vibe trace` product alias

1. vibe-cli subcommand `trace` per D5 (args passed through verbatim;
   spawn-failure prints the install recipe; exit code propagated).
   RUNTIME-GUIDE gains the three-line note.
2. Acceptance: `vibe trace explain spec://vibevm/…` == `discipline-rust
   trace explain …` output on this tree; missing-binary path exercised in
   a test with a scrubbed PATH; `vibe check` clean; floor green.
3. Commit: `feat(cli): vibe trace - delegating alias over discipline-rust`.

## 12. Phase 9 — PROP-025: vibe-native binary delivery (spec + implementation)

Owner-expanded (2026-07-07): the PROP lands AND §§2–5 are implemented in
this campaign (D6 carries the design decisions). Four sub-phases, each a
safe stop:

**9a — author the PROP** (`spec/modules/vibe-workspace/PROP-025-binary-delivery.md`):

- §1 problem: code-bearing packages ship bins consumers must
  `cargo install --path` by hand (GUIDE §13); n stacks × m tools = manual
  PATH management vibe already knows how to do for itself (PROP-019).
- §2 manifest surface: `[[binary]]` (name, crate path, required
  toolchain) declared by code-bearing packages.
- §3 build step: post-materialise, consent-gated like hooks (PROP-020
  consent precedent — an install-time build EXECUTES build.rs/
  proc-macros), `cargo build --release` in the slot; artifacts are
  slot-resident (outside the shippable tree per PROP-024 §2.2, so
  hashing is untouched and slot refresh = staleness for free).
- §4 dispatch: dumb shims in a global bin dir (Windows `.cmd` pair —
  the PROP-015 lesson) delegating to `vibe bin exec` (walk up to
  `vibe.lock` → the pinned slot's artifact → build-if-missing → exec;
  outside a project: newest from `~/.vibevm/bins.toml`); PROP-019
  shim-dir reconciliation.
- §5 staleness/offline: slot refresh invalidates; network-honest failure
  mode (cargo needs crates.io unless the cache is warm); `cargo install
  --path` stays as the degraded manual path.
- §6 cross-package path-deps at materialise time — the D1 (α) mechanism
  specced as the companion feature (manifest path rewriting + its
  interaction with shippable-tree hashing), **explicitly staged as v2,
  NOT implemented here**.
- §7 uninstall/GC + `vibe vars` reporting; §8 security posture (consent
  recorded, scope discipline); §9 the v1 cut (what 9b–9d ship vs §6/§7's
  GC deferrals).

**9b — manifest + build + consent:** `[[binary]]` parsing in vibe-core's
manifest model (schema'd, `deny_unknown_fields` posture consistent with
the existing tables); the post-materialise build step in vibe-install
(runs after skill projection, consent flow shared with PROP-020's,
`--assume-yes` honoured, decision recorded per (package, version,
content-hash)); toolchain probe (`cargo --version`) with an actionable
absent-toolchain error. Mock-source + fixture-package tests (a tiny
`[[binary]]` package with a hello-world bin) proving: consent asked
once, build runs, artifact lands in the slot, re-install with unchanged
slot skips the rebuild.

**9c — dispatch + shims:** `vibe bin` subcommand family per D6 (`exec`,
`list`, `path`, `sync`); `bins.toml` bookkeeping; shim writer (sh + .cmd
pair, `cmd /c` wrapping, idempotent `sync`); lockfile-walk resolution
with the newest-fallback. Tests: sandboxed-HOME integration test —
`sync` creates shims, `exec` resolves through a fixture project's
lockfile, exit codes pass through; Windows shim exercised on this box.

**9d — dogfood + docs:** run `vibe bin sync` on vibevm itself —
`discipline-rust`, `conform-rust`, `specmap-rust`, and (post-Phase 4)
`discipline-typescript`, `conform-typescript`, `specmap-typescript` get
shims; the three discipline packages' manifests gain their `[[binary]]`
tables (that content edit re-materialises via PROP-011 §2.6 mutability);
GUIDE §13 + the boot toolchain blocks + RUNTIME-GUIDE updated so
`vibe bin sync` is the primary recipe and `cargo install --path` the
fallback; `vibe vars` reports the bin dir.

Commits: `docs(spec): PROP-025 - vibe-native binary delivery`,
`feat(install): consent-gated [[binary]] builds (PROP-025 s3)`,
`feat(cli): vibe bin - shims and lockfile dispatch (PROP-025 s4)`,
`docs(packages): declare the discipline binaries + rewire the recipes`.
Acceptance: specmap ingests the new anchors clean; the fixture e2e green;
`vibe bin exec discipline-rust -- floor --path .` green on vibevm from a
shim; floor green.

## 13. Phase 10 — machine quirks into `spec/boot/90-user.md` (owner file)

`90-user.md` is owner-owned; the owner sanctioned edits on plan review
2026-07-07 ("меняй как хочешь"). Append a `## Machine quirks (this box)`
section carrying the five DISCIPLINE-SWEEP-v0.2 §3 items verbatim; the
sweep manual keeps its copy with a pointer note flipped to
"boot-resident since this campaign".
Commit: `docs(boot): adopt the machine-quirks list into the user snippet`.

## 14. Phase 11 — vibe-registry split + campaign checkpoint

1. `crates/vibe-registry/src/lib.rs` is at 599/600. Split by the module-
   grain precedent (the seven-file split of 2026-06-27): extract the
   largest coherent cell(s) into named modules (inspect at execution;
   candidates by shape: fetch/clone orchestration vs manifest parsing vs
   cache bookkeeping), each ≤600 with headroom, re-exports keep the seam
   stable, no behavior change (`cargo test -p vibe-registry` +
   characterization untouched).
2. Re-materialise vibedeps if any package content moved since Phase 5;
   regen specmap; full floor + the shipped `discipline-rust floor` +
   `discipline-typescript floor` on the demo.
3. WAL + CONTINUE.md checkpoint per the standing convention.
4. Commits: `refactor(registry): split lib.rs into module-grain cells`,
   `docs(wal)/docs(continue)` checkpoint pair.

## 15. Risks

- **Network flakiness** — the box refused both SSH endpoints and later
  authenticated on both within one day; mitigated by the campaign's
  zero-new-crate-deps posture (the D2 extractor is Node-side, so no
  Rust build path grows a crates.io need), the local npm cache
  (`typescript` resolves offline), and per-step re-probes for anything
  network-facing. Worst case the demo's tsc/extractor steps are
  red-pending-network, recorded, everything else lands.
- **Vendor drift** — impossible while `sync-engines --check` is in
  self-check (Phase 1.5); the gate is proven red once before trust.
- **Index instability across the moves** — the external_specs root bump is
  called out (§1); Phase 1 acceptance pins 0 dangling; every phase regens
  specmap before its floor check.
- **Extractor/protocol drift** — the extractor runs against whatever
  `typescript` version the consumer has; it is written against the
  stable public Compiler API surface only, carries `protocol_version`,
  and the bridge treats a mismatch as its own error class. The
  bridge-replay tests keep Rust-side coverage independent of node; the
  Phase 0 spike de-risks every promised fact class before the protocol
  freezes.
- **Node subprocess on Windows** — spawn quirks (`cmd /c`, path forms)
  are the PROP-015 lesson, applied from the start; the demo walk and the
  9c shim test both run on this box.
- **Scope creep in PROP-025** — the implemented surface is §§2–5 exactly;
  §6 (cross-package path-dep rewriting) and §7's GC live in the PROP as
  v2 and are NOT implemented; anything beyond goes through the owner.
- **Windows path depth/casing** in vendor copies and the demo walk —
  Phase 0 spike covers the build topology; the demo walk reuses the manual-
  walk script idioms (inline paths, real exit codes).

## 16. Campaign acceptance (what "done" looks like)

```sh
bash tools/self-check.sh; echo "EXIT=$?"        # exit 0, now incl. sync-engines --check
cargo xtask specmap --check                      # 0 suspects / 0 warnings / 0 dangling
cargo xtask conform check                        # 0 findings
discipline-rust floor --path .                   # green (vibevm, Rust floor)
discipline-rust ledger render --check            # DEBT.md / INTENT.md fresh
vibe trace explain "spec://vibevm/common/PROP-000#commits"   # delegates, renders
vibe bin sync && vibe bin list                   # shims for all declared [[binary]] tools
vibe bin exec discipline-rust -- floor --path .  # dispatch through the shim path: green
cd research/ts-demo && vibe install --assume-yes # from the in-repo registry (builds bins, consented)
npm install                                      # typescript devDep (tsc + the extractor)
discipline-typescript floor                      # tsc → node --test → conform → specmap: green
discipline-typescript trace explain "spec://ts-demo/PROP-001#req-…"
# spec/modules/vibe-workspace/PROP-025-binary-delivery.md exists, anchors resolve
# spec/boot/90-user.md carries the quirks; wc -l crates/vibe-registry/src/lib.rs < 550
```

All commits local; mirror and registry publish stay held for the owner's
word. Both are EXECUTABLE now that the network is back (SSH to both hosts
re-verified 2026-07-07) — the hold is policy, not capability.
