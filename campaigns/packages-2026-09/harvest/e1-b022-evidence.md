# E1 · B-022 — LEDGER-INTENT mechanisms: feasibility evidence

**Date:** 2026-08-03
**HEAD:** `779b3aaa docs(campaign): коэффициент параллельности — до 5 на запускалку, 10 всего`
**Subject:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/mechanisms/LEDGER-INTENT-v0.1.xml` (read whole)
**Engine:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/ledger.rs` (read whole, 303 lines)

**Owner directive (BACKLOG.md B-022):** «давай положим в бэклог исследование» (2026-08-01). This document is **evidence only** — facts with `file:line`, no verdicts, no build/skip recommendation. The recommendation stays with the boss. Every claim below carries a `path:line` citation; every absence claim names its perimeter and the search terms used.

**Default search perimeter** (used for every absence claim below unless a section widens it):

- ENGINE: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/`
- DRIVERS: `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/`, `vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/`, `vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/` (plus the three `-mcp` siblings)
- HOST: `crates/`, `xtask/`, `tools/`, `schemas/`
- DEPLOYMENT: `.ledger/` on disk, `terraform/REPORT.md`

Excluded always: `legacy-spec/**`, `target/**`, `.vibe/cache/**`, `vibedeps/**` (cited only as mirrors of a vendored engine crate, never as an independent second implementation). `campaigns/**` and `refs/**` are never evidence. **Name-distinction:** `rust-ai-native-cli/src/ledger.rs` is a *different* ledger — the DEBT/INTENT registry renderer (its own header says so) — and is not the LEDGER-INTENT interpretation cache studied here; see «Callers and vendored copies».

---

## M-A — `##ENTRY-CARRIES-ITS-PROVENANCE-FIELDS` (§4, `LEDGER-INTENT-v0.1.xml:61`)

The spec row `##ROW-CLASS-INTERPRETATIONS` (`LEDGER-INTENT-v0.1.xml:38`) names the interpretations key as *«(subject hashes, spec revs touched, **epoch**, producer id, prompt rev, model id)»* and its in-text annotation states *«the shipped key is three of these six … `spec revs touched`, `prompt rev` and `model id` are in no key, and `prompt_rev` / `model_id` appear nowhere in the crate outside the header comment that quotes this row.»* §4 (`:61`) then states each entry *«carries `{producer, model_id?, prompt_rev?, inputs (hashes + spec URIs ~r), epoch, cost, created_at, confidence}`»*.

### 1. What exists today

- **No entry type exists.** The structs in the module are `Epoch` (`ledger.rs:35-36`), `Telemetry` (`ledger.rs:82-88`) and `ProseRender { text, cached, epoch }` (`ledger.rs:112-117`); none is a stored entry. The on-disk object is the render text itself — `std::fs::write(&slot, &text)` at `ledger.rs:155`, where `text` is the prose string (`ledger.rs:151`).
- **Three of the eight fields survive only as ingredients of the cache key**, unreadable from the entry. The key is composed at `ledger.rs:132-137`: `PRODUCER = "explain.item/prose-template-1"` (`:132`), `subject = serde_json::to_string(&subgraph)` of the explain JSON (`:133-134`), `epoch = epoch(root)` (`:135`), `key = content_hash(&format!("{PRODUCER}\n{}\n{subject}", epoch.0))` (`:136`). So `producer`, `inputs` (the subgraph) and `epoch` are *hashed into the key*, not carried on the entry.
- **Five fields have no representation at all.** Re-measured at this HEAD over ENGINE + HOST (`crates/`, `xtask/`), terms `model_id`, `prompt_rev`, `created_at`, `confidence`, `cost` (as a field, not a comment): `model_id` 0 hits, `prompt_rev` 0 hits, `created_at` 0 hits in the engine/host Rust; `confidence` 0 in the engine (the one host hit is unrelated — `crates/progress-core/src/baseline/project.rs:19`); `cost` appears only as doc-comment prose inside `ledger.rs` itself at `:16` («cost, rot-rate plumbing»), `:77` («hit rate and cost feed the Charter's headline metric») and `:81` («the template producer recomputes from scratch, cost ~0») — never a field.
- **The module's own header** (`ledger.rs:7`) declares *«One query kind ships: `explain.item`»*, and `:9-11` frames the producer as *«a deterministic template (the tool MUST be fully useful without an LLM; an LLM prose producer slots in later under its own producer id + model id)»* — i.e. the design reserves the `model_id` slot but no producer fills it.
- **Tests covering the module** (`ledger.rs:225-303`): `second_identical_prose_call_is_a_cache_hit` (`:261`), `editing_cargo_lock_invalidates_the_render` (`:279`), `epoch_is_stable_for_unchanged_inputs` (`:298`); helpers `mini_map` (`:230`) and `seed_epoch_inputs` (`:256`). None exercises a stored entry with provenance fields (there is none to exercise); they assert on `cached`/`epoch`/telemetry counters and the rendered provenance *line* (`:274`).

### 2. What would have to be built

- **A stored entry type** with the eight fields, replacing (or wrapping) the bare-text write at `ledger.rs:155`. Today `object_path` (`ledger.rs:119-124`) addresses a slot whose content is opaque text; an entry type needs a serialisation and a reader.
- **A key that does not destroy the fields it hashes.** `ledger.rs:136` folds producer+epoch+subject into one `sha256`; an entry that *carries* them requires the key composition to change (structured tuple vs. opaque hash), which is exactly the §8 `##FAILURE-KEY-UNDER-SPECIFICATION` concern (M-E) read against this mechanism.
- **New fields with no producer to populate them.** `model_id`, `prompt_rev`, `cost`, `created_at`, `confidence` need a producer that has those values; the one shipped producer (`explain.item/prose-template-1`, `ledger.rs:132`) is deterministic and carries none of them. A timestamp (`created_at`) is the only one derivable without new infrastructure; `cost`/`model_id`/`prompt_rev` presuppose an LLM producer that does not exist (see M-C and the `vibe-llm` note below).
- **Surfaces touched:** `ledger.rs` (entry type, key, write/read), and — because the module is vendored (see «Callers and vendored copies») — the change propagates to six vendor copies via `cargo xtask sync-engines` (`sync-engines.toml`).

### 3. Dependencies and couplings

- **`model_id` couples to B-020 (external-LLM client).** BACKLOG.md B-020, `##B020-BUILD` (`BACKLOG.md:647`), states verbatim: *«(2) Второй „производитель текста" встаёт в готовый слот кэша; в ключ кэша добавляется идентификатор модели.»* B-020's disposition (`BACKLOG.md:642`) is `planned` — owner ruling 2026-08-01: *«это надо строить … лайтовый клиент для внешних нелокальных LLM … Возможно это будет fractality … разобраться позднее»*. So the `model_id`/`prompt_rev` half of this entry is gated on B-020; the deterministic-producer half is not.
- **No LLM producer exists anywhere.** `crates/vibe-llm/src/lib.rs` is a 9-line M0 stub: its header (`lib.rs:1-7`) reads *«**STATUS: M0 stub.** Concrete providers (Anthropic, OpenAI, OpenRouter, Ollama) land in the v1.5 LLM milestone per `VIBEVM-SPEC.md` §10.4; this crate is a deliberate placeholder until then, not forgotten work.»* `Cargo.toml` description (`crates/vibe-llm/Cargo.toml`): *«LLM provider abstraction for vibevm (M1.5+); stub in M0»*; dependencies are `anyhow` + `thiserror` only. So the second producer the spec reserves (`ledger.rs:9-11`) has no runtime to attach to.
- **The one shipped producer** is `explain.item/prose-template-1` at `ledger.rs:132`; there is no other producer id in the engine (terms `producer`, `prose-template`, `producer_id` over ENGINE return only this constant and its echo in the key).
- **Coupled to M-E.** Changing the key to carry fields rather than hash them is the same surface M-E addresses.

### 4. Callers / consumers (mechanism-level)

- The entry's only consumer is the render path `prose_explain` (`ledger.rs:131-163`), reached solely from the rust stack CLI — see «Callers and vendored copies». No host crate reads a ledger entry.

---

## M-B — `##GC-IS-LRU-WITH-A-PIN-SET` (§5, `LEDGER-INTENT-v0.1.xml:70`)

The spec (`:70`) states *«GC: LRU with a pin set (entries referenced by the current release slice are pinned). Size budget configurable; eviction never affects correctness, only cost.»*

### 1. What exists today

- **Nothing evicts a ledger object, ever.** The write path (`ledger.rs:155`) only ever adds; there is no remove, no sweep, no listing of the store. `object_path` (`ledger.rs:119-124`) addresses one object; nothing enumerates `.ledger/objects/`.
- **No LRU, pin set, or size budget in code or config.** Re-measured at this HEAD over ENGINE + HOST: terms `lru`, `pin_set`, `size_budget`, `evict`, `prune`, `gc`, `budget` — `lru` 0, `pin_set` 0, `size_budget` 0 in the engine/host Rust; `evict` hits only HTTP rate-limit buckets (`crates/vibe-index/src/server/rate_limit.rs`); `gc` hits only `vibe self gc` — the PROP-019 version-manager pruner (`crates/vibe-cli/src/commands/vvm/remove.rs:1-3`). Neither touches `.ledger/`. The specmap `Config` (`core-ai-native-specmap/src/config.rs`) carries no `budget`/`size`/`cost`/`gc`/`lru`/`evict`/`pin`/`max_*` field (grep over `config.rs` returns nothing).
- **The store layout exists and is sharded** (`ledger.rs:119-122`): `.ledger/objects/<sha256[0..2]>/<sha256>`. The spec's own `##STORAGE-LAYOUT-IS-SHARDED-LIKE-GIT-OBJECTS` annotation (`LEDGER-INTENT-v0.1.xml:69`) confirms the sharding ships and notes *«The **index** does not exist: the store is directory-only, and nothing enumerates it.»* — an index (or enumeration) is a precondition for any eviction pass.
- **On-disk state (re-measured):** `.ledger/` is **absent at this worktree root** (it is gitignored derived data — `.gitignore:49-50`: *«The local intent ledger (LEDGER-INTENT v0.1) — derived interpretations cache, never shipped.»* / `/.ledger/`). Its presence is environment state, not HEAD state; the prior measurement (`harvest/d7a-core-sync-reverify.md` F-159, `:795-796`) recorded the live store at the host main checkout holding exactly **1 object**. There is no bound enforced either way.

### 2. What would have to be built

- **A store index / enumeration** (today directory-only, per `LEDGER-INTENT-v0.1.xml:69`) — needed before anything can select candidates for eviction.
- **An LRU ordering**: `ledger.rs:141-149` reads and `:155` writes but records no access time; an LRU needs a last-used stamp on each entry, which presupposes the entry type M-A adds (a bare-text blob has no metadata to age).
- **A pin set sourced from the release slice.** No release slice exists (see M-D), so the pin set has nothing to read; this couples M-B's pin half to M-D.
- **A configurable size budget** in `Config` (`config.rs`) and an eviction pass invoked on write (`ledger.rs:151-157`) or a separate verb. `terraform/REPORT.md:21` records the ledger as a shipped MVP («`.ledger/` interpretations store; `trace explain --prose` epoch-keyed cache») with no budget mentioned.

### 3. Dependencies and couplings

- **Hard-coupled to M-A.** Eviction by recency or pin requires per-entry metadata (created_at / last-used / producer) that the bare-text store does not hold.
- **Hard-coupled to M-D for the pin set.** *«entries referenced by the current release slice are pinned»* (`LEDGER-INTENT-v0.1.xml:70`) — no release slice ⇒ no pin set.
- **No other consumers today** — the store has one writer/reader (`prose_explain`), so an eviction pass would not race existing callers, but it must not delete the single in-flight slot.

### 4. Callers / consumers (mechanism-level)

- Only `prose_explain` (`ledger.rs:131`) reads/writes the store; see «Callers and vendored copies» for the chain into the CLI.

---

## M-C — `##TELEMETRY-FEEDS-THE-HEADLINE-METRIC` (§5, `LEDGER-INTENT-v0.1.xml:72`)

The spec (`:72`) names four measures feeding the Charter's headline metric: *«hit rate, cost per query kind, **LLM-$ per merged change**, and the **contextual-rot rate** — fraction of epoch-invalidated entries whose re-verification *changed the answer*.»*

### 1. What exists today

- **The `Telemetry` struct carries two of the four measures' raw counters** (`ledger.rs:82-88`): `hits`, `misses` (→ hit rate), `rot_checks`, `rot_changed` (→ contextual-rot rate). It is persisted at `.ledger/telemetry.json` (`telemetry_path`, `ledger.rs:90-92`), loaded by `load_telemetry` (`:94-99`) and saved by `save_telemetry` (`:101-109`).
- **Hit rate is derivable** from `hits`+`misses`; incremented at `ledger.rs:142` (hit) and `:156` (miss).
- **Contextual-rot rate has plumbing but no data.** `rot_checks`/`rot_changed` are declared (`ledger.rs:86-87`) but never incremented anywhere in the module — the doc comment at `ledger.rs:79-81` says *«the rot counters are plumbing for the contextual-rot rate, incremented when a re-verification of an epoch-invalidated entry runs (**none do yet** — the template producer recomputes from scratch, cost ~0).»* So the rot rate is structurally always 0/0 today.
- **Cost-per-query-kind is absent, and unreachable in principle with one kind.** No `cost` field on `Telemetry` (`ledger.rs:82-88`); the module header (`ledger.rs:7`) ships exactly one query kind. The §6 `##QUERY-KIND-ADDED-ON-TWO-CONSUMERS` rule (`LEDGER-INTENT-v0.1.xml:81`) — *«a query kind is added when two distinct consumers ask for it»* — means the per-kind breakdown has nothing to break down.
- **LLM-$ per merged change is absent.** No field, and no LLM runtime to incur cost (`crates/vibe-llm/src/lib.rs:1-7`, M0 stub — see M-A §3).
- **Cross-document overstatement (fact, not verdict):** `terraform/REPORT.md:41` records *«LLM-$ per merged change | n/a | not instrumented — no `vibe-llm` runtime exists; the ledger's cost field is plumbed and zero-valued for the template producer.»* The code has no cost field (`ledger.rs:82-88`); `cost` exists only as comment prose at `ledger.rs:16`, `:77`, `:81`. `terraform/REPORT.md:40` separately records *«Ledger hit rate | n/a | live smoke 1 hit / 1 miss; counters in `.ledger/telemetry.json`; rot-rate plumbing in place, no data yet»* — consistent with the code.

### 2. What would have to be built

- **A `cost` field on the entry/telemetry** plus a producer that reports cost (gated on B-020 / `vibe-llm`, as in M-A).
- **A per-query-kind breakdown** — a map keyed by query kind. Needs the closed enum of M-E first (else it is «a string» keyed), and needs more than one kind to be non-trivial.
- **A rot-rate that actually fires** — a re-verification path that reads a prior-epoch entry. The spec's `##RECOMPUTE-DECISION-HAPPENS-ABOVE-THE-FLOOR` annotation (`LEDGER-INTENT-v0.1.xml:57`) states *«no producer reads a prior-epoch entry … there is no draft-input path in any engine crate»*; the rot counters increment only when such a path exists.

### 3. Dependencies and couplings

- **Cost halves gated on B-020** (external-LLM client, `BACKLOG.md:642-647`) and on `vibe-llm` (`crates/vibe-llm/src/lib.rs` stub) — identical to M-A's model_id coupling, because cost is incurred by the same LLM producer.
- **Per-kind breakdown gated on M-E** (the enum) and on a second query kind existing.
- **Rot-rate gated on a draft-input/re-verification path** that no producer exercises today (`LEDGER-INTENT-v0.1.xml:57`).

### 4. Callers / consumers (mechanism-level)

- `load_telemetry` is read in exactly one place outside the module: `rust-ai-native-cli/src/trace.rs:16`, for a human log line. The counters are otherwise write-only diagnostics. No host crate or MCP tool surfaces telemetry to an agent.

---

## M-D — `##RELEASE-SLICE-IS-EXPORTED-SIGNED-AND-SHIPPED` (§7, `LEDGER-INTENT-v0.1.xml:85`)

The spec (`:85`): *«At tag time, a frozen subset … is exported, **signed**, and shipped with the package.»* §7 continues: `##UNSIGNED-SLICES-ARE-NEVER-EXPOSED-REMOTELY` (`:87`) — *«Unsigned slices are not exposed remotely, full stop (PROP-014 §2.8.4)»*; and `##SIGNING-SCHEME-IS-AN-OPEN-QUESTION` (`:89`) — *«Signing scheme is Charter-level Open Question; until it lands, the slice exists for local use only.»*

### 1. What exists today

- **No export, freeze, sign, or ship of any slice.** Re-measured at this HEAD over ENGINE + HOST: terms `release_slice`, `slice`, `ed25519`, `minisign`, `cosign`, `sigstore`, `gpg`, `sign_`, `signature`, `export` (crypto sense), `freeze` — `release_slice` 0, `ed25519` 0, `minisign` 0, `gpg` 0; `sign_`/`signature` 0 in any cryptographic sense (only `render signature` / DFS-signature doc comments). The only crypto dependency in the workspace is `sha2 = "0.10"` (`Cargo.toml:121`), used for content hashing — no signing crate is vendored.
- **The implementing module's header contradicts the spec in one line.** `ledger.rs:15-18`: *«Storage: `.ledger/objects/<sha256[0..2]>/<sha256>` plus `.ledger/telemetry.json` … **Local per checkout; never shipped, never signed, never exposed** — `.ledger/` is git-ignored.»*
- **It is git-ignored, verifiably:** `.gitignore:49-50` (quoted in M-B §1). So three sources — spec (`:85`), engine header (`ledger.rs:17`), and the ignore file (`.gitignore:49-50`) — give different answers, and two agree against the spec.
- **No tag-time hook touches the ledger.** `terraform/REPORT.md:21` records the ledger MVP as *done* with no release-slice step; the terraform close-out lists no signing/export artefact.

### 2. What would have to be built

- **An export path** that selects a frozen subset (facts for the tagged tree + affirmed interpretations, per `:85`) from `.ledger/objects/` — needs the store index M-B/M-A lack.
- **A signing scheme** — the load-bearing absence. Per `LEDGER-INTENT-v0.1.xml:89` the scheme is an open Charter question; the candidates are enumerated in BACKLOG.md B-015 (signed git tags via SSH, minisign-class, sigstore-class — `BACKLOG.md:565`).
- **A ship step** attaching the signed slice to the package artefact, and a verify step at the consumer. The natural verify point is beside the existing integrity gate `fetch_with_expected_hash` (`crates/vibe-registry/src/git_package_registry/fetch.rs:274`, per the B-012 exemplar A5) — but that gate checks a mirror pin, not publisher identity.
- **Surfaces:** `crates/vibe-publish/` (produce), `crates/vibe-registry/` + `crates/vibe-install/` (verify), `crates/vibe-core/` (a signature type and likely a lockfile field), plus a new crypto dependency outside the current tree. This is Rule-4 (CI / signing / secrets) territory.

### 3. Dependencies and couplings

- **Signing is inside the parked security programme — BACKLOG.md B-015.** `##B015-DISPOSITION` (`BACKLOG.md:559`), quoted verbatim: *«`open` — **запаркована решением владельца, НЕ строить до его специального уведомления**; кодовых триггеров нет намеренно»*. B-015's SUT (`BACKLOG.md:562`) names the exposure directly: package-authored text reaching an agent's context is potential prompt injection, and the defence is cryptographic signing of package content. The reopen condition is **owner notice only** — no in-code event reopens it (`BACKLOG.md:563`).
- **The spec itself forbids exposing unsigned slices remotely** — `LEDGER-INTENT-v0.1.xml:87`: *«Unsigned slices are not exposed remotely, full stop (PROP-014 §2.8.4)»*. So M-D cannot ship an unsigned slice as an interim; it is gated on signing landing.
- **Coupled to M-A** (the slice is *«affirmed interpretations»* — entry records) and **to M-B** (a frozen subset needs the index).
- **Coupled to the runtime-channel work (B-018)** only if the slice is meant to be queryable by a consumer agent; today no consumer channel reads it.

### 4. Callers / consumers (mechanism-level)

- None today — nothing reads or writes a release slice. The slice's intended consumer (an agent debugging `vibe` at a tagged version, per `:85`) has no channel; see the B-012 exemplar A4 for the runtime-channel gap.

---

## M-E — the closed query-kind enum (§6 `##QUERY-*` + §8 `##FAILURE-KEY-UNDER-SPECIFICATION`, `LEDGER-INTENT-v0.1.xml:95`)

§8 `##FAILURE-KEY-UNDER-SPECIFICATION` (`:95`): *«query kinds are a closed enum with reviewed key schemas; adding a kind is a PR, not a string.»* §6 (`:76-79`) lists four kinds: `facts.extract` (`:76`), `explain.item` (`:77`), `classify.legacy_unit` (`:78`, annotated *«Specified, not built: this query kind has never been run»*), `propose.links` (`:79`).

### 1. What exists today

- **No enum exists.** Re-measured at this HEAD over ENGINE + HOST: terms `QueryKind`, `enum Query`, `QueryKind` — 0 hits; `enum Query` 0 hits. The one shipped kind is a string constant declared inside a function body: `const PRODUCER: &str = "explain.item/prose-template-1";` at `ledger.rs:132`, inside `prose_explain`.
- **Adding a second kind today *is* adding a string** — the precise failure `:95` claims to have designed out. There is no central registry of kinds, no reviewed key schema per kind, and the key composition (`ledger.rs:136`) bakes the producer string into an opaque hash with no kind discriminator.
- **Of the four §6 kinds, only `explain.item` is reachable.** Re-measured over ENGINE: `facts.extract`, `propose.links`, `classify.legacy_unit` (and snake-case variants) return 0 hits as functions. `classify.legacy_unit` is explicitly unbuilt per the spec's own annotation (`LEDGER-INTENT-v0.1.xml:78`); `facts.extract` is *«frontends (algorithmic)»* — the facts class is the conform engine's store, deliberately not this module (`ledger.rs:3-5`: *«the facts class is the conform engine's store … which this module deliberately does not touch»*); `propose.links` is *«Phase-2 mining (LLM)»* with no implementation.
- **The provenance render string** (`render_prose`, `ledger.rs:168-223`) emits `producer {producer}` in its last line (`:213-221`) — the kind reaches the reader as displayed text, not as a typed value.

### 2. What would have to be built

- **A `QueryKind` enum** (closed, reviewed) replacing the `:132` string constant, with one variant per shipped kind.
- **A reviewed key schema per variant** — today there is one key shape (`ledger.rs:136`) implicit in `prose_explain`; the enum would make the (subject, epoch, producer, …) tuple per-kind explicit and addressable, which is also what M-A's entry-type change needs.
- **A registration/review surface** so adding a kind is a PR — today there is nothing to PR against but the string.
- **Surfaces:** `ledger.rs` (the enum + dispatch), and — via `cargo xtask sync-engines` — the six vendor copies. Mechanically small; the design weight is the per-kind key schemas the spec says must be *«reviewed»*.

### 3. Dependencies and couplings

- **The packet's own characterisation** (`BACKLOG.md:674`, `##B022-COUPLING`): *«вид-запроса-как-enum — дешёвый и независимый»* («query-kind-as-enum is cheap and independent») — i.e. it does not gate on B-015/B-020 the way M-A/M-C/M-D do.
- **Coupled to M-A** (the key-shape change) and to **M-C** (per-kind cost breakdown needs the enum), but neither blocks the enum itself.
- **Adding real kinds couples to their producers:** a second *interpretations* kind (`propose.links`, `classify.legacy_unit`) needs the LLM producer (B-020 / `vibe-llm`) that does not exist; a second *facts* kind lives in the conform engine, a different module.

### 4. Callers / consumers (mechanism-level)

- The kind string is consumed only inside `prose_explain` (`ledger.rs:132, 136, 151`); the rendered provenance line is shown to the human at `rust-ai-native-cli/src/trace.rs:18`. No other caller keys on the kind.

---

## Callers and vendored copies

### The consumer chain (who calls the LEDGER-INTENT cache today)

- **Engine-internal:** the module is only *declared*, at `core-ai-native-specmap/src/lib.rs:36` (`pub mod ledger;`). No other engine crate calls `ledger::` (grep `ledger::` over ENGINE returns only the `rust-ai-native-cli` consumer below, which lives in a *driver* package, not the engine). The spec's namesake verb `get_or_compute(query) -> entry` (`LEDGER-INTENT-v0.1.xml:21`) does not exist under that name — 0 hits over ENGINE + HOST.
- **The single shipped consumer is the rust stack CLI's `trace explain --prose` path:**
  - `rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/trace.rs:8` — `pub fn run_trace_explain(root, target, json, prose)`; `:13` `if prose {`; `:14` `let render = specmap_core::ledger::prose_explain(root, &map, target)?;`; `:16` `let t = specmap_core::ledger::load_telemetry(root);` (for the log line at `:18`).
  - Entry/dispatch: `rust-ai-native-cli/src/main.rs:170-172` (the `prose: bool` flag, doc *«Deterministic prose render through the intent ledger»*); `:237-238` → `rust_ai_native_cli::run_trace_explain(&root, &target, json, prose)`; re-export `rust-ai-native-cli/src/lib.rs:43` `pub use trace::run_trace_explain`.
  - **Not behind `#[cfg(test)]`** — this is the shipped subcommand, the only live exercise of the cache. The `#[cfg(test)]` callers are solely the module's own tests (`ledger.rs:225-303`, listed in M-A §1).
- **Mirror copy of that CLI in the MCP package:** `rust-ai-native-mcp/v0.7.0/crates/rust-ai-native-cli/src/trace.rs:14,16` (byte-identical, synced per `sync-engines.toml`). It is reached by the `trace_explain` MCP tool — `rust-ai-native-mcp/v0.7.0/crates/rust-ai-native-mcp/src/tools_discipline.rs:206` dispatches to `rust_ai_native_cli::run_trace_explain` (per the B-012 exemplar A4 §1, `lib.rs:48-67` `TOOL_NAMES`).
- **The TypeScript and Go stacks vendor the engine but do not call `prose_explain`.** Their `trace_explain` tools exist (exemplar A4), but no `ledger::prose_explain` / `load_telemetry` call site appears outside the rust `rust-ai-native-cli` crate (grep over `typescript-ai-native-lang/v0.6.0`, `go-ai-native-lang/v0.1.0`, excluding `vendor/`/`vibedeps/`).
- **Host (`crates/`, `xtask/`) does not call the ledger at all.** `vibe trace` is a pass-through alias delegating to the installed `rust-ai-native` binary — `crates/vibe-cli/src/cli.rs:215-222` and dispatch `crates/vibe-cli/src/main.rs:217` (exemplar A4 §1) — returning a child exit code, not a `Result` through the ledger. Host `vibe-mcp` ships four tools, none traceability (exemplar A4 §1, `crates/vibe-mcp/src/tools.rs`).

### The other `ledger.rs` — do not conflate

- `rust-ai-native-cli/src/ledger.rs` (in both `rust-ai-native-lang/v0.7.0` and `rust-ai-native-mcp/v0.7.0`) is a **different ledger**: the DEBT/INTENT registry renderer. Its own header (`rust-ai-native-cli/src/ledger.rs:1-8`, per F-159 `harvest/d7a-core-sync-reverify.md:748-754`) renders *«the two BROWNFIELD §3 registries: `discipline/DEBT.md` from `debt.json` … and `discipline/INTENT.md` from `intent.json`»*. It is re-exported as `rust-ai-native-cli/src/lib.rs:41` `pub use ledger::run_ledger_render`. It shares a filename only; it has no cache, no epoch, no telemetry. Named here because it is the artefact most easily mistaken for the LEDGER-INTENT cache.

### Vendored copies of the engine crate (an engine edit is a release event)

- **The authored engine** is `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/` (the subject). `sync-engines.toml` declares it the single source: every `[[sync]]` block sets `source_root = "vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates"` and lists `core-ai-native-specmap` among the crates mirrored. The file's header states the contract: *«Crates are AUTHORED once and every other home is a byte-identical vendored copy … `--check` gates drift in self-check; the fix surface is ALWAYS the authored copy, never a vendored one.»*
- **Six vendored copies** of `core-ai-native-specmap/src/ledger.rs` under `vibevm/vibepacks/org.vibevm.ai-native/*/v*/crates/vendor/` (the packet's perimeter), each a `cargo xtask sync-engines` mirror of the v0.8.0 authored file:
  1. `rust-ai-native-lang/v0.7.0/crates/vendor/core-ai-native-specmap/src/ledger.rs`
  2. `typescript-ai-native-lang/v0.6.0/crates/vendor/core-ai-native-specmap/src/ledger.rs`
  3. `go-ai-native-lang/v0.1.0/crates/vendor/core-ai-native-specmap/src/ledger.rs`
  4. `rust-ai-native-mcp/v0.7.0/crates/vendor/core-ai-native-specmap/src/ledger.rs`
  5. `typescript-ai-native-mcp/v0.6.0/crates/vendor/core-ai-native-specmap/src/ledger.rs`
  6. `go-ai-native-mcp/v0.1.0/crates/vendor/core-ai-native-specmap/src/ledger.rs`

  **Byte-identity verified:** `diff -q` between the rust-lang vendor copy and the v0.8.0 authored file returns no differences; all three compared copies (v0.8.0 authored, v0.7.0 authored-slot, rust-lang vendor) are 303 lines.
- **A second authored version slot exists:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.7.0/crates/core-ai-native-specmap/src/ledger.rs` (303 lines, byte-identical `ledger.rs` to v0.8.0) — this is the `flow-core-ai-native/0.7.0` flow-package copy, a separately-versioned authored engine, not a `vendor/` mirror. The vendor copies pin the stacks to 0.7.0-level consumers but mirror the **v0.8.0** authored engine per `sync-engines.toml`.
- **Further mirrors under `vibedeps/**`** (`flow-core-ai-native/0.7.0`, `flow-delegation-rules/0.1.0`, `stack-rust-ai-native-lang/0.7.0`, etc.) are excluded as evidence per the perimeter — they are regenerated dep copies of the same crates, not independent implementations.
- **Consequence for any mechanism build:** an edit to `ledger.rs` propagates to the six vendor copies via `cargo xtask sync-engines`, gated by `--check` in self-check, and is a **release event** for the six consumer packages (each ships the byte-identical crate under its own version pin).

