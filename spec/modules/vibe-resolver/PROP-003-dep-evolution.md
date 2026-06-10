# PROP-003 — Dependency-model evolution: SAT solver, features, subskills, context activation, i18n

**Status.** Design proposal — not yet implementation-locked. Companion to [PROP-000](../../common/PROP-000.md) (project foundation), [PROP-002](../vibe-registry/PROP-002-decentralized-registry.md) (registry model). Supersedes the depsolver paragraphs of PROP-002 §2.8 (which left the solver upgrade path as a one-line "resolvo or libsolv slot reserved"); does not touch PROP-002's identity or registry decisions.

**Revision r2 (2026-05-04, post-PROP-004).** First revision shipped 2026-05-04 morning. Second revision shipped same day after the [PROP-004 Tessl comparative research](../../research/PROP-004-tessl-comparative-research.md) surfaced eight architectural improvements that were better folded into the design proposal *before* implementation than retrofitted later. Diff at the section level:

- §2.5 expanded with **three delivery modes** (eager / lazy-push / lazy-pull) as a primary axis of the subskill manifest, not a follow-up bolt-on.
- §2.5.1 subskill manifest grows a required **`description` field** for lazy-push activation — natural-language trigger that the agent matches against the current task (Tessl's load-bearing pattern, copied here verbatim semantics).
- §2.5.2 context-based activation **broadened**: alongside `if_present` and `if_provides`, new probes `if_files`, `if_command`, `if_env`, `if_describes_match` cover real-world triggers that don't require explicit capability/interface declarations.
- §2.5.3 LLM-inferred activation **refactored** from "LLM toggles subskills directly" into "LLM emits virtual capabilities into the dep graph" — same expressive power, single audit point, and normal activation channels (`if_present` / `if_provides`) handle the actual toggle.
- New §2.5.6 — **`describes` PURL on subskills** (not just packages), so a subskill targeting FastAPI 0.116.1 is a different object from the one for 0.117 even within the same parent.
- New §2.6.1 — **Conditional dependencies** (`[target."context(...)".dependencies]` ≈ Cargo's `cfg` deps), distinct from subskill activation.
- §2.4's `__exclusive` sigil replaced with a **named-group `[features.exclusive]` table** — TOML-idiomatic, no underscore namespace dance.
- §2.10 `vibe check` gains an **activation-conflict** check that catches description triggers that overlap across subskills (the same axis Tessl's review rubric scores under "activation distinctiveness").

The first-revision text is preserved in place; revision-r2 additions are inline at their natural locations.

**Scope.** This document specifies four interlocking upgrades to the vibevm dependency model:

1. **SAT-class solver** behind the existing `DepSolver` trait, replacing `NaiveDepSolver` for non-trivial graphs while keeping the trait surface and lockfile shape intact.
2. **Optional components (features)** in the cargo-features tradition — first-class declarations in the package manifest, with all the conditional-activation, additive-only, and feature-unification semantics of cargo's feature resolver v2.
3. **Subskills** — a vibevm-native concept: optional sub-documents inside a package with **three delivery modes** (eager / lazy-push / lazy-pull), addressable by feature mappings, by context-based activation rules, by natural-language description match, by `describes` PURL binding, and (post-M1.5) by LLM-emitted virtual capabilities. Subskills are *not* a re-skin of cargo features; they are a content unit with a much richer activation and delivery surface that features feed into.
4. **Internationalization** — first-class language preference at the project, package, and CLI level; deterministic fallback to canonical English; standardised file-naming pattern that doesn't fight existing OS / Git tooling.

**Why now (pre-release).** vibevm has no public release, no external users, and no migration cost yet. PROP-002 §2.7's lockfile schema v2 already had to absorb one revision; further schema churn before v0.1.0 is free. After release, every change to the dep-model would carry migration weight that we currently avoid. This is the right window to widen the contract.

**Reading order.** Top-to-bottom is fine; §2 sections cross-reference each other when concepts compose. §3 (algorithm) and §4 (rejected alternatives) can be skipped on first read.

---

## 1. Problem statement {#problem}

The current dependency surface (PROP-002) ships the right *minimum* for a walking-skeleton package manager: per-package decentralized registries, content-hashed identity, capability-based `[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` / `[conflicts]`, transitive resolution through a `DepSolver` trait. The first impl is `NaiveDepSolver` — depth-first, single-pass, no backtracking. PROP-002 §3.7 also explicitly *defers* optional / recommended / supplemental dependencies to v1.

Three concrete shortfalls block real-world graphs:

- **Disjunction without backtracking is a footgun.** `NaiveDepSolver` picks the first `one_of` alternative that resolves; if a later constraint contradicts that pick, the solver fails out instead of trying alternative #2. For a graph with two disjunctions intersecting through a shared capability, it produces a "no solution exists" diagnostic on graphs where a solution does exist. This is the same class of bug Cargo had before pubgrub-driven backtracking — observable, embarrassing, blocks adoption.
- **All-or-nothing packages don't compose.** A `flow:wal` package today brings *every* file it ships, every time, regardless of which project consumes it. Real-world specs are almost never one-shape-fits-all: the WAL flow has a `stack/rust`-specific section that should not materialise in a Python project, an "atomic commits only" subset that's useful when paired with `flow:atomic-commits` but redundant otherwise, an LLM-coordinator-specific addendum that only matters when the project is targeting Claude Code. Without optional components, the package author must ship the union (bloat) or fragment into multiple registry entries (combinatorial explosion + bad cohesion).
- **Specs are language-locked at file level.** Today every `*.md` file in a package is canonical English. A Russian-speaking team that wants `vibe install flow:wal` to land Russian-localised protocol files has no escape hatch except forking the package — which loses upstream. This is the dimension `cargo` doesn't need to think about (code has one canonical syntax) but vibevm fundamentally does (specs are *prose* and prose translates).

PROP-003 addresses all three, plus a fourth-dimension addition unique to vibevm:

- **LLM-driven contextual activation.** Once the M1.5 LLM-build pipeline is in place, the solver gains a fifth class of activation signal: the LLM, having read the effective spec corpus and the target feat, can decide that a particular subskill is relevant *for this build* even though no static manifest declared it. This is not feature inference (Cargo has nothing like it) — this is a runtime-contextual upgrade where activation keys flow from the LLM's reading of project intent rather than from the package author's foresight.

The four together are what makes vibevm the spec-driven companion to Claude Code / Claude Cowork — a Claude-native package manager that understands *which parts of a package matter for this project*, not just *which packages*.

## 2. Decisions {#decisions}

### 2.1 Solver upgrade path: SAT-class engine behind the existing `DepSolver` trait {#solver-upgrade}

**Decision.** Add a second `DepSolver` impl, `SatDepSolver`, alongside `NaiveDepSolver`. Both implement the same `crates/vibe-resolver/src/lib.rs::DepSolver` trait (`fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError>`). `NaiveDepSolver` stays in tree as the "small graphs / no features / no disjunctions" fast path; `SatDepSolver` becomes the default once the second-tier features in this PROP land.

The selector is a single line in `vibe-cli/install.rs` (and the parallel paths in `update`, `vendor`, `check`); selection key is `[meta].solver` in the lockfile (already wired in PROP-002 §2.7) plus an optional `--solver <naive|sat>` CLI override for debugging. Default after this PROP lands: `sat`.

**Why two impls, not "rip out Naive."** Naive is ~250 lines of straightforward Rust covering ~95 % of today's fixture graphs at constant-fold-of-DFS speed. The SAT-class engine, even when wrapping libsolv, is heavier to cold-start (rule encoding, watched-literals init); for trivial graphs that's pure overhead. Keeping both lets us regression-test the SAT impl against Naive's outputs on simple graphs, which is the cheapest oracle we'll ever have.

**`DepSolver` trait — minimal additions.** The trait gains one method:

```rust
trait DepSolver {
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError>;

    /// Hint the solver to prefer keeping packages already pinned in the
    /// caller's lockfile. The default impl ignores the hint (correct
    /// for `NaiveDepSolver`, which has no preference machinery).
    /// `SatDepSolver` honours it via libsolv's "favor" rules.
    fn pin_preferences(&mut self, _pins: &[(PackageRef, semver::Version)]) {}
}
```

`pin_preferences` is what enables the `vibe update` "minimum churn" property: re-resolve, but prefer the existing version of every untouched package. PROP-002 §2.7's `[meta].root_dependencies` carries the user-typed roots; the lockfile's `[[package]]` entries carry the satisfying pins. `pin_preferences` consumes the latter.

### 2.2 SAT solver backend: libsolv (BSD-3-Clause), via thin Rust FFI {#solver-backend}

**Decision.** The SAT engine of `SatDepSolver` is **libsolv** ([`https://github.com/openSUSE/libsolv`](https://github.com/openSUSE/libsolv)). Wrap it through a *thin* in-tree FFI layer (a new `vibe-resolver-libsolv` crate or feature-gated module under `vibe-resolver`); do not pull in `libdnf5` or any LGPL-licensed shim.

**License audit (load-bearing).** libsolv is dual-licensed BSD-3-Clause / FreeBSD ([`LICENSE.BSD`](https://github.com/openSUSE/libsolv/blob/master/LICENSE.BSD)). Permissive, satisfies PROP-000 §3 (third-party deps: permissive only — MIT / Apache-2.0 / BSD / Unlicense; MPL-2.0 case-by-case; **GPL/AGPL/LGPL forbidden**). Linking against libsolv as a C library or a static archive is fine. We MUST NOT link against `libdnf5` (LGPL-2.1-or-later) — its API is the most ergonomic layer over libsolv but its license places a copyleft obligation on every consumer.

**Why libsolv, not the alternatives.**

- **`resolvo`** ([`https://github.com/prefix-dev/resolvo`](https://github.com/prefix-dev/resolvo)): pure-Rust, BSD-3-Clause, used by Pixi / Rattler at conda scale. Strong candidate, was the leading PROP-002-era choice. Reasons not to pick first: (a) younger codebase (~3 years vs libsolv's ~17), (b) less battle-tested under adversarial inputs, (c) does not expose the rule-level introspection libsolv does, which we need for explanation-driven error messages. *We keep the door open*: `vibe-resolver-resolvo` could be a future second SAT impl behind the same trait if libsolv proves operationally heavy on Windows or surfaces unfixable upstream bugs.
- **`pubgrub`** (Cargo's solver): BSD-3-Clause (well, MIT/Apache-2.0 dual), pure-Rust, designed for SemVer-shaped constraints. Strong on disjunction explanation. Reasons not to pick first: pubgrub's cost model and rule encoding don't map cleanly onto our capability model (provides/requires/obsoletes/conflicts plus weak-deps); we'd have to encode capabilities as virtual packages and lose pubgrub's native explanation hooks.
- **Custom solver from scratch.** Out of scope. PROP-000 §15 ("dep weight not a decision factor") + §17 ("production architecture in prototype phase") both push us toward "use the best library, full stop."

**Rust FFI surface — minimal.** We expose only the libsolv calls we use:

```rust
// crates/vibe-resolver-libsolv/src/ffi.rs (sketch)
extern "C" {
    fn pool_create() -> *mut Pool;
    fn pool_free(pool: *mut Pool);
    fn solver_create(pool: *mut Pool) -> *mut Solver;
    fn solver_solve(solver: *mut Solver, jobs: *mut Queue) -> c_int;
    fn solver_problem_count(solver: *mut Solver) -> c_int;
    fn solver_findproblemrule(solver: *mut Solver, problem: c_int) -> Id;
    fn solver_describe_decision(solver: *mut Solver, p: Id, info: *mut c_int) -> Id;
    // … 20-30 more, all from libsolv's stable public header `solv/*`
}
```

Build-side: vendor libsolv as a git submodule (or fetch via build-script — preferred to avoid submodule fragility on Windows); compile with `cc` crate; link statically. Windows builds use the bundled C compiler from MSVC or MinGW (the same toolchains we already require for `cargo build`).

**Cargo features.** A `vibe-resolver-libsolv` crate is gated behind a workspace-level feature so a contributor on a fresh checkout without a C toolchain can still build the rest of the workspace and run `NaiveDepSolver` for tests. CI builds with the feature on.

### 2.3 SAT solver capabilities we rely on {#solver-features}

These are the libsolv-provided algorithmic guarantees that make the rest of PROP-003 tractable. None of them are present in `NaiveDepSolver` today.

1. **Conflict-driven clause learning (CDCL).** When a `[[requires_any]]` choice contradicts later constraints, the solver backtracks, learns a clause excluding the bad combination, and tries an alternative. This is the table-stakes property `NaiveDepSolver` lacks.
2. **Watched-literals propagation.** O(literals × decisions) propagation cost rather than O(rules × decisions), keeping per-decision work near-constant on graphs with hundreds of capabilities.
3. **Weak-deps semantics** — the four levels libsolv inherited from RPM:
   - `Recommends`: prefer to install, but don't fail solve if impossible.
   - `Suggests`: hint to the user; never auto-installed.
   - `Supplements`: install *me* if some other package in the graph wants it.
   - `Enhances`: hint that *I* enhance another package; UI surface only.
   These map onto vibevm `[recommends]` / `[suggests]` / `[supplements]` / `[enhances]` manifest sections (§2.9). The crucial property: a missing `[recommends]` package is a **warning**, not an error — `NaiveDepSolver` cannot represent this distinction at all today.
4. **Problem reporting (decision-tree explanation).** When the graph is unsatisfiable, libsolv returns a structured `Problem` per conflict — naming the offending rules, the chain that led there, and a list of `Solution`s the user can apply (relax constraint X, drop package Y, accept downgrade Z). We map these to vibevm's existing `SolveError` variants and surface them in `vibe install` / `vibe update`.
5. **Favor / disfavor / pin rules.** libsolv accepts soft-preference rules: "if multiple solutions exist, prefer the one keeping `<pkgref>@<version>` installed." This is what `pin_preferences` rides on; it gives `vibe update` predictable minimum-churn behaviour even on graphs where an unrelated update opens new flexibility.
6. **Multi-version / multi-arch handling we don't need today** (RPM-specific) is left disabled — libsolv supports it but vibevm has no parallel concept (every install is single-version per `(kind, name)`).

What we **don't** rely on from libsolv:

- RPM rich-dep boolean expressions (`(A or B)` in the `Requires:` field). Our `[[requires_any]]` covers the most common use case; richer logic can be added later if pulled by adoption.
- Module / stream / context machinery from `dnf modules` (it's RPM-specific and orthogonal to our subskill model — see §2.5).
- libsolv's repo-format readers (solv files, repomd.xml). Our `MultiRegistryResolver` already produces `ResolvedNode`s; we feed those into libsolv's pool, not the other way around.

### 2.4 Optional components (features) — cargo-tradition with vibevm twists {#features}

**Decision.** A package's `vibe-package.toml` gains a `[features]` table describing optional, conditionally-activated components:

```toml
[features]
default = ["wal-protocol", "atomic-commits-section"]
wal-protocol = []                    # zero-cost feature toggle
atomic-commits-section = ["dep:flow-atomic-commits"]
llm-prompt-templates = ["subskill:llm-coordinator/anthropic"]
rust-stack = ["subskill:stack/rust"]
python-stack = ["subskill:stack/python"]

# Mutually exclusive — solver enforces. Named groups, not the
# underscore-prefixed sigil from revision r1; TOML-idiomatic.
[features.exclusive]
stacks = ["rust-stack", "python-stack"]
```

**Semantics — copied from cargo's feature resolver v2 with one reduction and one extension.**

The cargo subset we keep, verbatim:

- **Additive only.** Enabling a feature can introduce additional content; never remove or contradict existing content. (Cargo enforces this informally; vibevm enforces it via `vibe check` since spec content collisions are easier to detect than code-level ones.)
- **Default features.** `default = [...]` lists features active when no override is given. `--no-default-features` on the install / update CLI omits them.
- **Feature-feature dependency.** `feat-A = ["feat-B"]` — enabling A enables B transitively.
- **Optional dep activation.** A `[dependencies.foo] optional = true` entry creates an implicit feature named `foo` that activates the dep; alternatively the explicit `dep:foo` syntax in a feature list activates the dep without exposing the implicit feature name.
- **Weak feature** (cargo's `dep?/feat` syntax): `feat-A = ["other-pkg?/some-feat"]` — if `other-pkg` is *already* in the graph, request `some-feat` on it; otherwise no-op. The `?` prevents activation-by-default of `other-pkg`.
- **Per-target feature activation** (cargo's `[target."cfg(...)".dependencies]` shape): for vibevm this maps onto `[target."context(stack:rust)".dependencies]` — see §2.6.
- **Feature unification across the dep graph.** If `pkg-A` and `pkg-B` both depend on `pkg-C` and request different features, the solver unifies — `pkg-C` is built/materialised once with the union of requested features.

The cargo subset we **drop**:

- `dev-dependencies` / `build-dependencies` distinction. vibevm has no compile-time graph; a single category of deps suffices.
- Feature unification opt-out (`resolver = "1"` in cargo). vibevm always unifies (resolver v2 only).
- `cfg(...)` based feature gating on rustc target triple. vibevm replaces this with our own context predicates (§2.6).

The vibevm-specific extension we add:

- **Mutual exclusion** via `[features.exclusive]` named groups — each value list is an at-most-one set, enforced by the SAT solver via direct conflict rules. cargo has no equivalent (because rustc cfg-conditioning makes mutual exclusion software-rebuilt, not solver-enforced); vibevm uses it for cross-cutting choices like `rust-stack` vs `python-stack` where both make sense individually but not together. Named groups (`stacks = [...]`, `languages = [...]`) read better than r1's underscore-prefixed `__exclusive = [[…], […]]`.
- **Feature → subskill mapping.** A feature can list `subskill:<path>` in its activation list, which directs the resolver to materialise the corresponding subskill (§2.5). This is the bridge between cargo-style features and vibevm-native subskill content.
- **Feature visibility.** Features prefixed with `_` (underscore) are *implementation details* — invisible to consumer manifests; cannot be activated by name from outside the package. Cargo has an informal convention here; we make it solver-enforced.

### 2.5 Subskills — vibevm-native optional content units with three delivery modes {#subskills}

**Decision.** A package may carry a `subskills/` subtree alongside its top-level content:

```
flow-wal/
├── vibe-package.toml
├── README.md                        # canonical, always materialised
├── boot/10-flow-wal.md              # canonical, always materialised
├── spec/flows/wal/
│   ├── WAL-PROTOCOL.md              # canonical, always materialised
│   └── morning-routine.md           # canonical, always materialised
└── subskills/
    ├── stack/rust/
    │   ├── vibe-subskill.toml       # the subskill manifest
    │   ├── README.md
    │   └── rust-specific-protocol.md
    ├── stack/python/
    │   ├── vibe-subskill.toml
    │   └── python-specific-protocol.md
    ├── feature/atomic-only/
    │   ├── vibe-subskill.toml
    │   └── atomic-commits-addendum.md
    └── llm-coordinator/anthropic/
        ├── vibe-subskill.toml
        └── claude-prompt-templates.md
```

A **subskill** is the smallest activatable content unit inside a package. Structurally it looks like a tiny package: own manifest, own files, own optional further subskill children (§2.5.5). What changes per subskill is the **delivery mode** (§2.5.0 below) and the **activation rules** (§2.5.2): together they decide when the subskill's content reaches the agent and how.

#### 2.5.0 Three delivery modes — eager, lazy-push, lazy-pull {#delivery-modes}

A subskill's `delivery` field is the **primary axis** of the manifest, not a follow-up bolt-on. It picks how the subskill's content reaches the agent. Three values, each well-defined:

- **`eager`** (default — the only mode in revision r1). Once activation matches, the subskill's files materialise into the project tree under `spec/...` at install time. They stay on disk until uninstall. Every agent session that opens the project sees them — analogous to PROP-002 §2.5 base-package behaviour. **Use for** rules-of-the-house content that should always be visible: foundational protocols, boot snippets, mandatory disciplines.
- **`lazy-push`**. Files are *not* materialised at install time. Instead, when an agent connects via the `vibe-mcp` server (M1.7), and the agent's current task description matches the subskill's natural-language `description` field (§2.5.1), `vibe-mcp` materialises the files **into the agent's current MCP context** — pushed to the agent on its behalf without disk-side cache. The push leaves no on-disk artefact unless `--persist` is passed. **Use for** workflow guidance (procedural skills) that's relevant only sometimes, and only on disk for the duration of one agent task.
- **`lazy-pull`**. Files never materialise except on explicit agent request through `vibe-mcp`'s `read_subskill(package, path)` tool. The agent decides when to consult them; the user never sees them in `spec/...`. **Use for** library-knowledge documentation: API references, framework deep-dives, edge-case catalogs that an agent should be able to query but that would bloat the project tree if eager-materialised.

The three modes mirror Tessl's "rules eager-push / skills lazy-push / docs lazy-pull" framing (research at [PROP-004 §2.10](../../research/PROP-004-tessl-comparative-research.md#delivery-modes)) — with the difference that vibevm makes the mode a **per-subskill choice** rather than a per-content-type one. A single package can ship eager rules + lazy-push workflows + lazy-pull deep references and the consumer sees each at the right moment.

**Lockfile impact.** Lockfile schema v3 (§2.9) records the resolved delivery mode per active subskill so the materialisation behaviour is reproducible across machines.

**Why all three modes need to exist in the schema from day one.** If we ship `eager` only and add `lazy-push` / `lazy-pull` later, every existing subskill manifest needs a default-mode declaration retroactively, and every existing lockfile needs the per-subskill-mode field. Pre-release schema churn is free; post-release it costs migrations. We pay the cost once, here.

#### 2.5.1 Subskill manifest (`vibe-subskill.toml`) {#subskill-manifest}

```toml
[subskill]
path = "stack/rust"                   # canonical addressable name within parent package
summary = "Rust-specific guidance for the WAL flow"

# Natural-language activation trigger — load-bearing for `lazy-push` mode
# and `vibe-mcp` exposure. Required for delivery = "lazy-push" /
# "lazy-pull"; recommended for delivery = "eager" so `vibe-mcp` /
# `vibe show subskills` can describe what the subskill is for. Style:
# "When you ...". Specificity beats verbosity — the agent matches this
# against task / files / conversation, so vague triggers ("about Rust")
# trip on every Rust-adjacent task; concrete triggers ("when adding a
# new WAL checkpoint to a Rust project that uses sqlx for storage")
# only fire when the situation actually applies. `vibe review` scores
# this string under the "activation" axis.
description = """
When you are adding or modifying WAL checkpoints in a Rust project,
especially when using sqlx, diesel, or similar SQL libraries, and need
the Rust-specific naming, error-handling, and trace-id conventions
that complement the canonical WAL protocol.
"""

# How this subskill reaches the agent. See §2.5.0.
delivery = "lazy-push"   # one of: "eager", "lazy-push", "lazy-pull"

# Optional: pin this subskill to an upstream OSS package version. PURL
# syntax (https://github.com/package-url/purl-spec). Set when the
# content is genuinely version-specific to the upstream library —
# e.g. a Rust-stack subskill that documents a sqlx 0.8 API. See
# §2.5.6.
describes = "pkg:cargo/sqlx@0.8.0"

# Activation rules — any one matches → subskill is active.
# Channels described in §2.5.2.
[activation]
# Manual: parent package's [features] map a feature name to this path.
# (No declaration needed here — the parent's [features] table holds
# the linkage. Stated for documentation only.)

# Context-based: capabilities/interfaces/files/commands/env/PURL match.
context.if_present = ["stack:rust"]
context.if_provides = ["interface/build-system"]
context.if_files = ["**/Cargo.toml"]
context.if_command = ["cargo"]
context.if_env = ["RUST_LOG"]
context.if_os = ["linux", "macos"]           # OS scope — the same probe
                                              # the [boot_snippet] `when`
                                              # gate ships (PROP-009 §2.4).
context.if_describes_match = true            # match if any package in
                                              # the graph `describes` an
                                              # upstream PURL whose `type`
                                              # equals this subskill's
                                              # `describes` type
                                              # (e.g. pkg:cargo/*).
context.if_language = ["en", "ru"]

# LLM-inferred activation: the post-M1.5 LLM build pipeline can emit
# *virtual* capabilities into the dep graph (§2.5.3), which then
# activate this subskill through the normal `if_present` /
# `if_provides` channels. Set `context.allow_llm_emission = false` to
# refuse virtual capabilities for this subskill specifically (default
# true — opt-out, not opt-in).
context.allow_llm_emission = true

# Soft-preference: if activated alongside any of these, prefer to also
# activate them (libsolv-Recommends-style).
[recommends]
subskills = ["feature/atomic-only"]

# Hard exclusion: never activate alongside any of these.
[conflicts]
subskills = ["stack/python"]

# Files this subskill ships, relative to its own root.
[content]
files_written = [
    "spec/flows/wal/rust-specific-protocol.md",
    "spec/boot/15-flow-wal-rust.md",        # boot-snippet prefix MUST not collide
                                            # with anything else in scope; vibe-check
                                            # gates this at install time.
]
```

The manifest is intentionally a strict subset of `vibe-package.toml` — same TOML idioms, same fields where applicable, same `deny_unknown_fields` discipline. New per-subskill fields (revision r2): `delivery`, `description`, `describes`, plus the expanded `[activation].context.*` probes.

**`description` is required for `delivery = "lazy-push"` and `lazy-pull`.** The activation trigger is the entire mechanism for those modes — without it, `vibe-mcp` has nothing to match against. `eager` mode also benefits but is not required. `vibe check` errors out (not warns) on a lazy-push subskill missing `description`.

**`description` length policy.** No hard limit, but `vibe review` activation-axis scoring penalises descriptions over ~600 characters as "vague when long" — Tessl's empirical finding (their skills cap at 500 lines for the body and ~5 lines for the description). Authors are expected to be precise.

#### 2.5.2 Subskill activation channels — broader probe surface {#subskill-activation}

A subskill becomes "active" if any one of these channels matches. Channels compose orthogonally; an active subskill activates once regardless of how many matched. The full set, more comprehensive than revision r1:

- **Manual via parent feature.** The parent's `[features]` table includes the subskill in some feature's activation list (`rust-stack = ["subskill:stack/rust"]`); the feature is active; the subskill activates.
- **Context-based: `if_present`.** Activates if the project's effective dep graph contains a named package or capability (`stack:rust`, `capability:wal-protocol`).
- **Context-based: `if_provides`.** Activates if any package in the graph declares `[provides]` matching the named interface tag (`interface:build-system`). Strictly more general than `if_present` (producer can be anyone fulfilling the role).
- **Context-based: `if_files`.** New in r2. Activates if the project tree matches one of the supplied glob patterns (`**/Cargo.toml`, `package.json`, `requirements.txt`). This is what `tessl init` infers implicitly when it auto-detects "you are in a Rust project, you are in a Python project" — vibevm makes the probe explicit and per-subskill.
- **Context-based: `if_command`.** New in r2. Activates if any of the listed commands resolve on the user's `PATH` (`cargo`, `python3`, `pnpm`). This is a **machine-state** trigger, distinct from project-state triggers — useful for tooling subskills that document a CLI the agent might shell out to.
- **Context-based: `if_env`.** New in r2. Activates if any of the listed env-vars are set (`RUST_LOG`, `CI`, `KUBECONFIG`). Useful for environment-specific guidance ("you're in CI, here's the CI-specific gotchas subskill").
- **Context-based: `if_os`.** Activates if the session's operating system is in the listed set — `windows`, `macos`, `linux` (`std::env::consts::OS` names). A **machine-state** trigger, alongside `if_command` / `if_env`. This is the same OS probe the `[boot_snippet]` `when` gate ships end-to-end (PROP-009 §2.4 / §2.6) — one OS grammar across both mechanisms. On the subskill side it is reserved in the schema for forward compatibility, inert until the activation engine is built.
- **Context-based: `if_describes_match`.** New in r2. Activates if any package in the graph (or the consumer project itself) declares `describes` with a PURL whose `type` matches this subskill's `describes` type. This is the bridge between the project's "I document FastAPI 0.116" PURL and a subskill's "I'm the Rust-specific cut of FastAPI guidance."
- **Context-based: `if_language`.** Activates if the consumer's resolved language preference (§2.7.3) is in the listed set. Carried over from r1.
- **LLM-emitted virtual capability.** New shape in r2 (replacing r1's "LLM-inferred" channel — see §2.5.3 below). Equivalent expressive power, single audit point.

Each channel is **opt-in per subskill** — silence on a probe means "this probe doesn't fire for this subskill." `[activation]` with no probes at all means the subskill activates only manually (via parent feature).

#### 2.5.3 LLM-emitted virtual capabilities (post-M1.5) — refactor of "LLM-inferred activation" {#llm-virtual-caps}

Revision r1 introduced "LLM-inferred activation": during `vibe build`, the LLM was given a list of inactive subskills and could pick which to turn on. This created an ad-hoc imperative side-effect with no audit trail and no way for the dep graph to observe the LLM's reasoning.

Revision r2 reformulates the channel as **virtual capability emission**:

1. During `vibe build`, after the static SAT solve and the post-pass static activation rules have run, the LLM is invoked with a prompt summarising the project's effective spec, the target feat, and the project's surrounding context (recent commits, file tree, env).
2. The LLM is asked: "What capabilities and interfaces, beyond those statically declared, should be considered present in this project for the purpose of context selection?"
3. The LLM responds with a list of **virtual capabilities** — `capability:claude-coordinator`, `interface:build-system`, `language:russian-comments`, etc. Each is a string in the same namespace as static capabilities/interfaces.
4. The virtual capabilities are added to the resolved graph as if a synthetic package emitted them. Normal activation channels (`if_present` / `if_provides`) handle the actual subskill toggle.
5. The lockfile's `[meta]` block records which virtual capabilities the LLM emitted plus the prompt-and-response trace ID. `vibe show effective` displays virtually-emitted capabilities with a `[virtual via LLM]` annotation. The audit trail is per-resolution; reproducing a run reproduces the trace.

The expressive power is identical: anything r1's "LLM picks subskill X" did, r2's "LLM emits capability Y → subskill X activates via `if_present = [Y]`" does. The differences are operationally meaningful:

- **Single audit point.** Every LLM contribution to activation passes through one boundary (capability emission), not scattered across N subskill toggles.
- **Static rules win.** Manually-declared `if_present` rules still apply uniformly to virtual emissions; the consumer can declare `[[overrides]] reject_virtual_capability = "language:..."` to shut off entire LLM-emitted dimensions if needed.
- **Transparency at the spec layer.** A virtual capability is a spec-layer object — operator can write `[provides]` for it, `[requires]` against it, see it in `vibe show config`. r1's LLM-inferred activation never crossed back into the spec ontology.
- **Per-subskill opt-out is unnecessary at the channel level.** Subskills that don't want LLM-emitted activation simply don't use `if_present` against the namespace the LLM is allowed to emit into. Project-level policy is `[llm].emission.allowed_namespaces = ["capability:*", "interface:*"]` (default — generous; restrict by namespace if security-sensitive).

#### 2.5.4 Why subskills, not just more packages {#subskill-rationale}

The same end-state could be achieved by splitting `flow:wal` into `flow:wal-base`, `flow:wal-rust`, `flow:wal-python`, etc. Two reasons we don't:

1. **Cohesion.** The Rust-specific notes belong *inside* the `flow:wal` package as a unit — author-time, the same person writes them, they reference each other across the boundary, they ship as a single tag `v0.1.0`. Splitting forces the author to coordinate version numbers across N repos.
2. **Discovery surface.** A registry browser sees one `flow:wal` and walks its subskills; with N split packages it sees a flood of micro-entries that don't communicate "these are different cuts of the same flow." This matters as soon as the registry has more than ~10 packages.

Cargo solves this through `[features]` in a single crate — vibevm goes one step further because the unit ("a feature") and the activated content unit (some files, structure preserved) are not the same object in vibevm. Hence the explicit `subskill` model. **Tessl** ships only flat skills with no subdivision (research at [PROP-004 §5.3](../../research/PROP-004-tessl-comparative-research.md)) — that's a meaningful gap they'll need to close once their registry exceeds atomic-skill complexity.

#### 2.5.5 Recursive subskills {#subskill-recursion}

A subskill may itself carry a `subskills/` directory; activation rules apply recursively. Practical limit: depth ≤ 3 (anything deeper is almost certainly a smell — the package should be split). `vibe check` warns at depth 4. Each nested subskill carries its own `delivery` mode independently of its parent; an `eager` parent can have a `lazy-pull` deep reference subskill nested inside.

#### 2.5.6 `describes` PURL on subskills {#subskill-describes}

Per §2.7 of [PROP-004](../../research/PROP-004-tessl-comparative-research.md#purl), Tessl's headline marketing claim — version-matched documentation — rides on the `describes` field at the tile level. vibevm goes one step further: the field is available **on subskills as well as packages**. A `flow:wal` package as a whole may not bind to any one library, but its `subskills/stack/rust/` cut binds specifically to `pkg:cargo/sqlx@0.8.0`; another `subskills/stack/rust-diesel/` cut binds to `pkg:cargo/diesel@2.x`. The two coexist in the same package, and the activation channel `context.if_describes_match` selects the right one for the consumer's actual library version.

**Format.** Standard Package URL spec ([`https://github.com/package-url/purl-spec`](https://github.com/package-url/purl-spec)) — `pkg:<type>/<namespace>/<name>@<version>` or `pkg:<type>/<name>@<version>` for unscoped. `<version>` may be a SemVer requirement (`^0.8`) rather than an exact version when the subskill applies to a range.

**Lockfile impact.** When a subskill activates via `if_describes_match`, the lockfile records both the subskill's `describes` PURL and the matched in-graph PURL. Drift detection (M1.9 + M1.10) then cross-references: when the consumer upgrades sqlx 0.8 → 0.9, `vibe outdated --upstream` flags subskills whose `describes` no longer matches.

### 2.6 Capability-based interface tags — the abstract layer {#interface-tags}

**Decision.** Extend PROP-002 §2.9's `[provides]` / `[requires]` capability surface with a new concept: **interface tags**.

```toml
[provides]
# concrete package identity (existing)
flow:wal = "0.1.0"
# capability provided (existing)
"capability:wal-protocol" = "*"
# NEW: interface tag — abstract role this package fills
"interface:build-system" = "*"
"interface:auth-provider" = "*"
```

```toml
[requires]
# require a concrete package or capability (existing)
flow:wal = "^0.1"
# NEW: require some package that fills an interface, regardless of name
"interface:build-system" = "*"
```

Interface tags differ from capabilities in two ways:

1. **Abstraction over name.** A package requiring `interface:build-system` doesn't care whether the consumer has `stack:rust-cargo`, `stack:python-poetry`, or `stack:nix-flake`; any of them with `[provides]` matching the interface satisfies. Capabilities (`capability:wal-protocol`) tend to be more specific and authored together.
2. **Discovery surface.** Subskills can `context.if_provides` against an interface to auto-activate when the consumer happens to have *any* package fulfilling the role. Capabilities are matched against `[requires]` only.

**Naming convention.** Interface tags use the `interface:<name>` namespace. The `<name>` segment uses `-` for word boundaries (kebab-case), `/` for category nesting (`interface:storage/sql`, `interface:storage/key-value`). Solver treats them as opaque strings; no semantic meaning beyond match/no-match.

**Provenance.** Both `[provides]` and `[requires]` interface tags are user-authored (no LLM inference). The author is making an intentional declaration about an architectural role; that's not a thing the LLM should be guessing at. Note: §2.5.3 introduces *virtual capabilities* the LLM may emit at runtime (post-M1.5) — those flow through a different channel and never persist into the package manifest.

### 2.6.1 Conditional dependencies — `[target."context(...)".dependencies]` {#conditional-deps}

**Decision.** Beyond subskill activation (which controls *content within an active package*), vibevm gains conditional dependencies (which control *whether a package is in the graph at all*), modelled on Cargo's `[target."cfg(...)".dependencies]` shape but predicated on vibevm's context probes rather than rustc target triples:

```toml
[target."context(stack:rust)".dependencies]
flow:rust-best-practices = "^0.1"
flow:cargo-discipline = "^0.1"

[target."context(if_files = '**/Dockerfile')".dependencies]
flow:container-best-practices = "^0.1"

[target."context(if_provides = 'interface:database-migrations')".dependencies]
flow:migration-discipline = "^0.1"
```

The `context(...)` predicate accepts the same `if_present` / `if_provides` / `if_files` / `if_command` / `if_env` / `if_describes_match` / `if_language` probes from §2.5.2, plus boolean composition (`and`, `or`, `not`).

**When to use which.** Subskills are *content shaped to context* — files inside a package. Conditional deps are *packages shaped to context* — entire packages added to the graph or not. Choose subskills when the content lives naturally inside an existing package; choose conditional deps when bringing in a separately-versioned, separately-authored package makes more sense.

**Solver impact.** Conditional deps are evaluated **after** the static SAT solve has run on unconditional deps (otherwise the solver doesn't know which probes will fire). The flow: solve unconditional → evaluate `[target.<...>.dependencies]` predicates → add new requirements → re-solve. Convergence guaranteed in finite steps because each pass only adds requirements, never relaxes them; libsolv handles the incremental rule addition cleanly.

**Cargo's resolution-stability lesson.** Cargo's `cfg`-based conditional deps were originally per-target evaluated at solve time, which produced different lockfiles per host triple. vibevm's `context(...)` is evaluated against the **resolved project state**, not host state — so the lockfile is host-invariant for the same project state. Build-host machine differences (e.g. `cargo` available or not) are explicitly out of scope; if the user wants those, they declare project-level capabilities.

### 2.7 Internationalization (i18n) — multi-language package content {#i18n}

**Decision.** Adopt a **sidecar file naming pattern** with **BCP-47 language tags** as suffixes, plus first-class language-preference declarations at three levels (CLI flag, project manifest, package manifest).

#### 2.7.1 File naming — the chosen pattern {#i18n-naming}

```
flow-wal/
├── README.md                        # canonical (default language: en)
├── README.ru.md                     # Russian translation
├── README.ja.md                     # Japanese translation
├── boot/
│   ├── 10-flow-wal.md               # canonical
│   └── 10-flow-wal.ru.md            # Russian
└── spec/flows/wal/
    ├── WAL-PROTOCOL.md
    ├── WAL-PROTOCOL.ru.md
    └── morning-routine.md           # only canonical — no translation yet, fallback used
```

A localised file is the canonical filename with a `.<lang>` segment inserted before the extension. `<lang>` is a [BCP-47](https://datatracker.ietf.org/doc/html/rfc5646) language tag — `en`, `ru`, `ja`, `zh-Hans`, `pt-BR`. We also accept short ISO-639-1 codes alone (`ru`, `ja`) as a convenience; they map to the BCP-47 tag with no region.

**Why sidecar (`README.ru.md`), not directory (`i18n/ru/README.md`) or suffix (`README_RU.md`):**

| Pattern | Pro | Con |
|---|---|---|
| Sidecar `README.ru.md` | filesystem-flat; trivial glob `*.ru.md`; `README.md` keeps original visibility; new languages added in place | one extra dot in filename |
| Directory `i18n/ru/README.md` | clean grouping per language; easy `i18n/<lang>/` cp-r for whole-language operations | doubles directory depth; mirroring the canonical tree under each `i18n/<lang>/` is fragile |
| Suffix `README_RU.md` | shortest visual diff | uppercase code conflicts with UNIX case-insensitive filesystems' case-folding; `_RU` is not a BCP-47 tag; collides with files that happen to end in `_<word>` |
| Inline TOML keys (`title.ru = "..."`) | great for short strings | doesn't scale to a multi-paragraph protocol document |

Sidecar wins on every operationally-relevant axis. It's also the pattern Pandoc, Gettext PO bundles, MDX, and Hugo i18n converge to.

#### 2.7.2 Language preference resolution — fallback chain {#i18n-fallback}

When materialising file `<X>` for the target project's preferred language `<lang>`:

1. **Exact match.** If `<X>.<lang>.<ext>` exists in the package, use it.
2. **Region fallback.** If `<lang>` carries a region (e.g. `pt-BR`), try `<X>.pt.<ext>` next.
3. **Canonical fallback.** Use `<X>.<ext>` (no language suffix; the de-facto canonical form, by convention English in the vibevm registry but nothing prevents a package from declaring its canonical to be Spanish or Mandarin).
4. **Hard error.** If even the canonical form is missing, fail the install with `MissingFile { logical_path }`.

Critical invariant: **every package must ship the canonical form of every file it lists in `[content].files_written`**. Translations are additive. This is what makes step 3 always reachable; it also lets a project install a package that has zero translation coverage for the user's preferred language without seeing errors.

#### 2.7.3 Language preference declarations — three layers {#i18n-prefs}

Same precedence model as PROP-002 §9.5 (CLI flag > env var > project manifest > package manifest > built-in default):

- **CLI flag**: `vibe install flow:wal --language ru` overrides everything else for this invocation.
- **Env var**: `VIBE_LANGUAGE=ru` matches the existing `VIBE_LOG` / `VIBE_REGISTRY_CACHE` env-var conventions.
- **Project `vibe.toml`**:
  ```toml
  [i18n]
  preferred = "ru"
  fallback = ["en"]    # if a package has no `ru`, try `en` before erroring;
                       # default behaviour is the same — explicit form for clarity
  ```
- **Package `vibe-package.toml`**: declares which languages the package itself ships:
  ```toml
  [i18n]
  canonical = "en"           # default; the form filenames-without-suffix carry
  available = ["en", "ru"]   # `ja` is in our fixture above but not declared here:
                             # vibe check would warn that `README.ja.md` is unindexed
  ```
- **Built-in default**: `en`. Hard-coded as the registry-wide canonical fallback so a malformed/empty `[i18n]` block in any layer doesn't paralyse install.

#### 2.7.4 Manifest-field translation (short strings) {#i18n-fields}

For short translatable strings inside `vibe-package.toml` itself (`description`, `summary`, `[features.<name>].description`), we adopt **dotted-key translations**:

```toml
[package]
name = "wal"
kind = "flow"
version = "0.1.0"
description = "Append-only checkpoint protocol"
description.ru = "Протокол append-only-чекпоинтов"
description.ja = "追記専用チェックポイント・プロトコル"

[features.rust-stack]
description = "Rust-specific guidance for WAL"
description.ru = "Руководство по WAL для проектов на Rust"
```

This is the syntax TOML 1.0 already supports (`description` is a string and `description.ru` is a key inside an inline `description` table — no, *that's wrong*: TOML disallows mixing a bare string and a table at the same key). Real TOML representation:

```toml
description = { en = "Append-only checkpoint protocol", ru = "Протокол ..." }
```

Or, more readably:

```toml
[package.description]
en = "Append-only checkpoint protocol"
ru = "Протокол append-only-чекпоинтов"
ja = "追記専用チェックポイント・プロトコル"
```

The parser accepts either form: a bare string `description = "..."` is interpreted as `{ en = "..." }`. Lookup walks the same fallback chain as files (§2.7.2). This is backward-compatible with all existing manifests in fixtures and on GitHub today (they use the bare-string form, which auto-promotes to `{en = "..."}`).

#### 2.7.5 Lockfile impact {#i18n-lockfile}

The lockfile records the **resolved language preference** under `[meta]` so a re-install on a different machine without an explicit flag produces the same materialised files:

```toml
[meta]
schema_version = 2
language = "ru"
language_fallback = ["en"]
```

`language_fallback` is the post-resolution chain, with built-in `en` appended if absent. Clearing this metadata (e.g. a checked-in lockfile from a teammate using `ru` when the current operator wants the canonical form) requires explicit `vibe update --language en` or hand-editing.

### 2.8 Manifest schema additions — the consolidated picture {#manifest}

Pulling together every section above, `vibe-package.toml` v0.2 schema looks like:

```toml
[package]
name = "wal"
kind = "flow"
version = "0.1.0"
description = { en = "Append-only checkpoint protocol", ru = "Протокол ..." }

[i18n]
canonical = "en"
available = ["en", "ru"]

[provides]
flow:wal = "0.1.0"
"capability:wal-protocol" = "*"
"interface:checkpointing" = "*"

[requires]
"interface:build-system" = "*"

[[requires_any]]
one_of = [
    { "stack:rust-cargo" = "^0.1" },
    { "stack:python-poetry" = "^0.1" },
]

[recommends]
flow:atomic-commits = "^0.1"

[suggests]
flow:sync-from-code = "^0.1"

[supplements]
"capability:claude-coordinator" = "*"

[enhances]
"capability:llm-build-pipeline" = "*"

[obsoletes]
flow:wal-legacy = "<0.1"

[conflicts]
flow:wal-experimental = "*"

[features]
default = ["wal-protocol"]
wal-protocol = []
atomic-commits-section = ["dep:flow-atomic-commits"]
rust-stack = ["subskill:stack/rust"]
python-stack = ["subskill:stack/python"]
__internal-helper = []          # underscore-prefixed: implementation detail

__exclusive = [["rust-stack", "python-stack"]]

[content]
files_written = [
    "spec/flows/wal/WAL-PROTOCOL.md",
    "spec/flows/wal/morning-routine.md",
    "spec/boot/10-flow-wal.md",
]
```

Each subskill carries its own `vibe-subskill.toml` per §2.5.1.

`deny_unknown_fields` everywhere — vibevm never silently drops unfamiliar manifest keys; we'd rather fail loud and add the section to the schema than corrupt provenance.

### 2.9 Lockfile schema impact (v3) {#lockfile-v3}

The lockfile gains:

- `[meta].language` and `[meta].language_fallback` (§2.7.5).
- `[meta].active_features = [...]` — full list of features active in the resolution. Per-package activation goes under each `[[package]]` entry.
- `[meta].virtual_capabilities = [...]` — capabilities emitted by the LLM during resolution (§2.5.3). Each entry carries `name`, `emitter` (the LLM provider/model identifier), `trace_id` (link into the audit log), and `emitted_at`.
- `[[package]]` entries gain `features = ["..."]`, `subskills_active = [...]` (with each entry being `{ path = "stack/rust", delivery = "lazy-push" }` so the materialisation behaviour is reproducible) and the latter's delivery mode persisted because eager / lazy-push / lazy-pull are operationally distinct on the consumer side.
- `[[package]]` entries gain optional `language` field if the package was materialised in a non-canonical language (otherwise inherits `[meta].language`).
- `[[package]]` and per-subskill entries gain optional `describes` PURL when set in the source manifest.

```toml
[meta]
schema_version = 3
solver = "sat"
language = "ru"
language_fallback = ["en"]
active_features = ["flow:wal/wal-protocol", "flow:wal/rust-stack", "flow:atomic-commits/atomic-commits-section"]
root_dependencies = ["flow:wal", "flow:atomic-commits"]

[[package]]
kind = "flow"
name = "wal"
version = "0.1.0"
registry = "vibespecs"
source_url = "https://github.com/vibespecs/flow-wal.git"
source_ref = "v0.1.0"
content_hash = "sha256:8136..."
features = ["wal-protocol", "rust-stack"]
subskills_active = ["stack/rust"]
language = "ru"
boot_snippet = "10-flow-wal.md"
files_written = [
    "spec/flows/wal/WAL-PROTOCOL.md",      # written from WAL-PROTOCOL.ru.md (or canonical fallback)
    "spec/flows/wal/morning-routine.md",
    "spec/flows/wal/rust-specific-protocol.md",   # from subskills/stack/rust/
    "spec/boot/10-flow-wal.md",
    "spec/boot/15-flow-wal-rust.md",              # from subskill
]
```

`schema_version = 3` triggers the v2 → v3 read-side migration on next `vibe install`. Schema-write side is unconditional v3 once this PROP lands — pre-release, no migration burden.

### 2.10 CLI surface — additions and adjustments {#cli}

New flags / commands:

- `vibe install <pkgref> [--features <a,b,c>] [--no-default-features] [--all-features]` — control feature activation (cargo-shape).
- `vibe install <pkgref> [--language <bcp47>]` — override resolved language for this install.
- `vibe show features <pkgref>` — list the package's features, default state, current activation in the project.
- `vibe show subskills <pkgref>` — list the package's subskills, activation state with reason ("active because feature `rust-stack`", "active because `stack:rust` is in the project", "available but not active", "would-activate-if-LLM-build" — post-M1.5).
- `vibe check`'s existing checks gain four new entries (numbered per VIBEVM-SPEC §12 expansion):
  - **i18n coverage**: every file declared in `[content].files_written` exists for the package's canonical language; missing translations warn (not error) per language declared in `[i18n].available`.
  - **subskill structure**: subskill manifests parse, activation rules are valid (`if_present` references exist, `if_provides` interface tags are well-formed, `if_files` glob patterns parse, `delivery` is one of the three allowed values, `description` is present when `delivery` ∈ {`lazy-push`, `lazy-pull`}).
  - **feature graph**: feature activations don't form cycles, exclusion sets are not violated by `default`, every `subskill:<path>` reference resolves to a real subskill in the package.
  - **activation conflict**: subskill `description` triggers don't materially overlap. Detection runs an LLM-judge (when available) or a heuristic substring-overlap pass (in `vibe check`'s static mode) over every pair of subskills with `delivery ∈ {lazy-push, lazy-pull}` in the same package, flagging pairs whose triggers contain ≥75% of each other's content keywords. Mirrors Tessl's review-rubric "activation distinctiveness" axis (research at [PROP-004 §2.11](../../research/PROP-004-tessl-comparative-research.md#review-rubric)). Authors are expected to either tighten one description or merge the two subskills.

Existing flags pick up new behaviours:

- `vibe update --features <list>` — re-resolve with a different feature set.
- `vibe show config` exposes the resolved language preference and its provenance per the existing precedence chain.
- `vibe show effective` materialises the effective spec at the project's resolved language, falling back per §2.7.2; `--all-languages` shows every available language side-by-side (debugging aid).

### 2.11 Migration path from `NaiveDepSolver` {#migration}

The codebase has no shipped users; migration is internal. Order:

1. **Land libsolv FFI** (`crates/vibe-resolver-libsolv`) and `SatDepSolver` impl behind a trait. Naive stays the default.
2. **Land manifest schema additions** (§2.8) without runtime activation logic — parser-only. Existing manifests parse unchanged.
3. **Land features semantics** in `SatDepSolver` (rule encoding, solving, activation map); `vibe install --features` and `--no-default-features` start working. Naive remains feature-blind.
4. **Land subskill materialisation** in `vibe-install`: walk activation rules post-solve, write subskill files, integrity-check (boot collision, file collision).
5. **Land i18n resolution** in `vibe-install`: at file-write time, walk the language fallback chain. CLI flag wired.
6. **Switch default solver** to `sat` in `vibe init` output and runtime resolution. Naive remains for fixtures/tests.
7. **Lockfile v3 migration** on read; unconditional v3 write.

Each step is its own PR, lockfile-shape-stable mid-step (we control the format pre-release; if a step needs to break, we break and don't carry compatibility).

## 3. SAT solver algorithm details {#algorithm}

### 3.1 Rule encoding — vibevm concepts → libsolv rules {#rule-encoding}

libsolv's solver is rule-based: every constraint becomes a clause in the SAT problem. We map our concepts:

| vibevm concept | libsolv rule |
|---|---|
| `requires X = "^1.2"` | `RULE_PKG_REQUIRES`: ¬this ∨ matching_X |
| `requires_any [{X, Y}]` | `RULE_PKG_REQUIRES`: ¬this ∨ X_or_Y (a synthetic literal expanded into actual choices via auxiliary clauses) |
| `provides cap:foo` | identity rule: this ⇒ "cap:foo" virtual literal asserted |
| `obsoletes X = "<2.0"` | `RULE_RPM_OBSOLETES`: ¬this ∨ ¬X<2 |
| `conflicts X` | `RULE_PKG_CONFLICTS`: ¬this ∨ ¬X |
| `recommends X` | `RULE_RECOMMENDS` (weak): solver tries to include X but skip on conflict |
| `suggests X` | `RULE_SUGGESTS` (informational only) |
| `supplements cap:foo` | `RULE_SUPPLEMENTS`: if cap:foo activated by some other package, prefer this |
| `enhances cap:foo` | `RULE_ENHANCES` (informational) |
| `__exclusive [[A, B]]` | `RULE_PKG_CONFLICTS` × pairs: ¬A ∨ ¬B |
| feature `f = ["dep:X"]` | activating f ⇒ requires X (conditional on f literal) |
| feature `f = ["X?/g"]` | weak: activating f ∧ X-already-in-graph ⇒ X has feature g |
| subskill activation by `if_present` | post-SAT pass: scan resolved graph, set subskill literals based on present capabilities (no SAT involvement; pure projection) |
| subskill activation by `if_provides` | same as `if_present`; interface tags are queried in the same scan |

This is the same encoding `dnf5` uses for RPM weak-deps; we just reuse the `RULE_*` constants from libsolv's public `solv/solver.h`.

### 3.2 Solver phases {#solver-phases}

For one `vibe install` invocation:

1. **Pool population.** Walk `MultiRegistryResolver::list_versions` for every root and transitively-discovered package. Each `(kind, name, version)` becomes a libsolv solvable; capabilities/interfaces become provides relations; deps become requires relations. Known-version cache from PROP-002 §2.4 cuts repeat lookups.
2. **Job formulation.** Each root `pkgref` becomes a `SOLVER_INSTALL | SOLVER_SOLVABLE_NAME` job with version constraint. Active features become enabling literals on root solvables.
3. **Rule materialisation.** Encode every constraint above as libsolv rules.
4. **Solve.** `solver_solve()` runs CDCL. On unsat, walk problems with `solver_problem_count()` → `solver_findproblemrule()` → reconstruct vibevm-shape `SolveError::Unsatisfiable { problems: Vec<Problem> }` for the CLI to render.
5. **Decoding.** Walk `solver_get_decisionqueue()` to extract the chosen solvables, decode back into `ResolvedNode`s with the version + features picked.
6. **Subskill projection.** For each resolved package, evaluate its subskills' `context.if_present` and `if_provides` rules against the full graph. Set activation flags. (No SAT round-trip; this is a deterministic post-pass.)
7. **Materialisation handoff.** `vibe-install` reads the final node list (with feature + subskill flags) and walks the i18n fallback at file-write time.

### 3.3 Determinism {#determinism}

libsolv's solver is **deterministic** for a fixed pool, fixed jobs, fixed rules — it does not use randomness or wall-clock-driven heuristics. Two solves with the same inputs produce the same result. This is a property we explicitly rely on for `cargo xtask check-codegen`-style drift checks: `vibe check --simulate-install <pkgref>` should produce a stable hash per (manifest, lockfile) pair. We test this via a fixture-driven integration test that runs the same install N times and asserts identical lockfile bytes.

### 3.4 Performance envelope {#perf}

libsolv is the engine YUM and DNF have used for ~15 years against repos with 50K+ packages and conflict-rich constraint sets. vibevm's expected scale (hundreds of packages, tens of features per package, depth-3 graphs typical) is comfortably within the linear regime. We don't anticipate performance pressure from libsolv; we anticipate it from the registry-fetch layer (network bound), which is unchanged.

The Rust FFI cost is negligible — one `solver_solve()` call per `vibe install` invocation; everything else is in-process.

## 4. Rejected alternatives {#rejected}

These were considered and explicitly *not* taken. Documented so the next reader doesn't re-derive.

### 4.1 `resolvo` as primary SAT engine

Already covered in §2.2. Re-stated for completeness: pure-Rust appeal is real, but libsolv's battle-testing, weak-deps semantics, and rule-introspection wins for v1. resolvo remains a viable second impl.

### 4.2 Pubgrub as primary SAT engine

Pubgrub's contribution (incremental version solving with native disjunction explanation) is genuinely best-in-class for SemVer-shaped constraints — Cargo's adoption is the existence proof. But pubgrub's data model assumes constraints are version ranges over named packages; capability/interface/virtual-package shapes need to be encoded as synthetic packages, and pubgrub's explanation hooks degrade when synthetic packages dominate the unsat core. Once the encoding is shoehorned in, the explanation quality slips below libsolv's. Door left open via `DepSolver` trait.

### 4.3 Composite content via packages-only (no subskills)

i.e. split `flow:wal` into N packages instead of one with subskills. Already covered in §2.5.3. Discoverability + cohesion losses outweigh the schema simplicity.

### 4.4 `_<lang>` filename suffix instead of `.<lang>`

Already covered in §2.7.1. Case-folding bugs and BCP-47 incompatibility are dealbreakers.

### 4.5 Whole-language directory pattern (`i18n/<lang>/<canonical-tree>`)

Already covered in §2.7.1. Path-depth doubling and filesystem-watch fragility are real costs; the operational wins of sidecar-per-file outweigh the grouping benefit of per-language trees. A package can still have a per-language directory under `subskills/` (e.g. `subskills/i18n/ru-extras/`) if it wants to ship language-specific *content* (not translation) — but that's subskills, not the i18n mechanism.

### 4.6 LLM-driven inference of `[provides]` / `[requires]` / interfaces

Tempting: the LLM reads the package and infers what it provides. We don't do it. `[provides]` is an architectural commitment and authorship matters — having the author intentionally declare interfaces is the only way the system stays auditable. The LLM channel is reserved for *activation* (which subskills to materialise) where the author has already declared the option space.

### 4.7 Inline-key i18n for entire markdown files

`description = { en = "...", ru = "..." }` works for short strings but doesn't scale to multi-paragraph documents (TOML strings without escapes are awkward; multiline literal strings disrupt the toml-edit roundtrip). Sidecar files are the right unit at the document level.

### 4.8 Multiple solvers concurrently selectable per-package

A package declaring `[meta] solver = "naive"` for itself while the rest of the graph uses `sat` — rejected. Single-solver-per-resolution keeps semantics consistent.

## 5. Out of scope for this PROP {#out-of-scope}

- **Module / stream concept** (à la dnf5 modules). Subskills cover the same use cases at a finer grain; modules are RPM-distribution-shape baggage we don't need.
- **Non-SemVer version schemes.** vibevm stays SemVer-only.
- **RPM-style boolean rich deps** (`Requires: (A or B)`). `[[requires_any]]` covers the 99 % case; if adoption pulls richer logic in, it lands as a follow-up PROP.
- **Federated registry trust models** (signing, attestation). Tracked under PROP-002 §6 / future PROP-004.
- **Translation tooling pipelines.** Authors hand-write or LLM-assist their translations; vibevm just resolves and materialises.

## 6. Phases / staging plan {#phases}

Concretely scoped slices, each shippable independently:

### Phase A — solver swap with no semantic change

- libsolv FFI crate.
- `SatDepSolver` impl matching `NaiveDepSolver`'s output on all fixture graphs.
- Naive stays default; `--solver sat` opt-in.
- Acceptance: every existing test passes with `--solver sat` *and* `--solver naive`.

### Phase B — features (cargo subset)

- Manifest schema for `[features]`.
- Feature activation in solver, lockfile v3 records.
- `--features` / `--no-default-features` / `--all-features` CLI flags.
- Acceptance: a feat that depends on `flow:wal` with `--features rust-stack` materialises the rust-specific files; without the flag, those files are absent.

### Phase C — subskills (manual + context-based + three delivery modes)

- Subskill manifest format, package layout convention.
- Manifest fields: `delivery`, `description`, `describes`, full `[activation]` probe set (`if_present` / `if_provides` / `if_files` / `if_command` / `if_env` / `if_describes_match` / `if_language`).
- Manual feature → subskill mapping (Phase B's hooks).
- Context-based activation post-pass: probes evaluated against the resolved graph + project tree + machine state.
- Eager-mode materialisation in `vibe-install`. Lazy-push / lazy-pull modes plumb through but require `vibe-mcp` (M1.7) to be useful — when M1.7 hasn't landed, the modes degrade to `eager` with a `tracing::warn!` so the consumer is never silently broken.
- `vibe show subskills` CLI.
- Acceptance: a package with `subskills/stack/rust/` (delivery=eager) activates under a `stack:rust` project without explicit user opt-in. A package with `subskills/sqlx-0.8/` (delivery=lazy-push, describes=`pkg:cargo/sqlx@0.8.0`) does NOT materialise files but registers itself for `vibe-mcp` exposure once M1.7 is in. Activation-conflict check fires when two lazy-push subskills' descriptions overlap.

### Phase D — i18n

- BCP-47 sidecar file resolution.
- Manifest field translations (dotted-key form).
- `--language` flag, `[i18n]` blocks at project/package level.
- Lockfile v3 records resolved language.
- Acceptance: `vibe install flow:wal --language ru` against a package shipping Russian translations writes the Russian files; `--language en` writes the canonical; `--language ja` (no translation in this package) writes the canonical with a `tracing::info!` "language ja not available, using en fallback".

### Phase E — switch default solver to SAT

- Default flips to `sat` in `vibe init`-generated lockfiles.
- Naive demoted to "tests + small graphs" path.
- Acceptance: clean runs of every smoke (M1.5-gate-v2, M1.6-mirror-vendor, plus new feature/subskill smokes) green on fresh install.

### Phase F — LLM-emitted virtual capabilities (post-M1.5)

- Wire into the `vibe build` LLM tool-use loop.
- LLM is invoked with the effective spec + project context + a request to emit virtual capabilities (§2.5.3).
- Subskills with matching `if_present` / `if_provides` rules activate through the normal channel.
- Audit surface: every emission logs `(name, emitter, trace_id, reasoning)` to the lockfile's `[meta].virtual_capabilities`. `vibe show effective` annotates virtually-emitted capabilities with `[virtual via LLM]`.
- Project-level policy: `[llm].emission.allowed_namespaces = ["capability:*", "interface:*"]` (default — generous; restrict by namespace if security-sensitive). Per-subskill `context.allow_llm_emission = false` is the per-subskill opt-out, but most subskills won't need it because they simply don't declare `if_present` against the LLM-emission namespace.
- Acceptance: a `feat:welcome-page × stack:rust` build emits `interface:llm-coordinator` based on detecting an LLM-mediated workflow, which activates the `flow:wal/llm-coordinator/anthropic` subskill via its `if_provides = ["interface:llm-coordinator"]` rule, and the lockfile records the emission with full audit trail.

## 7. Open questions {#open}

- **Mutually-exclusive feature sets across packages.** §2.4's `[features.exclusive]` is intra-package. Should we support cross-package mutual exclusion ("at most one of `stack:rust` or `stack:python` in the same project")? Today this is implicit via `kind`/`name` uniqueness on the resolution side, but a project that pulls both via different transitive paths is not currently rejected. Defer to adoption signal.
- **Subskill versioning.** Today a subskill is part of its parent package's version. Do we ever want subskills with their own SemVer? Probably not — would force the subskill into being its own package. Mark closed.
- **Translation provenance.** Should the lockfile record *which version of a translation* was materialised (translations may evolve faster than canonical)? Open — likely yes, requires schema extension to v4 if pursued.
- **Interface-tag namespacing in the registry.** Today interface tags are global (`interface:build-system` matches across all registries). For multi-tenant federations we may need scoping; defer until multi-registry adoption surfaces real conflicts.
- **Virtual-capability emission rate-limiting.** §2.5.3 leaves the LLM emission unconstrained. In the worst case the LLM could emit hundreds of virtual capabilities per resolution and inflate the activation graph. Open: should the schema cap emissions at N (configurable) and ask the LLM to prioritise? Lean toward `[llm].emission.max = 50` default with override at `vibe build --llm-emission-max=N`.
- **Workspace / monorepo support.** Cargo's `[workspace]` shape lets a repo carry many crates with shared lockfile. vibevm today is one-package-one-repo. With subskills + features, this constraint is workable; without them, monorepos for multi-package collections (`vibespecs/flow-collection` carrying `flow:wal` + `flow:atomic-commits`) become attractive. Defer to adoption.
- **`describes` PURL coverage.** Should `vibe check` warn when a `describes` PURL points at a package version that doesn't exist on the upstream registry (npm, pypi, crates.io)? Requires a probe layer that's opinionated about upstream-host APIs. Park as M2-territory.
- **Activation-conflict detection threshold.** §2.10's check uses 75% keyword-overlap as the threshold for "descriptions materially overlap." Is that the right number? Tessl doesn't publish theirs. Open — instrument and adjust based on real-world false-positive rate once we have a corpus.
- **Feature flags from environment variables.** `VIBE_FEATURES=foo,bar` — useful for CI/automation? Probably yes, mirrors `--features`. Cheap addition to Phase B if pulled.
- **LLM activation transparency to the consumer.** Per-project policy at `[llm].emission.allowed_namespaces` (§2.5.3) covers the bulk case. Should there also be a `[llm].emission.denylist = ["capability:dangerous-pattern"]` for explicit refusal of named capabilities? Lean yes; cheap; probably lands as part of Phase F.

## 8. References {#references}

- Cargo's `[features]` reference: <https://doc.rust-lang.org/cargo/reference/features.html>
- Cargo's feature-resolver-v2 design: <https://doc.rust-lang.org/cargo/reference/resolver.html#feature-resolver-version-2>
- Cargo source: `refs/study/cargo/src/cargo/core/resolver/features.rs`, `refs/study/cargo/src/cargo/core/summary.rs`.
- libsolv canonical: <https://github.com/openSUSE/libsolv> (BSD-3-Clause).
- libsolv internal docs: `doc/libsolv-bindings.txt`, `doc/libsolv-history.txt`, `examples/`.
- DNF5 source (LGPL-2.1, NOT linked — read for design only): `refs/study/dnf5/libdnf5/solv/`, `refs/study/dnf5/dnf5/commands/module/`, `refs/study/dnf5/libdnf5/comps/`.
- DNF legacy (Python 3 generation): `refs/study/dnf/dnf/`.
- BCP-47: <https://datatracker.ietf.org/doc/html/rfc5646>.
- TOML 1.0: <https://toml.io/en/v1.0.0>.
- vibevm's existing depsolver trait: `crates/vibe-resolver/src/lib.rs`.
- vibevm's existing manifest model: `crates/vibe-core/src/manifest/`.

---

*This PROP is a design proposal. Ratification — and the corresponding lockfile schema bump — happens through PR review against this document. Any field added here that doesn't land in the implementation by the end of Phase E is removed from the spec rather than carried as aspirational documentation.*
