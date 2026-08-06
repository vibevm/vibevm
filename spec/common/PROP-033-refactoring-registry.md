# PROP-033 — The refactoring registry: package-contributed, discovered, precompiled operations {#root}

<status stage="spec" state="done" action="continue" comment="B0 2026-07-24: design proposal v0.1, drafted for review; open to challenge until ratified; fact grain 2026-07-24"/>

@fact:status-line **Status.** Design proposal v0.1 — not implementation-locked. Drafted for review; open to challenge until ratified. It schedules no implementation of its own; it is the *packaging, discovery, and dispatch* layer over the operations of [PROP-031](spec://org.vibevm.core/vibevm/common/PROP-031#root) and the discovery surface of [PROP-032](spec://org.vibevm.core/vibevm/common/PROP-032#agent-first). @status:spec/done

@fact:companions **Companions.** [PROP-031 — algorithmic refactoring](spec://org.vibevm.core/vibevm/common/PROP-031#algebra) (the operations this registry catalogs) · [PROP-032 — the project model & agent-first substrate](spec://org.vibevm.core/vibevm/common/PROP-032#agent-first) (the registry is its discovery surface; its §2.8 three-tier packaging is where the registry lives) · PROP-009 (the loading model — `spec/boot/INDEX.md` is the generated-manifest precedent) · PROP-025 (binary delivery — `vibe bin exec`, the dispatch mechanism) · PROP-027 (mcp packages — `.mcp.json` is the second generated-manifest precedent) · PROP-029 (fully-qualified addresses — id namespacing) · PROP-018 (skills — the `[[skill]]` declaration idiom). @status:spec/done

---

## 1. Problem statement {#problem}

- @fact:PROBLEM-EXTENSIBLE `prop r1` — Refactorings must be **extensible by package install**: installing `rust-ai-native` should add its refactorings; a future `llm-refactorings` package should add LLM-only or hybrid ones; a base vibevm project should have none until it installs the substrate. @status:spec/done
- @fact:PROBLEM-CENTRAL-ENTITY That requires a **central entity that knows which refactorings exist** in *this* project. @status:spec/done
- @fact:PROBLEM-PRECOMPILED It must be **discovered once and cached** — precompiled from the installed set — not re-scanned on every invocation. @status:spec/done

- @fact:MECH-PROVEN The mechanism already exists in vibevm and is proven three times over. @status:spec/done
- @fact:MECH-DECLARE-COMPOSE A package declares a capability as a repeated TOML table in its `vibe.toml` — `[[binary]]`, `[[skill]]`, `[boot_snippet]` — and `vibe install` **discovers** those declarations across the lockfile-resolved set and **composes a cached, generated artifact**: `spec/boot/INDEX.md` (boot, PROP-009), `.mcp.json` (MCP servers, PROP-027), the binary dispatch table (PROP-025), the projected skills (PROP-018). @status:spec/done
- @fact:MECH-SAME-SHAPE *"Install a package, gain its bins/skills/boot"* already works exactly the way this PROP wants *"install a package, gain its refactorings"* to work. @status:spec/done
- @fact:CAPABILITY-LAW **A refactoring is simply another declared capability.** This PROP applies the existing pattern to a new table, `[[refactoring]]`. @status:spec/done

## 2. Decisions {#decisions}

### 2.1 Refactorings are a declared package capability {#declaration}

@fact:DECL-LAW `req r1` — A package contributes refactorings with a `[[refactoring]]` table in its `vibe.toml`, alongside the existing `[[binary]]` / `[[skill]]` / `[boot_snippet]`: @status:spec/done

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

@fact:DECL-FIELDS The fields are the *contract surface*: what the operation is (`id`, `title`, `kind`), where it applies (`applies_to`), how edits are produced (`provider` + `invoke`, or `prompt`, or both), its typed inputs (`params`), and — non-optional — its `gate`. @status:spec/done

### 2.2 The registry is a generated, cached manifest {#registry}

- @fact:REG-COMPOSED `req r1` — `vibe install` composes every `[[refactoring]]` across the lockfile-resolved packages into a **generated, cached manifest** — the "precompiled catalog" (`.vibe/refactorings.toml`, or a committed manifest in the `INDEX.md` mould). This is the central entity that knows what exists. @status:spec/done
- @fact:REG-MIRRORS-PRECEDENT It mirrors `spec/boot/INDEX.md` (PROP-009) and `.mcp.json` (PROP-027): derived from the installed set, regenerated on install, **invalidated on any lockfile change** (content-hash keyed, like vibe's other caches). @status:spec/done
- @fact:REG-NO-RESCAN Nothing rediscovers on a bare `vibe refactor` invocation. @status:spec/done

### 2.3 Precompile is a frozen dispatch table {#precompile}

@fact:PRECOMPILE-LAW `req r1` — "Precompile" is concrete work done once at install, not a re-scan: @status:spec/done

1. @fact:PC-RESOLVE **Resolve** each refactoring's provider binary path through the lockfile (PROP-025) and freeze it into a dispatch entry. @status:spec/done
2. @fact:PC-VALIDATE **Validate** the `params` schema so a malformed contribution fails at install, not at use. @status:spec/done
3. @fact:PC-COLLISIONS **Detect id collisions** across packages (ids are namespaced, §2.5) — two packages claiming the same id is an install-time error. @status:spec/done
4. @fact:PC-APPLICABILITY **Record applicability** (`applies_to`, language) so a client can filter *"what applies here"* without loading every provider. @status:spec/done
5. @fact:PC-WRITE **Write** the frozen table to the cache. @status:spec/done

- @fact:RUNTIME-LOOKUP Runtime is a table lookup → invoke. @status:spec/done
- @fact:PC-STABLE-UNTIL-LOCKFILE The precompiled table is the "compiled capabilities" the discovery never has to rebuild until the lockfile moves. @status:spec/done

### 2.4 Three kinds, one gated interface {#kinds}

@fact:KINDS-ONE-SHAPE `req r1` — Every refactoring, whatever its kind, is the same shape: **`(params) → proposed edits → gate → commit-or-rollback`** (PROP-031 §2.3). Only the *edit production* differs, and the registry entry declares which: @status:spec/done

| kind | edits produced by | dispatch |
|---|---|---|
| @fact:KIND-ALGORITHMIC `algorithmic` @status:spec/done | a deterministic engine @status:spec/done | `vibe bin exec <provider> -- <invoke>` (PROP-025) @status:spec/done |
| @fact:KIND-LLM `llm` @status:spec/done | an agent filling a `prompt` template @status:spec/done | hand the agent the prompt + context; it proposes edits @status:spec/done |
| @fact:KIND-HYBRID `hybrid` @status:spec/done | both — mechanical transform + an agent for the judgment part @status:spec/done | the provider orchestrates the callback @status:spec/done |

- @fact:UNIFORM-CLIENT The uniform contract (typed params, dry-run, **atomic + gated**) means a client — CLI or agent — treats all three identically. @status:spec/done
- @fact:GATE-TRUST The `gate` is what makes even an LLM refactor trustworthy (it is *done* only when the model re-checks clean, PROP-031 §2.3). @status:spec/done
- @fact:LLM-BOUNDARY-PACKAGED This is the LLM boundary (PROP-031 §2.2) made a first-class, packaged, gated operation rather than free-form editing. @status:spec/done

### 2.5 Ids are namespaced by the providing package {#namespacing}

- @fact:NS-LAW `prop r1` — A refactoring id is namespaced by its provider's group (PROP-029): `org.vibevm.world/specmark:rename-address`, `org.vibevm.ai-native/rust-ai-native:extract-cell`, `org.acme/refactors:my-thing`. @status:spec/done
- @fact:NS-SHORT-IDS Short ids resolve when unambiguous (the short-name mechanism); collisions are caught at precompile (§2.3). @status:spec/done
- @fact:NS-WHY Namespacing is what lets independent packages contribute freely without central coordination. @status:spec/done

### 2.6 The center is the library and the spec; CLI and MCP are thin surfaces {#surfaces}

- @fact:CENTER-LIBRARY `req r1` — The registry is, first, a **Rust library API and a specification** — the data layer agents and tools work against directly (the PROP-014 rule: fully useful without an LLM). @status:spec/done
- @fact:CLI-THIN `vibe refactor list [--applies-to <node>]` and `vibe refactor <id> <params> [--dry-run]` are a **thin core dispatcher** over the registry. @status:spec/done
- @fact:MCP-AGENT-FIRST The MCP tools `refactoring_list` / `refactoring_describe` / `refactoring_apply` are the **agent-first surface** (PROP-032 §2.6) — the agent asks *"what refactorings exist here"* and emits a typed `apply`. @status:spec/done
- @fact:CLI-NOT-CENTER The CLI is not the center: an agent drives the library/registry through MCP, and a human uses `vibe refactor` as one surface among several (library → CLI → MCP → GUI, PROP-032 §2.6). @status:spec/done

## 3. Where the registry sits — the three-tier product model {#tiers}

@fact:TIER-PLACEMENT `prop r1` — The registry lives in the **SDD-substrate tier** of the product model ratified in [PROP-032 §2.8](spec://org.vibevm.core/vibevm/common/PROP-032#packaging): @status:spec/done

1. @fact:TIER-BASE **Base vibevm** — no refactorings; `vibe refactor list` is empty. @status:spec/done
2. @fact:TIER-SDD **+ the SDD substrate** (specmark + specmap) — contributes the algorithmic core (`rename-address`, `move-unit`, `retarget-edge`) **and the registry itself**. @status:spec/done
3. @fact:TIER-AI-NATIVE **+ ai-native** (`rust-ai-native`, …) — contributes discipline refactorings (`extract-cell`, …) into the same registry. @status:spec/done

@fact:TIER-COMPOSITION The registry **composes across whatever tiers are installed**, so the available refactoring set grows monotonically with the installed packages — the exact "install a package, extend the refactorings" behaviour, delivered by the same install-time composition that already grows bins/skills/boot. @status:spec/done

## 4. Build-in-anticipation {#anticipation}

@fact:ANTICIPATION-LAW `req r1` — Before the engine ships, keep the design refactoring-registry-ready: @status:spec/done

1. @fact:ANT-BORN-DECLARED author the first operations (PROP-031's `rename-address`, `move-unit`) **as `[[refactoring]]` declarations from birth**, not bespoke subcommands, so the registry has real entries to compose; @status:spec/done
2. @fact:ANT-GENERATED keep the registry a **generated artifact** in the `INDEX.md` mould (derived, cached, lockfile-invalidated), never a hand-maintained list; @status:spec/done
3. @fact:ANT-NAMESPACED namespace every id (§2.5) from the first entry, so no un-namespaced id has to be migrated later. @status:spec/done

## 5. Rejected alternatives {#rejected}

1. @fact:REJ-RESCAN **Rediscover on every invocation.** Slow and non-deterministic across a session; the precompiled cache (§2.3) is the fix, exactly as INDEX.md/.mcp.json are computed once at install. @status:spec/done
2. @fact:REJ-HARDCODED **A hardcoded refactoring list in the CLI.** Not extensible by package install — the whole point. Refactorings must be *contributed*, not compiled in. @status:spec/done
3. @fact:REJ-CLI-CENTER **CLI as the center.** Agents need the library and the spec as the data layer (§2.6); a CLI-first design strands the agent-first primary consumer. @status:spec/done
4. @fact:REJ-OUTSIDE-LOCKFILE **A registry outside the lockfile** (a separate discovery config). Two sources of truth; the lockfile already *is* the installed-set authority, so the registry is derived from it, like every other composed artifact. @status:spec/done
5. @fact:REJ-PROBING **One registry entry per binary subcommand, discovered by probing binaries.** Probing is slow and unvalidated; a declared `[[refactoring]]` table is fast, checked at install, and lets a package expose *some* subcommands as refactorings and not others. @status:spec/done

## 6. Prior art & license posture {#prior-art}

@fact:prior-art-lead Ideas are free; code is not (PROP-000 §3). Roles explicit. @status:spec/done

| System | License (verify) | Role here |
|---|---|---|
| @fact:PA-VSCODE **VS Code `contributes.commands`** @status:spec/done | MIT @status:spec/done | **The closest model.** Extensions declare commands in their manifest; the host discovers and registers them into one command registry. This PROP is the same, applied to refactorings, composed at install. @status:spec/done |
| @fact:PA-LSP LSP code actions / `executeCommand` @status:spec/done | n/a (protocol) @status:spec/done | Dynamic "what actions apply here" advertisement — the `applies_to` filter surface. @status:spec/done |
| @fact:PA-OPENREWRITE OpenRewrite recipe registry @status:spec/done | Apache-2.0 @status:spec/done | Recipes discovered from the classpath, listed and composed — a typed, gated operation catalog. @status:spec/done |
| @fact:PA-CARGO cargo custom subcommands @status:spec/done | n/a (cargo) @status:spec/done | `cargo-<x>` on PATH extends the CLI by discovery — extensibility, but without a manifest or a precompiled cache (the gap this PROP closes). @status:spec/done |

@fact:DIFFERENTIATORS **Differentiators.** @status:spec/done

1. @fact:DIFF-COMPOSED refactorings are **composed at install into a cached manifest** (the vibevm INDEX.md/.mcp.json pattern), not probed at runtime; @status:spec/done
2. @fact:DIFF-THREE-KINDS the catalog spans **three implementation kinds** (algorithmic / llm / hybrid) under one gated contract; @status:spec/done
3. @fact:DIFF-AGENT-CONSUMER the primary consumer is an **agent** querying the registry over MCP; @status:spec/done
4. @fact:DIFF-NAMESPACED ids are **namespaced** so independent packages contribute without collision. @status:spec/done

## 7. Open questions {#open}

<status stage="spec" state="work" comment="B1 2026-07-24: four questions open; ratification pending"/>

1. @fact:open-manifest-vs-cache **Committed manifest vs `.vibe` cache.** INDEX.md is committed (it is human-visible boot state); the refactoring catalog may be pure cache (`.vibe/refactorings.toml`, gitignored) since it is fully derived from the lockfile. Lean: cache, regenerated on install, like `.mcp.json`. @status:spec/work
2. @fact:open-param-schema **Param schema language.** JTD (as the wire types already use) vs a TOML schema vs inline. Lean: reuse the JTD codegen pipeline for uniformity. @status:spec/work
3. @fact:open-llm-prompt-shipping **How LLM refactorings ship their prompt + verification.** A `prompt` file plus a `gate`; but the prompt is *instructions-shaped prose delivered into an agent* — the PROP-014 §2.8.4 prompt-injection concern applies, so LLM-refactoring packages may need the same signed-content posture. @status:spec/work
4. @fact:open-unified-capability **The latent unified capability-contribution abstraction.** `[[binary]]`, `[[skill]]`, `[boot_snippet]`, `[[refactoring]]` are four instances of one pattern. A unified `[[capability]]` model is possible but premature; follow the proven per-capability idiom until a fifth instance justifies the generalisation. @status:spec/work

---

- @fact:ratification-note *This PROP is a design proposal. Ratification happens through PR review against PROP-031 and PROP-032.* @status:spec/done
- @fact:first-step *Its first concrete step is authoring PROP-031's `rename-address` as a `[[refactoring]]` declaration in the SDD-substrate package.* @status:spec/done
- @fact:unexercised-removed *Any mechanism specified here that is not exercised by the second contributed refactoring is removed from the spec rather than carried as aspirational documentation (the PROP-014 §335 discipline, inherited).* @status:spec/done
