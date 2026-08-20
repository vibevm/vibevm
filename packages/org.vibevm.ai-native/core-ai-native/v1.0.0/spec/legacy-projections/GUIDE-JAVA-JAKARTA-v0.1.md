# GUIDE — Java + Jakarta EE / MicroProfile under the Discipline, v0.1 (overlay)

**Status.** Beta; overlay on `GUIDE-JAVA-v0.1.md`. Composes with the GraalVM overlay via build-time-CDI runtimes; never with the Spring overlay (one container per target). Only named sections rewired.

Framing note. Jakarta EE is the one platform in the whole study whose own architecture *is* the Discipline's triple: **specifications** (the Jakarta/MicroProfile documents) ↔ **implementations** (vendors) ↔ **conformance suites** (the TCKs). A platform that certifies implementations against a spec with an executable test kit has already conceded every philosophical point this Discipline argues; the overlay's job is merely to keep application code as honest as the platform's governance. Practically it is the Spring overlay's sibling: CDI is the container, the resolution is the same **framework-free core with ring placement** — so this guide states only the deltas.

**Scope honesty.** MicroProfile-era services on certified runtimes (Open Liberty, Quarkus, Helidon, Payara, WildFly). Classic monolithic EE estates enter via the brownfield protocol as adopted code.

---

## §0 additions — baseline {#baseline}

- **Pin the spec levels, not just the runtime:** the target Jakarta EE platform/profile version and MicroProfile version are declared in the build; vendor BOM pinned. **Portability is a check, not a hope:** cells and seams depend on `jakarta.*`/`org.eclipse.microprofile.*` APIs at most (boundary), vendor packages (`io.quarkus..`, `com.ibm..`, etc.) are boundary-of-boundary — ArchUnit-enforced, and the rule is what keeps the second implementation possible.
- **TCK resonance, used not admired:** where a seam wraps a platform service, the platform's own TCK answers "does the runtime behave"; conform answers "does our code behave" — the two suites are complementary GEP rows, not overlap.

## §1'/§3' — CDI as composition root {#cdi}

- **No `@Inject`, no scopes, no interceptor bindings inside cells.** Cells are `new`-constructed.
- **Producers are the registry:** explicit `@Produces` methods in a composition module construct cells with their capabilities — the hand-written switch in CDI clothing:

```java
@ApplicationScoped
public class SolverProducer {
    @Produces @ApplicationScoped
    DepSolver depSolver(AppConfig cfg, DepProvider p, Clock clock) {
        return switch (cfg.solver()) {        // R-001 at the seam
            case SAT   -> new SatDepSolver(p, clock);
            case NAIVE -> new NaiveDepSolver(p);
        };
    }
}
```

- **Bean discovery mode `annotated`** (explicit `beans.xml`), never `all` — implicit-bean archaeology is the CDI form of component scanning and is banned for the same reason.
- **MicroProfile Config is the runtime flag tier, with provenance built in:** ConfigSources are *ordinal-ordered* (system props > env > files > defaults) — the platform ships the Discipline's provenance chain natively; flags resolve once into a config record at the composition root, `@ConfigProperty` field-sprinkling in domain code is banned.

## §4' — boundary machinery {#boundary}

- **Fault-tolerance annotations (`@Retry`, `@CircuitBreaker`, `@Timeout`, `@Fallback`) are interceptors** — hidden control flow under R-021 — and live only on boundary/orchestration beans. If a retry policy is *domain* behavior, the cell implements it explicitly against its clock capability, testable without a container.
- **JPA entity quarantine:** entities are proxy-laden, lazily-loaded, lifecycle-managed objects — citizens of the second language. **Entities never cross into cells**; persistence sits behind repository seams returning records, mapping done in the adapter. The N+1 problem, detached-entity traps, and dirty-checking surprises all stay in the ring built to contain them.
- JAX-RS resources, servlets, messaging listeners: boundary adapters that parse (DTO → validation → domain types), call seams, translate sealed results to protocol responses with the REQ URI rendered on errors (PROP-014 §2.6). Health (`mp-health`) and metrics (`mp-metrics`/Telemetry) are boundary exports.

## §8' — additional risk rows {#risks}

| Footgun | Rule | Tier |
|---|---|---|
| `@Inject`/scope/interceptor annotation inside a cell | §1' | ArchUnit |
| bean discovery mode `all` | §1' | T-lex (descriptor) |
| vendor package import outside boundary-of-boundary | §0 | ArchUnit |
| JPA entity in a seam signature or inside a cell | §4' | ArchUnit |
| fault-tolerance annotation on domain logic | §4' | T-syn |
| `@ConfigProperty` outside the composition root | §3' | T-syn |
| second-implementation drift (vendor-only API creep) | §0 | build diff |

---

**Overlay note.** The platform already believes in specs, implementations, and conformance kits; this overlay only asks the application to live up to its own runtime's governance model. The framework-free-core test from the Spring overlay applies verbatim: cells compile without `jakarta.*` on the classpath.
