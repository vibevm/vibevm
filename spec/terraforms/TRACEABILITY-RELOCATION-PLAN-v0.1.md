# Traceability Relocation Plan v0.1 — move specmap + specmark into the rust-ai-native package

**status: PLANNED · not started · the deferred sibling of the conform relocation (PROP-024 Ф4) · executes in a fresh session from cold context**

> **Read-first / boot.** This plan is written to be executed cold. Boot the
> normal way first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files →
> `spec/WAL.md` → `CONTINUE.md`), then read this whole file. It is
> self-contained: the current-state facts (with file:line), the decision and
> its rationale, the phase-by-phase recipes, the risks, and the acceptance
> gates are all here. The git log is the authoritative per-item record; the
> WAL is the canonical living state and supersedes this plan if they diverge.

---

## 0. Why this exists (the reframe)

PROP-024 (code-bearing packages) made a package a project that ships runnable
code, so installing a discipline stack yields *working checkers*, not a
description of them. Phases Ф1–Ф7 relocated the **conform** half of the
discipline (the structural gate) into `stack:org.vibevm/rust-ai-native` and
proved it works in its shipped form.

But the conform relocation was scoped **conform-first**: it stripped the
conform crates of their `specmark` tags (Ф4a) so they could move *without*
specmark, which stayed in vibevm. That left two debts:

1. **The relocated discipline code is now *less* disciplined than the vibevm
   code it came from** — the conform crates lost their `scope!` traceability
   and can no longer carry `#[spec(deviates)]` testimony on their own code
   (they are `specmark`-free). The discipline stopped disciplining itself.
2. **The package ships only half the verification stack.** A consumer who
   installs `rust-ai-native` gets the conform structural gate but **not** the
   specmap/specmark spec↔code traceability system. The discipline is
   half-shipped.

**The fix (this campaign):** relocate the **traceability** toolchain —
`specmap-core`, `specmark`, `specmark-grammar` — into the same package, next to
conform. Then the package ships the *whole* Rust verification stack, the
conform crates **re-acquire their tags** (specmark is back, in the same
package), and the package can **trace and gate itself**. The discipline
disciplines itself again.

## 1. The decision: Option B (rust-ai-native), not discipline-core (now)

**Confirmed by the owner.** Put `specmap-core` + `specmark` +
`specmark-grammar` into **`stack:org.vibevm/rust-ai-native`**, alongside the
conform crates — *not* into the language-neutral `flow:org.vibevm/discipline-core`.

**Why not discipline-core now.** These crates are "language-neutral in logic"
but they are **Rust code** that is itself traced by a **Rust proc-macro**
(`specmark`). A neutral package depending on a Rust proc-macro is a backwards
core→stack edge. Resolving that cleanly means splitting `conform-core` /
`specmap-core` into a neutral trait/data layer + a Rust-impl layer — a research-
grade decomposition that is only worth doing once a *second* language actually
needs the shared core. It is premature today (no TypeScript code exists yet).
So the neutral `discipline-core` consolidation is a **separate, later,
owner-decided** step; this campaign does the pragmatic, complete-the-Rust-stack
move. (Recorded in `typescript/tools/conform-frontend-typescript.md` §3 as the
open question; this plan is the decision for the traceability half.)

## 2. Current-state facts (verified; do not re-discover)

- **Consumers of `specmap-core`: only `xtask`** (`xtask/Cargo.toml`) + itself.
  Same clean shape as `conform-core` — no product crate depends on it.
- **`specmark` dogfooders: 11 crates** carry `specmark.workspace = true`:
  `vibe-core`, `vibe-cli`, `vibe-check`, `vibe-index`, `vibe-install`,
  `vibe-mcp`, `vibe-publish`, `vibe-registry`, `vibe-resolver`,
  `vibe-workspace`, and `specmap-core`. **They all use `.workspace = true`**, so
  the rewire is **~3 lines in the root `Cargo.toml`** (`[workspace.dependencies]`
  repoint), NOT 11 per-crate edits — exactly how `env-audit` was repointed in
  Ф4b. The 11 crates keep their `scope!`/`#[spec]` tags untouched.
