# Self-Sufficiency Plan v0.1 — make the discipline packages complete, orthogonal, and consumer-ready

**status: PLANNED · not started · the convergence of two audits (package self-sufficiency + operational-procedure detachment) · executes in a fresh session from cold context**

> **Read-first / boot.** This plan is written to be executed cold. Boot the
> normal way first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files →
> `spec/WAL.md` → `CONTINUE.md`), then read this whole file. It is
> self-contained: the current-state facts (with file:line), the owner's
> directives, the design decisions and their rationale, the phase-by-phase
> recipes, the risks, and the acceptance gates are all here. The git log is
> the authoritative per-item record; the WAL is the canonical living state
> and supersedes this plan if they diverge.

---

## 0. Why this exists (the reframe)

PROP-024 (code-bearing packages) and the two relocation campaigns (conform
Ф1–Ф7, traceability Ph0–Ph4) made `stack:org.vibevm.ai-native/rust-ai-native` ship the
whole Rust verification *engine* — `conform-rust` + `specmap-rust`, policy as
data, the package tracing and gating itself. Two follow-up audits
(2026-07-06) then asked the next question: **can a project that is not vibevm
actually live on this discipline?** The answer today is *no*, for eight
verified reasons (§2, F1–F8), plus a second finding: the two procedures a
consumer will actually *run* — terraforming an existing codebase and the
recurring discipline sweep — exist only as vibevm-internal documents wired to
vibevm-internal xtask tooling (§2, T1–T4).

**The goal of this campaign is the final form of the discipline packages:**
a fresh project (or an existing brownfield one) installs
`stack:org.vibevm.ai-native/rust-ai-native`, and from that moment can

1. **bootstrap** — generate its policies and registries (`discipline-rust
   init`) and run the full verification floor with one shipped command
   (`discipline-rust floor`);
2. **terraform** — adopt the discipline on an existing codebase by following
   a *shipped* skill (`terraform-rust`) that walks the BROWNFIELD protocol;
3. **sweep** — run the recurring conformance sweep (daily or more often) via
   a *shipped* skill (`discipline-sweep`) backed by shipped tools
   (`health`, `test-gate`, `tripwire`, `trace`, `fast-loop`, `codemod`);
4. **trace** — have its own `spec://` namespace, and *resolve* the discipline
   spec units it cites, because the normative mechanism specs ship in
   `flow:org.vibevm/discipline-core` and the resolver reads installed
   packages' spec trees.

vibevm remains the dev repo of the packages and their first consumer, but
nothing a consumer needs may live only in vibevm.

## 1. The owner's directives (2026-07-06, binding for this plan)

1. **Move to `discipline-core` now, not at the TypeScript pilot.** Everything
   the audits identified as language-neutral *content* — the four mechanism
   specs, the sweep playbook, the campaign form, the WAL convention — moves
   into `flow:org.vibevm/discipline-core` in this campaign. (The *code*
   consolidation — a neutral engine layer under the Rust crates — remains
   deferred: that is a research-grade decomposition blocked by the
   proc-macro edge, per TRACEABILITY-RELOCATION-PLAN §1, and the owner has
   not reopened it. Documents move now; crates stay in the Rust stack.)
2. **WAL is optional but preferred.** The shipped procedures must not
   *require* a WAL. Every procedure that references it gets two branches:
   "if the project keeps a WAL (recommended, see the shipped convention doc):
   record the resume pointer / standing line there" and "if not: the
   procedure's own report is the resume pointer." A new core doc specifies
   the convention itself so a consumer can *choose* it.
3. **No xtask assumption.** The xtask pattern is vibevm's consumer choice,
   not part of the discipline. Every tool a procedure needs ships as a
   package binary/subcommand. vibevm keeps thin xtask shims for its own DX,
   but the shipped docs, skills, and engine messages never mention xtask.

## 2. Current-state facts (verified 2026-07-06; do not re-discover)

Audit 1 — package self-sufficiency (F1–F8):

- **F1 — the normative specs live in vibevm.** `spec/discipline/` holds
  `ENGINE-CONFORM-v0.1.md`, `PROP-014-specmap-bidirectional-traceability.md`,
  `BROWNFIELD-PROTOCOL-v0.1.md`, `LEDGER-INTENT-v0.1.md` (+ `README.md`).
  The package's own tags cite them: 79 `spec://vibevm/…` occurrences across
  28 package files; the real *normative* targets are exactly those four docs.
  The package self-trace is `--gate`-only (coverage, not resolution) purely
  because of this. The vibevm-side citation cascade is small: **7 code
  sites** (`crates/vibe-registry/src/git_package_registry/fetch.rs:267`,
  `crates/vibe-publish/src/redirect_sync.rs:246`,
  `crates/vibe-cli/src/main.rs:196`,
  `crates/vibe-resolver/src/activation.rs:261`,
  `crates/vibe-registry/src/lib.rs:416`,
  `crates/vibe-cli/src/commands/workspace/publish.rs:493`,
  `crates/vibe-index/src/cli/stop.rs:26` — all
  `…/ENGINE-CONFORM-v0.1#…`) plus historical docs and the regenerable
  `specmap.json`. `spec/discipline/README.md` also carries a **stale claim**
  ("ENGINE-CONFORM is an edge-less spec unit by design; scope! edges were
  dropped") — Ph4 (`dee0321`) restored those tags.
- **F2 — `SPEC_PACKAGE = "vibevm"` is a compile-time constant**
  (`…/specmap-core/src/lib.rs:44`, used at `mdspec.rs:269`). Every consumer's
  spec units would be minted as `spec://vibevm/…`. PROP-014 §7.1 records the
  group-qualified-URI deferral this must now partially pay.
- **F3 — no consumer entry point.** Grep for
  `conform-rust|specmap-rust|cargo run` over the package's `spec/` (boot
  snippet, GUIDE §0–12, nine cards, INDEX): **zero matches**. No package
  README. The only places the run commands exist are vibevm files
  (`tools/self-check.sh`, `CONTINUE.md`).
- **F4 — no policy bootstrap.** `conform-cli::load_config` →
  `Config::load` *fails* on an absent `conform.toml` while the `Config` doc
  comment promises "an absent file yields a usable default"
  (`…/conform-core/src/config.rs:19-20,105-109`). `specmap-rust` without a
  `specmap.toml` scans the hardcoded vibevm-shape default
  `["crates/*", "xtask"]` (`…/specmap-core/src/config.rs:60`;
  `conform-core/src/config.rs:70` same) — a `src/` single-crate project scans
  *nothing* and is vacuously green. No `init`, no templates, no shipped floor
  script.
