# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-28. **Code-bearing packages refactor — Ф1–Ф3 of 7 landed
green; Ф4 (conform relocation) is next, fully planned below.** This session
opened on `восстанови сессию`, then the owner directed a large refactor: make
the discipline packages self-sufficient by letting packages ship runnable code,
and relocate the hardcoded discipline tooling out of the vibevm workspace into
`stack:org.vibevm/rust-ai-native`. **8 commits this session (`b6f8132`→`0b22b69`,
incl. this checkpoint), local on `main`, NOT mirrored.** Floor green at the Ф3
code tip `cb05d16`: `self-check.sh` exit 0,
specmap 614/597/610/0/0/0._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the WAL
> disagree, the WAL wins. The **git log is the authoritative per-item record**.
> Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md`),
> then read this.

---

## TL;DR

The owner's directive: *packages aren't self-sufficient — the discipline's
verification tools (conform, specmap/specmark) are hardcoded inside vibevm, so a
user who installs `stack-rust-ai-native` gets a description of checkers, not the
checkers.* Fix: make a package a project (ship code, not only prompts) and move
the toolchain in. Planned as **7 phases (Ф1–Ф7)**; **Ф1–Ф3 are done, green,
committed.** Owner decisions taken this session (all locked):

1. Prompt dir inside a package is **`spec/`** (singular, project-identical), not
   `specs/`.
2. Move **conform + specmap/specmark** — BUT after Ф4 started and the
   traceability entanglement surfaced, the owner re-scoped to **conform first,
   specmap/specmark as a follow-up** (see "The Ф4 plan" below).
3. conform is **fully productised** (config-driven, runs on any project) — DONE
   in Ф3.
4. The owner **sanctioned editing the frozen `VIBEVM-SPEC.md`** package-model
   sections — DONE in Ф1.

**Resume at Ф4** (conform relocation) — the plan is spelled out below; nothing
blocks it. The one standing owner decision is the **mirror** (held; publishing
is outward-facing).

## Where work stands

- **Branch `main`**, tip `0b22b69`. **20 commits ahead of `origin/main`** (12
  from the prior session + 8 this session, incl. the two checkpoint commits),
  **NOT mirrored** — held for the owner's explicit "mirror".
- Working tree **clean** (the Ф4 crate-move spike was reverted; the floor was
  last green at the Ф3 code tip `cb05d16`, the two commits after are docs-only).
- Floor **green** at `cb05d16`: `bash tools/self-check.sh` exit 0 (fmt; all
  workspace tests + doctests; clippy `-D warnings`; `vibe check` 0/0/0;
  `cargo xtask conform check` 0 findings, 16 gated / 4 exempt). specmap clean:
  **614 units / 597 tagged / 610 edges / 0 suspects / 0 warnings / 0 orphans**.

## What landed this session (Ф1–Ф3)

- **Ф1 — code-bearing package model** (`b6f8132` docs + `5362b4f` specmap).
  New normative spec **`spec/common/PROP-024-code-bearing-packages.md`**: a
  package is a project — prompt content under its own `spec/`, arbitrary code at
  the root, one `vibe.toml`; the **shippable tree** (what's hashed / copied /
  materialised) excludes build output (`.git/`, `.vibe/`, `target/`,
  `node_modules/`, `.vibeignore`); consumption via external-path-dep into the
  materialised slot; the self-hosting bootstrap rides committed `vibedeps/`.
  **Frozen `VIBEVM-SPEC.md` amended under owner sanction** (§4.2, §7.2–7.4, §12,
  §13.1) — the same precedent as PROP-009. Four gating PROPs (002/009/020/022)
  got forward-pointers to PROP-024 **without changing their r1 obligations** (the
  real revision bumps ride with the implementing code later; the commit body
  carries `spec-editorial:` markers for the tripwire).
- **Ф2 — packages refactored to `spec/` layout** (`20190df` refactor +
  `8dc6e29` build-deps). All three packages (`discipline-core`,
  `rust-ai-native`, `typescript-ai-native`) moved their prompt content
  (`boot/`, `cards/`, guides, manifesto, appendix, legacy-projections) under a
  `spec/` subtree via `git mv` (100% renames, history kept); each `vibe.toml`
  `[boot_snippet].source` repointed `spec/boot/…`. `vibe install` re-materialised
  `vibedeps/` (the data-driven boot path-gen regenerated `INDEX.md` to
  `vibedeps/<slot>/spec/boot/…` with no code change) and re-locked.
- **Ф3 — conform productised** (`424ee17` refactor + `cb05d16` specmap). The
  conform checker's policy left compile-time constants for a runtime
  **`conform.toml`** parsed into a new **`conform_core::Config`**; the scan is
  config-driven (`<dir>/*` root → each subdir a crate, else literal); the rules
  own their gated lists (`&'static [&'static str]` → `Vec<String>`);
  `cell-has-oracle` no longer assumes `crates/<c>/tests/`. vibevm ships its own
  `conform.toml` capturing the former constants verbatim, so the gate is
  **behaviourally identical** (0 findings, 16 gated, 4 exempt) — only now the
  policy is data, and an external project can retune + run it.

## The Ф4 plan (conform-first relocation) — RESUME HERE

**Goal:** move the conform checker out of the vibevm workspace and INTO
`packages/org.vibevm/rust-ai-native/v0.2.0/`, so installing the package yields a
working checker. specmap/specmark deferred (owner-chosen — see below).

**Validated foundation (spike, this session):** a member of the vibevm root
workspace CAN depend, via path, on a crate inside a nested `[workspace]` under
`vibedeps/`/`packages/` when the root excludes those dirs (`exclude =
["packages","vibedeps"]`). Built green on Windows: `Adding spikelib → Compiling
→ Finished`, no "two workspaces" error. So PROP-024 §2.4 (own-workspace package +
external-path-dep) is the topology; the fallback (§4) is unneeded.

**Crates that move (the clean set — zero product-crate deps after de-tag):**
`conform-core`, `conform-frontend-rust`, `env-audit`. Plus a **new
`conform-cli`** crate (lib + `conform` bin) extracting the driver currently in
`xtask/src/conform.rs` (`load_config` + `build_rules` + `run_check` +
`run_freeze`), so the package ships a runnable `conform` binary and vibevm's
xtask calls the same library.

**Step 4a — decouple conform from specmark (committable, green on its own).**
`conform-core`/`conform-frontend-rust`/`env-audit` carry **inert** specmark tags
that must go so the crates can move WITHOUT specmark (which stays in vibevm).
Exact sites (grepped this session): **13 `specmark::scope!(…)`** lines —
conform-core `{baseline,facts,config,finding,store,sarif,rules/mod,rules/budget,
rules/diagnostics,rules/tests,rules/structure}.rs`, conform-frontend-rust
`lib.rs:12`, env-audit `lib.rs:19` — **plus one `#[specmark::spec(…)]`** on
`conform-core/src/sarif.rs:16`. Delete them all (they expand to nothing —
zero behaviour change) and drop `specmark.workspace = true` from the three
`Cargo.toml`s. Then `cargo build` + regen specmap.

**The specmap/ENGINE-CONFORM wrinkle (the one thing to resolve empirically).**
The scope! tags are the specmap edges binding the conform code to
`spec://vibevm/discipline/ENGINE-CONFORM-v0.1`. Stripping them (and later moving
the code out of `crates/*`) drops those edges, so the ENGINE-CONFORM spec units
may become specmap orphans. **Recommended resolution: KEEP `ENGINE-CONFORM-v0.1.md`
in vibevm's `spec/discipline/` and disposition the now-edgeless units in the
specmap ratchet** (the ratchet already supports dispositions — "0 dispositioned
(6 crates exempt)"). DO NOT move the spec file: **28 files reference
`ENGINE-CONFORM`** (product code, specs, terraform reports — grepped this
session), so relocating it triggers a dead-`spec://`-ref cascade that
`vibe check` would fail. Investigate the ratchet/disposition mechanism in
`crates/specmap-core` (`ratchet`, `ledger`) and `specmap-ratchet.json` /
`terraform/registry/` first; add a disposition, not a spec move.

**Step 4b — relocate.** `git mv crates/{conform-core,conform-frontend-rust,
env-audit}` → `packages/org.vibevm/rust-ai-native/v0.2.0/crates/…`; create
`conform-cli` there; write the package's root `Cargo.toml` (`[workspace]` +
`[workspace.package]` mirroring the fields the crates inherit + `[workspace.dependencies]`
for their third-party deps: anyhow, serde, serde_json, sha2, toml, walkdir,
syn, quote, proc-macro2, clap, tempfile). Rewire the vibevm root `Cargo.toml`:
remove the 3 from `members`/`default-members`, add `exclude =
["packages","vibedeps"]`, repoint the 3 `[workspace.dependencies]` to
`packages/org.vibevm/rust-ai-native/v0.2.0/crates/<c>` (consumers use
`.workspace = true`, so only these lines change), add `conform-cli`. `xtask`:
its `conform.rs` becomes a thin shim over `conform_cli`; `health.rs` keeps using
`conform_core` (now path-dep'd). Then `vibe install` re-materialises (the package
now ships code under its `crates/`), `self-check.sh` + `specmap --check` green,
commit.

**Consumers needing the repoint** (grepped): `env-audit` is used by `vibe-publish`
+ `vibe-cli`; `conform-core`/`conform-frontend-rust` only by `xtask` (+ each
other). `specmark` is used by **10 product crates + specmap-core** — but specmark
STAYS, so those are untouched this phase.

**Why conform-first (owner decision):** moving specmap/specmark is materially
harder — (1) **`specmap-core → vibe-wire`** is the one edge out of the discipline
set; relocating specmap-core needs that severed first (move the generated
`specmap` JTD types out of `vibe-wire`). (2) `specmark` is dogfooded by 10 crates.
(3) **`PROP-014` is split-implemented** (specmark moves, specmap-core stays) so
its spec can't cleanly co-move. conform implements ONLY ENGINE-CONFORM and has
zero product-crate deps after de-tag, so it lifts cleanly.

## Remaining phases (Ф5–Ф7)

- **Ф5 — clean the spec tails:** `spec/discipline/README.md` (the mechanism→crate
  table now names a moved `crates/conform-core`), `spec/discipline/ENGINE-CONFORM-v0.1.md`
  (still vibevm-hosted by decision, but its crate refs change),
  `spec/terraforms/DISCIPLINE-SWEEP-v0.1.md` (the Tier-0/1 operating manual),
  PROP-013, and package cards/guides `checker:` fields. Honour TERRAFORM-PLAN-v0.3
  §30's keep-list (`conform-baseline.json` + vibevm's own modules stay
  vibevm-specific).
- **Ф6 — TypeScript:** structural only — its prompts already moved under `spec/`
  in Ф2; scaffold a code-root; checkers stay `specified` (no TS tool exists in
  vibevm to move; verified empirically this session — zero `package.json`/
  `tsconfig`/eslint config/`.ts`, only markdown). Document the future
  `conform-frontend-typescript` atop the language-neutral `conform-core`.
- **Ф7 — floor green, commits, checkpoint; mirror on the owner's explicit word.**

## Non-obvious findings (this session)

- **The discipline's traceability couples spec-location + code-location + tags.**
  Moving discipline CODE out of vibevm's `crates/*` scan orphans the mechanism
  SPECS it implements in specmap; moving the SPECS out triggers dead-`spec://`-ref
  cascades (`vibe check`). This coupling is why Ф4 is conform-first and why the
  recommendation is to keep ENGINE-CONFORM in place + disposition the orphan, not
  move it (28-file blast radius).
- **The Cargo nested-workspace topology works on Windows** (spike): root
  `exclude = ["packages","vibedeps"]` + external-path-dep into a nested
  `[workspace]` builds clean. No `\\?\` / drive-case issue surfaced.
- **conform was nailed to the repo it compiled in** — `store::workspace_sources`
  hardcoded `crates/*/{src,tests}+xtask` and ALL policy was `const` in
  `xtask/src/conform.rs`. Now config-driven; an external project writes its own
  `conform.toml`. The gate stayed at 0 findings because vibevm's `conform.toml`
  replicates the old constants exactly.
- **The specmark tags on the conform crates are inert** (Agent C confirmed +
  verified: `scope!` expands to nothing, `#[spec]` injects a rustdoc line and
  emits the item unchanged) — stripping them is a no-op for behaviour, only
  dropping the build edge + the specmap edge.