- **The `specmap-core → vibe-wire` edge** (the one edge out of the discipline
  set): `crates/specmap-core/src/{explain,index,ratchet}.rs` import
  `vibe_wire::generated::specmap::{Specmap, Edge, SpecUnit, Suspect, Warning,
  EdgeVerb, SpecUnitKind, SpecUnitStatus}`. These are **JTD-generated** from
  `schemas/specmap.jtd.json` into `crates/vibe-wire/src/generated/specmap/mod.rs`
  (the codegen is `xtask/src/codegen.rs`). **No crate other than `specmap-core`
  uses `vibe_wire::generated::specmap`** — so severing this affects only
  specmap-core.
- **`specmap-core`'s scan is hardcoded to `crates/`**: `rscan.rs:294` and
  `ratchet.rs:92` both `read_dir(root.join("crates"))`; `mdspec` scans `spec/`.
  Like conform before Ф3 — needs productising (config-driven) to run from the
  package on an arbitrary project.
- **`specmark` is a proc-macro** (`crates/specmark/Cargo.toml: proc-macro =
  true`); `specmark-grammar` is the shared grammar for both the proc-macro and
  the scanner. Neither carries `scope!`/`#[spec]` tags on its own code — they
  are the bootstrap pair, exempt in `specmap-ratchet.json` ("cannot depend on
  specmark to carry scope! markers").
- **`specmap-core` DOES carry its own `scope!` tags** (e.g.
  `index.rs:13`, `ratchet.rs:25` → `spec://vibevm/discipline/PROP-014#index`).
  Because `specmark` moves *with* it, specmap-core **keeps** its specmark dep +
  tags throughout — there is **no decouple-from-specmark phase** (the single
  biggest difference from the conform relocation, which had to strip tags
  because specmark stayed behind).

## 3. What the conform relocation already taught us (lessons carried)

- **The shippable-tree bug is already fixed** (Ф4c): `copy_dir_recursive` +
  both `compute_content_hash` ports exclude build output. Moving more crates
  into the package will **not** re-trigger volatile hashes / copied `target/`.
  No Ф4c-equivalent is needed.
- **A discipline tag can be load-bearing for a gate** (Ф4a's `sarif::render`).
  Here the relevant tags are specmap `scope!` edges; the move drops the moved
  crates' edges from *vibevm's* specmap (handled by regen), and specmap-core
  keeps its own (specmark travels with it).
- **The package gate pattern** (Ф4b, self-check steps 6-8): the moved crates'
  tests run against the *package* manifest. **Watch the portability trap** —
  `specmap-core` has tests that scan "the real tree" and assert a substantial
  inventory (`real_tree_has_a_node_inventory` asserts `specUnits.len() > 100`;
  `index_is_deterministic_over_the_real_tree`). After the move, "the real tree"
  resolves (via `CARGO_MANIFEST_DIR`) to the **package** tree, which has far
  fewer spec units than vibevm. **These tests will break unless retargeted** —
  point them at a fixture, or relax the assertion, or drive them by config.
  This is the equivalent of conform-cli's tests being self-contained.
- **The binary naming convention** (just established): a package binary is
  language-suffixed. specmap's CLI binary is **`specmap-rust`** (the `rscan`
  frontend is Rust-specific), mirroring `conform-rust`.
- **Machine quirks**: edits via Edit/Write only (PS `Set-Content` corrupts
  UTF-8); `git commit -F - <<'MSG'`; `self-check.sh` via Git Bash; **check the
  real exit code, never a `| tail`'d pipe** (the tail masks the script's exit —
  use `; echo "EXIT=$?"`); don't `2>&1`-redirect native cargo in PowerShell.

## 4. Phases

The structure mirrors the conform relocation: **spike → sever → productise →
relocate → re-tag → checkpoint**. Each phase lands its own green commit(s);
the floor (`self-check.sh` exit 0 + `cargo xtask specmap --check` clean) must be
green at every phase boundary.

### Phase 0 — SPIKE: proc-macro path-dep across the `exclude` boundary (GATING)

