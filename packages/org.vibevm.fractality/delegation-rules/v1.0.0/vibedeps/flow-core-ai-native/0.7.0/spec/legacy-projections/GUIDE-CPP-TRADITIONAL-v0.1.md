# GUIDE — C++ (Traditional profile) under the Discipline, v0.1

**Status.** Beta; one of three sibling C++ profiles (`cpp-traditional`, `cpp-modern`, `cpp-misra2008`). Section structure is isomorphic vertically (to the Rust/TS/Python guides) and horizontally (across the three profiles — diff them; the profiles rewire *bindings*, never principles). Profile granularity is **per build target**: one repository may host targets under different profiles; a cross-profile boundary is treated as an adopted-code relationship (see `GUIDE-CPP-MISRA2008-v0.1.md` §0, which imports that semantics from MISRA Compliance:2020 §6).

Framing note. C++ is the language where subset selection is not an instrument but survival. The Traditional profile is the subset that has compiled everywhere since the early 2010s and behaves identically everywhere: the dialect of embedded, games, trading systems, and long-lived vendor toolchains. Its bet: *shrink to the eternal core*. Patterns here only half-dissolve: Strategy = abstract base + factory; Decorator = wrapper; Observer = explicit callback seam; Visitor genuinely survives for open sets — but closed sets use `enum class` + `switch` (§4). Singleton = forbidden; the Meyers function-local static is its C++ disguise.

**Scope honesty.** For system cores that must build on conservative toolchains (vendor compilers, console SDKs, decade-old enterprise GCC). Greenfield code on current mainstream compilers belongs in `cpp-modern`. Certification-bound code belongs in `cpp-misra2008`.

---

## 0. Language baseline {#baseline}

- **Standard pinned: C++14**, `-pedantic-errors`, no compiler extensions. (C++11 fallback is a documented per-target deviation, not a default.)
- **`-fno-exceptions -fno-rtti`** project-wide — the defining old-believer move. Expected failures are values (§4); invariant violations are `assert` + configured abort handler. `typeid`/`dynamic_cast` do not exist; closed-set dispatch is `enum class` + `switch`, open-set dispatch is virtual functions.
- **Warnings:** `-Wall -Wextra -Wconversion -Werror`; **`-Werror=switch`** is load-bearing (§4 exhaustiveness).
- **Evidence providers:** clang-tidy (`bugprone-*`, `cppcoreguidelines-*`, `performance-*`, selected `google-*`) consumed via conform as SARIF-class evidence; clang-format for layout (out of discipline scope). `compile_commands.json` is mandatory — it is what feeds T-sem.
- **Suppression policy (xfail-strict by construction):** bare `// NOLINT` is banned; only `// NOLINTNEXTLINE(<check-id>): <reason>` is legal. A conform sweep re-runs the named check against suppressed lines and **fails on stale suppressions** — the suppression registry shrinks truthfully, same mechanism as `@ts-expect-error` and pyright's unnecessary-ignore.
- **Ownership:** `std::unique_ptr` for owning, raw pointers/references are non-owning observers by convention (clang-tidy `cppcoreguidelines-owning-memory` assists); naked `new`/`delete` outside factories is banned.
- **Build/deps:** CMake with pinned toolchain files; third-party code is vendored (`third_party/`) or pinned via `FetchContent` — no live package-manager resolution at build time (reproducibility, A2).
- **Tests:** GoogleTest (BSD-3). **Sanitizers are the safety net the compiler era lacks:** ASan+UBSan on the test suite is a MUST gate; findings are failures, not warnings.

## 1. Cells {#cells}

A cell is a static-library CMake target behind one seam: one public include directory exposing the seam header and the factory declaration — nothing else.

