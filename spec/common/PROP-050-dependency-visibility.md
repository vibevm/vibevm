# PROP-050: Dependency visibility — access, friendship, and the friend closure {#root}

<status stage="spec" state="work" comment="2026-08-23: core model owner-ratified (F1-F4, F6 as drafted; F5 overruled to no-legacy); F7 concepts / F8 allow-lists / F9 override still open; implementation not started"/>

@fact:status-line **Status: DESIGN — core owner-ratified 2026-08-23** (ordering word: «Я предлагаю переработать систему транзитивности… Не переходи к реализации, вначале нужно хорошо запроектировать»). The owner ruled F1–F4 and F6 as drafted, and **overruled F5**: no legacy read, no backward compatibility — the system is primary and irreversible (§6). F7 (concepts), F8 (provider allow-lists) and F9 (root-side `override`) remain open in §9. Nothing is implemented yet. @status:spec/work

@fact:related **Related:** [PROP-009](../modules/vibe-workspace/PROP-009-loading-model.md) (the loading model and the `link` axis), [PROP-034](../modules/vibe-workspace/PROP-034-transitive-links-boot-graph.md) / [PROP-035 §12](../modules/vibe-workspace/PROP-035-spec-compiler.md#transitive-inline) / [PROP-038](../modules/vibe-workspace/PROP-038-hybrid-boot-linking.md) (the linker this proposal re-layers), [PROP-048](PROP-048-tokenomics.md) (tokenomics — the law this system serves), [PROP-049](PROP-049-snippet-genre.md) (the snippet genre — §4.3 settles its fate under visibility), [PROP-046](PROP-046-adoption-facts-registry.md) (consumer sovereignty — the same philosophy), [PROP-024](PROP-024-code-bearing-packages.md) (role equipotence — visibility is role-blind), [PROP-028](PROP-028-package-families.md) (collections — the migration's largest client). @status:spec/work

---

## 1. Motivation — seepage is a budget defect, not a secrecy defect {#motivation}

@fact:seepage-today Today the requires closure IS the effective set: every transitive dependency of every dependency reaches the consumer root, lands in `INDEX.md` (or, forced, in the static lane), and spends the root's context budget. The only controls are materialisation-side (`link` modes decide *how* a package loads, never *whether* it arrives) and the PROP-049 genre machinery (which polices *textual presupposition*, not structural arrival). The WAL leak — every new project sprouting `spec/WAL.md` because a package chain delivered the wal flow — is the type specimen of the defect class. @status:spec/work

@fact:VISIBILITY-IS-BUDGET-CONTROL **The inversion thesis.** In every prior-art system (C++ `friend`, Java module exports, OSGi, Bazel `visibility`) visibility is *provider-declared secrecy*: the provider protects its invariants by naming who may see in. In vibevm the scarce good is not the provider's encapsulation but the **consumer's context budget** (PROP-048): a package that seeps uninvited costs tokens in every session of every downstream project. Visibility here is therefore **budget control**: the provider *marks* how far each edge may seep (`access`), and the consumer *opts in* to more (friendship). Nothing seeps by default; nothing the consumer did not cause can spend the consumer's budget. This inverts the direction of every friendship mechanism in prior art — ours is consumer-declared opt-in, not provider-declared allowlisting — and the consequences of that inversion are traced through §5. @status:spec/work

@fact:owner-mandate **Owner mandate (2026-08-23, verbatim core):** «Каждая зависимость в vibe.toml должна обладать несколькими новыми опциональными свойствами… Пакеты с доступом public транзитивно просачиваются вдоль всей иерархии… потребительский корень проекта — это причина, почему дальше начинают происходить вычисления всей остальной иерархии пакетов… должна работать транзитивность внутренних друзей… Таким образом у нас получается некое общее "замыкание друзей"… Чтобы это замыкание не росло бесконтрольно, нужно предусмотреть параметры с исключениями». @status:spec/work

---

## 2. The model {#model}

### 2.1 Perspectives — relations first, materialisation at the root {#perspectives}

@fact:PER-NODE-PERSPECTIVE Every visibility notion in this document is **per-node**: each manifest's declarations define how the graph looks *from that node*. An interior node's friendships, unfriendings and exclusions are «просто отношение между пакетами» — pure relations, materialising nothing. @status:spec/work

@fact:ROOT-DRIVES-MATERIALISATION **The consumer root is the cause of computation** (owner law). Only when a node is the session's consumer root (PROP-024: `[project]` or `[package]`, equipotently) do its perspective's relations resolve into an **effective set**, get version-resolved, materialised into `vibedeps/`, and compiled into lanes. «Потребительский корень проекта — это причина, почему дальше начинают происходить вычисления всей остальной иерархии пакетов.» @status:spec/work

### 2.2 Edge access — how far this edge may seep {#access}

@fact:ACCESS-LEVELS Each `[requires.packages]` edge gains an optional `access` property with three values — the provider-side seepage mark on the edge `P → Q`, declared by `P` about its own dependency `Q`: @status:spec/work

```toml
[requires.packages]
"flow:org.x/api-conventions" = { version = "^1.0", access = "public" }       # seeps to everyone above
"flow:org.x/wal"             = { version = "^1.0" }                          # access = "private" — the default
"flow:org.x/internal-style"  = { version = "^1.0", access = "friends-only" } # seeps to those who befriended P
```

- @fact:ACCESS-PUBLIC `access = "public"` — `Q` seeps through `P` to *every* consumer above, transitively along the whole hierarchy, with no opt-in. For edges whose target is part of the declarant's substance for *all* consumers — a collection's members, a stack's core. @status:spec/work
- @fact:ACCESS-PRIVATE `access = "private"` — **the default.** `Q` does not seep through `P` at all: the edge is traversed only when `P` itself is the consumer root (§4.4 — the dev world). An implementation detail in the strictest sense. @status:spec/work
- @fact:ACCESS-FRIENDS `access = "friends-only"` — `Q` seeps through `P` only into consumers whose friend closure contains `P` (§2.4): those who deliberately named `P` (directly or transitively) a friend. The curated middle. @status:spec/work

@fact:DEFAULT-PRIVATE-RATIONALE **Default private is the safety inversion.** Today's default is effectively «everything public» — seepage is the path of least resistance and the WAL class of leaks is its harvest. Under this proposal an author must *say* that an edge seeps; forgetting the mark costs a consumer a missing package (loud, fixable in one line) rather than costing every consumer an unwanted one (silent, discovered as budget drain or a phantom `spec/WAL.md`). Fail-closed, per PROP-048. @status:spec/work

### 2.3 Friendship — consumer-declared, two grant forms {#friendship}

@fact:FRIENDSHIP-IS-CONSUMER-DECLARED Friendship is declared by the **consumer of content**, never by its provider: naming a package your friend means *you receive* what it shares friends-only. There is no provider-side allowlist (§9 F8 keeps the door open). Two grant forms: @status:spec/work

- @fact:FRIEND-EDGE-FLAG **Per-edge `friend = true|false` (default `true`).** Every dependency edge extends friendship to its target by default — a node naturally trusts what it itself requires. `friend = false` is arms-length usage: «I require Q but do not opt into Q's friends-only sharing.» @status:spec/work
- @fact:FRIENDS-LIST **Node-level `friends = ["group/name", …]`.** Explicit befriending of *any* package — «можно указать и свои прямые зависимости, и вообще любые зависимости» — including ones the node does not require directly (e.g. a practice known to arrive deep in the closure whose friends-only companions the root wants). @status:spec/work

@fact:PUBLIC-PLUS-FRIEND **`access` and `friend` compose independently on one edge** (owner-confirmed 2026-08-23: «access и friend разные понятия»). The two marks point in opposite directions, so their product is well-defined. `access = "public", friend = true` — the common shape — reads: «I deliver Q to everyone above me; and *for myself* I enter friendship with Q, so Q's friends-only doors are open in my own perspective». The friendship half does **not** travel onward through a public edge — re-exporting friendship is exactly what the `friends-only` mark does (##PUBLIC-GIVES-PRESENCE-NOT-FRIENDSHIP) — so in another root's closure a public edge contributes presence and nothing more. `access = "public", friend = false` is transit without relations: deliver to all, take none of the gated substance yourself. The remaining corner — «deliver publicly *and* vouch friendship onward» in a single edge — is deliberately inexpressible: a consumer that wants Q's gated substance opts in with its own `friends` line, which is the payer-decides invariant. @status:spec/done

### 2.4 The friend closure — transitivity through friends-only edges {#closure}

@fact:GRANTS-DEFINITION For a node `N`, define `grants(N)` = the targets of `N`'s edges carrying `friend = true`, plus `N`'s `friends` list, minus `N`'s `unfriend` list (§2.6). These are the packages `N` *directly* befriends. @status:spec/work

@fact:FRIEND-CLOSURE **The friend closure** `C(R)` of a root `R` is the least fixpoint of: @status:spec/work

```
seed:  every G ∈ grants(R) is in C(R)
grow:  if F ∈ C(R), and G ∈ grants(F), and F's edge F → G has access = "friends-only",
       then G ∈ C(R)
```

@fact:closure-reading Reading: my friends are those I named; and when a friend marks one of its own befriended dependencies `friends-only` — «this one is part of my substance, shared with my friends» — that dependency becomes my friend too, recursively. The `friends-only` mark on `F → G` is thus simultaneously (a) the seepage gate of §2.2 and (b) the **re-export of friendship** that makes the closure transitive. This identification — the ordering message's per-edge «internal» mark IS `access = "friends-only"` — is **owner-ratified** (F1, 2026-08-23: «правильный термин — friends-only»). @status:spec/done

@fact:PUBLIC-GIVES-PRESENCE-NOT-FRIENDSHIP **Public gives presence, not friendship.** A package that seeps to `R` through public edges does *not* thereby join `C(R)`, and its own friends-only edges stay closed to `R` unless `R` (or a chain of friends-only re-exports reaching `R`) befriends it explicitly. Friendship never grows through public or private edges — only through `friends-only` ones, which are rare by construction (the default is private). This is the built-in answer to «чтобы это замыкание не росло бесконтрольно»: the closure grows exactly along deliberate marks and nowhere else. Fork **F6**. @status:spec/work

@fact:closure-determinism `C(R)` is a monotone least fixpoint over per-edge static predicates: deterministic, order-independent, computable in `O(nodes + edges)`, no interaction with the effective set (which is computed *after* it, §2.5) and none with materialisation (§3). Cycles in `requires` remain a hard generate-time error exactly as in the linker today (PROP-034 §2.3). @status:spec/work

### 2.5 Traversability and the effective set {#effective-set}

@fact:EFFECTIVE-SET **The effective set** `E(R)` — the packages that exist from `R`'s perspective — is everything reachable from `R` over *traversable* edges, along chains not killed by an `exclude` (§2.7). An edge `P → Q` is traversable for `R` iff any of the three rules below holds; private edges fail all three for `P ≠ R`, so a private dependency of a non-root package does not exist for the root — not resolved, not fetched, not materialised, not in any lane (§4.1–4.2). @status:spec/work

```
(1) P = R                                  — the root's own edges always count
(2) access(P → Q) = "public"               — unconditional seepage
(3) access(P → Q) = "friends-only" ∧ P ∈ C(R)   — seepage to a friend of P's
```

@fact:effective-set-is-the-universe Everything downstream — version resolution, `vibedeps/` materialisation, link-mode assignment, lane compilation, `installed:` predicates, the facts registry — operates on `E(R)` and nothing else. Visibility is computed once, first, and every other system consumes its output (§3). @status:spec/work

### 2.6 `unfriend` — node-scoped pruning of the closure {#unfriend}

@fact:UNFRIEND-IS-NODE-SCOPED `unfriend = ["group/name", …]` (node-level) removes the named packages from the declaring node's `grants(…)` — and therefore from every friend closure *as seen through that node*. The unfriended package «притянется, но будет явно исключён из цепочки транзитивности внутренних друзей»: still usable at the declaring level (its edge, if any, still traversable by its own access), just never re-exported as a friend through the declarant. Node-scoped by owner law: «они выбрасываются из замыкания ТОЛЬКО с точки зрения той ноды, которая объявила их unfriend — а какой-нибудь другой пакет в иерархии может нормально включить их в замыкание» — another node's friends-only chain delivers the same package untouched. The name **`unfriend`** is owner-ratified (F2, 2026-08-23); the property list's `enemy` is retired. @status:spec/done

### 2.7 `exclude` — hard subtree exclusion {#exclude}

@fact:EXCLUDE-IS-EDGE-SCOPED `exclude = ["group/name", …]` (per-edge) kills the named packages in every chain passing through the declaring edge — «исключены из цепочки транзитивных подключений вообще, даже если внутри они объявлены как public». Maven-exclusions semantics, **owner-ratified** (F4, 2026-08-23: «exclude per-ребро Maven-style. Можно per-ребро делать глубокие эксклюды по иерархии — это не глобальный deny-list, это сужение в рамках поддерева»): the pruning reaches arbitrarily deep, but only within *this edge's subtree*; a different path still delivers the package, and then it simply exists in `E(R)` via that path — classic diamond behaviour, no global veto. Exclusion is pure **subtraction** — it can only narrow what providers granted, never widen — which is why any node may declare it as part of shaping its own delivery; the expansive counterpart is F9's root-only `override`, which may also re-house this syntax (the semantics here stand either way). @status:spec/done

### 2.8 Worked example — the owner's chain {#example}

@fact:worked-example The ordering message's chain `A → B → C → D` (D a practice), with each intermediate marking its dependency friends-only and all `friend` flags at their `true` default: `grants(A) = {B}`, so `B ∈ C(A)`; `B`'s edge `B → C` is friends-only and `C ∈ grants(B)`, so `C ∈ C(A)`; `C → D` likewise, so `D ∈ C(A)` — «загрузившись в D мы получим, что по цепочке транзитивности, D является другом для A». Traversal: `A → B` by rule (1); `B → C` by rule (3) with `B ∈ C(A)`; `C → D` by rule (3) with `C ∈ C(A)` — so `D ∈ E(A)` and D materialises for A. If `B` had declared `unfriend = ["…/C"]`, the chain would break at `B` for every root above `B` — while a sibling path `A → B′ → C` (B′ friends-only-marking C) would deliver C and D intact. @status:spec/work

---

## 3. Layering — visibility above materialisation {#layering}

@fact:VISIBILITY-ABOVE-MATERIALISATION **Owner ruling, adopted as the layer boundary:** «признак типа static-transitive должен применяться уже ПОСЛЕ вычисления замыкания друзей, потому что замыкание друзей — это свойство логики связей в приложении, а транзитивная статичность — это свойство материализации, более логически низкая структура». The pipeline becomes: @status:spec/work

```
1. visibility    — compute C(R), then E(R)                    (this PROP; pure relations)
2. resolution    — version-resolve E(R) only                  (PROP-003/017, unchanged in kind)
3. link modes    — declared / suggested / default, per edge   (PROP-009 §2.4, unchanged)
4. linker        — zones, static-transitive forcing, dedup,   (PROP-034/035 §12/038,
                   topo order, hoisting                        re-scoped to E(R))
5. emission      — STATIC / INDEX lanes                       (unchanged)
```

- @fact:LINK-AXIS-ORTHOGONAL The `link` axis (`static` / `dynamic` / `static-transitive` / `static-hard`) survives intact as **materialisation strength within the visible graph**: «возможность спускать вниз по иерархии признак статичности всё так же очень нужна… как отдельная ось, ортогональная нашему замыканию друзей». @status:spec/work
- @fact:FORCING-NEVER-WIDENS **Forcing never widens visibility.** A `static-transitive` edge propagates staticness across *traversable* edges only: it can make a visible package static, never make an invisible package visible. The hybrid linker's forced-descent (`resolve_zone`, PROP-038 §2.2) simply walks `E(R)`'s subgraph instead of the raw requires graph; hoisting counts (PROP-038 §5.2) count within `E(R)`. @status:spec/work
- @fact:layer-law-fit This ordering is THE-LAYER-LAW (PROP-048) applied to the pipeline itself: the logical stratum (who exists) is more stable and more upstream than the materialisation stratum (how it loads); a change in link mode never invalidates the visibility computation above it. @status:spec/work

---

## 4. System interactions {#interactions}

### 4.1 Resolver and lock {#resolver}

@fact:RESOLVE-EFFECTIVE-ONLY Version resolution operates on `E(R)` only: private edges of non-root packages contribute no constraints, fetch nothing, and cannot conflict. `vibe.lock` records `E(R)` — the lock of a consumer no longer contains other packages' dev-world entries. Version unification (one node per `(group, name)`, PROP-003/017) is unchanged *within* the effective set. A welcome simplification vs code ecosystems: the Cargo-RFC-1977 problem («may private deps duplicate at different versions?») does not arise — an invisible package has no copies at all. @status:spec/work

@fact:RESOLVE-PRUNE-INTERLEAVING **The interleaving, named honestly.** Computing `E(R)` needs edges; edges live in manifests of *resolved* versions; constraints come only from `E(R)` — so the walk is joint: the resolver expands the graph following only traversable edges, with `C(R)` recomputed monotonically as newly-resolved manifests contribute grants and marks (all predicates are static edge attributes of already-chosen nodes, so the joint fixpoint stays deterministic). Reading a manifest to learn that its edge is private is a **metadata read, not materialisation** — package *content* is fetched for `E(R)` members only. The Cargo lesson (RFC 1977 lost six years to resolver entanglement; RFC 3516 lives by decoupling) is honoured structurally: visibility stays a pure edge predicate the walk *consults* — never a quantity the version solver optimises over. @status:spec/work

### 4.2 Materialisation and `vibedeps/` {#materialisation}

@fact:MATERIALISE-EFFECTIVE-ONLY `vibedeps/` holds exactly `E(R)`: an excluded or invisible package leaves no slot, no cache entry for the root's world, no lane text. This is the structural fix for the WAL specimen: a wal flow declared `private` (or `friends-only`) by whatever requires it simply never arrives in a consumer's tree — no snippet, no `spec/WAL.md` scaffold, no INDEX row. @status:spec/work

### 4.3 PROP-049 under visibility — what the genre machinery still does {#prop-049-fate}

@fact:flows-control-identification The ordering message asks whether «наша система с контролем flows» is still needed. No standalone "flows control" system exists in the spec tree (verified 2026-08-23: PROP-009's `flow` is an ordering *category*; PROP-028's flows are package *kinds*); the referent is read as the PROP-049 genre machinery — `installed:` predicates, snippet fragments, `concepts`, and the presupposition gate — built precisely against the WAL seepage this PROP now fixes structurally. Fork **F7** confirms the referent. @status:spec/work

@fact:PROP-049-DIVISION-OF-LABOUR **Recommendation: keep PROP-049, re-scoped — the two systems solve different halves.** Visibility controls *structural arrival* (whether a package exists in `E(R)`); the genre machinery controls *textual behaviour* (what a snippet may presuppose, and how text adapts to a neighbour's presence). With visibility live: (a) `installed:` predicates and fragments remain THE adaptivity mechanism — an `installed:` predicate now queries `E(R)`, and a fragment binding to an absent friend stays physically omitted exactly as today; (b) the presupposition gate remains the authoring lint that catches a snippet textually assuming what its package did not lawfully receive; (c) what *dissolves* is the unbundling pressure — a future redbook-like collection may keep a wal-like member on a `friends-only`/`private` edge instead of expelling it. @status:spec/work

@fact:CONCEPT-GATE-DIRECTION **Direction (F7 follow-up): concept-gated fragments.** Today the `concepts` array feeds exactly one consumer — the presupposition gate's dictionary (`crates/vibe-check/src/checks/snippet_presupposition.rs`): lexeme → owning package, matched against unconditional snippet text. A natural completion is `when = "concept:WAL"` — a fragment gated on «some package in `E(R)` declares this concept», resolving a discipline as an **interface** rather than a named implementation (`installed:` stays the exact-package form). Several packages could then carry one discipline interchangeably. Deferred to the owner's F7 ruling. @status:spec/work

@fact:DEPS-EXEMPTION-NARROWS **One PROP-049 rule must tighten.** The presupposition lint currently exempts mentions of the package's own declared dependencies. Under visibility that exemption is too wide: a mention of an own **private** dependency in *unconditional* snippet text presupposes a package that never reaches any consumer — lawful only inside an `installed:`-gated fragment (which, for a private dependency, fires only in the declarant's own dev world, §4.4 — exactly right). The exemption narrows to *edges that seep* (public, or friends-only). This is a concrete PROP-049 amendment shipped with the implementation wave. @status:spec/work

### 4.4 Equipotence and the dev world {#dev-world}

@fact:PRIVATE-IS-THE-DEV-WORLD Rule (1) of §2.5 — the root's own edges always traverse — combined with default-private resolves the open tail of the equipotence wave (PROP-024): a package's `[requires]` is simultaneously its dev-set and its contract, **split per-edge by `access`** rather than by section. When the package is the consumer root (a dev checkout — `[project]` or `[package]`, equipotently), *all* its edges traverse and its private tooling materialises; when it is consumed as a dependency, only its seeping edges do. No separate dev-dependencies section needed. @status:spec/work

### 4.5 vibefacts {#vibefacts}

@fact:facts-scope-follows-visibility The adoption-facts registry (PROP-046) keys per-source files by installed packages; its universe follows `E(R)` mechanically. Registry entries for packages that leave the effective set surface through the existing `facts_sync` / lifecycle machinery (PROP-046 L5) — no new mechanism, one new reason entries become stale. @status:spec/work

### 4.6 Collections and families {#collections}

@fact:COLLECTIONS-DECLARE-PUBLIC A PROP-028 collection's whole point is aggregation: its member edges are the type case for `access = "public"` (redbook's ~21 exact-pinned members must seep to redbook's consumers — that is what depending on the collection *means*). A family's internal shared core is the type case for `friends-only` (members befriend each other through the family's marks; outsiders opt in or see nothing). Migration inventory in §6. @status:spec/work

---

## 5. Prior art — what the neighbours teach {#prior-art}

@fact:prior-art-method Three research sweeps (2026-08-23; JVM lineage — JPMS / sealed / Kotlin / Swift; module-graph lineage — OSGi / Eclipse PDE / Bazel / Buck / Pants; dependency-manager lineage — Gradle / Maven / Cargo / npm / C++ `friend` and C++20 modules) ground this section; the full worker reports are archived in [`spec/research/dependency-visibility-2026-08/`](../research/dependency-visibility-2026-08/01-jvm-lineage.md). The map, on the two axes that matter — who declares, and what a denial costs: @status:spec/work

| System | Direction | Closest primitive to ours | Verdict used here |
|---|---|---|---|
| JPMS `requires transitive` | provider re-export | the closure `grow` rule — same recursive shape | adopt shape; adopt its usage norm |
| JPMS `exports … to` | provider allowlist | `friends-only` (opposite direction) | unknown target = warning, not error |
| Java `sealed … permits` | provider allowlist | friends list | allowlist needs one maintenance domain — ours has it (the consumer's own file) |
| Swift SE-0409 access-level-on-imports | **consumer, per-edge** | `access` — the closest precedent alive | validates the core; they deferred the default flip twice |
| Swift `@_spi` | bilateral handshake | `friends-only` ∧ friendship | conjunction, adopted (§2.5) |
| OSGi `Require-Bundle; visibility:=reexport` | provider re-export | `friends-only` as re-export | its irreversibility lesson → ##ACCESS-IS-SEMVER-SURFACE |
| Eclipse `x-friends` | provider allowlist | friends-only | provider lists rot because the payer can't edit them — the inversion argument |
| Bazel `deps` + `exports*` chains | provider re-export | the closure formula, verbatim | battle-tested shape of §2.4 |
| Buck `within_view` | **consumer cap, wins conflicts** | `exclude` | exclusion beats any grant, adopted |
| Pants dependency/dependents rules | both ends must agree | rule (3) of §2.5 | conjunction as the core evaluation rule |
| Gradle `api`/`implementation` | provider | `public`/`private` | the `compile`-removal precedent: leakage-by-default is unpayable |
| Maven `<exclusions>` / enforcer | consumer edge / assertion pass | `exclude` / deferred deny-list | two-layer split, adopted in F4 |
| Cargo RFC 1977 → 3516 | provider mark | `public` | six years lost to resolver entanglement — keep visibility a pre-filter |
| npm `exports` map | provider surface seal | (none yet) | sub-package surfaces — deferred direction |
| C++ `friend` | provider, deliberately non-transitive | the anti-model | what non-transitivity protects → ##CLOSURE-DRIFT-CONTROL |

@fact:PRIOR-ART-ADOPTIONS **Adopted into the model on prior-art strength:** (1) the **conjunction rule** — an edge materialises iff provider access permits ∧ consumer grant permits (Pants' "both ends must agree", Swift `@_spi`'s handshake) — §2.5 rule (3) is exactly this; (2) the **closure shape** — first hop a direct grant, then zero-or-more re-export marks — is Bazel's `deps`-then-`exports*` and JPMS implied readability, both battle-tested recursive; (3) **friend groups need no new primitive** — befriending a collection whose member edges are `friends-only` already delivers the members through the ordinary closure, giving Bazel-`package_group`-style composition for free and dodging Swift's rejected "large, complex manifests mingling all the layers"; (4) **unknown allowlist targets warn, never fail** (JPMS qualified-export precedent) — a `friends`/`unfriend` entry naming a package absent from every chain is a lint, §7; (5) **denial is legibility, not failure** — every surveyed system hard-fails because a missing symbol breaks a build; a missing prompt lane merely changes what loads, so our enforcement output is the pruning report (`vibe why`, §7), and only graph cycles remain hard errors. @status:spec/work

@fact:ACCESS-IS-SEMVER-SURFACE **An access mark is versioned surface.** OSGi's verdict on `reexport` — «once you have added re-export you cannot remove it without considering the corresponding API change» — and Cargo's `public = true` semver reasoning transfer whole: widening an edge (`private → friends-only → public`) is a feature; **narrowing one is a breaking change** of the declaring package, versioned like any contract change. The publish gate (C6 family) checks access narrowing against the previous published version. @status:spec/work

@fact:CLOSURE-DRIFT-CONTROL **What C++'s non-transitivity protected, restored by other means.** C++ keeps `friend` non-transitive so the set that can touch you stays enumerable *by reading one file*; our marks keep the closure enumerable but **not local** — a mid-graph package can widen `C(R)` in a patch release the root never reads. Three controls restore the audit: (a) the lock records `E(R)` with per-package lane cost, so `vibe update` surfaces closure drift as a reviewable diff («+2 packages, +3.1k static-lane tokens»); (b) ##ACCESS-IS-SEMVER-SURFACE makes silent widening a versioning violation; (c) an optional per-lane token budget cap (a PROP-048 direction) turns runaway growth into a loud stop. @status:spec/work

@fact:REEXPORT-USAGE-NORM **Authoring norm for seepage marks** (JPMS community rule, transposed): mark an edge `public`/`friends-only` only when your own boot text is *unreadable without* the target's text; otherwise leave it private and let consumers declare their own need. Aggregator-style «one edge pulls the world» is lawful only for collections, whose members are their declared substance (§4.6). Advisory, policed by the §7 lints, enforced by nobody — exactly as strict-deps culture, with the autofix command taking the place of ceremony. @status:spec/work

@fact:prior-art-rejections **Deliberately not taken:** OSGi `uses:`-style implicit constraint propagation (prompt text has no class space — a budget concern must never become a global satisfiability problem); Kotlin's compilation-unit boundary (visibility binds to declared edges and coordinates, never to «whatever landed in one lane»); graph-global unification of any visibility knob (Cargo's feature-unification scar — closures are per-root by construction, §2.1, and never merge); provider-side allowlists (F8); npm-`exports`-style named sub-package surfaces — a real direction (a grant admitting one declared fragment rather than a package's whole contribution) that composes naturally with PROP-049 fragments, deferred until the package-level system has run in anger. @status:spec/work

---

## 6. Migration {#migration}

@fact:MIGRATION-NO-LEGACY **Owner ruling (2026-08-23) — no legacy read, no backward compatibility:** «нужно переделать существующие пакеты, никакого legacy-чтения и обратной совместимости; система, которую мы сейчас сделаем, становится основной и не откатываемой никуда; пакеты предыдущих форматов не имеют смысла (поэтому их нужно улучшить)». Strict (default-private) reading is the **only** reading; the implementation wave re-marks every manifest in the tree in the same wave the machinery lands. This overrules the draft's legacy-read/ratchet proposal. @status:spec/done

@fact:migration-flag-day-scope Why the §5 flag-day graveyard does not forbid this: those lessons price a default flip across an **open** ecosystem of foreign artefacts (JPMS: the world's jars; Swift: every existing package). This tree is a closed world — every manifest that exists lives in this repository, C6 publication is frozen, and there are no external consumers yet — so the «flag day» is an ordinary all-packages refactoring wave over owned files. The lesson that *does* carry: land machinery and re-marking in **one atomic wave**, with the §7 golden snapshot proving the intended composition — a half-marked tree is the one state that must never be committed. @status:spec/done

@fact:migration-inventory Re-marking inventory (mandatory, in-wave per ##MIGRATION-NO-LEGACY) for the tree at hand: redbook's 21 member edges → `public` (§4.6); stack aggregators → `public` toward their cores; `delegation-first → delegation-rules` → `public` (the directive delivers its calculus); `wal-specspaces → wal` → `public` (the overlay presupposes its base by design); host root edges — no change needed (rule 1 covers the root); `git-practices` family internals → first candidate for `friends-only` dogfooding. The 8 world packages' `[boot_snippet] link = "static"` self-suggestions are materialisation-axis and untouched. @status:spec/work

@fact:migration-shape Implementation shape (post-approval, separate wave, ask-first per Rule 4 only where its list applies): (1) `vibe-core` manifest schema — the new edge/node fields, strict semantics only; (2) `C(R)`/`E(R)` computation as a new pure module consumed by resolver + bootgen; (3) linker re-scoped to `E(R)`; (4) PROP-049 exemption narrowing; (5) package-tree/`vibe why` observability (§7); (6) re-marking of **every** package manifest in the tree (the inventory above); (7) the golden landing snapshot + measurement pass (§7). Steps 1–5 are machinery, step 6 is the flip, and the wave lands **atomically** — no commit leaves the tree half-marked (##migration-flag-day-scope). @status:spec/done

---

## 7. Verification and measurement — «РЕАЛЬНО ХОРОШО» {#verification}

@fact:verification-plan The owner's bar is explicit: «перестроить статические и динамические лоадеры и проверить, что всё работает РЕАЛЬНО ХОРОШО. Измерить, проверить, посмотреть на тестовых примерах». The implementation wave carries, as first-class deliverables: @status:spec/work

- @fact:VERIFY-GOLDEN-CHAIN **Golden chains.** Fixture worlds for every §2 rule — the owner's `A → B → C → D` chain verbatim (test №1), the unfriend break, the diamond-with-exclude, public-presence-without-friendship, the dev-world flip (same package as root vs as dependency) — each asserting the exact `E(R)`, lock content, and lane bytes. @status:spec/work
- @fact:VERIFY-BYTE-STABILITY **Determinism and the landing snapshot.** Re-runs over identical inputs are byte-identical (the PROP-038 §5 stability bar, extended to visibility inputs). Since there is no legacy mode (##MIGRATION-NO-LEGACY), the correctness artefact is the **golden snapshot of the re-marked world**: the wave captures the intended `E(R)` and lane composition per root, the one-time diff against the pre-visibility lanes is read by hand at landing (expected: WAL-class removals, zero accidental losses), and a panel cell pins the snapshot thereafter. @status:spec/done
- @fact:VERIFY-MEASURE **Measurement.** Before/after per-root metrics, recorded in the wave's harvest: effective-set package count, lane byte sizes, fetch/materialise counts and install wall-time. The tokenomics claim (§1) must show up as numbers, not prose. @status:spec/work
- @fact:VIBE-WHY **Observability: `vibe why <group>/<name>`.** For any package, print the chains that admit it into `E(R)` — each hop annotated with its rule ((1)/(2)/(3)), access mark, and friendship provenance — and for an absent package, the nearest blocked chains and *what* blocked them (private edge / missing friendship / unfriend / exclude). The debugging surface without which a visibility system rots into folklore; `vibe tree` gains the same annotations. @status:spec/work
- @fact:VERIFY-LOCK-DIFF **Closure-drift visibility.** The lock carries `E(R)` with each member's lane cost (bytes/tokens of its contribution); `vibe update` prints the closure diff — packages entering/leaving and the lane-cost delta — so a mid-graph re-export widening (##CLOSURE-DRIFT-CONTROL) is a reviewed event, not a silent seep. @status:spec/work
- @fact:VERIFY-LINTS **Hygiene lints.** Dead `friends`/`unfriend`/`exclude` entries naming packages that never appear in any chain (warning, per the JPMS qualified-export precedent); friends-only edges whose declarant nobody befriends (unreachable sharing); a `friend = false` edge to a package with no friends-only edges (a no-op mark); grants whose admitted chains contribute no lane text this root ever loads — each reported with the lane cost the grant admits, so an unused or cost-heavy friendship is visible (the consumer-side mirror of Eclipse's rotting x-friends lists). Advisory, in `vibe check`. @status:spec/work

---

## 8. Rollback {#rollback}

@fact:rollback-shape **By owner ruling — none.** Until implementation this document changes nothing. Once the wave lands, visibility is the primary and irreversible model — «становится основной и не откатываемой никуда» — and pre-visibility manifest reading retires with it; there is no legacy mode to fall back to, and the old formats are not preserved («пакеты предыдущих форматов не имеют смысла»). `formats/EPOCHS.toml` is untouched; the safety that replaces rollback is the atomic wave + golden snapshot of §6/§7. @status:spec/done

---

## 9. Open forks for the owner {#forks}

@fact:fork-list The reconstruction forks, with the owner's 2026-08-23 rulings recorded; F7–F9 remain live: @status:spec/work

1. @fact:FORK-F1-INTERNAL **F1 — «internal» ≡ `access = "friends-only"`. RATIFIED** («правильный термин — friends-only»): one mark is both the seepage gate and the friendship re-export (§2.4); no separate `internal` field exists. @status:spec/done
2. @fact:FORK-F2-NAMING **F2 — the anti-list is `unfriend`. RATIFIED**; `enemy` is retired. @status:spec/done
3. @fact:FORK-F3-DEFAULT-FRIEND **F3 — `friend = true` by default on a node's own edges. RATIFIED** («по-умолчанию true на своих рёбрах»): a node befriends what it directly requires unless it says otherwise; the rarity of `friends-only` marks bounds closure growth. @status:spec/done
4. @fact:FORK-F4-EXCLUDE-SCOPE **F4 — `exclude` is per-edge, Maven-style, arbitrarily deep within the edge's subtree. RATIFIED** («можно per-ребро делать глубокие эксклюды по иерархии — это не глобальный deny-list, это сужение в рамках поддерева»). No global deny-list. F9 may re-house the syntax inside `override`; the semantics stand. @status:spec/done
5. @fact:FORK-F5-MIGRATION **F5 — migration. OVERRULED by the owner:** no legacy read, no backward compatibility, the system is primary and irreversible; every existing package is re-marked in the landing wave (##MIGRATION-NO-LEGACY, ##migration-flag-day-scope). The draft's autodetect+ratchet proposal is retired. @status:spec/done
6. @fact:FORK-F6-PUBLIC-NO-FRIENDSHIP **F6 — public gives presence, not friendship. RATIFIED** («access и friend разные понятия, public даёт присутствие но не дружбу»); the composition of the two marks on one edge is pinned as ##PUBLIC-PLUS-FRIEND. @status:spec/done
7. @fact:FORK-F7-PROP-049 **F7 — PROP-049's fate. OPEN.** This draft keeps the genre machinery re-scoped (§4.3) with the deps-exemption narrowed. 2026-08-23: the owner asked how `concepts` actually works before ruling — mechanics delivered (the array feeds exactly one consumer today: the presupposition gate's dictionary; no load-time gating exists), plus the ##CONCEPT-GATE-DIRECTION completion sketch (`when = "concept:X"`). Ruling pending. @status:spec/work
8. @fact:FORK-F8-PROVIDER-ALLOWLIST **F8 — provider-side allowlists. OPEN, still recommended deferred.** 2026-08-23: mechanics explained at the owner's request — an `allow-friends = […]` on the provider would gate **entry into friendship only** (a grant naming an unlisted root is ignored with a warning; `public`/`private` delivery untouched), the Eclipse x-friends / JPMS `exports…to` direction. Kept out because the payer-decides inversion removes the need and provider lists rot (§5); syntax stays reserved. @status:spec/work
9. @fact:FORK-F9-OVERRIDE **F9 — root-side `override`: the sanctioned break-in. OPEN (owner-proposed 2026-08-23, analysis delivered, ruling pending).** The owner proposes an `override` list able to *rewrite foreign edges* — change their `access` or `friend`, or kill them — replacing ad-hoc Maven/Gradle-style resolution hacks with a first-class verb, possibly absorbing `exclude`. Analysis: **topologically sound and cheap** — the override table is static, applied as a mask over edge attributes *before* both fixpoints run, so determinism, order-independence and `O(V+E)` survive; overrides create no edges, so acyclicity is untouched; an entry naming an edge outside metadata reach is a dead entry (warning, per the §5 unknown-target precedent). The load-bearing boundary the analysis surfaces: **subtraction vs expansion**. Subtractive acts (exclude, unfriending) only narrow what providers granted — safe at any node, contractual. Expansive acts (`private → friends-only/public`, forcing friendship) hand a root content its provider never promised — a deliberate breach of ##ACCESS-IS-SEMVER-SURFACE, so the provider's next release may break the broken-into content unannounced. Recommendation: (a) expansive `override` is **root-only** (JPMS `--add-exports`/`--add-opens` and Gradle `resolutionStrategy` precedent: break-ins live with the party that runs and pays, never mid-graph where they would silently rewrite contracts for everyone downstream); (b) keep per-edge `exclude` as the subtractive verb any node may use on its own delivery (F4 as ratified) rather than folding it in — one list mixing the two genres would blur the safety boundary; (c) every override is loud: install notice, lock provenance mark, and `vibe why` chains annotated «via root override». Sketch: `[override]` `"org.x/col -> org.x/m" = { access = "public" }` · `{ friend = false }` · `{ exclude = true }`. @status:spec/work

---

## 10. Version history {#history}

- @fact:HISTORY-RATIFIED-CORE **2026-08-23 (later the same day) — core ratified by the owner.** F1 (friends-only ≡ the re-export mark), F2 (`unfriend`), F3 (`friend = true` default on own edges), F4 (per-edge deep-subtree `exclude`) and F6 (access ⊥ friendship; ##PUBLIC-PLUS-FRIEND added) ruled as drafted. F5 **overruled**: no legacy read, no backward compatibility, irreversible — ##MIGRATION-NO-LEGACY replaces the autodetect+ratchet proposal; §7/§8 re-anchored to the atomic wave + golden snapshot. F7 (concepts mechanics + ##CONCEPT-GATE-DIRECTION) and F8 (allow-list mechanics) explained, rulings pending; F9 (`override`, the root-side sanctioned break-in) filed with the subtraction-vs-expansion analysis. @status:spec/done
- @fact:HISTORY-DRAFTED **2026-08-23 — drafted (owner-ordered).** The visibility model: per-edge `access` (public / private-default / friends-only), consumer-declared friendship (per-edge flag + node list), the friend closure as a least fixpoint over friends-only re-exports, `unfriend` (node-scoped) and `exclude` (edge-scoped) growth controls, root-driven materialisation. Layering: visibility above materialisation (owner ruling); the `link` axis survives re-scoped to the effective set. Interactions: resolver/lock/vibedeps on `E(R)` only; PROP-049 kept re-scoped with the deps-exemption narrowed; the dev-world reading resolves the equipotence wave's open tail. Migration by per-manifest legacy read; verification plan with golden chains, byte-stability, measurement, and `vibe why`. Prior-art synthesis and §9 forks await the owner. @status:spec/work