**Why first.** The conform spike validated that a vibevm-root member can
path-dep into a **library** crate inside the nested package workspace. specmark
is a **proc-macro** — compiled for the host, special build treatment. Eleven
vibevm crates will path-dep into the packaged specmark. If a proc-macro
path-dep across `exclude = ["packages","vibedeps"]` does **not** work, Option B's
topology fails and we need a fallback (see §6). **Validate before any code
moves.**

**Recipe.** Create a throwaway proc-macro crate under
`packages/org.vibevm/rust-ai-native/v0.2.0/crates/spike-macro/` (a trivial
attribute or derive macro), add it to the package `[workspace]` members + a
root `[workspace.dependencies]` path entry, make ONE vibevm crate (e.g. a temp
`#[cfg(test)]` use, or `vibe-core`) consume the macro via `.workspace = true`,
and `cargo build -p vibe-core`. **Green = the topology holds; delete the spike,
proceed. Red = STOP**, record the error, and reassess (§6 fallback). Do not
commit the spike.

### Phase 1 — Sever the `specmap-core → vibe-wire` edge (in vibevm, green)

**Goal.** specmap-core owns the `Specmap` data model itself, so it can move
without dragging vibe-wire.

**Recipe.**
1. Read `xtask/src/codegen.rs` to learn how it maps `schemas/*.jtd.json` →
   `crates/*/src/generated/`. Find the `specmap.jtd.json` → vibe-wire mapping.
2. **Redirect the specmap codegen target** from
   `crates/vibe-wire/src/generated/specmap/` to
   `crates/specmap-core/src/generated/` (keep it JTD-GENERATED — do **not**
   hand-write the types, or byte-identity of `specmap.json` is at risk). Move
   `schemas/specmap.jtd.json` if the codegen wants schemas co-located, else
   leave it at `schemas/` and just repoint the output.
3. Regenerate (`cargo xtask codegen` or whatever the command is — discover it
   in `xtask/src/main.rs`). Confirm the generated types appear under
   `specmap-core/src/generated/` and are byte-identical to the old vibe-wire
   ones (same serde attrs, same field order).
4. Repoint specmap-core's imports: `vibe_wire::generated::specmap::*` →
   `crate::generated::specmap::*` in `explain.rs`, `index.rs`, `ratchet.rs`
   (+ anywhere else — `grep -rn 'vibe_wire::generated::specmap' crates/specmap-core`).
5. Drop `vibe-wire.workspace = true` from `crates/specmap-core/Cargo.toml`.
   Remove the specmap schema/codegen wiring from vibe-wire (and its
   `check-codegen` gate entry if specmap had one).
6. **Acceptance:** `cargo build` green; `cargo xtask specmap --check`
   **byte-identical** (614/583/596/0/0 or whatever the live numbers are — the
   point is *no drift*); `self-check.sh` exit 0. If specmap.json drifts, the
   type move changed serialization — fix until byte-identical.

**Commit:** `refactor(specmap): own the Specmap types, sever the vibe-wire edge`.

> **Risk:** byte-identity of `specmap.json`. The committed index is byte-compared
> by `--check`. The generated types must serialize identically after the move.
> Keeping the JTD codegen (not hand-writing) is what protects this. If
> `xtask/src/codegen.rs` cannot easily retarget one schema, the fallback is to
> move the *entire* generated module file verbatim (`git mv` the generated
> `mod.rs` into specmap-core, drop it from the codegen set, and own it as
> committed source) — uglier but byte-safe.

### Phase 2 — Productise specmap (config-driven scan, in vibevm, green)

**Goal.** specmap-core scans whatever a `specmap.toml` says, so it runs from the
package on an arbitrary project (the exact move conform made in Ф3).

**Recipe.**
1. Create `specmap_core::Config` (mirror `conform_core::Config` in
   `config.rs`): `scan_roots` (default `["crates/*", "xtask"]`), `spec_roots`
   (default `["spec"]`), and fold the orphan-ratchet `exempt` + `dispositioned`
   (currently `specmap-ratchet.json`) into it — or keep `specmap-ratchet.json`
   and add only the scan/spec roots to a new `specmap.toml`. Prefer ONE
   `specmap.toml` that supersedes `specmap-ratchet.json` (fewer files), but
   preserve the ratchet's schema/data.