- **Import-is-execution, C++ edition: the static initialization order fiasco.** Namespace-scope objects with dynamic initialization are banned in cells — top level admits constants (`constexpr`/POD), type and function definitions. No registration-at-load, no global ctors. Enforced: T-syn (clang-tidy `cppcoreguidelines-avoid-non-const-global-variables` + a conform check for dynamic initializers).
- **No sibling-cell includes** (R-002), including transitively; the include graph is checked at T-syn from `compile_commands.json`. Cross-seam type references go through the seam header only.
- **Platform capabilities are injected:** cells never touch files, sockets, clocks, environment, or global loggers directly — those are seams passed at construction. Consequence: cell tests need no link-time substitution tricks.
- **PImpl SHOULD** be used by cells whose seam crosses a binary or team boundary — the compile-time firewall is this profile's substitute for modules.
- **Promotion** to a separately versioned library when: heavy optional deps, independent release cadence, or ~2 kLoC.

Cell manifest (carrier, §5):

```cpp
/// @spec implements spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade r2
/// @cell seam=DepSolver variant=sat replaces=naive flag=solver
class SatDepSolver final : public DepSolver { /* ... */ };
```

## 2. Seams {#seams}

- A seam is a **pure abstract base class** in core/seams: virtual destructor, no data members, no implemented methods, non-copyable. Factory functions return `std::unique_ptr<Seam>`.
- **Composition over inheritance is a MUST at seams:** no base classes with behavior, no template methods (hidden control flow, R-021). Implementation inheritance is confined to cell internals; a cell derives from at most one implementation class plus pure interfaces.
- Templates as seams (static polymorphism) are legal *inside* a cell; **cross-cell seams are runtime ABCs** — they keep the compile firewall and stay analyzable without instantiating the world.
- Seam methods that can fail in expected ways return `Result` (§4); the failure surface is in the header.

## 3. Registry and flags {#flags}

R-001 binding — flag at the seam, never in the veins:

```cpp
// registry.cpp — the only flag reader in the binary target.
std::unique_ptr<DepSolver> makeDepSolver(Flags const& flags, DepProvider& provider) {
    switch (flags.solver()) {                  // provenance: default | env | cli | lockfile
        case Flags::Solver::Sat:   return std::unique_ptr<DepSolver>(new SatDepSolver(provider));
        default:                   return std::unique_ptr<DepSolver>(new NaiveDepSolver(provider));
    }
}
```

- **Two tiers, never confused:** CMake options that include/exclude *targets* answer "is the code in the binary" (the cargo-feature analog — selection by linking, not by `#ifdef` soup); runtime flags answer "is the cell selected". A build option must not change a seam's surface; a runtime flag must not require recompilation.
- **Preprocessor confinement:** product `#ifdef` is legal only in the CMake-generated config header and at registry/adapter sites — never inside cell bodies (T-lex).
- **No link-time registration magic:** no self-registering statics, no `__attribute__((constructor))`, no singleton registries. The hand-written `switch` is the system's table of contents.

## 4. Errors as contract {#errors}

With exceptions off, the language finally tells the truth in signatures:

- **Expected failures are values.** A minimal vendored `Result<T, E>` (single header in core; `tl::expected` MAY be vendored instead — CC0). `E` carries `code` plus the violated REQ URI as a `static constexpr char const*` on the error category; user-facing rendering appends the URI (PROP-014 §2.6).
- **Invariants are `assert` + abort policy** — the panic analog; there is no `throw` to abuse. Release builds keep a configured subset of assertions (cheap ones stay).
- **Exhaustiveness:** closed sets are `enum class` handled by `switch` **without a `default` clause**, under `-Werror=switch` — the compiler errors on a missing enumerator. This is the profile's `assert_never`; adding `default` to silence it is the graveyard move and is banned (T-syn).
- **Out-parameters are banned at seams** in this profile — `Result` carries the payload; reference out-params survive only in hot inner loops inside a cell, documented.
- **Threading:** this profile predates structured concurrency; cells are single-threaded by default, and any thread ownership is a seam (`Executor`-style), never a detached `std::thread` (T-syn: `.detach()` banned).