- **F5 — shipped engine messages recommend vibevm commands.**
  `…/specmap-core/src/index.rs:302` ("run `cargo xtask specmap` first") and
  `:316` ("Run `cargo xtask specmap`, review the drift…"), plus xtask
  references in shipped doc comments (`specmap-core/src/lib.rs:1`,
  `specmark/src/lib.rs:7,119`, `ledger.rs:8`, `rscan.rs:287`, cli lib docs).
- **F6 — the `every_crate_is_gated_or_exempt` invariant lives only in the
  consumer** (`xtask/src/conform.rs:40-102`, a `#[cfg(test)]` test). The
  engine does not enforce it; a consumer never gets it.
- **F7 — binary delivery is unshaped.** The vibedeps slot ships sources
  (8 crates + `Cargo.toml` + `Cargo.lock`); running means a long
  `cargo run --manifest-path vibedeps/… -p conform-cli --bin conform-rust -- check`
  that is written down nowhere in the package; building in the slot drops
  `target/` into the consumer's git-tracked `vibedeps/`. No PROP-020 hooks
  declared; no vibe binary-management feature.
- **F8 — the JTD schema and codegen live in vibevm.**
  `schemas/specmap.jtd.json` is the source of the generated Specmap types;
  the generated module says "Generated by `cargo xtask codegen`. DO NOT
  EDIT." The package cannot evolve its own wire types.
- The engine itself is clean where it matters: `ErrorMessageCitesReq` only
  checks for a `spec://` substring
  (`…/conform-core/src/rules/diagnostics.rs:260`), findings cite
  package-relative `discipline://rust-ai-native/cards/…` URIs, both CLIs are
  project-root-driven (`--path`, policy at the target root).

Audit 2 — operational procedures (T1–T4):

- **T1 — the terraforming methodology is `BROWNFIELD-PROTOCOL-v0.1`**
  (inventory-not-gate, the three registries, xfail-strict test gate, spec
  lifecycle, characterization, monotone frontier) — language-neutral in
  content, vibevm-hosted (part of F1). The campaign *form*
  (plan → BASELINE → PREDICTIONS → LOG → REPORT) exists only by example in
  `spec/terraforms/*` + `terraform/*`. `03-RAID-PLAYBOOK.md` (already in
  discipline-core) §3 cites `PLAYBOOK-TERRAFORM-VIBEVM-v0.2`, which no
  package ships (legacy artifact).
- **T2 — the sweep manual is three layers in one file.**
  `spec/terraforms/DISCIPLINE-SWEEP-v0.1.md` declares itself
  "vibevm-specific" and mixes (1) the portable standing-sweep template
  (tiers 0–3, cadence, collector-first, gates-are-the-floor, the
  WISH→census→Rule ladder), (2) portable Rust idioms (tests-out, doctest
  idiom set, newtype cascade), (3) the vibevm instance (snapshot numbers,
  crate names, `terraform/health/` paths) **and this machine's quirks** (§1:
  PS 5.1, Git Bash, UAC — machine-scoped, not even project-scoped).
- **T3 — nine discipline tools, two shipped.** xtask subcommands
  (`xtask/src/main.rs:67-190`): `conform`, `specmap` (thin shims over the
  package ✓); `test-gate` (engine **already** in `specmap-core::testgate`,
  nextest driver in `xtask/src/test_gate.rs`), `tripwire` (engine in
  `specmap-core::tripwire::evaluate`, git driver in `xtask/src/tripwire.rs`),
  `trace` (engine in `specmap-core::explain`, driver `xtask/src/trace.rs`;
  main.rs note "promotion to `vibe trace` is a Phase 4 decision"),
  `fast-loop` (`xtask/src/fast_loop.rs`, cargo per-cell), `health`
  (`xtask/src/health.rs` — imports only `conform_core` +
  `conform_frontend_rust` + serde_json + the optional `crate::mirror` probe:
  the move is nearly free), `codemod` (`xtask/src/codemod.rs`, the
  scaffold-I pilot); `codegen`/`check-codegen`, `mirror` (vibevm-specific,
  stay). Default registry paths are vibevm topology:
  `terraform/registry/tests-baseline.json`, `terraform/registry/debt.json`,
  `terraform/health/latest.json`.
- **T4 — the delivery mechanism for procedures already exists.**
  PROP-015 §2.6 skill materialisation + §2.8 `[[skill]]` declarations
  (`name`, `path`, `include` globs) project package skills into agent skill
  directories; `vibe skill` is a live CLI command
  (`crates/vibe-cli/src/main.rs:91`). No discipline package declares any
  skill today. (Also noted: `DEBT.md`/`INTENT.md` "generated human view"
  from BROWNFIELD §3 has **no generator** — manual today; named a deferred
  gap, §10.)

Environment facts:

- Packages live at `packages/org.vibevm/{discipline-core,rust-ai-native,typescript-ai-native}/v0.2.0/`;
  vibedeps slots are `vibedeps/{flow-discipline-core,stack-rust-ai-native,stack-typescript-ai-native}/0.2.0/`
  (legacy slot naming — the slot name comes from the lockfile, discover the
  0.3.0 slot names from `vibe.lock` after the first install rather than
  assuming). Project `vibe.toml` requires `^0.2.0` for all three.
- `rust-ai-native` `[requires]`: `flow:org.vibevm/discipline-core = "^0.2"`.
- Mirror is HELD (13 commits ahead as of plan authoring); GitVerse SSH was
  refused on 2026-07-06 — re-check reachability before any mirror talk.

## 3. Target end-state (the final form)