2. Drive `rscan.rs` (`read_dir(root.join("crates"))` → config roots),
   `ratchet.rs` (same), and `mdspec`/`index.rs` (`spec/` → config spec roots)
   from the Config. The `<dir>/*` glob convention from conform's Config
   (`crates/*` → each subdir is a crate) applies.
3. vibevm ships `specmap.toml` replicating today's hardcoded roots so the index
   is **behaviourally identical**.
4. **Acceptance:** `cargo xtask specmap --check` byte-identical; `self-check.sh`
   exit 0.

**Commit:** `refactor(specmap): config-driven scan via specmap.toml`.

### Phase 3 — Relocate the three crates into the package

**Goal.** specmap-core + specmark + specmark-grammar live in
`packages/org.vibevm/rust-ai-native/v0.2.0/crates/`, consumed by external
path-dep; the package ships a `specmap-rust` binary; vibevm drives the same
library through an xtask shim.

**Recipe.**
1. `git mv crates/{specmap-core,specmark,specmark-grammar}
   packages/org.vibevm/rust-ai-native/v0.2.0/crates/…` (100% renames). Move
   `schemas/specmap.jtd.json` into the package too if Phase 1 kept it at
   `schemas/` (its home should follow specmap-core).
2. **Package `Cargo.toml`** (`packages/.../v0.2.0/Cargo.toml`): add the three to
   `[workspace] members`; add their third-party deps to
   `[workspace.dependencies]` (the union not already there: `syn`, `quote`,
   `proc-macro2`, `serde`, `serde_json`, `sha2`, `walkdir`, `anyhow`,
   `thiserror`?, `toml` — grep each moved Cargo.toml for `.workspace = true`).
   Add internal path entries `specmap-core`/`specmark`/`specmark-grammar`.
3. **New `specmap-cli` crate** in the package (lib + `[[bin]] name =
   "specmap-rust"`), extracting the driver from `xtask/src/specmap.rs`
   (`run_specmap(check)` + `run_ratchet_gate`) parameterised by `root: &Path`
   (exactly the conform-cli pattern: `pub fn run_check(root, …)` etc.). clap CLI
   with `--path` default `.`.
4. **Rewire the vibevm root `Cargo.toml`:** remove the 3 from
   `members`; repoint the 3 `[workspace.dependencies]` (`specmap-core`,
   `specmark`, `specmark-grammar`) to the package paths (path-only, drop the
   version, like the conform deps); add `specmap-cli`. **The 11 dogfooders need
   NO edits** — they inherit via `specmark.workspace = true`.
5. **xtask:** `xtask/src/specmap.rs` → thin shim over `specmap_cli` (delegating
   `run_specmap`, re-exporting any `load_config` health.rs needs); keep any
   vibevm-specific specmap invariant tests. `xtask/Cargo.toml`: add
   `specmap-cli`; keep `specmap-core` if health.rs/other xtask modules use it
   directly (grep `xtask/src` for `specmap_core`).
6. **conform.toml:** remove `specmark`, `specmark-grammar`, `specmap-core` from
   `gated_crates` / `gated_pub_doctest` (they left `crates/`; vibevm's conform no
   longer scans them) — the `every_crate_is_gated_or_exempt` test enforces this.
7. **specmap config:** vibevm's `specmap.toml` scan roots already exclude the
   moved crates once they leave `crates/` (the `crates/*` glob just stops
   finding them); the ratchet `exempt` entries for `specmark`/`specmark-grammar`
   become dormant — remove them (they are no longer vibevm crates), mirroring the
   Ф4b ratchet revert.
8. **Retarget the portability-trapped specmap-core tests** (§3): the real-tree
   inventory/determinism tests must not assume the vibevm tree. Point them at a
   committed fixture under the package, or relax to a structural assertion.
9. `vibe install --registry packages --assume-yes` to re-materialise (the
   shippable-tree exclusion from Ф4c keeps `target/` out). Remove any stray
   `packages/.../target` before the final install so the content_hash is over
   source only (the Ф4c fix makes this robust, but a clean install is tidy).