- **specmap "unbumped-hash" tripwire is warn-only** and offers two dispositions
  on a req-section content change: bump `r`, or mark the commit body
  `spec-editorial: <anchor>`. Used `spec-editorial:` in Ф1 (the gating-PROP edits
  are forward-pointers preserving r1).
- **Machine quirks (unchanged):** edit via Edit/Write, never PS `Set-Content`
  (UTF-8 corruption); `git commit` via `-F - <<'MSG'` heredoc; `self-check.sh`
  through Git Bash; don't `2>&1`-redirect native cargo in PowerShell (false
  NativeCommandError — stderr is captured already).

## Repository map (deltas this session in **bold**)

```
vibevm/                      Rust workspace; binary = `vibe`; tooling = cargo xtask
├─ CLAUDE.md / AGENTS.md / GEMINI.md   the four rules + boot directives (identical)
├─ VIBEVM-SPEC.md            owner-frozen spec — **§4.2/§7.2-7.4/§12/§13.1 amended (PROP-024, sanctioned)**
├─ **conform.toml**          NEW — vibevm's conform policy (was consts in xtask/conform.rs)
├─ conform-baseline.json     the conform ratchet baseline (empty / clean)
├─ spec/
│   ├─ boot/                 00-core, 90-user (owned); INDEX.md (generated)
│   ├─ WAL.md                CANONICAL living state (this session's section at top)
│   ├─ common/               PROP-000.. + **PROP-024-code-bearing-packages.md (NEW)**
│   ├─ modules/              per-subsystem PROPs (002/009/020/022 got PROP-024 pointers)
│   ├─ discipline/           ENGINE-CONFORM, PROP-014, BROWNFIELD, LEDGER, README (Ф5 tails)
│   └─ terraforms/           DISCIPLINE-SWEEP, TERRAFORM-PLAN-v0.3 (the move boundary), …
├─ packages/org.vibevm/      in-repo authoring registry (`--registry packages`)
│   ├─ discipline-core/v0.2.0/      **spec/** {00-MANIFESTO,01-format,02-scaffolds,03-raid,appendix,boot/10,legacy-projections} + vibe.toml + README
│   ├─ rust-ai-native/v0.2.0/       **spec/** {boot/20,cards,rust/GUIDE,rust/tools/vibe-tcg} + vibe.toml  ← Ф4 ADDS crates/ + Cargo.toml here
│   └─ typescript-ai-native/v0.2.0/ **spec/** {boot/20,cards,typescript/GUIDE,…} + vibe.toml
├─ vibedeps/                 materialised install (git-TRACKED), now **spec/**-layout slots
├─ crates/
│   ├─ conform-core/         **config-driven; config.rs NEW; Config/ExemptEntry** — Ф4 MOVES to package
│   ├─ conform-frontend-rust/  Rust syn frontend — Ф4 MOVES to package
│   ├─ env-audit/            designated unsafe audit crate — Ф4 MOVES to package
│   ├─ specmark / specmark-grammar   traceability macros — STAY (deferred follow-up)
│   ├─ specmap-core/         traceability engine (→ vibe-wire edge) — STAYS (deferred)
│   └─ vibe-* (core/cli/install/registry/resolver/workspace/mcp/check/publish/index/wire/graph/llm)
├─ xtask/                    conform.rs (**config-driven; driver → conform-cli in Ф4**), health.rs (**config-driven**), specmap, mirror, …
├─ tools/self-check.sh       the 5-step floor gate
├─ mirrors.toml              source-mirror targets (GitVerse + GitHub)
└─ specmap.json              traceability index (614 units / 610 edges)
```