```
packages/org.vibevm/discipline-core/v0.3.0/
├─ vibe.toml                       version 0.3.0
├─ README.md                       (exists)
└─ spec/
   ├─ 00-MANIFESTO.md              §8 map updated (mechanisms + new playbooks)
   ├─ 01-PATTERN-CARD-FORMAT.md
   ├─ 02-EXECUTABLE-SCAFFOLDS.md
   ├─ 03-RAID-PLAYBOOK.md          §1.4 WAL optional-preferred; §3 cites 05, not the legacy playbook
   ├─ 04-SWEEP-PLAYBOOK.md         NEW — the portable standing-sweep template
   ├─ 05-CAMPAIGN-FORM.md          NEW — plan/BASELINE/PREDICTIONS/LOG/REPORT form
   ├─ 06-WAL-CONVENTION.md         NEW — WAL/CONTINUE convention, optional but preferred
   ├─ mechanisms/                  NEW — moved from vibevm spec/discipline/
   │  ├─ ENGINE-CONFORM-v0.1.md
   │  ├─ PROP-014-specmap-bidirectional-traceability.md
   │  ├─ BROWNFIELD-PROTOCOL-v0.1.md
   │  └─ LEDGER-INTENT-v0.1.md
   ├─ appendix/ …                  (exists)
   ├─ legacy-projections/ …        (exists)
   └─ boot/10-flow-discipline-core.md   names the new docs

packages/org.vibevm.ai-native/rust-ai-native/v0.3.0/
├─ vibe.toml                       0.3.0; requires discipline-core ^0.3; [[skill]] ×2
├─ README.md                       NEW — what ships + how to run it (F3/F7)
├─ Cargo.toml                      9 workspace members (+ discipline-cli)
├─ schemas/specmap.jtd.json        NEW HOME (F8; moved from vibevm schemas/)
├─ specmap.toml                    self-trace policy (namespace = "rust-ai-native")
├─ crates/
│  ├─ conform-core                 + load_or_default + tree-invariant (F4/F6)
│  ├─ conform-frontend-rust
│  ├─ conform-cli                  bin conform-rust (unchanged surface)
│  ├─ env-audit
│  ├─ specmap-core                 + Config.namespace + [[external_spec]] (F2)
│  ├─ specmap-cli                  bin specmap-rust (unchanged surface)
│  ├─ specmark / specmark-grammar  (bootstrap pair, unchanged)
│  └─ discipline-cli               NEW — bin `discipline-rust`:
│        init | floor | conform | specmap | trace | test-gate |
│        tripwire | health | fast-loop | codemod
└─ spec/
   ├─ boot/20-stack-rust-ai-native.md   + "the shipped toolchain" block
   ├─ rust/GUIDE-AI-NATIVE-RUST.md      + §13 wiring guide + Rust sweep idioms
   ├─ cards/ …                          Band-3 checker statuses reflect shipped tools
   └─ skills/
      ├─ terraform-rust/SKILL.md        NEW — brownfield adoption procedure
      └─ discipline-sweep/SKILL.md      NEW — the recurring sweep procedure
```

Consumer experience (the acceptance scenario, §9): `vibe install` → one
`cargo install --path vibedeps/<rust-ai-native-slot>/crates/discipline-cli`
(or `cargo run --manifest-path …`) → `discipline-rust init` →
`discipline-rust floor` green → tags resolve against the shipped mechanism
specs via `[[external_spec]]` → `/terraform-rust` and `/discipline-sweep`
skills available in the agent.

vibevm end-state: `spec/discipline/` reduced to a pointer README; retagged
onto `spec://discipline-core/mechanisms/…`; `specmap.toml` gains
`namespace = "vibevm"` + `[[external_spec]]` entries and **resolves** the
package-hosted units (0 dangling — vibevm becomes the first proof of
cross-package resolution); living registries at `discipline/registry/` +
`discipline/health/` + `discipline/golden/`; xtask shims delegate to
`discipline_cli`; the sweep manual is a thin v0.2 instance over the core
playbook.

## 4. Design decisions

- **D1 — one umbrella binary, `discipline-rust`.** New `discipline-cli`
  crate (lib + bin). Subcommands: `init`, `floor`, `conform <check|freeze>`,
  `specmap [--check|--gate]`, `trace <…>`, `test-gate`, `tripwire`,
  `health`, `fast-loop`, `codemod <…>`. `conform-rust` / `specmap-rust`
  binaries stay (narrow engines, compatibility); discipline-cli depends on
  conform-cli + specmap-cli as libraries. `floor` is the portable
  self-check: fmt → test → clippy → conform → specmap → (test-gate if a
  baseline exists) → (fast-loop when asked), `--keep-going`/`--quiet`,
  prints per-gate config status ("conform.toml: found / defaulted") so a
  defaulted policy can never masquerade as a configured green.
- **D2 — namespace is config, not constant.** `specmap.toml` gains
  `namespace = "<name>"`. With a `specmap.toml` present the field is
  REQUIRED (`deny_unknown_fields` stays); with no `specmap.toml` the default
  config uses namespace `"project"` and the tools warn. `SPEC_PACKAGE`
  const is deleted; vibevm sets `namespace = "vibevm"` in the same commit so
  its URIs and `specmap.json` are byte-identical.
- **D3 — cross-package resolution via `[[external_spec]]`.**
  `specmap.toml` gains repeated `[[external_spec]] { namespace, root }`
  entries. mdspec scans each external root with that namespace and feeds the
  units into **resolution only** (dangling suppression + queries), never
  into the committed `specmap.json` (which stays the project's own content —
  byte-identity is preserved). `discipline-rust init` generates entries by
  scanning `vibedeps/*/*/vibe.toml` (`[package] name` → namespace,
  `<slot>/spec` → root, skipping packages with no `spec/`). This partially
  pays the PROP-014 §7.1 deferral; the URI grammar is unchanged
  (`spec://<namespace>/<docpath>#<anchor>`).
- **D4 — mechanism specs live in `discipline-core/spec/mechanisms/`**, URIs
  become `spec://discipline-core/mechanisms/<DOC>#<anchor>` (anchors
  unchanged, so the retag is a pure prefix rewrite). The four docs move
  now (owner directive 1). Rust-specific passages inside PROP-014 (tag
  syntax) stay but get a one-line "per-language projection" note.
- **D5 — registries convention over configuration.** Default paths become
  `discipline/registry/{tests-baseline,debt,intent}.json`,
  `discipline/health/latest.json`, `discipline/golden/` — overridable by
  flags. vibevm migrates its living instances (`git mv terraform/registry
  discipline/registry`, `terraform/health` → `discipline/health`,
  `terraform/golden` → `discipline/golden`); the historical campaign records
  stay under `terraform/`.
- **D6 — binary delivery = documented cargo, not a new vibe feature.**
  The README documents two supported forms: (a)
  `cargo install --path vibedeps/<slot>/crates/discipline-cli` (puts
  `discipline-rust` on PATH; same for the two engine bins), (b)
  `cargo run --manifest-path vibedeps/<slot>/Cargo.toml -p discipline-cli
  --bin discipline-rust -- …` (zero-install). Plus a `.gitignore` note for
  `vibedeps/**/target/`. A vibe-native binary manager (shims on install) is
  a named deferral (§10) — a future PROP, not this campaign.