10. **Acceptance:** `self-check.sh` exit 0 (real exit code) — now its package
    gate (steps 6-8) also builds/tests the three moved crates; vibevm's
    `cargo xtask specmap --check` clean (the index shrank by the moved crates'
    units/edges — regen + commit); `vibe check` 0/0/0; the standalone
    `specmap-rust` binary runs (`cargo run --manifest-path …/Cargo.toml -p
    specmap-cli --bin specmap-rust -- --check` against a fixture).

**Commits (topic-grouped):** `refactor(specmap): relocate the traceability
toolchain into the package` + `chore(specmap): regen for the relocation` +
(if needed) `build(deps): re-materialise vibedeps for the traceability move`.

> **Risk (the proc-macro topology):** if Phase 0's spike passed, this is fine.
> If a proc-macro path-dep behaves differently at scale (11 consumers, build
> ordering), watch for "can't find derive macro" or duplicate-proc-macro errors
> and fall back to §6.

### Phase 4 — Re-acquire the conform crates' tags + package self-traceability (the payoff)

**Goal.** The discipline disciplines itself again: the conform crates carry
their `scope!`/`#[spec]` tags once more (specmark is back, in the same
package), and the package traces + gates **itself**.

**Recipe.**
1. **Re-add the tags stripped in Ф4a** to `conform-core` (the 13 `scope!`),
   `conform-frontend-rust`, `env-audit` — restore `specmark.workspace = true`
   to their Cargo.tomls (specmark is now a sibling package crate) and the
   `scope!("spec://…/ENGINE-CONFORM-v0.1#…")` module markers. The git history of
   `26858dc` (the Ф4a strip) is the exact list of what to restore. **Leave
   `sarif::render` total** (`.unwrap_or_default()`) — it needs no deviation
   testimony; do not revert that to `.expect` + `#[spec(deviates)]`.
2. **Give the package its own specmap.** Write a `specmap.toml` at the package
   root (scan_roots `["crates/*"]`, spec_roots `["spec"]`) + a package
   `specmap-ratchet.json` (or fold into the toml). The package's `spec/` already
   hosts the discipline mechanism docs? — check; if the ENGINE-CONFORM /
   PROP-014 spec units the tags point at are vibevm-hosted (they are — kept in
   vibevm per Ф4/Ф5), the package's scope! edges will point at
   `spec://vibevm/…` URIs the package's own spec/ does not define → they become
   **edge-into-foreign-unit**. Decide: (a) the package self-trace only checks
   *orphans* (untagged code), tolerating cross-repo `spec://` targets as
   dangling-but-known; or (b) ship copies of the mechanism specs in the package.
   **(a) is lighter and correct** — the package's specmap gate is about "is the
   package's code tagged," not "do the targets resolve in this repo." Configure
   the package's specmap to not fail on cross-repo targets (a `--no-dangling`
   mode may need adding).
3. **Wire a package-specmap step into `self-check.sh`** (a step 9, mirroring the
   package conform/test/clippy steps): `specmap-rust --check` against the package
   manifest, so the package's own traceability cannot drift.
4. **Acceptance:** vibevm `self-check.sh` exit 0; vibevm `specmap --check` clean;
   the package's own `specmap-rust --check` clean (its crates tagged, 0 orphans);
   `vibe install` re-materialises; `vibe check` 0/0/0.

**Commits:** `feat(specmap): re-tag the conform crates + package self-trace` +
the regen/materialise chores.

> **This phase carries the real design subtlety (step 2):** the package's code
> is tagged against vibevm-hosted `spec://` units. The package's self-specmap
> must gate *coverage* (orphans), not *resolution* (dangling targets), or every
> tag looks dangling. If that mode does not exist in specmap-core yet, adding it
> is part of this phase. Keep it small — an orphans-only flag.

### Phase 5 — Floor green, checkpoint, mirror on the owner's word

`self-check.sh` exit 0 (real code), both specmaps clean, `vibe check` 0/0/0.
Update `spec/WAL.md` (current-state lead + a session section) and rewrite
`CONTINUE.md`. Commit topic-grouped (`docs(wal)` + `docs(continue)`). **Mirror
is outward-facing — present it and HOLD for the owner's explicit word**
(`cargo xtask mirror`); do not mirror autonomously.

## 5. End-state (what "done" looks like)

