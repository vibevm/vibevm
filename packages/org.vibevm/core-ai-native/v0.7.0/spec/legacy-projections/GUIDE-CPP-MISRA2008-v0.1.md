# GUIDE — C++ (MISRA C++:2008 profile) under the Discipline, v0.1

**Status.** Beta; one of three sibling C++ profiles (`cpp-traditional`, `cpp-modern`, `cpp-misra2008`). Sections isomorphic vertically and horizontally. **Precedence rule of this profile:** where a Discipline binding and a MISRA guideline conflict, MISRA wins; the Discipline may only *add* strictness, never relax — the same only-stricter direction the GRP matrix itself enforces.

**Normative stack.** Rule corpus: *MISRA C++:2008* (228 rules in three classes — Required, Advisory, Document — targeting ISO C++:2003). Process layer: *MISRA Compliance:2020*, which supersedes the compliance/deviation/process sections of the 2008 text. This guide does not re-litigate MISRA; it **operationalizes** it: the Discipline's machinery turns the Compliance:2020 paper organs into executable, versioned artifacts (§8). Successor note: MISRA C++:2023 (C++17-based, absorbing the AUTOSAR guidelines) is the edition a new contract should probably cite; this profile is pinned to 2008 because that is the normative text in hand — a `cpp-misra2023` profile is a mechanical sibling once that text is available.

Framing note. This profile is not a style preference but a contractual condition: automotive, rail, and medical supply chains demand demonstrable compliance, with a named technical authority behind every deviation. That last clause is axiom A4 arriving from the outside world: **the agent may draft deviation records; only the owner approves them.** Licensing note: the MISRA texts are copyrighted; this guide cites rule numbers and paraphrases tersely, and the repo must hold its own licensed copies rather than reproductions.

**Scope honesty.** Delivered target code only. Test harnesses, build tooling, and generators are outside the MISRA scope (they are not delivered) and follow `cpp-traditional` instead; the oracle logic that *is* compiled into both worlds must satisfy this profile (§7).

---

## 0. Language baseline {#baseline}

