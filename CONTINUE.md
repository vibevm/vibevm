# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-28. **Code-bearing packages refactor: Ф4–Ф6 of 7 landed green;
the relocated conform engine is PROVEN to catch violations in its shipped form;
Ф7 is this checkpoint. The only thing left is the mirror — HELD for the owner's
explicit word.** 9 commits this continuation (`26858dc`→`6c5ee9e`), local on
`main`, NOT mirrored. Floor green at the tip: `self-check.sh` 8 steps exit 0,
specmap 614/583/596/0/0/0, vibe check 0/0/0._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the WAL
> disagree, the WAL wins. The **git log is the authoritative per-item record**.
> Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md`),
> then read this.

---

## TL;DR

The owner's directive across Ф1–Ф7 (PROP-024): *the discipline packages ship
only prompts; the verification tools (conform, specmap/specmark) are hardcoded
inside vibevm, so installing `stack-rust-ai-native` gives a description of
checkers, not the checkers.* Fix: make a package a project that ships runnable
code and move the toolchain in. **Ф1–Ф3** (prior session) built the model +
spec/ layout + productised conform onto `conform.toml`. **Ф4–Ф6** (this
continuation) relocated the conform engine into the package, fixed a
shippable-tree bug the relocation surfaced, **proved the shipped checker
actually works**, cleaned the spec tails, and specced the TypeScript frontend.

The make-or-break was the owner's emphasis that *the verification systems must
actually work*. They were proven: the standalone `conform` binary — and the
binary built from the materialised consumer slot — runs against a real project,
catches a planted violation, and exits non-zero, with a permanent integration
test freezing that property.

**Resume at Ф7's last step: the mirror.** Nothing else is pending. The mirror is
outward-facing and has always been held for the owner's explicit word.

## Where work stands

- **Branch `main`**, tip `6c5ee9e`. **30 commits ahead of `origin/main`**, NOT
  mirrored. Working tree clean except the in-flight WAL/CONTINUE checkpoint edits
  (this commit).
- Floor **green** at the tip: `bash tools/self-check.sh` exit 0 (8 steps: fmt /
  test / clippy / vibe check / conform — plus the new package gate: fmt / test /
  clippy against the rust-ai-native package manifest); `cargo xtask specmap
  --check` 614 units / 583 tagged / 596 edges / 0 suspects / 0 warnings / 0
  orphans (6 crates exempt); `vibe check` 0/0/0.

## The active blocker (the one human action)

**The mirror.** All 30 ahead-of-origin commits are local. Publishing is
outward-facing and is held for the owner's explicit word (PROP-016 hub-and-spoke:
mainline is the maintainer's single-writer local `main`; GitVerse + GitHub are
read-replicas synced by `cargo xtask mirror`). The unblocking action is the
owner saying "mirror" — then `cargo xtask mirror` (reads `mirrors.toml`, pushes
`main` + tags fast-forward-only to every target). `cargo xtask mirror --check`
verifies sync first.

## What landed this continuation (Ф4–Ф6 + verification)

- **Ф4a — decouple conform from specmark** (`26858dc` + `f2e2ab7`). Stripped 13
  `specmark::scope!` + 1 `#[specmark::spec(deviates)]` from the 3 crates, dropped
  the specmark dep. **The `#[spec(deviates)]` on `sarif::render` was NOT inert** —
  conform reads it textually to excuse `render`'s `.expect`; render made total
  (`.unwrap_or_default()`). specmap regen + ratchet exempt (later reverted).
- **Ф4b — relocate** (`2b0e6f6`). `git mv` conform-core / conform-frontend-rust /
  env-audit into `packages/org.vibevm/rust-ai-native/v0.2.0/crates/` + new
  `conform-cli` (lib + `conform` bin). Package is its own Cargo workspace; vibevm
  root `exclude`s packages/ + vibedeps/ and path-deps in. xtask conform → thin
  shim; conform.toml de-gated 16→13; **self-check grew a package gate (steps
  6-8)**.
- **Ф4c — shippable-tree exclusion** (`12c8592`, discovered-necessary). The first
  code-bearing install showed `copy_dir_recursive` + both `compute_content_hash`
  ports (vibe-registry + vibe-index) walked the whole tree → copied `target/`
  into the slot + a volatile hash. Fixed all three with a shippable-tree
  `filter_entry` (PROP-024 §2.2) + tests. `vibe.lock` reproducible now.
