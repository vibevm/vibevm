# GUIDE — Java + Spring under the Discipline, v0.1 (overlay)

**Status.** Beta; overlay on `GUIDE-JAVA-v0.1.md` (the trunk). Only the sections named here are rewired; everything else — carrier, naming, errors-as-sealed-results, suppression policy, risk rows — is inherited. Composition: combines with the GraalVM overlay (Spring Boot AOT); never with the Jakarta overlay (one container per target).

Framing note. Spring is the place where the Discipline's oldest ban — *no DI containers* — meets the ecosystem where DI containers won. The overlay does not pretend Spring away and does not surrender to it; it resolves by **ring placement**: Spring is legalized as *machinery of the composition root and the boundary*, and nowhere else. The trunk's second language (runtime assembly via reflection and proxies) gets a territory with a border, and the border is enforced by ArchUnit, not by hope. The slogan of this overlay: **framework-free core** — a cell compiled without Spring on the classpath is the test that it belongs to the first language.

**Scope honesty.** Spring Boot 3.x-era applications. Legacy XML/Spring 4-5 codebases enter through the brownfield protocol as adopted code, not through this overlay.

---

## §0 additions — baseline {#baseline}

- **Boot BOM pinned**; starters are audited dependencies, not magic: every starter admitted to the build is listed with the auto-configurations it activates (one-time inventory, then diffed). `spring-boot-properties-migrator` class tooling at upgrades.
- **ArchUnit ring rule (the overlay's load-bearing check):** `..cells..` may not depend on `org.springframework..` — committed as a test, failing red the day someone "just adds `@Component`".

## §1' — cells {#cells}

Inherited, plus: **no Spring types in cells, period.** No stereotypes (`@Component`/`@Service`), no `@Autowired`, no `ApplicationContext` awareness, no Spring events, no `@Value`. A cell is constructed by `new`, in a `@Bean` method, with everything it needs — which is precisely why it tests in microseconds without a context (§7').

## §2' — seams {#seams}

Inherited, plus: **no Spring types in seam signatures** — no `ResponseEntity`, no Spring Data `Page`/`Pageable`, no `MultipartFile`. Where pagination or streaming is part of the domain contract, the seams module defines its own small records (`Slice`, `PageRequest`); boundary adapters translate. A seam that leaks a framework type has annexed the cell into the second language.

## §3' — registry and flags {#flags}

The trunk's hand-written registry **is** idiomatic Spring, written the way Spring's own documentation recommends for libraries:

```java
@Configuration(proxyBeanMethods = false)
public class SolverConfig {

    @Bean
    @ConditionalOnProperty(name = "app.solver", havingValue = "sat")
    DepSolver satDepSolver(DepProvider p, Clock clock) {     // R-001: the flag, at the seam
        return new SatDepSolver(p, clock);
    }

    @Bean
    @ConditionalOnMissingBean(DepSolver.class)
    DepSolver naiveDepSolver(DepProvider p) {
        return new NaiveDepSolver(p);
    }
}
```

- **Explicit `@Bean` methods are the registry**; component scanning is confined to boundary packages and never reaches cells — wiring stays readable as code, not discoverable as folklore. `proxyBeanMethods = false` keeps the configuration class itself proxy-free.
- **`@ConditionalOnProperty` is the literal R-001 binding** — the flag is read by the container, at the seam, once, with provenance supplied by Spring's property-source ordering (cli > env > file > default), which maps one-to-one onto the Discipline's provenance chain.
- **Runtime tier:** `@ConfigurationProperties` records with constructor binding, validated at startup — the config object the trunk's §3 demanded, container-built. No `@Value` scatter; no `Environment` lookups outside the boundary.
- **Constructor injection only.** Field injection is banned (it is reflection-through-the-back-door and makes the framework-free test impossible); single-constructor classes need no annotation at all. A circular dependency is a design failure to fix, not a `@Lazy`/setter workaround to apply.

## §4' — errors and proxies {#errors}

Inherited (sealed results at seams), plus the overlay's two honesty paragraphs:

- **Exception translation is boundary work:** Spring's unchecked hierarchies (`DataAccessException` and kin) are translated into seam error types in adapters; framework exceptions never cross inward.
- **Proxy honesty:** `@Transactional`, `@Cacheable`, `@Async`, `@Retryable` are dynamic proxies — **self-invocation silently skips them**, final methods defeat them, and they constitute hidden control flow (R-021) if sprinkled into domain logic. They are legal only on boundary/use-case orchestration classes; transaction boundaries live where use-cases are composed, never inside cells — which is where the DDD mainstream puts them anyway. A cell that needs "a transaction" actually needs a unit-of-work capability on its seam.

## §7' — replacement and tests {#tests}

Inherited, plus the test pyramid binding: **cell tests are plain JUnit** — no context, no `@SpringBootTest`, no Mockito (trunk rule), constructor + injected fakes, microseconds. Boundary slices use `@WebMvcTest`/`@DataJpaTest`-class slices; **full `@SpringBootTest` contexts are few and live at the top** — each one is counted, because context startup is the test suite's wish-ratio. Testcontainers (MIT) for real-infrastructure boundary tests. A cell test that needs the container running is a misplaced boundary test — move it, don't speed it up.

## §8' — additional risk rows {#risks}

| Footgun | Rule | Tier |
|---|---|---|
| `org.springframework..` import inside a cell | §1' | ArchUnit |
| stereotype annotation (`@Component`/`@Service`) on a domain class | §3' | T-syn |
| field/setter injection anywhere | §3' | T-syn |
| component scan reaching cell packages | §3' | T-syn (config audit) |
| Spring type in a seam signature | §2' | ArchUnit |
| `@Transactional`/`@Async`/`@Cacheable` inside a cell; self-invocation of a proxied method | §4' | T-syn + T-sem |
| `@Value` / `Environment` lookup outside boundary | §3' | T-syn |
| `@SpringBootTest` exercising a single cell | §7' | T-syn (test audit) |
| unaudited starter / auto-configuration drift | §0 | build diff |

---

**Overlay note.** Everything this overlay forbids has a sanctioned home one ring out; the discipline is placement, not abstinence. The framework-free-core test (cells compile without Spring) is the overlay's single most valuable artifact — cheap, binary, and brutally honest.