- **C++03, conforming mode, no extensions** — the only profile where the language version is pinned by the normative text itself.
- **The GRP is a committed artifact:** `terraform/registry/misra-grp.json` records each guideline's class and the project's re-categorization. The only-stricter matrix of Compliance:2020 §5 (Required→Mandatory allowed; Mandatory immovable; Advisory may move anywhere including Disapplied-with-rationale) is enforced by a conform meta-check **on the GRP file itself** — an illegal re-categorization fails the build, not a review.
- **House additions on top of MISRA** (legal: projects may add rules; these are additions, not relaxations):
  - **Exceptions disabled** by build flag. MISRA *regulates* exceptions (15-0-1 confines them to error handling; 15-3-1 to after-startup/before-termination; 15-5-1 forbids throwing destructors; 15-4-1 polices exception-specifications) — this profile takes the stricter branch: with the heap banned anyway (below), exception machinery buys little and costs analysis; the 15-x rules stay in the GEP as guarded-vacuous.
  - **Virtual bases avoided:** 10-1-1 (Advisory: don't derive from virtual bases) is GRP-promoted to Mandatory; with virtual bases gone, the one place MISRA *requires* `dynamic_cast` (5-2-2: virtual-base downcast) becomes unreachable, downcasts are banned outright (promoting Advisory 5-2-3), and **RTTI is disabled** consistently.
- **Already in the corpus, congenial to the Discipline:** no heap (18-4-1); no unions (9-5-1); no C library (18-0-1), no `<cstdio>` (27-0-1), no `<ctime>` (18-0-4), no `errno` (19-3-1), no `setjmp`/`longjmp` (17-0-5); C-style casts banned (5-2-4); function-like macros banned (16-0-4), `#undef` banned (16-0-3); size-named scalar typedefs (3-9-2 → `uint8_t`-family); no shadowing (2-10-2); single-arg constructors `explicit` (12-1-3); dead/unreachable/unused code banned (0-1-1, 0-1-9, 0-1-10..12).
- **No-heap consequence stated plainly:** allocating standard containers are unusable in delivered code; cells use fixed-capacity arrays and pools sized at compile time. This single rule reshapes §1 and §3 more than any other.
- **Suppression policy:** an in-source tool suppression is legal **only when it cites a deviation id** (`D-NNNN`); conform's T-lex sweep fails any suppression token without one, and any `D-NNNN` without a registry record. Deviations live as records (§5, §8), never as naked pragmas.
- **Checkers:** commercial MISRA analyzers slot in as conform **evidence providers** (ENGINE §2) via their reports; their published coverage matrices — fully / partially / not statically checkable, as in the LDRA-style summaries — import directly as GEP fragments declaring per-rule enforcement strength.

## 1. Cells {#cells}

A cell is a static-library target behind one seam, as in the sibling profiles — under static-allocation physics:

- **All cell instances are statically allocated.** No factories returning ownership (nothing to own); construction is **two-phase**: trivially-constructible statics, then an explicit `init()` pass invoked from `main` in deterministic order before any use — the profile's answer to the initialization-order fiasco, since `constinit` does not exist in C++03. Registration-at-load is banned as in every profile.
- **No sibling-cell includes** (R-002, T-syn include graph); ODR discipline is normative here (3-2-2).
- **Platform capabilities are injected** — doubly mandatory in this profile, since the C library is gone (18-0-1) and time/io/env have no legal ambient form anyway. The capability seams are where boundary modules wrap whatever the certified platform provides.
- **Brownfield resonance:** the 0-1-x family (unreachable, dead, unused) is enforced as a **ratchet** per BROWNFIELD §B1 — baseline first, monotone shrink after — not as a day-one wall.

## 2. Seams {#seams}

- Seams are pure abstract base classes; overriding functions repeat the `virtual` keyword (10-3-2 — C++03 has no `override`), exactly one definition per virtual in the hierarchy (10-3-1), pure-virtual overridden only by pure-virtual where intended (10-3-3).
- Non-copyability uses the C++03 idiom MISRA itself codifies: copy assignment declared protected/private in abstract classes (12-8-2); copy constructors private and undefined.
- Member data private (11-0-1); composition over inheritance MUST; no behavior-bearing bases; no virtual call reaches a derived body from ctor/dtor (12-1-1).
- **Seam ABI under no-heap:** methods take inputs by `const&` and produce through caller-provided storage (§4) — payload ownership never crosses the seam because ownership transfer does not exist here.

## 3. Registry and flags {#flags}

- **The build tier dominates.** Link-time selection (which variant's library is linked) is the primary flag mechanism: it costs nothing at runtime and no static footprint for unselected variants. Runtime selection is still legal but priced honestly: every selectable variant is a live static instance, and the registry returns a `Seam&` chosen once at startup.

```cpp
// registry.cpp — the only flag reader; no heap (18-4-1), so selection binds references.
DepSolver& depSolver(Config const& cfg)
{
    static_assert(true, "");
    // both instances statically allocated; init() called from main before first use
    if (cfg.solver == Config::SolverSat) { return g_satDepSolver; }
    return g_naiveDepSolver;                     // single exit shape per 6-6-5
}
```

- Preprocessor confinement is normative, not just house style: `#define` only at global scope (16-0-2), no `#undef` (16-0-3), no function-like macros (16-0-4), include hygiene per 16-0-1.
- No self-registration, no singletons-as-wiring — same as siblings, with less language left to cheat with.

## 4. Errors as contract {#errors}

The isomorphism "expected failures are values" survives, in its oldest clothing:

- **Status-enum return + caller-provided out-parameter.** No exceptions (house), no heap (18-4-1), no unions (9-5-1), no `variant` (C++03): the failure *is* the return value, the payload travels through a reference the caller owns. This is an honest degradation from value-semantic `Result` and the guide says so.
- **Every error-bearing return is consumed:** 0-3-2 (error information shall be tested) and 0-1-7 (non-void returns used) make the Discipline's "errors are part of the contract" a *normative obligation* with tool enforcement — the one place MISRA is stricter than the sibling profiles.
- **REQ provenance:** each Status enumerator maps to a `static const char*` REQ URI in a per-seam table; rendering at the boundary appends it (PROP-014 §2.6).
- **Exhaustiveness under 6-4-6:** MISRA mandates a final `default` clause — which would silence `-Werror=switch`. Resolution: the `default` arm is a trap (assert-fail handler), and **enum coverage moves to a T-sem conform check** that proves every enumerator has an explicit case. Same Discipline rule as the siblings, third binding.
- Single point of exit (6-6-5) and terminated `else if` chains (6-4-2) shape function bodies; run-time failure minimization is a documented strategy per 0-3-1 (a Document-class obligation the conform report links to).
- **Concurrency:** out of the 2008 corpus's scope and out of this profile's cells; any threading lives behind platform capability seams under the project's safety case.

## 5. specmark carrier {#specmark}

The shared `/// @spec` carrier (Traditional §5) — chosen precisely because it survives C++03 toolchains. Two profile-specific extensions:

- **External normative namespace:** MISRA guidelines enter the specmap as read-only foreign units under `misra://cpp2008/<rule-id>` (e.g. `misra://cpp2008/9-5-1`). Code never `implements` them; it may only `deviates` them. This is the first foreign namespace in the specmap and is flagged as a pending PROP-014 amendment.
- **Deviation citation form:**

```
/// @spec deviates misra://cpp2008/<rule-id> permit=<permit-id> reason="..." deviation=D-NNNN
```

  The line is the in-source anchor; the *record* lives in the registry (§8) carrying the Compliance:2020 §4.2 fields. Reason mandatory; `D-NNNN` mandatory; permit optional but preferred (§8).

## 6. Naming (R-020/R-021 bindings) {#naming}

- Computed `{Variant}{Seam}` grammar as everywhere.
- The theater list is largely **subsumed by the corpus itself**: implicit conversions (12-1-3), function-like macros (16-0-4), C-style casts (5-2-4), shadowing (2-10-2), const-stripping (5-2-5), array decay at calls (5-2-12). House residue: template metaprogramming confined to infra (templates are regulated, 14-x, not banned — but TMP in domain cells fails R-021 regardless); operator overloading beyond value semantics; `friend` beyond test fixtures.

## 7. Replacement protocol (R-040 binding) {#replacement}

Differential oracle as in the siblings, with a scope split: the **harness** (generators, comparison driver) lives outside MISRA scope and follows `cpp-traditional`; the **cells under test** compile under this profile unchanged. Golden artifacts follow the promotion protocol; a healed known-failing case is promoted, never silently absorbed (xfail-strict, BROWNFIELD §4).

## 8. The Compliance:2020 framework, operationalized {#framework}

The discovery this profile is built on: MISRA Compliance:2020 and the Discipline are convergent evolution — the same organs grown independently. The mapping is therefore mechanical:

| Compliance:2020 organ | Discipline realization |
|---|---|
| Guideline Enforcement Plan (§3.3) | conform's rule registry: guideline → check-id → tier (T-lex/T-syn/T-sem) → tool+version+config; commercial-checker coverage matrices import as GEP fragments; manual-review rows become review-procedure records |
| Re-categorization Plan (§5) | `misra-grp.json`, committed; only-stricter matrix enforced by a conform meta-check on the file |
| Deviation record (§4.2) | registry record `D-NNNN` (guideline, use-case, Reason 1–4, requirements, scope, approver) + in-source `@spec deviates` anchor; **agent drafts, owner approves — A4** |
| Deviation permit (§4.3) | `spec/permits/*.md`: pre-approved use-cases deviations cite; negotiated up front, exactly as the document recommends |
| Message investigation (§3.4, categories 2–4) | **intent-ledger interpretation** keyed (file-hash, finding-id, tool+version): the justification for a false-positive or benign finding is cached and survives until its epoch breaks — never pay twice for the same investigation (A2) |
| Decidability (§3.5) | decidable rules bind to T-lex/T-syn/T-sem checks; undecidable rules get tool-"possibly" handling: conservative idioms + investigation records; enforcement strength is *declared*, not implied |
| Adopted code (§6) | boundary modules under their own GRP scope; this is also the composition semantics for **mixed-profile repositories** — a `cpp-modern` target seen from a `cpp-misra2008` target is adopted code |
| Compliance Summary (§7.3) | `conform report --gcs`: per-guideline Compliant / Deviations / Violations / Disapplied, generated at release, signed into the release slice (LEDGER §release) |

## 9. Doc layer {#docs}

Doxygen on tagged exports as in the siblings, plus the release obligations: the GCS, the GEP, deviation records and permits ship with the delivery package per Compliance:2020 §7.4 — all generated artifacts of the registries, not hand-maintained prose.

---

**First carrier note.** No C++ exists in vibevm, and this profile additionally requires licensed normative texts and (for a credible GEP) a qualified commercial checker as evidence provider. It ships genre-complete, unexercised, and carrier-relative under the same house clause as its siblings.
