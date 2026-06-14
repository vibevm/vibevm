# PROP-017 — Resolvo as the production resolver {#prop-017}

**Status.** Design proposal — accepted, implementation in progress (owner
decision, 2026-06-14). Companion to [PROP-003](PROP-003-dep-evolution.md)
(dependency-model evolution) and [PROP-002](../vibe-registry/PROP-002-decentralized-registry.md)
(registry / depsolver seam).

**Supersedes.** The solver-*backend* decision of [PROP-003 §2.2](PROP-003-dep-evolution.md#solver-backend)
(libsolv via thin FFI) and the libsolv-specific algorithm detail of
[§2.3](PROP-003-dep-evolution.md#solver-features) and
[§3.1–3.2](PROP-003-dep-evolution.md#rule-encoding). **The production
solver is [`resolvo`](https://github.com/prefix-dev/resolvo) (pure-Rust,
BSD-3-Clause), not libsolv.** PROP-003's dependency *vocabulary* — features
(§2.4), subskills (§2.5), interface tags (§2.6), conditional deps (§2.6.1),
i18n (§2.7), manifest/lockfile schema (§2.8–2.9) — is **unchanged**; this
PROP only swaps the engine that satisfies it and records how the vocabulary
maps onto resolvo's model.

---

## 1. Why resolvo now — reversing the libsolv-first call {#why}

PROP-003 §2.2 picked libsolv "for v1" and explicitly *kept the door open*
for resolvo (§4.1: "resolvo remains a viable second impl"). It gave three
reasons to defer resolvo. By 2026-06 all three have decayed, while
libsolv's costs — which §2.2 underweighted — are structural and do not
decay:

**The three deferral reasons, re-evaluated:**

- *"Younger codebase (~3 years vs ~17)."* resolvo is now ~4–5 years old,
  at `0.11.0`, multi-threaded, and the production resolver for `pixi`,
  `rattler` (the Rust conda stack), and `rip` (PyPI-in-Rust).
- *"Less battle-tested under adversarial inputs."* conda dependency graphs
  are among the most conflict-rich in existence; resolvo runs against them
  in production at scale. This is the reason that has decayed most.
- *"No rule-level introspection for explanation-driven errors."* resolvo
  now ships structured conflict explanation
  (`Conflict::display_user_friendly` / `Conflict::graph`).

**libsolv's structural costs (do not decay):**

- **C toolchain + FFI + `unsafe`.** libsolv is C; a thin FFI shim is still
  ~20–30 `unsafe extern` calls — the **first C dependency in the
  workspace** and the first `unsafe` surface in a crate that today carries
  `#![forbid(unsafe_code)]`. It forces a C compiler into CI on every
  platform. resolvo is pure Rust: the `forbid(unsafe_code)` posture on
  `vibe-resolver` survives intact.
- **Eager pool population.** libsolv wants the whole pool materialised
  before solving ([PROP-003 §3.2 phase 1](PROP-003-dep-evolution.md#solver-phases):
  walk `list_versions` for every transitively-reachable package up front).
  PROP-003 §3.4 itself names the *network-bound fetch layer* as the real
  bottleneck — and eager population maximises exactly those fetches.
  resolvo's provider is pulled **lazily / on demand**, so metadata is
  fetched only for packages the search actually visits.
- **Windows operational risk.** §2.2 named "libsolv proves operationally
  heavy on Windows" as the trigger to switch to resolvo. The maintainer's
  primary platform is Windows; a C library via FFI (MSVC/MinGW, submodule
  or build-script fragility) is precisely that risk, on precisely that
  platform.

**Net:** resolvo is pure-Rust (no `unsafe`, no C toolchain), BSD-3-Clause
(clean under [PROP-000 §3](../../common/PROP-000.md) with no owner ruling
needed), lazy-fetch (aligned with the real bottleneck), gives
human-readable conflict explanations, and gets "prefer newest" almost free
via candidate ordering. pubgrub was also considered and remains the weaker
fit for vibevm: its range-over-named-packages model encodes the
capability / virtual-package vocabulary as synthetic packages and degrades
exactly the explanations we want (PROP-003 §4.2 records this).

This decision was taken by the owner directly; the libsolv reasoning is
retained in PROP-003 §2.2 as decision history.

---

## 2. Architecture — adapter behind the stable `DepSolver` seam {#architecture}

The consumer seam does not move. `crates/vibe-resolver/src/lib.rs` keeps:

```rust
pub trait DepSolver {
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError>;
}
```

The install / update / vendor / check pipelines call `DepSolver::solve`
and are untouched. resolvo arrives as one new `impl DepSolver`:

- **`ResolvoDepSolver<P: DepProvider>`** — a `#[cell(seam = "DepSolver",
  variant = "resolvo")]`. Its `solve` builds a `VibevmResolvoProvider`
  from the roots + the vibevm `DepProvider`, runs `resolvo::Solver`, and
  maps the chosen solvables back into a `ResolvedGraph`.
- **`VibevmResolvoProvider`** — implements resolvo's two traits, `Interner`
  + (async) `DependencyProvider`, adapting vibevm's world (package
  identities, version sets, manifests) to resolvo's `NameId` / `SolvableId`
  / `VersionSetId` model. This struct **is the swap unit** (§5): a
  different engine, or a future resolvo major, means rewriting this adapter
  and nothing the consumers can see.
- **`SemverVersionSet`** — a `resolvo::utils::VersionSet` with
  `type V = semver::Version`; an `enum { Any, Req(semver::VersionReq),
  None }` so that `VersionSpec::Latest → Any`, a `[conflicts]` /
  obsoletes range → a complement or `None` (match-nothing) set.

### 2.1 Sync CLI, no async runtime {#runtime}

resolvo `0.11`'s `DependencyProvider` methods are `async fn`, but `Solver`
defaults to `NowOrNeverRuntime`, which polls each future exactly once and
**panics if it ever yields `Pending`**. Our adapter methods compute
synchronously — they read from the sync vibevm `DepProvider` (which may
block on the network, but blocking is not yielding) — so every future is
`Ready` on first poll. **vibevm pulls in no async runtime, no `tokio`, no
`pollster`.** The one sharp edge, recorded so no future edit trips it: an
adapter method must never `.await` a genuinely-pending future under the
default runtime.

### 2.2 Provider enrichment — candidate enumeration {#provider-enrichment}

vibevm's `DepProvider` was shaped around the naive solver: `resolve_version`
picks *one* concrete version for a `PackageRef`, baking version selection
into the provider. A real solver must enumerate candidates and choose the
optimum itself. So `DepProvider` gains one method:

```rust
fn list_versions(&self, group: &Group, name: &str)
    -> Result<Vec<semver::Version>, DepProviderError>;
```

backed by the registry layer's existing `Registry::list_versions`
(`vibe-registry`), which every registry impl already provides. This is an
enrichment of the *world model*, not a change to the consumer seam;
`resolve_version` stays for the naive / sat cells.

### 2.3 Shared output contract {#output}

The roots-first ordering, exact-version pinning (`=x.y.z` on every
dependency edge, the lockfile reproducibility contract), and
obsolete-dropping that `naive.rs` builds today are extracted into a
`pub(crate)` output builder and reused by `ResolvoDepSolver`. Both cells
therefore satisfy the *same* observable contract by construction — which
is what lets the differential oracle (§4) hold them to byte-identical
graphs.

### 2.4 Rich conflict explanation {#unsatisfiable}

On `Err(UnsolvableOrCancelled::Unsolvable(conflict))`, resolvo gives a
human-readable derivation via `conflict.display_user_friendly(&solver)`.
`SolveError` gains a variant to carry it:

```rust
SolveError::Unsatisfiable { explanation: String }
```

This is the user-facing payoff of the switch: "package A needs C ^1 but B
needs C ^2, and only C 1.0 and 2.0 exist" instead of a bare UNSAT. The
structured `SolveError` variants (`VersionConflict`, `CapabilityUnmet`,
`DisjunctionUnsatisfiable`, `ConflictsDeclared`) remain for cases the
adapter can attribute precisely.

---

## 3. Vocabulary → resolvo encoding {#encoding}

vibevm's dependency vocabulary is RPM-lineage (provides / requires /
conflicts / obsoletes plus the four weak-dep levels) — the same lineage
resolvo inherits from libsolv, so the mapping is natural. resolvo's
constraint channels: `KnownDependencies.requirements: Vec<ConditionalRequirement>`
("must be satisfied", pulls the package in) and
`KnownDependencies.constrains: Vec<VersionSetId>` ("*if* present, the
version must match" — does not pull the package in). Requirements are
`Requirement::Single(VersionSetId)` or `Requirement::Union(VersionSetUnionId)`.
Single-version-per-name is enforced by resolvo automatically.

| vibevm concept | resolvo encoding | Slice |
|---|---|---|
| `[requires.packages]` (concrete dep `X ^v`) | `requirements += Single(VersionSetId(X, ^v))` | S2 |
| version selection ("prefer newest") | `sort_candidates` orders a name's solvables by `semver::Version` **descending** → first solution is newest-feasible | S2 |
| single version per `(kind, name)` | automatic (one `SolvableId` per `NameId`) | S2 |
| `[[requires_any]]` (`one_of = [A, B, …]`) | `requirements += Union(VersionSetUnionId[A, B, …])` — native OR + backtracking | S4 |
| capability / interface (`provides` / `requires.capabilities`) | virtual `NameId`; `get_candidates(cap)` returns the providing packages' solvables (a reverse index the adapter builds); a `requires.capabilities` entry is a `Single(VersionSetId(cap, ^v))` | S4 |
| `[conflicts]` (X conflicts Y) | `constrains += VersionSetId(Y, None)` in X's deps — if Y is forced in, the match-nothing set conflicts | S4 |
| `[obsoletes]` (X obsoletes Y `< v`) | `constrains += VersionSetId(Y, complement(<v))`; obsoleted entries dropped by the shared output builder | S4 |
| `[recommends]` (weak: prefer, don't fail) | `Problem::soft_requirements` (best-effort) — a missing recommend is a warning, never a solve failure | S5 |
| `[supplements]` (install me if Y wants me) | resolved **above** the solver (reverse weak-dep): the adapter expands a satisfied supplement into a forward requirement before the solve | S5 |
| `[suggests]` / `[enhances]` | UI surface only — never reach the solver | S5 |
| `[features.exclusive]` (at-most-one group) | pairwise `constrains` between group members' activation markers | S5 |
| feature unification | stays in `features.rs` above the solver (already implemented); the solver sees the unified requirement set | — |

Conditional dependencies ([PROP-003 §2.6.1](PROP-003-dep-evolution.md#conditional-deps))
keep their fixpoint shape: solve unconditional → evaluate context
predicates → add requirements → re-solve. resolvo is cheap to re-run, and
its laziness means the re-solve only re-touches the changed subtree.

---

## 4. Correctness — the differential dominance contract {#dominance}

`crates/vibe-resolver/tests/solver_properties.rs` already drives two
`DepSolver` cells over the same generated world and demands they agree;
it was built (its own docs say so) as "DBT-0011's landing pad" for exactly
this. `ResolvoDepSolver` plugs into the same socket as `Sat`, under a
**dominance** contract:

- **naive solves ⟹ resolvo solves identically.** Provable for the
  concrete-dep worlds the generator emits: when naive's greedy first-pick
  succeeds, that pick is the highest version satisfying the first
  constraint, and (since it also satisfies all others) it equals the
  highest version satisfying *all* constraints — which is resolvo's
  optimum. So the graphs are byte-identical. Any drift here is a bug.
- **naive fails ⟹ resolvo may solve.** The first-pick-wins trap arises
  naturally in generated worlds (a root takes a dep's highest version,
  another path carets a lower major); resolvo's complete CDCL search finds
  the feasible lower version. This *is* resolvo's reason to exist.
- **resolvo fails where naive solves ⟹ always a bug.**
- **both fail ⟹ pass without comparing error discriminants.** Unlike the
  `Sat` cell — which re-emits naive's own `SolveError` verbatim and so
  shares its discriminant — resolvo produces its own richer errors
  (`Unsatisfiable` with a derivation). Demanding discriminant equality
  would force resolvo to throw away its better diagnostics. The relaxation
  is deliberate and recorded here (card scaffold-d: "a divergence is
  recorded with its debt id before the assertion is relaxed").

The capability / disjunction / conflict / obsolete / weak-dep vocabulary
is *not* exercised by the generator (it emits `[requires.packages]` only);
those land with their own unit tests mirroring `naive/tests.rs`, plus
resolvo-only cases (disjunction backtracking) naive cannot pass.

---

## 5. Abstraction & the swap requirement {#swap}

**Requirement (owner, 2026-06-14).** Changing the resolver engine — a
different solver, or a future incompatible resolvo major — must cost
*one new provider/adapter and nothing else*. The consumer-facing
`DepSolver` and the world-facing `DepProvider` are the stable boundaries;
everything resolvo-specific is confined to `ResolvoDepSolver` +
`VibevmResolvoProvider` + `SemverVersionSet`.

Concretely, a future swap is: write a new `impl DepSolver` whose adapter
bridges the same vibevm `DepProvider` world to the new engine, register it
as a `#[cell]` variant, and add it to the differential oracle so it is
held to the same dominance contract. No consumer, no manifest, no lockfile
change. This is the same `GitBackend`-style indirection PROP-001 uses to
keep the git backend swappable, applied to the solver.

---

## 6. Phases / staging {#phases}

Each slice is one topic commit with green gates; any slice is a safe stop.

- **S0** — this document + the PROP-003 §2.2 supersede note.
- **S1** — `DepProvider::list_versions` across all impls + doctests.
- **S2** — `resolvo` dep; `ResolvoDepSolver` + `VibevmResolvoProvider` +
  `SemverVersionSet` + shared output builder + `SolveError::Unsatisfiable`;
  `[requires.packages]` + newest + narrowing + single-version; unit tests.
- **S3** — `differential_naive_vs_resolvo_dominance` in the oracle.
- **S4** — capabilities, `[[requires_any]]`, `[conflicts]`, `[obsoletes]`.
- **S5** — weak-deps + `[features.exclusive]`.
- **S6** — `[meta].solver` + `--solver resolvo` + default flip to resolvo.
- **S7** — gates (conform / specmap / file-length / self-check), WAL +
  CONTINUE, mirror rollout.

naive and sat stay in tree: naive as the small-graph fast path and the
oracle's reference cell; sat as a recorded pure-Rust backtracker. The
default solver becomes `resolvo` at S6.

---

## 7. Determinism & performance {#determinism}

- **Determinism.** `list_versions` returns a stable order; `sort_candidates`
  is a total order over `semver::Version`; the output builder re-sorts into
  roots-first + `(group, name)` order. Given deterministic provider
  responses, `solve` is deterministic — the property the oracle's
  `solve_is_deterministic` pins.
- **Performance.** resolvo's laziness preserves vibevm's network profile:
  a package's versions are fetched only when the search first asks for that
  name, a manifest only when a solvable is explored. "Prefer newest" costs
  a sort, not a separate optimisation pass. At vibevm's scale (hundreds of
  packages, depth-3 graphs — PROP-003 §3.4) the solve is far from the
  bottleneck; the fetch layer is, and laziness is the right lever there.

---

## 8. Future work {#future-work}

- **Capability resolution via a registry reverse-index.** The near-term
  capability handling (§3) resolves `[requires.capabilities]` against a
  pre-scan of the transitive package closure — correct, and strictly
  stronger than naive's already-seen-graph matching, but it forgoes
  laziness for capabilities and only finds providers reachable through
  the package-dependency graph. A fuller design adds a real
  `capability → providers` **reverse-index** to the registry (the
  git-backed registry has none today): the resolver would then enumerate
  capability providers *lazily*, exactly as it enumerates package
  versions, and so find providers that no package edge references. This
  is new registry infrastructure — an index format, a publish-time
  emitter, and a query path — recorded here as the capability layer's
  natural evolution. Not scheduled; the trigger is capability routing
  across packages-not-yet-seen becoming load-bearing.

## 9. References {#references}

- resolvo: <https://github.com/prefix-dev/resolvo> (BSD-3-Clause), crate
  `resolvo 0.11` (MSRV 1.85.1); `Interner` + async `DependencyProvider`,
  `Solver` / `Problem` / `Requirement` / `KnownDependencies` /
  `conflict::Conflict`, `utils::{Pool, VersionSet}`, `runtime::NowOrNeverRuntime`.
- [PROP-003](PROP-003-dep-evolution.md) — dependency-model evolution
  (vocabulary retained; §2.2 backend superseded here).
- [PROP-002 §2.8](../vibe-registry/PROP-002-decentralized-registry.md) —
  the `DepSolver` / `DepProvider` seam.
- [PROP-000 §3](../../common/PROP-000.md) — permissive-license policy
  (resolvo BSD-3-Clause is clean).
