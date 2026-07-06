# GUIDE — C++ (Modern profile) under the Discipline, v0.1

**Status.** Beta; one of three sibling C++ profiles (`cpp-traditional`, `cpp-modern`, `cpp-misra2008`). Sections isomorphic vertically (Rust/TS/Python guides) and horizontally (sibling profiles). Profile granularity is per build target; cross-profile boundaries follow the adopted-code semantics referenced in the Traditional guide's header.

Framing note — the opposite bet from Traditional. Instead of shrinking C++ to its 2010 core, this profile rides the standard's own convergence toward the Discipline: **concepts make seam contracts compiler-checked, `std::expected` makes errors values, `constinit` makes import-purity a keyword, `std::variant` makes exhaustiveness free.** The subset is still selected — but selected from C++23, bounded by what GCC, Clang, and MSVC all ship today. Patterns dissolve nearly as far as in Rust: Strategy = concept + composition root; Visitor = `variant` + `visit`; Decorator = wrapper; Observer = explicit signal seam; Singleton = forbidden.

**Scope honesty.** For code targeting current mainstream toolchains. Conservative/vendor toolchains → `cpp-traditional`; certification contracts → `cpp-misra2008`. Per-feature honesty: features enter the baseline only when all three majors ship them; this guide marks the exceptions explicitly rather than pretending the front is even.

---

## 0. Language baseline {#baseline}

- **Standard: C++23**, restricted to the three-major intersection. In baseline: `std::expected`, `std::print`, ranges, concepts, `constinit`/`consteval`, `std::jthread`/`stop_token`, designated initializers, `std::span`/`string_view`. **Out of baseline, named honestly:** modules (toolchain/CMake shear still real — per-cell experiments only, behind a build option); C++26 contracts and static reflection (the latter is the future native specmark carrier — when reflection lands across the majors, `/// @spec` comments become introspectable annotations; noted, not used).
- **Exceptions: panic-only.** The standard library throws, so `-fno-exceptions` is dishonest here; instead: `throw` is reserved for invariant violations (the panic analog), **expected failures cross seams as `std::expected`**, and every seam function is either `noexcept` or `expected`-returning — the failure surface lives in the signature, compiler-visible (§4).
- **`-fno-rtti` stays.** Closed-set dispatch is `variant`+`visit`; open-set is virtual dispatch; `dynamic_cast` has no legitimate cell use.
- **Warnings:** `-Wall -Wextra -Wconversion -Werror`; clang-tidy (`modernize-*`, `bugprone-*`, `cppcoreguidelines-*`, `concurrency-*`) as SARIF evidence provider; `compile_commands.json` mandatory. Suppression policy identical to Traditional §0: `NOLINTNEXTLINE(<check>): <reason>` only, stale-suppression sweep fails the gate.
- **`constexpr`-first:** pure logic SHOULD be `constexpr`; `consteval` where compile-time-only is the contract. A `static_assert` over `constexpr` logic is a test that costs nothing at runtime — prefer it where it fits.
- **Build/deps:** CMake presets + vcpkg manifest mode (MIT), lockfile committed (A2). Sanitizers (ASan/UBSan, TSan for concurrent cells) gate the test suite; libFuzzer feeds differential oracles (§7).

## 1. Cells {#cells}

A cell is a static/OBJECT CMake target behind one seam, public headers via CMake file sets.

- **Import-purity becomes a keyword.** Every namespace-scope object in a cell is `constexpr` or `constinit` — dynamic initialization at load is thereby a *compile error*, not a lint finding. The same rule that is a T-lex grep in Traditional and a two-phase-init convention in MISRA is compiler-enforced here; this tier upgrade is the profile's best argument for itself.
- **No sibling-cell includes** (R-002); include graph checked at T-syn; cross-seam type refs through seam headers only.
- **Platform capabilities are injected** (clock, filesystem, network, randomness, log sink) as seams; cells take them by reference at construction. No ambient singletons, no `std::filesystem` calls from domain code.
- **Promotion** to a separately versioned library on the usual triggers (heavy optional deps / release cadence / ~2 kLoC).

```cpp
/// @spec implements spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade r2
/// @cell seam=DepSolver variant=sat replaces=naive flag=solver
class SatDepSolver final : public DepSolver { /* ... */ };
```

## 2. Seams {#seams}

- **Concepts are the primary seam form** for in-process composition: the contract is compiler-checked at the use site, zero virtual cost, and the registry monomorphizes at the composition root.
- **Runtime-selected seams** (anything behind a flag) get the type-erased form: a pure ABC or a small hand-rolled erased wrapper. The concept and the ABC describe the same contract; conform checks the ABC satisfies the concept (`static_assert(DepSolverConcept<AbcFacade>)` — drift between the two forms is a build error).
- Composition over inheritance is a MUST at seams; no behavior-bearing bases, no CRTP in domain code (infra only). Customization-point-object machinery is infra-only; domain code calls named functions.
- Values crossing seams SHOULD be aggregates with designated initializers; owning transfers are `unique_ptr`/value moves — no raw owning anywhere.

## 3. Registry and flags {#flags}

