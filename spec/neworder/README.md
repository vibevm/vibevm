# Terraform vibevm — discipline package, v0.2-beta

The complete document set required to begin terraforming vibevm: turning an AI-assisted **unfinished** legacy codebase into the reference implementation of the Discipline. The package is itself a product in beta — execute it against vibevm, measure, revise.

© 2026 Oleg Chirukhin. License: TBD (target: UPL-1.0).

**v0.2 changelog.** Brownfield revision: the package no longer assumes a healthy baseline. New BROWNFIELD-PROTOCOL document; Charter amended with axiom A6; spec-authoring guide gains lifecycle statuses (`planned` / `disputed`); PROP-014 gains unit statuses and `conflicts_with` edges; Playbook rewritten to v0.2 (inventory-not-gate, xfail-strict test gate, debt/intent registries, characterization capture, carry-over reconciliation).

## Document map

| File | Layer | Role | Status |
|---|---|---|---|
| `DISCIPLINE-CHARTER-v0.1.md` | product / T0–T1 | Axioms (A1–A6), tier architecture, rule schema + samples, metrics, governance | beta (amend. v0.1.1) |
| `BROWNFIELD-PROTOCOL-v0.1.md` | product / T1 | Terraforming unfinished projects: registries, xfail-strict, disputes, characterization, carry-over guarantee | beta |
| `PROP-014-specmap-bidirectional-traceability.md` | product mechanism (vibevm-hosted) | Bidirectional spec↔code traceability: anchors, revisions, tags, statuses, index, queries | beta |
| `GUIDE-SPEC-AUTHORING-v0.1.md` | product / T1 guide | Writing specs with bidirectional binding; lifecycle statuses; checklists; importer contract | beta |
| `GUIDE-RUST-v0.1.md` | product / T2 guide | Rust bindings: cells, seams, errors, flags, naming, risk table, conform checks | beta |
| `GUIDE-TYPESCRIPT-v0.1.md` | product / T2 guide | TS/JS bindings: subset selection, Result-at-seams, capability injection, import hygiene, risk table | beta |
| `GUIDE-PYTHON-v0.1.md` | product / T2 guide | Python bindings: pyright-strict gate, Protocol seams, decorator carrier, dynamism bans, risk table | beta |
| `GUIDE-CPP-TRADITIONAL-v0.1.md` | product / T2 guide (profile) | C++14 old-believer subset: no exceptions/RTTI, ABC seams, vendored Result, `-Werror=switch` exhaustiveness | beta |
| `GUIDE-CPP-MODERN-v0.1.md` | product / T2 guide (profile) | C++23 three-major subset: concept seams, `std::expected`, `constinit` import-purity, variant exhaustiveness | beta |
| `GUIDE-CPP-MISRA2008-v0.1.md` | product / T2 guide (profile) | MISRA C++:2008 corpus + Compliance:2020 operationalized: GRP/GEP/deviations/permits/GCS via conform & registries | beta |
| `GUIDE-GO-v0.1.md` | product / T2 guide | Go bindings: gap closure (closed error sets, loud conformance, owned goroutines, init ban), directive carrier, native buildinfo release map | beta |
| `GUIDE-JAVA-v0.1.md` | product / T2 guide (trunk) | Java trunk: sealed Results over checked exceptions, annotation carrier (CLASS retention, @Documented), ArchUnit as T-syn engine, virtual-thread-era seams | beta |
| `GUIDE-JAVA-SPRING-v0.1.md` | product / T2 overlay | Framework-free core; @Configuration/@Bean as the registry; @ConditionalOnProperty as R-001; proxy honesty; test pyramid | beta |
| `GUIDE-JAVA-GRAALVM-v0.1.md` | product / T2 overlay | Closed world as A3-physics; build-time init dividend; reflect-config growth as magic-debt metric; AOT flag-tier migration audit | beta |
| `GUIDE-JAVA-JAKARTA-v0.1.md` | product / T2 overlay | Spec/TCK resonance; CDI producers as registry; MP Config native flag provenance; JPA entity quarantine | beta |
| `ENGINE-CONFORM-v0.1.md` | product / T3 | Cross-language conformance engine: escalation tiers, compiler frontends, fact store, SARIF | beta |
| `LEDGER-INTENT-v0.1.md` | product / T3 mechanism | Intent ledger: memoized understanding, fact vs interpretation classes, epochs, release slice | beta |
| `PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md` | carrier-specific | The operative plan Claude Code executes inside the vibevm repo; inventory, phases, gates, stop conditions | beta |

Placement after ratification: product docs → the Discipline's own repository (to be created); `PROP-014` → `vibevm/spec/common/`; the playbook → `vibevm/terraform/PLAYBOOK.md` (working copy inside the carrier).

## Reading order

**Human (first pass):** Charter → BROWNFIELD → PROP-014 §1–2 → this README → Playbook. Guides and engine docs on demand.

**Claude Code (working session in vibevm):** Playbook §0 is the boot text; it references everything else lazily. Do not preload the full package every session — pull sections when a phase needs them (axiom A2 applies to reading, too).

## Known beta gaps (deliberate)

- C++ delivered as three sibling profiles (traditional / modern / misra2008) with per-target granularity; **open:** profile-composition semantics are defined only via the adopted-code/GRP-scope mechanism in the MISRA guide — untested for non-MISRA profile mixes; threshold: if per-profile rule forks exceed ~30% of conform's C++ checks, collapse profiles into parameterized checks.
- Java delivered as a **trunk + three overlays** (Spring / GraalVM / Jakarta) — a second genre structure beside C++'s siblings: overlays inherit the trunk and rewire named sections only; composition matrix declared in the trunk header. **Open:** overlay inheritance is prose-level — conform has no mechanism yet to evaluate "trunk rules minus overlay overrides" as a computed rule set.
- Pending PROP-014 amendment: external read-only normative namespaces in the specmap (`misra://cpp2008/<rule>`), introduced by GUIDE-CPP-MISRA2008 §5; code may `deviates` such units, never `implements`.
- `cpp-misra2023` profile: contingent on licensed MISRA C++:2023 text; mechanical sibling of the 2008 profile.
- Known defect (found via the isomorphism contract while writing the Python guide): `GUIDE-TYPESCRIPT-v0.1.md` lacks the boundary runtime-validation paragraph (zod/valibot, parse-don't-validate) that §0 of the Python guide carries; scheduled as TS guide v0.1.1.
- The legacy-spec importer and the conflict-scan heuristics exist as contracts (GUIDE-SPEC-AUTHORING §7, BROWNFIELD §5), not as tuned tools — precision is measured, then tuned, on the real corpus.
- Signing of the release ledger slice: designed (LEDGER §7), not implemented; blocks public runtime exposure, not local work.
- Cell granularity default (module vs crate) is provisional in GUIDE-RUST §1 and explicitly a pilot-measured decision.

## Change protocol

Beta documents are revised on pilot evidence only — each carries the house footer: mechanisms not exercised by their named playbook phase are removed rather than carried as aspiration. Once the Discipline repo exists, these documents become specmap'd spec units themselves (anchored, revisioned, hash-audited), and the registries fold into the metamodel per BROWNFIELD OQ-4.