```
packages/org.vibevm/rust-ai-native/v0.2.0/crates/
├─ conform-core            (re-tagged; specmark dep restored)
├─ conform-frontend-rust   (re-tagged)
├─ conform-cli             (bin: conform-rust)
├─ env-audit               (re-tagged)
├─ specmap-core            (owns the Specmap types; config-driven; carries its scope! tags)
├─ specmark                (proc-macro)
├─ specmark-grammar
└─ specmap-cli             (bin: specmap-rust)   ← NEW
```
- The package ships the **whole Rust verification stack** (conform + specmap),
  builds two binaries (`conform-rust`, `specmap-rust`), and **traces + gates
  itself** (package-level conform + specmap in self-check).
- vibevm consumes both engines by external path-dep; `cargo xtask conform` and
  `cargo xtask specmap` are thin shims; the 11 dogfooders are unchanged and
  still tagged, traced by the packaged specmap-core via the shim.
- vibe-wire no longer owns the Specmap types; the `specmap-core → vibe-wire`
  edge is gone.
- **Deferred (separate owner decision):** consolidating the neutral engines
  (conform-core + specmap-core) into `flow:org.vibevm/discipline-core` for
  cross-language reuse — only when a second language needs it.

## 6. Risks & fallbacks

- **Proc-macro path-dep fails across `exclude` (Phase 0 red).** Fallback options,
  in order of preference: (a) keep `specmark` + `specmark-grammar` in vibevm
  (they are the bootstrap pair, language-neutral grammar) and move only
  `specmap-core` + `specmap-cli` to the package — the package still ships the
  *scanner/engine*, and the dogfooders keep their in-vibevm specmark; the conform
  crates re-acquire tags by path-dep'ing the in-vibevm specmark (a stack→vibevm
  edge — acceptable for the self-hosting repo). (b) A Cargo `[patch]` or a
  vendored proc-macro. Reassess with the owner if Phase 0 is red.
- **specmap.json byte-identity drift (Phase 1/2).** The `--check` byte-compare is
  the guard. Keep the JTD codegen; if drift appears, diff old-vs-new generated
  types for serde-attr differences.
- **Portability-trapped specmap-core tests (Phase 3).** Known; retarget to a
  fixture. Do not let `cargo test` against the package tree assert vibevm-tree
  inventory.
- **Foreign-`spec://`-target self-trace (Phase 4).** The package's tags point at
  vibevm-hosted units; gate coverage (orphans), not resolution. Add an
  orphans-only mode if absent.
- **Reversal cost.** Phases 1–2 are in-vibevm and independently revertible.
  Phase 3 is the `git mv` + rewire — revert by `git revert` of that commit (the
  renames invert cleanly). Each phase is its own commit so a bad phase rolls back
  without losing the prior ones.

## 7. Quick-start (for the executing session)

```sh
# boot, then verify the starting floor is green:
bash tools/self-check.sh; echo "EXIT=$?"          # must be 0
cargo xtask specmap --check                        # baseline numbers — record them
cargo xtask conform check                          # 0 findings / 13 gated / 4 exempt

# Phase 0 spike (proc-macro path-dep) — see §4 Phase 0. Delete the spike after.
# Phase 1 — discover the codegen mapping first:
grep -n 'specmap\|generated\|jtd' xtask/src/codegen.rs
ls crates/vibe-wire/src/generated/specmap/

# after each phase: real exit code, never a tail-masked pipe
bash tools/self-check.sh; echo "EXIT=$?"
cargo xtask specmap --check                         # must stay byte-identical (Ph1/2) or regen+commit (Ph3+)
```

## 8. Acceptance for the whole campaign

- The three crates live in the package; the package builds `conform-rust` +
  `specmap-rust`; `vibe install` materialises both engines into the slot with no
  `target/`.
- The conform crates are tagged again; the package's own `specmap-rust --check`
  is clean (0 orphans over its crates).
- vibevm's `self-check.sh` exit 0 (now with a package-specmap step), vibevm
  `specmap --check` clean, `vibe check` 0/0/0, the 11 dogfooders unchanged and
  still traced.
- WAL + CONTINUE updated; mirror held for the owner's explicit word.
- **The discipline ships whole and disciplines itself** — the debt the
  conform-first scoping created is paid.