```cpp
// registry.cpp — the only flag reader.
auto make_dep_solver(Flags const& flags, DepProvider& provider)
    -> std::unique_ptr<DepSolver> {
  switch (flags.solver()) {                       // provenance: default | env | cli | lockfile
    case Solver::sat:  return std::make_unique<SatDepSolver>(provider);
    default:           return std::make_unique<NaiveDepSolver>(provider);
  }
}
```

- **Two tiers:** CMake-level target inclusion ("is it in the binary" — the cargo-feature analog) vs runtime selection. Same invariants as Traditional §3: build option never changes a seam surface; runtime flag never requires recompilation; `#ifdef` confined to the generated config header and registry sites.
- **Delivery-mode honesty:** in-process C++ has no cheap lazy code loading; eager is the only mode, presence is the build tier's job. Shared-library plugin loading is boundary infrastructure, not a cell concern.
- No self-registering statics (now impossible in cells anyway — `constinit` killed them), no DI frameworks, no link-seam magic.

## 4. Errors as contract {#errors}

- **Expected failures are values:** seams return `std::expected<T, E>`; `E` is a small struct carrying `code` and the violated REQ URI (`static constexpr std::string_view spec`); rendering appends the URI (PROP-014 §2.6). `std::error_code` interop MAY exist at boundaries.
- **`throw` = invariant violation only** (panic analog); never throw across a seam — every seam function is `noexcept` or returns `expected`, and conform checks exactly that disjunction (T-sem). Inside a cell, RAII + exceptions for truly exceptional paths is legal; the seam is where the contract is owed.
- **Exhaustiveness for free:** closed sets are `std::variant`; `std::visit` with an exhaustive overload set **fails to compile** when an alternative is unhandled — the `assert_never` analog needs no discipline, only the ban on catch-all `auto&&` arms in domain visitors (T-syn).
- **Structured concurrency:** `std::jthread` + `stop_token` over raw `std::thread`; `.detach()` banned; coroutines are legal **only behind an async seam that owns their lifetime** — no detached `co_spawn`-style fire-and-forget. Direction note: senders/receivers (`std::execution`, C++26; stdexec today, Apache-2.0) is where this profile's async seam form is headed; not yet baseline.
- **Lifetime honesty:** `string_view`/`span` never cross a seam into storage; parameters yes, members no (T-sem: tidy dangling checks + review).

## 5. specmark carrier {#specmark}

Identical to Traditional §5 — the shared `/// @spec` Doxygen-comment carrier, read via libclang comment attachment; one carrier across all three profiles because uniformity within a language beats per-profile cleverness. Future note: C++26 static reflection is the path to a native, introspectable carrier; when the three majors ship it, a `specmark` annotation library supersedes comments by mechanical migration. ≤3 edges per item or split.

## 6. Naming (R-020/R-021 bindings) {#naming}

- Computed `{Variant}{Seam}` → `SatDepSolver`; linted against the manifest.
- **Forbidden in cells regardless of elegance:** `std::enable_if`/SFINAE where a concept suffices (concepts are MUST for constraints); macro metaprogramming; implicit conversions (`explicit` MUST, conversion operators banned); operator overloading beyond value semantics; ADL customization tricks in domain code; `reinterpret_cast`/`const_cast` outside boundary files; clever `auto` that hides ownership (owning returns are spelled `std::unique_ptr<T>` or `T`, never deduced).

## 7. Replacement protocol (R-040 binding) {#replacement}

A cell with `replaces=` ships a differential oracle: property-style tests driving old and new cells through the seam — libFuzzer or seeded generators — asserting agreement modulo a documented divergence list; `/// @verifies`-tagged; sanitizers on. Golden artifacts follow the promotion protocol (CI never regenerates).

## 8. Risk table (what conform must cover for this profile) {#risks}

| Footgun | Rule | Tier |
|---|---|---|
| namespace-scope object that is not `constexpr`/`constinit` | §1 | compiler |
| `throw` escaping a seam; seam neither `noexcept` nor `expected` | §4 | T-sem |
| `string_view`/`span` stored beyond the call | §4 | T-sem (tidy) |
| catch-all `auto&&` arm in a domain `visit` | §4 | T-syn |
| raw `new`/owning raw pointer | §2 | T-syn + tidy |
| `std::thread` raw / `.detach()` / unowned coroutine | §4 | T-syn |
| `enable_if` in domain code where a concept fits | §6 | T-syn |
| `dynamic_cast`/`typeid` token (RTTI off) | §0 | T-lex |
| sibling-cell include | R-002 | T-syn |
| `#ifdef` inside a cell body | §3 | T-lex |
| bare `NOLINT` / stale suppression | §0 | T-lex + sweep |
| flag read outside the registry | R-001 | T-syn |
| public export without own/inherited spec edge | PROP-014 §3.2-6 | T-syn + index |

## 9. Doc layer {#docs}

Doxygen on every tagged export: error codes with REQ URIs, lifetime/ownership of every view and pointer, `noexcept` rationale, concurrency assumptions, complexity traps. Carrier doubles as doc line; spec stays thin.

---

**First carrier note.** No C++ exists in vibevm; profile ships genre-complete but unexercised. House clause is carrier-relative, identical to the Traditional guide's footer.
