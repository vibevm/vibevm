# JVM-lineage visibility & transitivity — research notes for PROP-050

> Research worker report, 2026-08-23. Consumer: [PROP-050 §5](../../common/PROP-050-dependency-visibility.md#prior-art).
> Scope: JPMS, Java sealed classes, Kotlin `internal` / explicit API, Swift access levels / SE-0386 / SE-0409 / `@_spi`.

## 0. One-line orientation

Four systems, four different answers to "who sees what, and how far does it travel". Only two of them (Swift `package`/`@_spi`, JPMS qualified exports) actually implement *friendship*; both are **provider-declared**. Our inversion (consumer-declared, budget-motivated) flips several lessons — flagged inline as **[INVERSION]**.

---

## 1. JPMS

### Primitives

| Primitive | Effect |
|---|---|
| `requires M` | This module *reads* M. Not propagated to my consumers. |
| `requires transitive M` | I read M **and** every module that reads me also reads M ("implied readability"). The re-export mark. |
| `requires static M` | Compile-time only; optional at runtime. |
| `exports pkg` | pkg's `public` types readable by any module that reads me. |
| `exports pkg to A, B` | Qualified export — only named modules. |
| `opens pkg [to …]` | Deep reflective access (incl. private members) at runtime; no compile-time access. |
| `open module` | Every package opened. Migration convenience. |
| `uses` / `provides` | Service loader wiring (an *indirect* readability path — worth noting: services deliberately bypass the naming graph). |

### Defaults & rationale

Default is **nothing exported, nothing opened, nothing read** — strong encapsulation. Rationale: reliable configuration + preventing accidental dependence on internals (the `sun.misc.Unsafe` problem the whole JSR existed to solve).

### Transitivity — exact answer

**Yes, it is a full recursive closure over multiple hops.** JLS §7.7.1: *"For each enumerated module A that `requires` B: A reads B. If B `requires transitive` C, then A reads C as well as B. This augmentation is recursive: since A reads C, if C `requires transitive` D, then A reads D as well as C and B."* ([JLS se11 §7.7.1](https://docs.oracle.com/javase/specs/jls/se11/html/jls-7.html#jls-7.7.1)). So in the A→B→C→D example **A does read D**. Stated rationale in the spec: to permit *arbitrary amounts of refactoring* — a module author can split their content into sub-modules without breaking consumers.

Note the asymmetry: `exports` is **not** transitive; readability is. Access = readability ∧ export. Two independent gates.

### Exclusion / pruning

JPMS has **no pruning primitive** — no `unfriend`, no `exclude`. Once implied readability is granted upstream, a downstream consumer cannot refuse it. The only escapes are command-line overrides that *widen*, never narrow: `--add-reads`, `--add-exports`, `--add-opens`, `--add-modules`. **This is a real gap and `unfriend`/`exclude` is the right instinct** — but note *why* JPMS could skip it: readability costs nothing at runtime. **[INVERSION]** For us readability costs tokens, so pruning is mandatory, not optional.

### Enforcement point

Compile time (`javac` — "package is not visible" / "module not found"), plus **module-resolution time** at JVM startup (missing module, split package, cycle → hard fail before `main`), plus runtime `IllegalAccessError` / `InaccessibleObjectException` for reflection. Three-phase enforcement, and resolution-time is the important one for us.

### Community verdicts

- **`requires transitive` = crutch, not design principle.** Consensus rule: use it *only* when your exported signatures mention the dependency's types (a `Driver` returning `java.util.logging.Logger` forces `java.sql requires transitive java.logging`). Otherwise make consumers declare it. Aggregator modules (`java.se`) are cited as the downside: consumers get a pile they never asked for ([Coderanch: "panacea or crutch?"](https://coderanch.com/t/746105/java/requires-transitive-JPMS-panacea-crutch), [nipafx](https://nipafx.dev/java-modules-implied-readability/), [dev.java](https://dev.java/learn/modules/implied-readability/)).
- **Qualified exports = special-case tool, not routine.** Intended for multi-module frameworks sharing internals; they hard-code consumer names into the provider, i.e. deliberate coupling. Notable detail: **target modules need not exist — an unresolvable target is a warning, not an error** ([dev.java](https://dev.java/learn/modules/qualified-exports-opens/)). Also `exports pkg` and `exports pkg to …` are mutually exclusive for the same package.
- **The flag-day lesson is brutal and directly applicable.** Classpath→module-path was a dual-world split: libraries had to test *both* because behaviour differs, `jlink`'s payoff was locked behind a *fully* explicit graph (automatic modules block it), split packages were fatal for existing artifacts, and Colebourne's verdict for library authors was literally ["negative benefits" — don't modularize yet](https://blog.joda.org/2018/03/jpms-negative-benefits.html). Adoption stalled ~a decade. The JDK-internal flip was staged over years: JDK 9 relaxed → [JEP 396](https://openjdk.org/jeps/396) default-strong with `--illegal-access` escape → [JEP 403](https://openjdk.org/jeps/403) (17) escape removed.

**COPY:** the two-gate model (readability ≠ export), resolution-time failure, warn-don't-fail on an unknown allowlist target, and the "re-export only what appears in your own surface" rule of thumb — for us: *a package marks a dep transitive only when its own boot text is unreadable without that dep's text*. **AVOID:** the aggregator pattern (a single `requires` pulling the world is exactly context seepage), the absence of consumer-side pruning, and above all the flag-day. Never make the static-lane/compaction payoff require 100 % of the closure to declare `access` — that is `jlink`'s chicken-and-egg trap verbatim.

---

## 2. Sealed classes — the allowlist lesson

| Primitive | Effect |
|---|---|
| `sealed … permits A, B` | Only the named types may directly extend. |
| `permits` omitted | Inferred from same-compilation-unit subtypes. |
| `final` / `sealed` / `non-sealed` on each permitted subtype | Mandatory: close, re-close narrower, or **reopen**. |

Constraint that matters: *"The sealed class and its permitted subclasses must belong to the same module, and, if declared in an unnamed module, to the same package"* ([JEP 409](https://openjdk.org/jeps/409); [Oracle](https://docs.oracle.com/en/java/javase/21/language/sealed-classes-and-interfaces.html)). Enforced at compile time; the JVM re-checks `PermittedSubclasses` at load time. Motivation: a *closed world* the compiler can reason over (exhaustive `switch`), which is only sound if the whitelist is verifiable — hence the same-maintenance-domain rule.

**Lesson for a friends-allowlist:** an allowlist is only trustworthy when **the allowlist and its members share one maintenance domain**, and there is an explicit, syntactically visible **escape hatch** (`non-sealed`) rather than silent widening. **[INVERSION]** Our friendship is consumer-declared at the root — that *gives* us the single maintenance domain for free (one file, one owner), which is strictly better than `permits`. The cost transfers to **name stability**: the root allowlists nodes it did not author, so package coordinates become load-bearing API. Mitigate: allowlist entries are matched against the resolved closure at install time, unmatched entries warn (JPMS qualified-export precedent), and a `non-sealed`-style per-node `open` marks a hole visibly.

---

## 3. Kotlin

| Primitive | Effect | Default |
|---|---|---|
| `public` | everywhere | **yes** |
| `internal` | visible within the **module** | — |
| `protected` / `private` | class+subclasses / class-or-file | — |

**"Module" = "a set of Kotlin files compiled together"**: an IntelliJ module, a Maven project, a Gradle source set, an Ant task invocation ([docs](https://kotlinlang.org/docs/visibility-modifiers.html)). This is the friction: the visibility boundary is **not a declared entity** — it is whatever the build tool happened to compile together, so it shifts when the build changes, and needs special cases (the `test` source set can see `main`'s `internal`). Second friction: the JVM has no `internal`, so it compiles to **public + name mangling** (`foo$module_name`); Java callers can reach it, and public members of internal classes aren't mangled at all ([Kotlin/Java interop](https://kotlinlang.org/docs/java-to-kotlin-interop.html), [4comprehension](https://4comprehension.com/kotlins-internal-visibility-modifier-and-java-interoperability/)). Enforcement: compile-time only, advisory at the bytecode layer.

**Explicit API mode** (`explicitApi()` / `explicitApiWarning()` / `-Xexplicit-api={strict|warning}`) forces explicit visibility modifiers and explicit return types on the public surface, production sources only ([docs](https://kotlinlang.org/docs/whatsnew14.html)). It is the **opt-in, per-project, warning-then-error ratchet** JPMS never offered.

**COPY:** Explicit API mode wholesale — a per-root `explicit_visibility = warn|strict` flag that requires every edge to state `access` instead of defaulting silently. That is how you flip a default without a flag day. **AVOID:** Kotlin's module definition. Bind visibility to the **declared edge / package coordinate**, never to "whatever landed in the same materialised lane" — an implicit compilation-unit boundary is exactly the mistake, and in our system the lane composition is itself the variable.

---

## 4. Swift — the closest match to our design

| Primitive | Effect |
|---|---|
| `private` / `fileprivate` / `internal` (default) / `package` / `public` / `open` | linear lattice; `internal` = module |
| `package` ([SE-0386](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0386-package-access-modifier.md)) | visible to modules built with the **same `-package-name` string**; SwiftPM supplies it; `packageAccess: false` opts a target out |
| `@_spi(Name)` on a decl | usable only by clients whose import says `@_spi(Name) import M` — **bilateral handshake**; shipped via a separate `.private.swiftinterface` |
| `internal import M` / `package import M` / `public import M` ([SE-0409](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0409-access-level-on-imports.md)) | **per-edge**, consumer-declared: how far this dependency propagates to *my* clients |
| `@_implementationOnly import` (legacy) | predecessor; hid transitive imports only, didn't block direct access |

**The exact gap `package` filled:** a helper module inside a multi-module package had to be `public` to be usable by its siblings, which also exposed it to the world — no level meant "shared within my group, invisible outside". Rejected alternatives are directly instructive: submodules (bakes structure into ABI), `internal(PackageName)` (too verbose/general), and **named groups within a package — abandoned because it "would tend to lead to large, complex manifests mingling the details of all the layers."** That is a direct warning against fine-grained per-edge friendship grants at the root.

Enforcement: compile-time, by comparing the package-name string recorded in the built interface. Transitivity: **none** — `package` is a flat, symmetric, unordered group, not a graph relation.

**SE-0409 is the single closest precedent to our per-edge `access`**: the *consumer* annotates its own import edge to control leakage to its own clients — dependency-creep control, i.e. our seepage control. Critically, its default is still `public` in Swift 5 **and 6**, with the flip to `internal` deferred to a future language mode. Apple, with full control of compiler and build tool, still refused the flag day.

**COPY:** (a) SE-0409's per-edge, consumer-declared propagation control — that is our model, validated; (b) `@_spi`'s **bilateral handshake** (provider names a group, consumer opts in by name) — it makes seepage impossible to acquire accidentally and gives both sides a veto; (c) `packageAccess: false` as the shape of `exclude`; (d) the staged default flip. **AVOID:** string-matched implicit groups (`-package-name`) — invisible in source, silently coupling; and heed the rejected-alternatives note: **keep friendship grants coarse (named groups / roles), not per-edge pairs**, or the root manifest becomes the "large, complex manifest mingling all the layers" Swift explicitly refused to build. **[INVERSION]** Since ours is budget control, not secrecy, the enforcement output should be a *pruning report* (what was excluded, tokens saved) rather than an error — Swift/JPMS can hard-fail because a missing symbol breaks the build; a missing prompt lane merely makes the agent dumber, so the failure must be made **legible** instead.
