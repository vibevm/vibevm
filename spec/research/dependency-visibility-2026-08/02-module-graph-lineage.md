# Module/build-graph visibility & transitivity — research notes for PROP-050

> Research worker report, 2026-08-23. Consumer: [PROP-050 §5](../../common/PROP-050-dependency-visibility.md#prior-art).
> Scope: OSGi, Eclipse PDE (`x-internal`/`x-friends`), Bazel, Buck/Buck2, Pants.

**Frame first.** Every system below is one point in a 3-axis space: *who declares* (provider allowlist vs consumer restriction), *unit* (package vs module/target), *when checked* (resolve / analysis / compile / IDE). The proposed per-edge `access` is, almost exactly, OSGi's `Require-Bundle; visibility:=private|reexport` plus a third value — so OSGi's verdict on `reexport` is the most directly transferable lesson in this report.

| System | Direction | Unit | Enforcement point | Transitivity primitive |
|---|---|---|---|---|
| OSGi | provider (`Export-Package`) | **package** | runtime resolver, hard fail | `visibility:=reexport`; `uses:` |
| Eclipse PDE | provider (`x-friends`) | package | **IDE compiler only** (runtime opt-in) | none (flat allowlist) |
| Bazel | provider (`visibility`) | target/package | analysis phase, hard fail | `exports` edges; `transitive_visibility` |
| Buck/Buck2 | provider `visibility` **+ consumer `within_view`** | target | graph parse | none |
| Pants 2.16+ | **both ends must agree** | target/file | dep-rule check | explicitly non-transitive |

---

## 1. OSGi

| Primitive | Effect |
|---|---|
| `Export-Package: p` | only listed packages leave the bundle; everything else is bundle-private ([spec 3.6.5](https://docs.osgi.org/specification/osgi.core/8.0.0/framework.module.html)) |
| `Import-Package: p;version="[1,2)"` | fine-grained wire to *some* exporter of `p`; resolver free to choose |
| `Require-Bundle: B` | coarse wire to a named bundle; imports **all** of B's exports |
| `visibility:=private` (default on Require-Bundle) | "all visible packages from the required bundles are **not** re-exported" |
| `visibility:=reexport` | "bundles that require this bundle will **transitively** have access to these required bundle's exported packages" |
| `uses:="q,r"` on an export | class-space constraint: importing `p` forces your wire for `q,r` to match the exporter's |
| `mandatory:="attr"` on an export | importer must *name* a matching attribute to resolve — a provider-side opt-in gate (a "password", used for split packages) ([bnd](https://bnd.bndtools.org/heads/export_package.html)) |
| `include:`/`exclude:` on an export | class-level filtering *within* an exported package — pruning below the sharing unit |
| `Fragment-Host` | fragment attaches to host, **shares the host's classloader**; its packages merge into the host's namespace |
| `resolution:=optional`, `DynamicImport-Package` | soft/late edges |

**Default + rationale.** Nothing is shared unless exported: modularity is opt-in *by the provider*, because Java packages otherwise leak everything public. Re-export is off by default on Require-Bundle for the same reason.

**Transitivity.** Import-Package is strictly non-transitive (each bundle declares what it needs). Require-Bundle is transitive *one level per `reexport` hop*, chaining through consecutive reexport edges. `uses:` is the *implicit* transitivity: if an exported type's signature mentions types from another package, that package's wiring becomes a constraint on every importer, recursively — "the Framework must ensure that none of its bundle imports conflicts with any of that bundle's implied packages" (3.7.6). This solved the real bug it was invented for: two bundles resolving to *different* providers of a shared type → `ClassCastException` between "the same" class.

**Pitfalls / community verdict.** Require-Bundle is "the gateway drug for split packages" — without it, splits are impossible, since Import-Package selects exactly one provider ([eclipsesource](https://eclipsesource.com/blogs/2008/08/22/tip-split-packages-and-visibility/), [Chris Daniel](http://eclipseandlinux.blogspot.com/2013/04/on-evilness-of-split-packages-in-osgi.html)). Arguments against `reexport`, verbatim in spirit from practitioners: (a) it over-constrains the resolver — "importing at the granularity of packages allows the resolver to be more flexible… if a package is already available and resolved from another bundle, the resolver could use that" ([Red Hat/Karaf best practices](https://docs.redhat.com/en/documentation/red_hat_jboss_fuse/6.3/html/deploying_into_apache_karaf/bestpractices-buildbundles)); (b) **it is irreversible** — "once you have added re-export you cannot remove it without considering the corresponding API change" ([OpenChrom](https://openchrom.wordpress.com/2015/01/20/osgi-visibilityreexport-directive/)); (c) you inherit someone else's semver discipline. `uses:` constraints are the other famous pain — resolution failures ("uses constraint violation") are notoriously opaque and appear only at assembly time, far from the edit ([Spring](https://spring.io/blog/2008/10/20/understanding-the-osgi-uses-directive/)).

**COPY/AVOID.** COPY the default (`private`, nothing propagates) and COPY the *unit below the module*: OSGi shares packages, not bundles — for vibevm that means visibility could attach to the **lane/snippet** a package contributes, not to the package as a whole; `include:`/`exclude:` at class level is the precedent for pruning inside a shared unit. COPY the irreversibility warning verbatim: `access = public` on an edge is a **public API commitment** of the intermediate package and must be versioned as such. AVOID `uses:`-style implicit transitivity: it exists because Java has a type system that can be *incoherent*; prompt text has no class-space, so a seepage system must never turn a soft budget concern into an unsatisfiable global constraint. Fragments (attach into the host's namespace) are worth stealing as a separate concept — "this package injects text into that package's lane" — but keep it out of the visibility axis.

---

## 2. Eclipse/Equinox — `x-internal` / `x-friends` (closest prior art to friends-only)

| Primitive | Effect |
|---|---|
| `Export-Package: p;x-internal:=true` | exported, but PDE marks it **discouraged**; default `false` |
| `Export-Package: p;x-friends:="a,b"` | named bundles get *accessible*; everyone else gets **discouraged** ([wiki](https://wiki.eclipse.org/Export-Package)) |
| not exported at all | **forbidden** — compile *error*, and a real classloading failure at runtime |

**Enforcement.** Three-level ladder: accessible / discouraged / forbidden. "PDE translates these runtime visibility rules into compiler access restriction rules at compile time" — forbidden = error, discouraged = **warning** plus demoted content-assist priority ([PDE Access Rules](https://help.eclipse.org/latest/topic/org.eclipse.pde.doc.user/guide/tools/editors/manifest_editor/access_rules.htm)). Crucially: "Internal packages are visible to downstream plug-ins **by default**… hidden only when Eclipse is launched in **strict** mode (`-Dosgi.resolverMode=strict`)". So the friends allowlist is **advisory tooling, not a runtime boundary** — the package is fully exported to the framework.

**Real-world lessons.** (1) Provider-declared allowlists impose cross-team latency: a new consumer needs a *commit in the provider's manifest*, so lists accrete and never shrink. Wildcards were requested precisely because the lists got long ([bug 422397](https://bugs.eclipse.org/bugs/show_bug.cgi?id=422397)). (2) Friend status collides with the *other* enforcement layer — API Tools still flags friends' usage, requiring a separate exemption ([bug 230279](https://bugs.eclipse.org/bugs/show_bug.cgi?id=230279)). (3) Governance grew around it: Eclipse required a written `API.readme.txt` justification and PMC approval to add/remove internal exports, and platform guidance is "using x-friends… should always be preferred over x-internal" — i.e. *name your leaks* ([vogella](http://blog.vogella.com/2010/05/25/x-friends-in-equinox/), [Provisional API Guidelines](https://wiki.eclipse.org/Provisional_API_Guidelines)). (4) Warnings-not-errors means violations accumulate as noise; whole projects flip discouraged-access severity globally ([xtext#1175](https://github.com/eclipse/xtext/issues/1175)).

**COPY/AVOID.** COPY the *three*-level ladder (accessible / discouraged / hard-denied) — a middle "you can, but it's flagged and counted" tier is what makes an opt-in budget system survivable. COPY the requirement of a written justification per grant. **AVOID the direction**: Eclipse's list is provider-declared and it rots because the party who pays (the consumer) can't edit it. The vibevm inversion — consumer-declared friendship — removes exactly that failure mode, and it is the right call for a budget concern (the consumer owns the token bill). But it inherits the mirror risk: *consumer* grants also rot — a root that once granted friendship to pull one lane keeps paying for it forever. Mitigate with what Eclipse lacked: report actual token cost per grant and flag unused grants.

---

## 3. Bazel

| Primitive | Effect |
|---|---|
| `//visibility:public` / `private` | all packages / "only targets in this location's package can use this target" |
| `//foo/bar:__pkg__` / `:__subpackages__` | that package / that package + all sub-packages ([docs](https://bazel.build/concepts/visibility)) |
| `package_group(packages=[…], includes=[…])` | named allowlist; `includes` is **transitive**; `packages` supports negation `-//foo/bar/...` |
| `package(default_visibility=…)` | per-package default; **private** if unset (and inside symbolic macros) |
| `transitive_visibility` (package param) | restricts who may depend **indirectly**; "if a target has multiple transitive dependencies with transitive visibility restrictions, it must meet **all**" |
| `visibility()` in `.bzl` | load visibility; **default public** |
| `exports` on `java_library` | "X can access Y if there is a dependency path that begins with a `deps` edge followed by **zero or more `exports` edges**" ([java rules](https://bazel.build/reference/be/java)) |
| `--strict_java_deps` | you must declare what you *directly* use; error names the fix: `buildozer 'add deps //:dog' //:park` ([blog](https://blog.bazel.build/2017/06/28/sjd-unused_deps.html)) |

**Transitivity.** Ordinary visibility is checked **per direct edge only** ("fail… if it violates the visibility of one of its dependencies"), which is why `transitive_visibility` had to be added as a *separate system*. `exports` is the re-export chain and it **does chain** — but the first hop must be a `deps` edge, and "an exported rule is not a regular dependency: if B exports C and wants to also use C, it has to also list it in its own `deps`." That asymmetry (re-export ≠ use) is worth copying literally.

**Negation trap.** "negative patterns in one package group do not affect the result of included package groups" — the union is computed per-group, then unioned. A node-local `unfriend` hits this exact question: does a subtraction survive composition? Bazel decided **no**, deliberately, to keep group composition monotone and predictable.

**Scale lessons.** Guidance is explicitly anti-churn: prefer `__subpackages__` over `__pkg__` "to avoid needless visibility churn as that project evolves"; use a `package_group` rather than repeating lists "to prevent the lists from getting out of sync". At real scale it still needs tooling — Salesforce shipped a [visibility tool](https://github.com/salesforce/bazel-visibility-tool) for "centralized definition of visibility layers… centralized allow and block list management". Strict deps is the counterpart insight for *budget*: transitive availability is a bug, not a feature; make each consumer declare what it actually uses, and ship a **mechanical autofix** so the discipline is free.

**COPY/AVOID.** COPY three things: (a) `deps`-then-`exports*` as the exact formal rule for the friend closure — precise, cheap to implement, battle-tested; (b) `package_group` + `includes` as named, composable friend sets (never inline lists at each root — they desync); (c) **strict-deps with an autofix command** — for prompt packages the analogue is "this lane's text was never referenced/needed → `vibe unfriend X`", emitted as a runnable command, plus an `unused_deps`-style reporter. COPY the "prefer subtree grants" ergonomics. AVOID pure hard-fail on first violation without an autofix path (Bazel gets away with it only because buildozer exists), and AVOID making negation propagate through composed groups.

---

## 4. Buck / Pants — the consumer-side axis

**Buck/Buck2** adds `within_view`: "a list of visibility patterns restricting **what this target can depend on**"; unset = unrestricted; `PUBLIC` is valid in `visibility` but **not** in `within_view`; and "in case of logically-conflicting lists, `within_view` takes precedence over `visibility`" — if `//foo:bar` lists `//hello:world` as visible but `//hello:world` does not list `//foo:bar` in `within_view`, the dep is denied ([buck](https://buck.build/concept/visibility.html), [buck2](https://buck2.build/docs/concepts/visibility/)). Both are settable in `PACKAGE` files and inherited down the tree.

**Pants 2.16+** is the cleanest formalisation of the two-sided model: `__dependencies_rules__` = "this X may only import from …", `__dependents_rules__` = "this X may only be imported from …", and "**the dependency link as a whole is only allowed if both ends of the dependency allow it**" ([docs](https://www.pantsbuild.org/stable/docs/using-pants/validating-dependencies)). Actions are allow / `!` deny / `?` **warn**. Rule sets are selected by first-matching selector; rules propagate to subtrees unless overridden, with `extend=True` to append parent rules; path globs have four anchoring modes (`//` root, `/` BUILD-file, `.` target-file, floating). Defaults: no rules ⇒ everything allowed, but "it is an error for there to not be any matching rule, **if any rules are defined**" — i.e. opt-in per subtree, then exhaustive. And explicitly: "visibility rules only operate on direct dependencies — they do not validate dependencies transitively", with the stated rationale that transitive validation would forbid using a public API that internally uses private modules.

**COPY/AVOID.** COPY Pants' conjunction as the core evaluation rule: an edge is materialised iff *provider access permits* AND *consumer grant permits* — this is precisely `friends-only` + consumer friendship, and it means the two mechanisms need no special-case interaction logic. COPY `?`-warn as a first-class action (budget systems need "allowed but reported"). COPY Buck's `within_view` as the model for `exclude`: a consumer-side, precedence-winning cap that is *not* expressible as `PUBLIC` — an exclusion should always beat any grant, including a re-export chain. COPY Pants' subtree inheritance with explicit `extend` for node-local `unfriend`. AVOID Pants' "no matching rule is an error" as a global default; for token budget, default-allow-with-report until a root opts in. Note the domain inversion once more: all four systems police *correctness/coupling*, so a denied edge is a build break; for vibevm a denied edge is merely text not loaded — which means **the enforcement point should be materialisation-time with a cost report, not a hard resolver failure**, and only `exclude` should be a hard error when violated.