---

## Method and freshness

- **Re-measured at HEAD `779b3aaa` (this worktree):** every struct/fn/const citation in `ledger.rs`; the vendored-copy count and byte-identity (`diff -q`, `wc -l`); the caller inventory (`grep` for `ledger::`, `prose_explain`, `ProseRender`, `get_or_compute`, `run_trace_explain` over ENGINE + the three driver packages + HOST, excluding `vendor/`/`vibedeps/`); the absence of LRU/pin/budget/enum/signing fields and crypto deps (terms listed per section); `vibe-llm` size; `sync-engines.toml` contents; `.gitignore:49-50`; `terraform/REPORT.md:21,40,41`. `.ledger/` was checked at the worktree root (absent — gitignored derived data).
- **Carried (not re-measured) and flagged as such:** the live `.ledger/objects/` count of 1 object and the `telemetry.json` value `{hits:1, misses:1, rot_checks:0, rot_changed:0}` come from the prior measurement F-159 (`harvest/d7a-core-sync-reverify.md:795-796, 810-812`), taken at the host main checkout; this worktree holds no `.ledger/` because it is fresh and the directory is gitignored. Its presence is environment state, not a property of HEAD.
- **Perimeter named with every zero:** each absence claim above states the layers searched (ENGINE / DRIVERS / HOST / DEPLOYMENT) and the search terms; `refs/**` and `vibedeps/**` are excluded per the packet (F-159 `harvest/d7a-core-sync-reverify.md:857-861` separately reports `refs/` carries only third-party mentions of `LEDGER-INTENT`, not an implementation).
- **Figures are properties of this HEAD.** All counts and citations are as of `779b3aaa`; the vendored copies are byte-identical to the authored engine at this HEAD.
- **Read-only throughout.** No build, no test run, no `vibe` command, no git, no writes outside this one file.