- **Verification PROVEN** (`302454b`). Standalone `conform` bin: vs vibevm = 0/13
  gated/4 exempt (== xtask); vs a dirty fixture catches the unwrap + exits 1; and
  the bin built **from the materialised vibedeps slot** does the same. Frozen
  into `conform-cli/tests/catches_violations.rs`.
- **Ф5 — spec tails** (`b0d1830` + `56c08a0` + `ac50f72`). Mechanism table +
  DISCIPLINE-SWEEP + health.rs repointed from the Ф3-defunct const policy
  (`CONFORM_GATED` in `xtask/src/conform.rs`) to `conform.toml`. Terraform
  history + WAL history + PROP-024 motivation kept.
- **Ф6 — TypeScript spec** (`6c5ee9e`). Added
  `typescript/tools/conform-frontend-typescript.md` — the future TS frontend atop
  language-neutral conform-core; status specified.

## Next-steps recipe (whoever picks up)

1. **Mirror on the owner's word**: `cargo xtask mirror --check` then `cargo xtask
   mirror`. This is the only pending item to close Ф4–Ф7.
2. **Optional follow-ups, none blocking** (each is its own clean unit):
   - **specmap/specmark relocation** — the deferred sibling of Ф4. Harder:
     `specmap-core → vibe-wire` is the one edge out of the discipline set;
     `specmark` is dogfooded by 10 crates; PROP-014 is split-implemented. Move the
     generated `specmap` JTD types out of `vibe-wire` first.
   - **Promote `conform-core` to `flow:org.vibevm/discipline-core`** so the future
     `conform-frontend-typescript` can reuse it without a cross-package dep (the
     open question in `typescript/tools/conform-frontend-typescript.md`).
   - **Split `crates/vibe-registry/src/lib.rs`** (599 lines, at the 600 budget's
     edge — extract `copy_dir_recursive` + `compute_content_hash` + the
     shippable-tree helper into a module).
   - **`.vibeignore` glob support** in the shippable-tree filter (PROP-024 §2.2
     calls it "optional"; only the built-in dir/file excludes are wired today).

## Non-obvious findings (still in force)

- **A discipline tag can be load-bearing for the gate, not just for specmap.**
  conform reads `#[spec(deviates)]` attribute TEXT (via the frontend's
  `is_spec_deviates`) to excuse unwrap/unsafe/env in domain code. Stripping such
  a tag without handling the excused site turns the gate red. (This is why Ф4a's
  "zero behaviour change" framing was wrong for `sarif::render`.)
- **The Cargo nested-workspace topology works on Windows**: vibevm root `exclude
  = ["packages","vibedeps"]` + external-path-dep into a nested `[workspace]`
  builds clean (~12s). cargo builds path-dep crates into the ROOT target/, so
  `packages/.../target/` only appears if you build the package standalone.
- **The package content_hash must be over the shippable tree, not the raw dir.**
  Build output (`target/` etc.) is volatile; hashing it makes `vibe.lock`
  non-reproducible. Fixed in Ф4c. The in-workspace `file://` source (PROP-011
  §2.6) re-materialises every install regardless, so the slot always refreshes.
- **The conform engine is language-neutral** (Fact model + rules + `Frontend`
  trait); a TypeScript frontend is a frontend, not a second engine. Its one
  wrinkle: conform-core homes in the Rust stack today.
- **Machine quirks (unchanged):** edits via Edit/Write only (PS `Set-Content`
  corrupts UTF-8-no-BOM); `git commit -F - <<'MSG'` heredoc (backtick `-m`
  mangled messages twice); `self-check.sh` through Git Bash; don't `2>&1`-redirect
  native cargo in PowerShell (false NativeCommandError — stderr is captured).

## Repository map (deltas this continuation in **bold**)

