# GUIDE — Rust under the Discipline, v0.1

**Status.** Beta; the genre-fixing T2 guide (TS/JS, Python, C++ follow its shape). Scope: only what the axioms require — this is not a general style guide. House baselines (`clippy -D warnings`, `cargo fmt`, `#![forbid(unsafe_code)]`) are assumed, not restated.

A general note that shapes everything below: Rust dissolves much of the classical pattern catalog into language features. Strategy is a trait. Visitor is `enum` + `match`. Iterator is `std`. Decorator is a wrapper implementing the same trait. Singleton is forbidden. The guide therefore speaks Rust-native forms, not GoF vocabulary.

---

## 1. Cells {#cells}

A **cell** is the unit of feature: one Rust module (default) behind one seam, registered in one place, selected by at most one flag, bound to its REQ units.

**Granularity decision procedure (provisional — pilot-measured, Charter OQ-3):** cell = *module* inside its subsystem crate, **promoted to its own crate** when any of: (a) it carries heavy optional dependencies (feature-gate at the crate boundary — the `vibe-resolver-libsolv` precedent); (b) compile-time isolation pays (independent iteration cadence); (c) it exceeds ~2 kLoC or needs its own audit boundary (e.g. an `unsafe`-bearing cell).

**Cell manifest** — v0.1 carries it as a structured attribute on the cell's root item (a dedicated `cell.toml` is a later promotion if manifests outgrow one line):

```rust
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade", r = 2)]
#[cell(seam = "DepSolver", variant = "sat", replaces = "naive", flag = "solver")]
pub struct SatDepSolver<P: DepProvider> { /* … */ }
```

Isolation rule (R-002 binding): a cell imports **seams and core only** — never a sibling cell. Enforced by the conform import-graph check (T-syn).

## 2. Seams {#seams}

- Seam traits live in the subsystem's root (or a `seams` module) — never inside a cell.
- Cell-boundary seams are **object-safe** (`dyn`-capable): the registry must hold them uniformly. Inside a cell, generics are free.
- A seam carries behaviorally-neutral defaults only; policy lives in cells. A default method that makes a decision is a hidden cell.
- Existing vibevm seams (`DepSolver`, `DepProvider`, `RepoCreator`, `GitBackend`) are already conformant; they are the reference shapes.

## 3. Registry and flags {#flags}

R-001 binding — *flag at the seam, never in the veins*:

```rust
// crates/vibe-cli/src/registry.rs — the only module allowed to read selection flags.
pub fn dep_solver(cfg: &Flags, provider: impl DepProvider) -> Box<dyn DepSolver> {
    match cfg.get("solver") {            // recorded provenance: default | env | cli | lockfile
        "sat"   => Box::new(SatDepSolver::new(provider)),
        _       => Box::new(NaiveDepSolver::new(provider)),
    }
}
```

- **Two tiers, never confused:** cargo features answer *"is the code in the binary"* (heavy deps, platform code); runtime flags answer *"is the cell selected"*. A runtime flag must not change the type surface; a cargo feature must not encode product choice.
- The flag registry is data: name, default, provenance chain (CLI > env > project file > built-in), birth date, sunset criterion. The CI flag-matrix (defaults + each-toggled + declared pairs — R-060) is generated from it.
- An explicit `match` registry is chosen **over** `inventory`/`linkme`-style distributed registration deliberately: link-time magic violates R-021 (terse-magical) and costs determinism of review; one `match` is the system's table of contents.

## 4. Errors as contract {#errors}

- One `thiserror` enum per layer; variants that signal a requirement carry the requirement's edge (enum-level `#[spec]` minimum; variant-level where precision pays).
- Constructors/helpers that surface user-visible failures take `#[track_caller]`; user-facing rendering appends the violated REQ URI (PROP-014 §2.6 — every failure is a doorway into the metamodel).
- `anyhow`-style erasure is allowed only at the binary edge (`vibe-cli`); library crates keep typed errors — erased errors cannot carry REQ edges.
- Panics in library code are defects; `expect` with an invariant message is permitted only for statically-guaranteed states, and the message names the invariant.

## 5. specmark usage {#specmark}

`#[spec(implements|documents|informs = "uri", r = N)]` on items · `#[spec(deviates = "uri", r = N, reason = "…")]` — reason mandatory · `#[verifies("uri", r = N)]` on tests · `specmark::scope!("uri", r = N);` at module top for inheritance (item tags replace the inherited set) · ≤ 3 edges per item or split (lint).

## 6. Naming (R-020/R-021 bindings) {#naming}

- Canonical cell type name is **computed** from the manifest: `{variant}{Seam}` → `SatDepSolver`, `NaiveDepSolver` (matches the existing codebase; the grammar codifies reality rather than fighting it). Hand-written names are linted against the computation.
- Length is free; ambiguity is not. `MultiRegistryResolverWithRedirectFollowing` is acceptable *iff* every token is backed by manifest/structure; a structural token nothing enforces is slop in a good hat.
- Forbidden regardless of elegance: hidden control flow — `Deref`-based polymorphism, decision-making `Default` impls, effectful `From`, proc-macro magic in domain cells (proc-macros live in dedicated infra crates: the `specmark` precedent).

## 7. Replacement protocol (R-040 binding) {#replacement}

A cell with `replaces = "X"` ships with a **differential oracle**: a property test (`proptest`, MIT/Apache-2.0) asserting equivalence — or documented divergence — against the old cell on the shared seam, tagged `#[verifies]` on the governing REQ. The old cell is deleted only after the oracle has held for the agreed window *and* the flag passed its sunset review. The naive/sat solver pair is the canonical instance.

## 8. Risk table (what conform must cover for Rust) {#risks}

| Footgun | Rule | Tier |
|---|---|---|
| flag reads outside the registry | R-001 | T-syn |
| cell importing a sibling cell | R-002 | T-syn |
| public item without own/inherited spec edge | PROP-014 §3.2-6 | T-syn + index |
| name asserting structure the manifest lacks | R-020 | T-syn + index |
| `unsafe` outside a designated audit crate | house | T-lex |
| panic/unwrap in library crates | §4 | T-syn |
| erased errors below the binary edge | §4 | T-syn |
| `Deref` polymorphism / effectful conversions | R-021 | T-syn (pattern list) |
| spec-tagged item whose pinned `r` is stale | PROP-014 §2.2 | index |
| flag past sunset still alive | R-050/flags | registry telemetry |

## 9. Doc layer {#docs}

Every tagged public item's rustdoc states the practically-important behavior — errors, edge cases, performance traps. Rustdoc is the human-facing **detail layer**; the spec stays thin (contract); the ledger renders machine explanations from both. Duplication between rustdoc and spec is a defect on the spec side.

---

*Any rule binding here without a corresponding conform check (or explicit `WISH` mark in the Charter rule record) by Playbook Phase 4 is removed rather than carried as aspiration.*