- **D7 — WAL optional-preferred** (owner directive 2). New
  `06-WAL-CONVENTION.md` specifies the two-file convention (WAL = volatile
  current-state checkpoint, rewritten not appended; CONTINUE = cold-resume
  snapshot; the 24h staleness rule). Every shipped procedure branches:
  *with WAL* — write the resume pointer / bump the standing line there;
  *without* — the procedure's report (sweep output §8-style, campaign LOG)
  is the resume pointer, and the skill says to commit it.
- **D8 — versions bump to 0.3.0 first.** Both packages `git mv v0.2.0 →
  v0.3.0` at Phase 0 (semver: new features + URI break), so every later
  `git mv` lands in the final home. typescript-ai-native stays 0.2.0
  (untouched this campaign). Consumer `vibe.toml` requires bump accordingly.
- **D9 — the JTD schema moves to the package** (`schemas/specmap.jtd.json`);
  vibevm's `xtask codegen` repoints its *input* path (codegen output already
  routes to the package). Regeneration stays a maintainer dev-op in the dev
  repo — honest and recorded in the package README; the consumer never needs
  it.
- **D10 — the gated-or-exempt invariant moves into the engine.** New
  `conform_core::config::validate_against_tree(root, &config)` implementing
  exactly `xtask/src/conform.rs:40-102` (duplicates, both-listed,
  unclassified-on-disk, phantom-entry, empty-reason); `run_check` calls it
  and fails on violations. The xtask test becomes a thin call (kept as
  vibevm's regression net).
- **D11 — engine messages are self-referential.** Shipped runtime strings
  and doc comments name `specmap-rust` / `conform-rust` /
  `discipline-rust` ("or your project's wrapper"), never xtask (owner
  directive 3).
- **D12 — skills ship in the stack, template in the core.** The two
  SKILL.md files live in rust-ai-native (they invoke Rust commands); they
  cite the core playbooks (04/05 + mechanisms) for the method. A future
  TypeScript stack ships its symmetric pair.
- **D13 — test fixtures use a neutral namespace.** Test-only URIs in the
  package (`spec://vibevm/x`, `spec://vibevm/T#req-t`, `spec://vibevm/A`,
  `spec://vibevm/test/DOC` …) retag to `spec://project/…` alongside the
  namespace work, so the shipped crates' test suite carries no vibevm
  flavor. Doc-comment *examples* that cite real vibevm spec units (e.g.
  specmark's PROP-003 examples) may stay — they are illustrative citations
  of a real consumer, not machinery.

## 5. Phases

Each phase lands its own topic-grouped commits (Rule 3); the floor —
`bash tools/self-check.sh; echo "EXIT=$?"` → 0, `cargo xtask specmap --check`
clean, `cargo xtask conform check` clean, `vibe check` 0/0/0 — must be green
at every phase boundary. Nothing is mirrored during the campaign; the mirror
stays HELD for the owner's word.

### Phase 0 — Baseline + version bump (machinery, no content change)

1. Verify the starting floor green; record the live specmap numbers.
2. Check whether 0.2.0 of either package is published in the `vibespecs`
   registry (`vibe show` / registry query / github.com/vibespecs listing).
   Published or not, the bump proceeds (URI break + new binaries warrant it);
   if published, note that 0.2.0 must remain immutable there.
3. `git mv packages/org.vibevm.ai-native/rust-ai-native/{v0.2.0,v0.3.0}` and
   `git mv packages/org.vibevm/discipline-core/{v0.2.0,v0.3.0}`.
4. Package `vibe.toml`s: `version = "0.3.0"`; rust-ai-native `[requires]`
   `"flow:org.vibevm/discipline-core" = "^0.3"`.
5. Project `vibe.toml`: `"flow:org.vibevm/discipline-core" = "^0.3.0"`,
   `"stack:org.vibevm.ai-native/rust-ai-native" = "^0.3.0"` (typescript stays ^0.2.0).
6. Repoint every dev-repo path that names `v0.2.0`:
   `grep -rn 'rust-ai-native/v0\.2\.0\|discipline-core/v0\.2\.0' --include='*.toml' --include='*.sh' --include='*.rs' .`
   — expect at least: root `Cargo.toml` `[workspace.dependencies]` package
   paths, `tools/self-check.sh` `PKG_MANIFEST`/`PKG_DIR`, `xtask/src/codegen.rs`
   specmap output route. (Doc/plan mentions of v0.2.0 are history — leave.)
7. `cargo run -p vibe-cli -- install --registry packages --assume-yes` —
   materialises 0.3.0 slots, regenerates `spec/boot/INDEX.md` (verify its
   entries point at the 0.3.0 slots). `git rm -r` the orphaned 0.2.0 slots if
   install does not remove them; discover the actual 0.3.0 slot names from
   `vibe.lock` (do not assume the legacy `stack-rust-ai-native` shape).
8. **Acceptance:** floor green; `vibe check` 0/0/0; boot INDEX names 0.3.0
   slot paths; no `v0.2.0` references outside history/docs.

**Commits:** `build(packages): bump the discipline packages to 0.3.0` +
`build(deps): re-materialise vibedeps at 0.3.0`.

### Phase 1 — Engine generalisation (F2, F4, F5, F6; vibevm byte-identical)

All edits inside the package crates + minimal consumer-side config additions.

1. **Namespace (F2).** `specmap_core::Config` gains `pub namespace: String`.
   Load semantics: file present → field required (a missing field is a
   config error with a fix hint); file absent → default config with
   `namespace = "project"` and a stderr warning naming `discipline-rust
   init`. Delete `SPEC_PACKAGE` (`specmap-core/src/lib.rs:41-44`); thread
   `&cfg.namespace` into `mdspec.rs:269` (`canonical URI mint`) and any
   other `spec://{SPEC_PACKAGE}` use (`grep -rn SPEC_PACKAGE`). Retag
   test fixtures per D13 (`spec://vibevm/x|A|T|test/DOC|spec/…` →
   `spec://project/…`) and fix the mdspec/index/rscan/explain/ledger unit
   tests that assert minted URIs. vibevm `specmap.toml` adds
   `namespace = "vibevm"` in the same commit.
2. **External spec roots (D3).** `Config` gains
   `pub external_specs: Vec<ExternalSpec>` (`namespace`, `root` —
   root-relative path). `index::build` scans each external root via the
   existing `scan_spec_tree` with that namespace and adds the units to the
   **resolution set only**: dangling-edge classification and `explain`
   queries see them; the serialised `Specmap` does not change. Unit tests: a
   tag citing `spec://other/DOC#a` resolves iff an external root defines it.
3. **Conform defaults + invariant (F4, F6).** New
   `Config::load_or_default(root) -> (Config, ConfigOrigin)`
   (`Loaded | Defaulted`); `run_check`/`run_freeze` use it, print the origin,
   and a `Defaulted` origin prints "no conform.toml — nothing is gated; run
   `discipline-rust init`". Fix the doc-comment/behavior mismatch
   (`config.rs:19-20`). Auto-detect default roots at load time: `crates/`
   exists → `["crates/*"]`, else `["."]`; drop `"xtask"` from the shipped
   default (vibevm's own `conform.toml`/`specmap.toml` name it explicitly —
   verify they do; specmap default gets the same autodetect).
   Implement D10 (`validate_against_tree`) + call it from `run_check`;
   rewrite `xtask/src/conform.rs` test as a delegate to it.
4. **Message scrub (F5, D11).** `specmap-core/src/index.rs:302,316` → name
   `specmap-rust` (with "or your project's wrapper"); sweep the shipped doc
   comments (`grep -rn 'xtask' packages/org.vibevm.ai-native/rust-ai-native/v0.3.0/crates/`)
   — rewrite every consumer-facing mention; internal history notes in
   `*-cli/src/lib.rs` headers may mention that the code was extracted from
   the dev repo, but phrased engine-first.
5. **Acceptance:** vibevm `cargo xtask specmap --check` **byte-identical**
   (the namespace field reproduces today's URIs); `cargo xtask conform
   check` identical findings (0); package fmt/test/clippy green (self-check
   6–8); package `--gate` green; floor green.

**Commits:** `feat(specmap): config-driven namespace + external spec roots`
· `feat(conform): policy autodetect, config origin, tree invariant` ·
`fix(discipline): scrub xtask from shipped engine messages`.

### Phase 2 — Normative specs → discipline-core + retag (F1, T1 partial)

1. `git mv` the four docs from `spec/discipline/` to
   `packages/org.vibevm/discipline-core/v0.3.0/spec/mechanisms/` (names
   unchanged). Fix intra-doc relative links; keep every heading anchor
   verbatim (URIs depend on them). Add the one-line per-language-projection
   note to PROP-014 (D4).
2. Author the three new core docs:
   - **`04-SWEEP-PLAYBOOK.md`** — the portable standing sweep distilled from
     `DISCIPLINE-SWEEP-v0.1.md` layers 1: tier structure (0 floor / 1
     ratchet / 2 drift / 3 judgment), cadence table, collector-first
     principle, "the gates are the floor, the sweep is the ceiling", the
     WISH → census → Rule promotion ladder, sweep output contract (report,
     topic commits, health snapshot diff as the trend); tool references
     abstract ("the stack's floor command / health collector / test-gate /
     tripwire") with a pointer to the Rust stack's `discipline-rust`.
     WAL branch per D7.
   - **`05-CAMPAIGN-FORM.md`** — the campaign form: plan document shape
     (read-first boot, current-state facts, phases with per-phase acceptance
     + commits, risks/fallbacks, quick-start, whole-campaign acceptance —
     the shape this very plan follows), BASELINE / PREDICTIONS / LOG /
     REPORT artifacts and what each is for, phase-gate discipline,
     WAL-backed resumability as the preferred branch (D7). Notes that
     RAID-PLAYBOOK §1 is the in-flight skeleton and this doc is the
     campaign's paper trail.
   - **`06-WAL-CONVENTION.md`** — WAL (volatile checkpoint, rewrite not
     append, supersedes snapshots), CONTINUE (cold-resume), the 24h
     staleness rule, resume/end-session behaviors, and the explicit
     "projects without a WAL" fallback contract that 04/05 reference.
3. Update core docs: `00-MANIFESTO.md` §8 map (+ `mechanisms/`, 04–06);
   `03-RAID-PLAYBOOK.md` §1.4 (WAL optional-preferred → cite 06) and §3
   (cite 05 instead of `PLAYBOOK-TERRAFORM-VIBEVM-v0.2`);
   `boot/10-flow-discipline-core.md` (name the new docs, keep the
   minimal-sufficiency read rule).
4. **Retag** every normative citation `spec://vibevm/discipline/<DOC>#a` →
   `spec://discipline-core/mechanisms/<DOC>#a`:
   - package crates (grep `spec://vibevm/discipline/` under
     `packages/…/rust-ai-native/v0.3.0/crates/` — the scope! markers in
     conform-core (11 modules), conform-frontend-rust, env-audit,
     specmap-core (tripwire/testgate/ledger/rscan/mdspec/index/explain/
     ratchet/config/lib), specmark/tests/usage.rs);
   - the 7 vibevm code sites (F1 list);
   - `spec/terraforms/DISCIPLINE-SWEEP-v0.1.md` §1d canonical deviates
     target (this file is edited again in Phase 5 — fine);
   - `terraform/registry/debt.json` evidence URIs if any cite the moved
     docs (grep; regenerate nothing — hand-edit evidence strings).
   Historical docs (WAL history, past terraform plans/reports, LOG) are NOT
   retagged — they are records.
5. vibevm `spec/discipline/README.md` → a short pointer: the four mechanisms
   now ship in `flow:org.vibevm/discipline-core` (`spec://discipline-core/
   mechanisms/…`), the implementing crates in `stack:org.vibevm/
   rust-ai-native`; fix the stale edge-less-ENGINE-CONFORM claim while
   rewriting.
6. vibevm `specmap.toml`: add
   `[[external_spec]] namespace = "discipline-core"
   root = "vibedeps/<discipline-core-slot>/spec"` (slot path from
   `vibe.lock`). Re-materialise vibedeps first if the mechanisms move has
   not landed there yet (order: edit package → `vibe install` → regen).
7. Regenerate vibevm `specmap.json` (`cargo xtask specmap`): the discipline
   spec units leave the index; the 7 retagged edges + any package citations
   now resolve through the external root. **New gate: 0 dangling edges** on
   the vibevm tree (previously cross-repo dangling was tolerated as
   warnings).
8. Package self-trace stays `--gate` (coverage) in the dev repo — the
   package cannot see vibedeps from inside `packages/`; full resolution is
   proven consumer-side in Phase 4's fresh-project test and by vibevm
   itself.
9. **Acceptance:** floor green; vibevm specmap clean with **0 dangling / 0
   orphans / 0 suspects**; package `--gate` 0 orphans;
   `grep -rn 'spec://vibevm/discipline/' crates/ xtask/ packages/` → 0
   (historical spec/terraform docs excluded); `vibe check` 0/0/0.

**Commits:** `feat(discipline-core): ship the mechanism specs + sweep,
campaign, and WAL playbooks` · `refactor(discipline): retag onto
package-hosted spec URIs` · `chore(specmap): regen for the mechanism
relocation` · `build(deps): re-materialise vibedeps`.

### Phase 3 — The umbrella tool: `discipline-cli` (T3, F4-floor, D1, D5)

1. New package crate `crates/discipline-cli` (lib + `[[bin]] name =
   "discipline-rust"`). Library functions mirror the conform-cli pattern
   (`pub fn run_*(root: &Path, …) -> Result<…>`); the bin is clap over them.
   Workspace deps to add: `clap`, plus whatever the moved modules use
   (discover from `xtask/Cargo.toml`).
2. Port the five xtask drivers, generalising paths (D5 defaults + flags):
   - `test-gate` ← `xtask/src/test_gate.rs` (nextest + libtest fallback;
     `--baseline` default `discipline/registry/tests-baseline.json`;
     `--path` root; the diff/promote semantics from BROWNFIELD §4 —
     now citing `spec://discipline-core/mechanisms/BROWNFIELD-PROTOCOL-v0.1#test-gate`);
   - `tripwire` ← `xtask/src/tripwire.rs` (git change-set collection +
     `specmap_core::tripwire::evaluate`; `--debt` default
     `discipline/registry/debt.json`; warn-only contract preserved);
   - `trace` ← `xtask/src/trace.rs` (over `specmap_core::explain`; readonly
     queries; reads the committed `specmap.json` + config; external specs
     from Phase 1 participate in resolution);
   - `health` ← `xtask/src/health.rs` minus the `--mirrors` probe (that
     stays in the vibevm shim as a vibevm-only flag composed on top);
     `--out` default `discipline/health/latest.json`; keep the
     "pure of the source tree, advisory, no-LLM" contract and byte-stable
     output;
   - `fast-loop` ← `xtask/src/fast_loop.rs` (workspace members via
     `cargo metadata` or manifest walk — must not assume vibevm layout);
   - `codemod` ← `xtask/src/codemod.rs` (port the existing operations as-is;
     they are the scaffold-I pilot — read the module first, port
     conservatively, parameterise repo-root only).
3. Implement `floor` (D1): steps fmt → test → clippy → conform → specmap →
   test-gate (only if the baseline file exists) → fast-loop (only with
   `--fast-loop`); `--keep-going`, `--quiet`; per-step headers + config
   origin line; real exit code contract. Implement `conform` / `specmap` as
   delegating subcommands so one binary suffices.
4. Implement `init` (F4): generates (never overwrites; `--force` to
   overwrite) `conform.toml` (autodetected roots; empty `gated_crates`;
   every discovered crate in `[[exempt]]` with reason
   `"pre-adoption — flip after draining (expand-as-you-conform)"`;
   `max_file_lines = 600`), `specmap.toml` (namespace = dir name or
   `--namespace`; autodetected scan_roots; `spec_roots = ["spec"]`;
   `[[external_spec]]` per D3 from vibedeps scan), the three registry files
   (empty valid forms), `discipline/` directory layout, and prints the
   next-step recipe (add specmark dep, write the first spec unit, run
   floor).
5. vibevm shims: rewrite `xtask/src/{test_gate,tripwire,trace,health,
   fast_loop,codemod}.rs` as thin delegates into `discipline_cli` (health
   shim re-adds the `--mirrors` composition); `xtask/Cargo.toml` +=
   `discipline-cli` path dep. Behavior must be flag-compatible.
6. vibevm living-path migration (D5): `git mv terraform/registry
   discipline/registry`, `git mv terraform/health discipline/health`,
   `git mv terraform/golden discipline/golden`; update every live reference
   (`grep -rn 'terraform/registry\|terraform/health\|terraform/golden'
   --include='*.rs' --include='*.sh' --include='*.toml' --include='*.md'` —
   code + sweep manual + boot docs; historical plan/report texts stay).
7. Package `specmap.toml` exempt list += `discipline-cli` (CLI driver, like
   the other two); package self-trace stays green.
8. **Acceptance:** `cargo xtask test-gate|tripwire|trace|health|fast-loop`
   byte-compatible with pre-phase behavior on vibevm (health JSON
   diff-clean modulo the new default path); `discipline-rust floor --path .`
   green on vibevm and equals self-check steps 1–5 in verdicts;
   `discipline-rust health` from the *package manifest* runs against vibevm
   (proves no dev-repo assumption); floor green; self-check steps 6–8 green
   with the new crate.

**Commits:** `feat(discipline-cli): ship the umbrella discipline-rust tool`
· `refactor(xtask): delegate sweep tooling to the packaged discipline-cli` ·
`chore(discipline): relocate living registries under discipline/` ·
`build(deps): re-materialise vibedeps`.

### Phase 4 — Fresh-project proof (the acceptance frozen as a test)

1. New integration test `crates/discipline-cli/tests/fresh_project.rs`
   (package test suite — runs in self-check step "package test"): in a temp
   dir, lay down a minimal single-crate project (`Cargo.toml` + `src/lib.rs`
   with one `pub fn`, a `spec/PROP-001.md` with one anchored req unit),
   call the library entry points directly (`run_init`, then conform/specmap
   runs):
   - `init` produces the six artifacts; re-running without `--force` is a
     no-op (idempotence);
   - with the crate tagged (`scope!("spec://demo/PROP-001#req-hello")`,
     namespace `demo`) specmap builds a resolving index (0 dangling, 0
     orphans once gated);
   - flipping the crate into `gated_crates` with an untagged second `pub fn`
     module → the orphan ratchet catches it (exit non-zero);
   - conform on the fixture: `no-unwrap-in-domain` catches a planted
     `.unwrap()` in the gated crate (reuse the conform-cli
     `catches_violations` fixture idiom);
   - an `[[external_spec]]` entry pointing at a second fixture spec tree
     resolves a foreign-namespace citation.
   Keep it hermetic (no network, no vibe, no git needed — skip tripwire in
   the test or `git init` the temp dir if the tripwire path is exercised).
2. **Acceptance:** the new test green via
   `cargo test --manifest-path packages/org.vibevm.ai-native/rust-ai-native/v0.3.0/Cargo.toml -p discipline-cli`;
   floor green.

**Commit:** `test(discipline-cli): freeze the fresh-project bootstrap
end-to-end`.

### Phase 5 — Consumer docs, skills, and the sweep manual v0.2 (F3, T2, T4, D6, D12)

1. **Package README.md** (rust-ai-native root): what the stack ships (three
   binaries + the spec corpus dependency), the two run forms (D6), the
   `.gitignore` note, the init → floor → sweep lifecycle, the schema note
   (D9), and the pointer to GUIDE §13.
2. **GUIDE-AI-NATIVE-RUST.md**: new §13 "Wiring the gates in a consumer
   project" — install, specmark path-dep recipe
   (`[workspace.dependencies] specmark = { path = "vibedeps/<slot>/crates/specmark" }`),
   `discipline-rust init`, the floor contract, registries, external specs;
   new §14 "Sweep idioms" hosting the portable Rust idiom set lifted from
   DISCIPLINE-SWEEP (tests-out pattern with its two gotchas, the four
   doctest idioms, the newtype-cascade/`let-else`/`from_validated`
   restructure-beats-testify list, the flip-only-after-drain rule);
   renumber/cross-ref per the guide's `(≈ Rust §N)` conventions.
3. **Boot snippet** `20-stack-rust-ai-native.md`: append a compact "shipped
   toolchain" block (the three binaries, `discipline-rust init|floor`, the
   two skills, where policies live) — boot-budget-sized (≤15 lines).
4. **Cards**: update Band-3 "checker status" where the shipped tools now
   implement the checker (at minimum: scaffold-e-fast-loop → shipped
   (`discipline-rust fast-loop`), scaffold-f/g conform-backed checks →
   shipped via `conform-rust`, scaffold-d cell-has-oracle → shipped,
   scaffold-i → pilot-shipped (`discipline-rust codemod`)); update
   `cards/INDEX.md` statuses accordingly. Do not inflate: a card whose
   checker is still partial stays `specified` with a note.
5. **Skills (T4, D12).** `spec/skills/terraform-rust/SKILL.md`: trigger
   ("terraform this codebase", "adopt the discipline"); procedure walking
   BROWNFIELD (`spec://discipline-core/mechanisms/BROWNFIELD-PROTOCOL-v0.1`):
   precondition (workspace compiles), `discipline-rust init`, inventory
   (run the test suite → fill `tests-baseline.json` with reality; harvest
   debt/intent from WAL/TASKS/TODO/FIXME into the registries),
   characterization (golden transcripts for passing flows), then the
   phased card raids per RAID-PLAYBOOK + 05-CAMPAIGN-FORM, exit criteria
   (carry-over guarantee, floor green, expand-as-you-conform started);
   WAL branch per D7. `spec/skills/discipline-sweep/SKILL.md`: trigger
   ("sweep", "discipline sweep", recurring); procedure = 04-SWEEP-PLAYBOOK
   instantiated with the Rust commands (`discipline-rust floor` → `health`
   → tier work → `tripwire` weekly → report/WAL branch). Both skills name
   only shipped commands (directive 3). Declare in the package `vibe.toml`:
   `[[skill]] name = "terraform-rust" path = "spec/skills/terraform-rust"` +
   the sweep twin.
6. **vibevm sweep manual v0.2**: rewrite
   `spec/terraforms/DISCIPLINE-SWEEP-v0.1.md` → `DISCIPLINE-SWEEP-v0.2.md`
   as the thin vibevm *instance*: cite 04-SWEEP-PLAYBOOK for the method,
   keep only vibevm's snapshot numbers, gated-crate specifics, and paths
   (now `discipline/…`), and the machine-quirks section marked explicitly
   machine-scoped ("this box; candidates for `spec/boot/90-user.md` — owner
   copies at will" — 90-user is owner-owned, do not edit it; OWNER-ASK).
   Old file: `git mv` to the new name (history preserved), leave no stub
   (the terraforms dir is versioned by name).
7. **Dogfood the skills**: run `vibe skill` (discover its exact CLI shape
   first — `cargo run -p vibe-cli -- skill --help`) to project both skills
   into this repo's agent config; verify the files land and read coherently.
8. **Acceptance:** `vibe check` 0/0/0 (skill decls valid); `vibe install`
   re-materialises; `vibe skill` lists/installs both skills; floor green;
   boot INDEX unchanged except intended.

**Commits:** `docs(rust-ai-native): README + consumer wiring guide + boot
quickstart` · `feat(rust-ai-native): ship the terraform-rust and
discipline-sweep skills` · `docs(cards): reflect shipped checkers in Band-3
statuses` · `docs(sweep): rebase the vibevm manual on the core playbook
(v0.2)` · `build(deps): re-materialise vibedeps`.

### Phase 6 — Final floor, checkpoint, mirror question

1. Full gate panel: `bash tools/self-check.sh; echo "EXIT=$?"` → 0 (all 9
   steps; step 9 now over the 0.3.0 package); `cargo xtask specmap --check`
   clean **with 0 dangling**; `cargo xtask conform check` 0; `vibe check`
   0/0/0; `discipline-rust floor --path .` green; the fresh-project test
   green.
2. Walk §9 manually once (a real temp project, the real `vibe install`) —
   the frozen test covers the library path; the manual walk covers the
   `vibe install` + slot + `cargo install` path.
3. Update `spec/WAL.md` (standing line + session section) and rewrite
   `CONTINUE.md`. Commit (`docs(wal)` + `docs(continue)`).
4. **Mirror is outward-facing: present the ahead-count and HOLD for the
   owner's explicit word.** Note: GitVerse SSH was refused at plan time —
   run `cargo xtask mirror --check` only when the owner asks, and report
   host reachability then.

## 6. Risks & fallbacks

- **URI retag cascade (Phase 2).** Mitigation: the retag is a mechanical
  prefix rewrite with anchors untouched; gate with
  `grep -rn 'spec://vibevm/discipline/'` = 0 over live trees AND the new
  0-dangling specmap gate. Historical docs deliberately keep old URIs
  (records, not references) — the grep excludes `spec/WAL.md`,
  `spec/terraforms/` (pre-v0.2 texts), `terraform/`, `CONTINUE.md`.
- **specmap.json byte-identity (Phase 1).** namespace="vibevm" must
  reproduce today's URIs exactly; `--check` byte-compare is the guard. If
  drift appears, diff minted URIs first (the only mint site is
  `mdspec.rs:269`).
- **External-spec scan cost/loops (Phase 1).** External roots are read-only
  resolution inputs; do not let them into orphan/coverage math or the
  serialised index. A missing external root is a WARNING (the package may
  not be installed yet), never a hard failure — `init`-generated entries
  must not brick a fresh clone before `vibe install`.
- **The 0.3.0 rename breaks a hidden path (Phase 0).** The grep in Phase 0
  step 6 is the net; the floor run right after is the proof. Slot names come
  from `vibe.lock`, not assumptions.
- **Port drift in the five drivers (Phase 3).** Byte-compatibility
  acceptance on vibevm (same inputs, same outputs) before the shims switch;
  port one driver per commit-reviewable unit if any port is non-trivial
  (`codemod` is the likely candidate — read it fully first).
- **nextest absence on a consumer box.** test-gate keeps the documented
  libtest fallback; the skill mentions `cargo install cargo-nextest` as the
  preferred path.
- **`vibe skill` UX unknowns (Phase 5).** Discover the command surface
  before authoring the `[[skill]]` decls; if projection semantics don't fit
  (e.g. include-glob needs), PROP-015 §2.8 `include` is available. If
  `vibe skill` turns out to be partial, ship the SKILL.md files anyway
  (they are readable directly from vibedeps) and file the gap.
- **Scope creep.** This plan deliberately does NOT: build a vibe binary
  manager (deferred PROP, §10), generate DEBT.md/INTENT.md views (§10),
  consolidate engine *code* into discipline-core (still owner-deferred),
  touch typescript-ai-native beyond nothing, or edit owner-frozen files
  (`spec/boot/00-core.md`, `90-user.md`, `VIBEVM-SPEC.md`, `refs/book/`).
- **Reversal.** Phases are independently revertible commit groups; Phase 0
  is pure renames (invert cleanly); Phase 2's doc moves are `git mv` (100%
  renames). Revert order is reverse-phase.

## 7. Machine quirks (unchanged, they have bitten before)

- Edits via Edit/Write tools only (PowerShell `Set-Content` corrupts
  UTF-8-no-BOM); recover with `git restore`.
- Commits via `git commit -F - <<'MSG'` heredoc (backticks in `-m` have
  corrupted messages twice).
- `self-check.sh` through Git Bash; check the REAL exit code
  (`; echo "EXIT=$?"`), never a `| tail`'d pipe.
- `bash … > "$VAR/file" 2>&1` with an unset `$VAR` writes to `/file` and the
  command silently never runs — inline scratch paths or set the var on the
  same line.
- Windows UAC blocks test executables named `*install*` (os-740) — keep the
  fresh-project test binary name clear of "install".

## 8. Quick-start (for the executing session)

```sh
# boot, then verify the starting floor:
bash tools/self-check.sh; echo "EXIT=$?"          # must be 0
cargo xtask specmap --check                        # record the live numbers
cargo xtask conform check                          # 0 findings
cargo run -q -p vibe-cli -- check --path .         # 0/0/0

# Phase 0 discovery:
grep -rn 'v0\.2\.0' --include='*.toml' --include='*.sh' Cargo.toml tools/ xtask/ vibe.toml
cargo run -p vibe-cli -- show                      # published-version check, then bump

# per-phase gate, every time:
bash tools/self-check.sh; echo "EXIT=$?"
cargo xtask specmap --check
cargo run -q -p vibe-cli -- check --path .
```

## 9. Acceptance for the whole campaign (the fresh-project scenario)

On a clean machine-independent walk (frozen as the Phase 4 test for the
library path; walked manually once in Phase 6 with real `vibe install`):

```sh
mkdir demo && cd demo && git init
# vibe.toml: [project] + requires stack:org.vibevm.ai-native/rust-ai-native = "^0.3"
vibe install --assume-yes
cargo install --path vibedeps/<slot>/crates/discipline-cli   # discipline-rust on PATH
discipline-rust init                                # policies + registries + external specs
# add a crate; specmark path-dep per GUIDE §13; write spec/PROP-001.md {#req-hello};
# tag the crate: specmark::scope!("spec://demo/PROP-001#req-hello")
discipline-rust floor                               # green, config origin "Loaded"
discipline-rust specmap --check                     # unit + edge resolve; 0 dangling
#   citations of spec://discipline-core/mechanisms/… resolve via [[external_spec]]
# /terraform-rust — walks BROWNFIELD on an existing codebase
# /discipline-sweep — the recurring sweep, WAL branch if the project keeps one
```

…and simultaneously in vibevm: floor green end-to-end, specmap 0 dangling /
0 orphans / 0 suspects, conform 0, the package self-trace green, both skills
projected, `cargo xtask <tool>` shims behavior-identical, WAL + CONTINUE
checkpointed, **mirror HELD for the owner's word**.

The one-sentence definition of done: **a project that has never heard of
vibevm's dev tree can adopt, verify, terraform, and sweep the discipline
using only what `vibe install` puts in `vibedeps/`.**

## 10. Deferred, named (not this campaign)

- **vibe-native binary delivery** (install-time build + shims/PATH; a future
  PROP — today's answer is documented cargo, D6).
- **DEBT.md / INTENT.md generated views** (BROWNFIELD §3 names them;
  no generator exists — candidate `discipline-rust` subcommand later).
- **Engine-code consolidation into discipline-core** (neutral trait/data
  layer under the Rust crates) — still owner-deferred until a second
  language implements the frontends.
- **`vibe trace` as a product command** (the xtask note) — superseded by
  `discipline-rust trace`; a vibe-level alias is a product decision.
- **typescript-ai-native symmetry** (its `conform-typescript` /
  `specmap-typescript` / skills twins) — lands with the TS pilot.
- **Owner-court**: copying the machine-quirks list into
  `spec/boot/90-user.md` (owner-owned file).