## 5. specmark carrier {#specmark}

All three C++ profiles share one carrier — the Doxygen-style triple-slash comment — because comments are the only channel that survives every toolchain from C++03 vendors to current clang, and (as in TS) the carrier doubles as the hover/doc surface:

```
/// @spec implements <uri> r<N>              one edge per line; lines repeat
/// @spec deviates <uri> r<N> reason="..."   reason mandatory
/// @verifies <uri> r<N>                     above TEST(...)
/// @specScope <uri> r<N>                    file-top block: file-level inheritance
```

The sidecar reads them via libclang's comment attachment; T-lex regex is the degraded fallback on toolchains without libclang. C++11 attributes (`[[vibevm::spec(...)]]`) were considered and rejected: unknown-attribute warnings vary by compiler and the form buys nothing over comments here. ≤3 edges per item or split.

## 6. Naming (R-020/R-021 bindings) {#naming}

- Canonical cell type name is computed: `{Variant}{Seam}` → `SatDepSolver`; hand-written names are linted against the manifest. Length free, ambiguity not.
- **Forbidden in cells regardless of elegance** — the Traditional theater list: non-`explicit` single-argument constructors and implicit conversion operators (`explicit` is MUST); operator overloading beyond value semantics (no DSL operators); function-like macros in domain code; template metaprogramming and CRTP in domain cells (infra libraries only — the proc-macro parallel); default arguments on virtual functions; virtual calls in constructors/destructors; `const_cast`/`reinterpret_cast` outside designated boundary files; Meyers singletons as wiring; `friend` beyond test fixtures.

## 7. Replacement protocol (R-040 binding) {#replacement}

A cell with `replaces=` ships a differential oracle: GoogleTest parameterized suites driving both cells through the seam with deterministic seeded generators, asserting agreement (documented-divergence list otherwise), `/// @verifies`-tagged, run under sanitizers. Golden files follow the promotion protocol — CI never regenerates; local regeneration carries a debt/intent reference in the commit body.

## 8. Risk table (what conform must cover for this profile) {#risks}

| Footgun | Rule | Tier |
|---|---|---|
| dynamic-initialized namespace-scope object in a cell | §1 | T-syn |
| naked `new`/`delete` outside factories; owning raw pointer | §0 | T-syn + tidy |
| `throw`/`catch`/`typeid`/`dynamic_cast` token anywhere | §0 | T-lex |
| `default` clause on a closed-enum `switch` | §4 | T-syn |
| sibling-cell include (direct or transitive) | R-002 | T-syn |
| non-`explicit` single-arg ctor / conversion operator | §6 | T-syn |
| virtual call in ctor/dtor | §6 | T-sem |
| object slicing (polymorphic pass-by-value) | §2 | T-sem (tidy) |
| uninitialized members / fields | §0 | T-sem (tidy) |
| `#ifdef` inside a cell body | §3 | T-lex |
| `std::thread::detach`; thread created outside an Executor seam | §4 | T-syn |
| bare `NOLINT` / stale suppression | §0 | T-lex + sweep |
| flag read outside the registry | R-001 | T-syn |
| public export without own/inherited spec edge | PROP-014 §3.2-6 | T-syn + index |

## 9. Doc layer {#docs}

Doxygen on every tagged export: error codes and their REQ URIs, ownership and lifetime expectations of every pointer/reference parameter, threading assumptions, edge cases. The `/// @spec` line is already a Doxygen line — zero extra machinery. Spec stays thin; duplication between Doxygen and spec is a defect on the spec side.

---

**First carrier note.** No C++ exists in vibevm; this profile ships genre-complete but unexercised. The house clause is therefore carrier-relative:

*Rules binding here remain DRAFT until a first C++ carrier exists; at that carrier's first milestone, any rule without a conform check (or explicit `WISH` mark in the Charter rule record) is removed rather than carried as aspiration.*
