# PROP-033 — The refactoring registry: package-contributed, discovered, precompiled operations {#root}

**Status.** Design proposal v0.1 — not implementation-locked. Drafted for review; open to challenge until ratified. It schedules no implementation of its own; it is the *packaging, discovery, and dispatch* layer over the operations of [PROP-031](spec://vibevm/common/PROP-031#root) and the discovery surface of [PROP-032](spec://vibevm/common/PROP-032#agent-first).

**Companions.** [PROP-031 — algorithmic refactoring](spec://vibevm/common/PROP-031#algebra) (the operations this registry catalogs) · [PROP-032 — the project model & agent-first substrate](spec://vibevm/common/PROP-032#agent-first) (the registry is its discovery surface; its §2.8 three-tier packaging is where the registry lives) · PROP-009 (the loading model — `spec/boot/INDEX.md` is the generated-manifest precedent) · PROP-025 (binary delivery — `vibe bin exec`, the dispatch mechanism) · PROP-027 (mcp packages — `.mcp.json` is the second generated-manifest precedent) · PROP-029 (fully-qualified addresses — id namespacing) · PROP-018 (skills — the `[[skill]]` declaration idiom).

---

## 1. Problem statement {#problem}

`prop r1` — Refactorings must be **extensible by package install**: installing `rust-ai-native` should add its refactorings; a future `llm-refactorings` package should add LLM-only or hybrid ones; a base vibevm project should have none until it installs the substrate. That requires a **central entity that knows which refactorings exist** in *this* project, and it must be **discovered once and cached** — precompiled from the installed set — not re-scanned on every invocation.

The mechanism already exists in vibevm and is proven three times over. A package declares a capability as a repeated TOML table in its `vibe.toml` — `[[binary]]`, `[[skill]]`, `[boot_snippet]` — and `vibe install` **discovers** those declarations across the lockfile-resolved set and **composes a cached, generated artifact**: `spec/boot/INDEX.md` (boot, PROP-009), `.mcp.json` (MCP servers, PROP-027), the binary dispatch table (PROP-025), the projected skills (PROP-018). *"Install a package, gain its bins/skills/boot"* already works exactly the way this PROP wants *"install a package, gain its refactorings"* to work. **A refactoring is simply another declared capability.** This PROP applies the existing pattern to a new table, `[[refactoring]]`.

## 2. Decisions {#decisions}

### 2.1 Refactorings are a declared package capability {#declaration}

`req r1` — A package contributes refactorings with a `[[refactoring]]` table in its `vibe.toml`, alongside the existing `[[binary]]` / `[[skill]]` / `[boot_snippet]`:

```toml
[[refactoring]]
id         = "rename-address"          # namespaced by the provider's group (PROP-029)
kind       = "algorithmic"             # algorithmic | llm | hybrid
title      = "Rename a spec:// or code:// address"
applies_to = ["spec-node", "code-node"]    # node kinds / addresses it operates on
provider   = "rust-ai-native-specmap"  # for algorithmic/hybrid: which [[binary]] implements it (PROP-025)
invoke     = "rename-address {from} {to}"  # subcommand template
params     = "spec/refactorings/rename-address.params.toml"  # typed parameter schema
gate       = "specmap-check"           # the mandatory post-check (PROP-031 §2.3)
dry_run    = true

[[refactoring]]
id     = "modernize-idiom"
kind   = "llm"
prompt = "spec/refactorings/modernize-idiom.prompt.md"   # the instruction template the agent fills
gate   = "cargo test"                  # even an LLM refactor is done only when the gate is green
```

The fields are the *contract surface*: what the operation is (`id`, `title`, `kind`), where it applies (`applies_to`), how edits are produced (`provider` + `invoke`, or `prompt`, or both), its typed inputs (`params`), and — non-optional — its `gate`.

### 2.2 The registry is a generated, cached manifest {#registry}

`req r1` — `vibe install` composes every `[[refactoring]]` across the lockfile-resolved packages into a **generated, cached manifest** — the "precompiled catalog" (`.vibe/refactorings.toml`, or a committed manifest in the `INDEX.md` mould). This is the central entity that knows what exists. It mirrors `spec/boot/INDEX.md` (PROP-009) and `.mcp.json` (PROP-027): derived from the installed set, regenerated on install, **invalidated on any lockfile change** (content-hash keyed, like vibe's other caches). Nothing rediscovers on a bare `vibe refactor` invocation.

### 2.3 Precompile is a frozen dispatch table {#precompile}

`req r1` — "Precompile" is concrete work done once at install, not a re-scan:

1. **Resolve** each refactoring's provider binary path through the lockfile (PROP-025) and freeze it into a dispatch entry.
2. **Validate** the `params` schema so a malformed contribution fails at install, not at use.
3. **Detect id collisions** across packages (ids are namespaced, §2.5) — two packages claiming the same id is an install-time error.
4. **Record applicability** (`applies_to`, language) so a client can filter *"what applies here"* without loading every provider.
5. **Write** the frozen table to the cache.

Runtime is a table lookup → invoke. The precompiled table is the "compiled capabilities" the discovery never has to rebuild until the lockfile moves.

### 2.4 Three kinds, one gated interface {#kinds}

`req r1` — Every refactoring, whatever its kind, is the same shape: **`(params) → proposed edits → gate → commit-or-rollback`** (PROP-031 §2.3). Only the *edit production* differs, and the registry entry declares which:

| kind | edits produced by | dispatch |
|---|---|---|
| `algorithmic` | a deterministic engine | `vibe bin exec <provider> -- <invoke>` (PROP-025) |
| `llm` | an agent filling a `prompt` template | hand the agent the prompt + context; it proposes edits |
| `hybrid` | both — mechanical transform + an agent for the judgment part | the provider orchestrates the callback |

The uniform contract (typed params, dry-run, **atomic + gated**) means a client — CLI or agent — treats all three identically; the `gate` is what makes even an LLM refactor trustworthy (it is *done* only when the model re-checks clean, PROP-031 §2.3). This is the LLM boundary (PROP-031 §2.2) made a first-class, packaged, gated operation rather than free-form editing.

### 2.5 Ids are namespaced by the providing package {#namespacing}

`prop r1` — A refactoring id is namespaced by its provider's group (PROP-029): `org.vibevm.world/specmark:rename-address`, `org.vibevm.ai-native/rust-ai-native:extract-cell`, `org.acme/refactors:my-thing`. Short ids resolve when unambiguous (the short-name mechanism); collisions are caught at precompile (§2.3). Namespacing is what lets independent packages contribute freely without central coordination.

### 2.6 The center is the library and the spec; CLI and MCP are thin surfaces {#surfaces}

`req r1` — The registry is, first, a **Rust library API and a specification** — the data layer agents and tools work against directly (the PROP-014 rule: fully useful without an LLM). `vibe refactor list [--applies-to <node>]` and `vibe refactor <id> <params> [--dry-run]` are a **thin core dispatcher** over the registry; the MCP tools `refactoring_list` / `refactoring_describe` / `refactoring_apply` are the **agent-first surface** (PROP-032 §2.6) — the agent asks *"what refactorings exist here"* and emits a typed `apply`. The CLI is not the center: an agent drives the library/registry through MCP, and a human uses `vibe refactor` as one surface among several (library → CLI → MCP → GUI, PROP-032 §2.6).

## 3. Where the registry sits — the three-tier product model {#tiers}

`prop r1` — The registry lives in the **SDD-substrate tier** of the product model ratified in [PROP-032 §2.8](spec://vibevm/common/PROP-032#packaging):

1. **Base vibevm** — no refactorings; `vibe refactor list` is empty.
2. **+ the SDD substrate** (specmark + specmap) — contributes the algorithmic core (`rename-address`, `move-unit`, `retarget-edge`) **and the registry itself**.
3. **+ ai-native** (`rust-ai-native`, …) — contributes discipline refactorings (`extract-cell`, …) into the same registry.

The registry **composes across whatever tiers are installed**, so the available refactoring set grows monotonically with the installed packages — the exact "install a package, extend the refactorings" behaviour, delivered by the same install-time composition that already grows bins/skills/boot.

## 4. Build-in-anticipation {#anticipation}

`req r1` — Before the engine ships, keep the design refactoring-registry-ready: (1) author the first operations (PROP-031's `rename-address`, `move-unit`) **as `[[refactoring]]` declarations from birth**, not bespoke subcommands, so the registry has real entries to compose; (2) keep the registry a **generated artifact** in the `INDEX.md` mould (derived, cached, lockfile-invalidated), never a hand-maintained list; (3) namespace every id (§2.5) from the first entry, so no un-namespaced id has to be migrated later.

## 5. Rejected alternatives {#rejected}

1. **Rediscover on every invocation.** Slow and non-deterministic across a session; the precompiled cache (§2.3) is the fix, exactly as INDEX.md/.mcp.json are computed once at install.
2. **A hardcoded refactoring list in the CLI.** Not extensible by package install — the whole point. Refactorings must be *contributed*, not compiled in.
3. **CLI as the center.** Agents need the library and the spec as the data layer (§2.6); a CLI-first design strands the agent-first primary consumer.
4. **A registry outside the lockfile** (a separate discovery config). Two sources of truth; the lockfile already *is* the installed-set authority, so the registry is derived from it, like every other composed artifact.
5. **One registry entry per binary subcommand, discovered by probing binaries.** Probing is slow and unvalidated; a declared `[[refactoring]]` table is fast, checked at install, and lets a package expose *some* subcommands as refactorings and not others.

## 6. Prior art & license posture {#prior-art}

Ideas are free; code is not (PROP-000 §3). Roles explicit.

| System | License (verify) | Role here |
|---|---|---|
| **VS Code `contributes.commands`** | MIT | **The closest model.** Extensions declare commands in their manifest; the host discovers and registers them into one command registry. This PROP is the same, applied to refactorings, composed at install. |
| LSP code actions / `executeCommand` | n/a (protocol) | Dynamic "what actions apply here" advertisement — the `applies_to` filter surface. |
| OpenRewrite recipe registry | Apache-2.0 | Recipes discovered from the classpath, listed and composed — a typed, gated operation catalog. |
| cargo custom subcommands | n/a (cargo) | `cargo-<x>` on PATH extends the CLI by discovery — extensibility, but without a manifest or a precompiled cache (the gap this PROP closes). |

**Differentiators.** (i) refactorings are **composed at install into a cached manifest** (the vibevm INDEX.md/.mcp.json pattern), not probed at runtime; (ii) the catalog spans **three implementation kinds** (algorithmic / llm / hybrid) under one gated contract; (iii) the primary consumer is an **agent** querying the registry over MCP; (iv) ids are **namespaced** so independent packages contribute without collision.

## 7. Open questions {#open}

1. **Committed manifest vs `.vibe` cache.** INDEX.md is committed (it is human-visible boot state); the refactoring catalog may be pure cache (`.vibe/refactorings.toml`, gitignored) since it is fully derived from the lockfile. Lean: cache, regenerated on install, like `.mcp.json`.
2. **Param schema language.** JTD (as the wire types already use) vs a TOML schema vs inline. Lean: reuse the JTD codegen pipeline for uniformity.
3. **How LLM refactorings ship their prompt + verification.** A `prompt` file plus a `gate`; but the prompt is *instructions-shaped prose delivered into an agent* — the PROP-014 §2.8.4 prompt-injection concern applies, so LLM-refactoring packages may need the same signed-content posture.
4. **The latent unified capability-contribution abstraction.** `[[binary]]`, `[[skill]]`, `[boot_snippet]`, `[[refactoring]]` are four instances of one pattern. A unified `[[capability]]` model is possible but premature; follow the proven per-capability idiom until a fifth instance justifies the generalisation.

---

*This PROP is a design proposal. Ratification happens through PR review against PROP-031 and PROP-032. Its first concrete step is authoring PROP-031's `rename-address` as a `[[refactoring]]` declaration in the SDD-substrate package. Any mechanism specified here that is not exercised by the second contributed refactoring is removed from the spec rather than carried as aspirational documentation (the PROP-014 §335 discipline, inherited).*
