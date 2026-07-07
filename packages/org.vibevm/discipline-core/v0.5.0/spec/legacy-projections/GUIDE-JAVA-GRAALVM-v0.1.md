# GUIDE — Java + GraalVM Native Image under the Discipline, v0.1 (overlay)

**Status.** Beta; overlay on `GUIDE-JAVA-v0.1.md`. Composes with the Spring overlay (Boot AOT) and with Jakarta-family runtimes built for ahead-of-time CDI; only named sections are rewired.

Framing note. Native Image is the profile where the Discipline's anti-magic rules stop being style and become **physics**: the closed-world assumption means reflection, dynamic proxies, JNI, resources, and serialization exist only if *declared at build time* — everything reachable is statically known, which is axiom A3 enforced by a compiler instead of a review. The trunk already banned runtime magic in cells; this overlay collects the dividend: a trunk-conformant core needs **zero reachability metadata of its own**, and every line of metadata the *boundary* needs becomes visible, versioned, and countable. The price is paid in toolchain honesty: build minutes, library compatibility vetting, and a debugging story different from the JVM's.

**Scope honesty.** CLIs, serverless/short-lived workloads, and services where startup latency and footprint dominate. JIT-on-JVM remains the right target for peak-throughput long-running services unless measured otherwise; this overlay never pretends the trade is free.

---

## §0 additions — baseline {#baseline}

- **Toolchain:** a maintained Native Image distribution, pinned like any compiler; `native-maven-plugin`/Gradle equivalent in the build, native compilation in CI from day one (a "we'll go native later" project discovers its reflection debt at the worst moment).
- **Library admission = adopted-code vetting:** a dependency enters only if it ships its own reachability metadata, is covered by the shared GraalVM reachability-metadata repository, or passes the tracing agent cleanly. The vetting result is recorded per dependency (a facts-class ledger entry keyed by artifact+version — never re-vetted twice, A2).
- **Build reporting on:** the image build's analysis output (included classes/methods) is retained per release — it is the most literal A1 inventory any profile in the set produces: the binary's contents as a machine-readable list.

## §1' — cells and build-time initialization {#cells}

Import-is-execution returns at a *third* time coordinate: class initializers can run **at image build**, their resulting heap snapshotted into the binary.

- The trunk's ban on side-effectful static init now pays out directly: **cells are build-time-initializable by construction** — faster startup, smaller image, and the rule that was discipline on the JVM is an optimization flag here.
- Anything that must initialize at run time (touches environment, clocks, file descriptors, randomness seeds) lives at the boundary and is enumerated in the **run-time-init manifest** — a committed, diffed list. An unexplained addition to it is a finding.
- **Heap-snapshot caution:** nothing captured at build time may embed build-host state (paths, env, time, random seeds). A conform check greps the snapshot-eligible set for the platform-capability types the trunk already bans in cells.

## §3' — flags and the AOT tier migration {#flags}

The overlay's most important finding, stated plainly: **under AOT, the two flag tiers migrate.** Conditions the container or the build evaluates ahead of time (Spring AOT resolves `@ConditionalOn...` and profiles at build; image building freezes the classpath) turn what the trunk called *runtime* flags into *build* flags — silently, if no one audits.

- Rule: every flag in the registry is classified `build | runtime` **per target**; composing this overlay forces a re-audit of the classification, and the audit diff is part of the overlay's adoption PR.
- Flags that must remain runtime under Native Image use mechanisms that survive AOT (config values read at startup by ordinary code — the trunk's config record — not container conditionals).
- Tree-shaking honesty: the unselected cell is genuinely absent from the image — the assembly tier finally has C++/cargo-grade meaning in Java.

## §4'/§6' — magic accounting {#magic}

- **Reachability metadata is generated, not hand-grown:** the tracing agent runs over the boundary test suite; outputs are committed and **diffed like code**. The metric the overlay contributes to the Charter's dashboard: **`reflect-config` (and proxy/resource/serialization configs) size and growth rate = magic debt**, tracked exactly like wish-ratio. A trunk-conformant core contributes zero; every entry is boundary-attributable or it is a finding.
- Dynamic proxies require declaration; the trunk's seams (plain interfaces, `new`-constructed cells) need none. `Unsafe`, agents, `invokedynamic`-heavy bytecode tricks: boundary-of-boundary, vetted per library.
- Observability honesty: no agent attach, JFR reduced, no hot bytecode tooling — the boundary exports metrics/health explicitly; debugging uses native debug info (kept, not stripped — same rule as Go's symbol retention, same A1 reason).

## §8' — additional risk rows {#risks}

| Footgun | Rule | Tier |
|---|---|---|
| undeclared reflection/resource/proxy hit at run time | §4' | runtime (fail-closed) + agent diff |
| dependency without metadata or vetting record | §0 | build (admission) |
| build-time-initialized class capturing host state | §1' | T-syn + snapshot check |
| unexplained growth of run-time-init manifest or reflect-config | §1', §4' | diff review (debt metric) |
| flag classified runtime but frozen by AOT | §3' | audit check |
| serialization config reintroducing `Serializable` domain types | trunk §6 | T-syn |

---

**Overlay note.** This is the profile where the Discipline and the platform want the same world — closed, declared, inventoried. Adopting it on a trunk-conformant codebase is cheap; adopting it on a reflection-rich codebase is the brownfield protocol with a compiler holding the baseline.