```
vibevm/                      Rust workspace; binary = `vibe`; tooling = cargo xtask
├─ CLAUDE.md / AGENTS.md / GEMINI.md   the four rules + boot directives (identical)
├─ VIBEVM-SPEC.md            owner-frozen spec (§4.2/§7.2-7.4/§12/§13.1 amended for PROP-024)
├─ conform.toml              vibevm's conform policy — **de-gated 16→13; audit_crates empty**
├─ conform-baseline.json     conform ratchet baseline (empty / clean)
├─ Cargo.toml                **root: exclude=[packages,vibedeps]; 3 conform deps repointed; conform-cli added**
├─ spec/
│   ├─ boot/                 00-core, 90-user (owned); INDEX.md (generated)
│   ├─ WAL.md                CANONICAL living state (this continuation's section at top)
│   ├─ common/               PROP-000.. + PROP-024-code-bearing-packages.md
│   ├─ discipline/           ENGINE-CONFORM, PROP-014, README (**mechanism table repointed**)
│   └─ terraforms/           DISCIPLINE-SWEEP (**repointed to conform.toml**), *-PLAN (history, kept)
├─ packages/org.vibevm/      in-repo authoring registry (`--registry packages`)
│   ├─ discipline-core/v0.2.0/      spec/ {manifesto,format,scaffolds,raid,boot/10,…} + vibe.toml
│   ├─ rust-ai-native/v0.2.0/       **Cargo.toml + LICENSE.md + crates/{conform-core,conform-frontend-rust,conform-cli,env-audit}** + spec/ + vibe.toml
│   └─ typescript-ai-native/v0.2.0/ spec/ {…, **tools/conform-frontend-typescript.md (NEW)**} + vibe.toml
├─ vibedeps/                 materialised install (git-TRACKED); **rust-ai-native slot now ships crates/ (no target/)**
├─ crates/
│   ├─ vibe-registry/        **lib.rs: shippable-tree filter on copy + hash (599 lines, at budget edge)**
│   ├─ vibe-index/           **content_hash.rs: same shippable-tree filter (parity)**
│   ├─ specmark / specmark-grammar / specmap-core   traceability — STAY (deferred follow-up)
│   └─ vibe-* (core/cli/install/registry/resolver/workspace/mcp/check/publish/index/wire/graph/llm)
├─ xtask/                    conform.rs (**thin shim over conform_cli**), health.rs (**repointed**), specmap, mirror, …
├─ tools/self-check.sh       **8-step floor gate (5 vibevm + 3 package)**
├─ mirrors.toml              source-mirror targets (GitVerse + GitHub)
└─ specmap.json              traceability index (614 units / 596 edges)
```

## Recent commit chain (newest first)

```
6c5ee9e docs(typescript): spec the future conform-frontend-typescript   (Ф6)
ac50f72 docs(conform): drop a stale CONFORM_GATED reference             (Ф5)
56c08a0 docs(discipline): reflect the conform.toml policy in the sweep   (Ф5)
302454b test(conform): prove the shipped gate catches violations e2e     (verification)
b0d1830 docs(discipline): point the mechanism table at the relocated conform (Ф5)
12c8592 fix(registry): exclude build output from the content hash and copy (Ф4c)
2b0e6f6 refactor(conform): relocate the conform toolchain into the package (Ф4b)
f2e2ab7 chore(specmap): regen + exempt the conform crates from the ratchet (Ф4a)
26858dc refactor(conform): drop the specmark tags from the conform crates (Ф4a)
05ff5d5 docs(continue): refresh checkpoint commit accounting after wind-down (prior)
0b22b69 docs(wal): checkpoint — code-bearing packages Ф1-Ф3                (prior)
c560e79 docs(continue): cold-resume checkpoint — code-bearing packages Ф1-Ф3 (prior)
cb05d16 chore(specmap): regen for the conform Config seam                  (Ф3)
424ee17 refactor(conform): config-driven policy via conform.toml          (Ф3)
8dc6e29 build(deps): re-materialise vibedeps for the spec/ layout          (Ф2)
20190df refactor(discipline): move package content under spec/             (Ф2)
5362b4f chore(specmap): regen for PROP-024 + reconciliations               (Ф1)
b6f8132 docs(spec): code-bearing packages — PROP-024 + frozen amend        (Ф1)
3d9cb28 docs(continue): cold-resume — TS stack, card migration, §2.6       (prior)
bdde0f2 docs(wal): checkpoint — in-workspace file:// sources mutable       (prior)
```

## Quick-start

```sh
bash tools/self-check.sh                 # 8-step floor gate; currently exit 0
cargo xtask specmap --check              # clean (614 / 583 / 596 / 0 / 0)
cargo xtask conform check                # 0 findings, 13 gated / 4 exempt (via conform_cli shim)
cargo run -q -p vibe-cli -- check --path .   # vibe check 0/0/0
cargo run -p vibe-cli -- install --registry packages --assume-yes   # re-materialise vibedeps

# Run the SHIPPED conform engine directly (the consumer surface):
cargo run --manifest-path packages/org.vibevm/rust-ai-native/v0.2.0/Cargo.toml \
  -p conform-cli --bin conform -- check --path .

cargo xtask mirror --check               # confirm GitVerse + GitHub sync (HELD; do not mirror without owner word)
```

The WAL supersedes this snapshot wherever they diverge. Session-resume phrase:
`восстанови сессию`. The candidate next work above (the mirror) is the owner's
call, not a standing mandate — a resume boots into a status report and waits.