## Recent commit chain (newest first)

```
0b22b69 docs(wal): checkpoint — Ф1-Ф3 landed, conform relocation next (checkpoint)
c560e79 docs(continue): cold-resume — code-bearing packages Ф1-Ф3     (checkpoint)
cb05d16 chore(specmap): regen for the conform Config seam            (Ф3)
424ee17 refactor(conform): config-driven policy via conform.toml     (Ф3)
8dc6e29 build(deps): re-materialise vibedeps for the spec/ layout    (Ф2)
20190df refactor(discipline): move package content under spec/       (Ф2)
5362b4f chore(specmap): regen for PROP-024 + reconciliations         (Ф1)
b6f8132 docs(spec): code-bearing packages — PROP-024 + frozen amend  (Ф1)
3d9cb28 docs(continue): cold-resume — TS stack, card migration, §2.6 (prior session)
bdde0f2 docs(wal): checkpoint — in-workspace file:// sources mutable  (prior)
```

## Quick-start

```sh
bash tools/self-check.sh                 # the 5-step floor gate; currently exit 0
cargo xtask specmap --check              # clean (614 units / 610 edges)
cargo xtask conform check                # 0 findings, 16 gated / 4 exempt (config-driven)
cargo run -q -p vibe-cli -- check --path .   # vibe check 0/0/0
cargo run -p vibe-cli -- install --registry packages --assume-yes   # re-materialise vibedeps
cargo xtask mirror --check               # confirm GitVerse + GitHub in sync (held; do not mirror without owner word)
```

## Next-steps recipe (whoever picks up)

1. **Resume Ф4** at "Step 4a" above: strip the 13 `specmark::scope!` + 1
   `#[specmark::spec]` from `conform-core`/`conform-frontend-rust`/`env-audit`,
   drop the specmark dep, build + regen specmap, **resolve the ENGINE-CONFORM
   orphan via a ratchet disposition (do NOT move the spec)**. Commit Ф4a green.
2. **Step 4b:** `git mv` the 3 crates + create `conform-cli` (extract the driver
   from `xtask/src/conform.rs`) + package `Cargo.toml` + rewire the vibevm root
   `Cargo.toml` (members/exclude/deps) + xtask shim. `vibe install`,
   self-check + specmap green, commit.
3. **Ф5/Ф6/Ф7** per "Remaining phases" above.
4. **Mirror** only on the owner's explicit word (`cargo xtask mirror`).

The WAL supersedes this snapshot wherever they diverge. Session-resume phrase:
`восстанови сессию`. The candidate next work above is not a standing mandate.
