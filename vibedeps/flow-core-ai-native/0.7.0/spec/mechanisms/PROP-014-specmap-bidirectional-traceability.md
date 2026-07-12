# PROP-014 — specmap: bidirectional spec↔code traceability and the source metamodel

**Status.** Design proposal v0.1 — not implementation-locked. Drafted for review; every decision below is open to challenge until ratified.

**Companions** (vibevm-hosted, the pilot project's spec tree — cited as context, not shipped here): PROP-000 (foundation, license policy §3), PROP-003 (the LLM-boundary philosophy this PROP extends), PROP-009 (boot/loading model — specmap becomes its intra-project counterpart), PROP-013 (category C "drift" — specmap mechanizes its detection), Red Book ch. 2 (files as IPC) and ch. 3 (Sync-from-Code — specmap is its instrumentation).

**Home.** `flow:org.vibevm.ai-native/core-ai-native`, `spec/mechanisms/` — this mechanism ships with the Discipline (URIs `spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#…`); its Rust implementation ships in `stack:org.vibevm.ai-native/rust-ai-native-lang` (`specmap-core` + the `rust-ai-native-specmap` binary). The tag *syntax* shown throughout (`#[spec]`, `scope!`) is the Rust projection; other language stacks ship their own projection of the same model.

---

## 1. Problem statement {#problem}

vibevm is ~52K lines of Rust governed by ~20 PROP documents plus a 99KB owner-frozen spec. The linkage between the two layers exists today only as (a) prose cross-references in doc comments, (b) `spec://` URIs cited in commit bodies per the Rule-2 discipline, and (c) the maintainer's head. AUDIT category C (drift) is detected by periodic human sweep. As the codebase grows, the question "which code realises this decision?" and its inverse "which decision justifies this code?" become unanswerable at acceptable cost.

The desired end-state, by analogy with JS/TS source maps: from any compiled artefact you can reach the source; from any source you can reach the artefacts. One spec unit may touch many code items; one code item may serve many spec units. Many-to-many is undesirable but real; the system must represent it while linting against its growth.

### 1.1 Where the source-map analogy holds, and where it breaks {#analogy}

Source maps work because a **compiler emits them as a free, deterministic byproduct of every build**. Between a vibevm spec and its Rust realisation there is no compiler — there is a human or an LLM session. Consequences:

1. The mapping cannot be *generated*; it can only be *carried and verified*. Until M1.5's `vibe build` exists, every edge is authored metadata.
2. Authored metadata rots unless three forces hold simultaneously:
   - **Edges travel with the artefacts.** Code-side links live *in the code* (attributes on items) and survive any refactor that moves the item. Spec-side links live *in the spec* (stable anchors). External sidecar maps are rejected (§5.1).
   - **Invariants are machine-checked.** Dangling references, uncovered requirements, orphan code, and — the load-bearing one — **staleness**: a spec unit carries a revision + content hash; when it changes, every edge pinned to the old revision flips to *suspect* until re-affirmed.
   - **The map is load-bearing in daily work.** A map that is only audited dies (the classical requirements-traceability graveyard). specmap must feed (i) agent context paging — working on a REQ pulls its code, editing an item pulls its specs; (ii) `vibe explain`; (iii) error provenance — failures cite the violated REQ.
3. **M1.5 convergence.** Once `vibe build` generates code from specs, the generator emits specmap edges as a true compiler byproduct — the analogy becomes literal. Hand-authored tags remain as the human-override lane. This PROP defines the format that the future generator will target.

### 1.2 The runtime vision (AI-native open source) {#runtime-vision}

For an open-source project, the metamodel (and, on demand, the source behind it) is exposed to *consumers* of the tool at runtime: an agent driving `vibe` can ask not just `--help` but "why does `vibe install` behave this way, under which decisions, realised where, with which known deviations" — and receive a structured subgraph or a rendered explanation. Distribution rides the existing registry: the metamodel index ships with the package; source fragments are fetched by content hash. Closed-source projects ship a redacted profile (§2.8.3).

---

## 2. Decisions {#decisions}

> Deliverable (в) of the design brief — the binding model — is this section in its entirety.

### 2.1 Addressing: spec-side {#addressing-spec}

`req r1`

**Decision.** Extend the existing `spec://` URI scheme into the canonical spec-side address:

```
spec://<package>/<doc-path>#<anchor>            — a spec unit
spec://<package>/<doc-path>#<anchor>~r<N>       — a unit at revision N
```

- `<package>` is today the repo name (`vibevm`); the grammar reserves group-qualification (`spec://org.vibevm.world/wal/...`) for cross-package tracing per PROP-008, deferred (§7.1).
- `<anchor>` is the explicit `{#kebab-anchor}` already used by every PROP heading. **Anchors are immutable once published and never reused.** Renaming a unit keeps its anchor; retiring a unit tombstones the anchor (`<!-- RETIRED: superseded by #new-anchor -->`) rather than deleting it.
- A **spec unit** is the span from an anchored heading (or an explicit `REQ` block, §2.2) to the next same-or-higher heading / next unit marker.

### 2.2 Spec units, normativity, and the two-tier revision discipline {#spec-units}

`req r1`

**Decision.** Four unit kinds, each with a different default edge semantics:

| Kind | Carries | Typical edges |
|---|---|---|
| `prop` | a decision + rationale ("why") | `decides`, referenced by REQs |
| `req` | a normative contract (RFC-2119 MUST/SHOULD/MAY) | `implements`, `verifies` |
| `design` | shape of a solution ("how", non-binding) | `informs` |
| `guide` | usage documentation | `documents` |

A unit declares normativity with a one-line marker directly under its heading:

```markdown
### Conditional dependencies resolve to a fixed point {#req-conditional-fixpoint}
`req r2` — predicates are evaluated against resolved project state; each
pass MUST only add requirements (monotone), guaranteeing convergence.
```

**Revisions are two-tier:**

- `r<N>` is an **author-asserted semantic revision**. Bump it only when the *meaning* changes. Editorial edits (typos, wording) do not bump.
- The indexer computes a **content hash** of the unit text. Hash changed while `r` did not → `vibe trace` warns: *"editorial-or-forgot-to-bump — confirm."* This catches the human failure mode without making typo fixes expensive. (Prior art: OpenFastTrace's `~rev` integers; Doorstop's reviewed-hash stamps — ideas only, see §6.)

**Asymmetric invalidation rule (load-bearing).**

- Spec unit `r` bumps → every edge pinned to the old `r` becomes **suspect**; CI gate lists them; each is cleared by re-affirming (updating the pin) after review.
- Code item changes → linked edges stay **valid** (the contract didn't move; implementation detail is free to change). Exception: edges of type `deviates` flip to *review* on either side changing, because a deviation is a statement about both sides.

### 2.3 Addressing: code-side — tags that travel {#addressing-code}

`req r1`

**Decision.** Code-side links are inert attributes provided by a tiny `specmark` crate (workspace-internal at first; publishable later). The attribute is a no-op for the compiler — its consumers are (a) the source scanner and (b) rustdoc, into which the macro injects a rendered "Spec:" line so the link is visible in generated docs.

```rust
use specmark::spec;

/// Parses the `context(<key>)` predicate grammar.
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#conditional-deps", r = 2)]
pub enum ConditionalPredicate { /* … */ }

#[spec(deviates = "spec://vibevm/modules/vibe-resolver/PROP-003#conditional-deps", r = 2,
       reason = "boolean composition (`and`/`or`/`not`) intentionally unimplemented; \
                 surfaces as PredicateError::Unsupported pending PROP-014-pilot decision")]
impl ConditionalPredicate {
    pub fn parse(raw: &str) -> Result<Self, PredicateError> { /* … */ }
}

#[cfg(test)]
mod tests {
    use specmark::verifies;

    #[test]
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#conditional-deps", r = 2)]
    fn fixed_point_is_monotone() { /* … */ }
}
```

Grammar (one edge per attribute; attributes repeat for multiple edges):

```
#[spec( <verb> = "<spec-uri>" [, r = <N>] [, reason = "<text>"] )]
#[verifies("<spec-uri>" [, r = <N>])]            // sugar for tests
specmark::scope!("<spec-uri>" [, r = <N>]);      // module-level inheritance marker
```

Rules:

- **Verbs:** `implements`, `verifies`, `documents`, `deviates`, `informs`. `deviates` REQUIRES `reason`.
- **Unit of code = the item** (fn, struct, enum, trait, impl block, mod). Never lines, never expressions. Line/column spans appear only in the *derived* index (§2.5), where volatility is harmless because the index is regenerated.
- **Scope inheritance.** `specmark::scope!(…)` at the top of a module gives every item inside a default `implements` edge unless the item carries its own `#[spec]` (own tags **replace** the inherited set in v0.1; merge syntax is an open question, §7.2). Private helpers therefore usually need no annotation. Rust note: a true inner attribute (`#![spec(…)]`) on modules is unstable for proc-macros, hence the macro-invocation form.
- **Generated code** (e.g. `vibe-wire/src/generated/`) is excluded from orphan checks via a directory marker; the *generator input* (JTD schema file) is the taggable unit instead.
- **Error enums are contract.** Every public error variant whose meaning comes from a REQ carries `implements` on the variant's enum (or `#[spec]` on the variant where precision pays). This is what lets a failing command cite the violated requirement (§2.6).
- **Multiplicity lint.** An item carrying more than **3** spec edges is flagged by `vibe check`: either the item does too much or the spec units are cut too fine. (Threshold configurable; mirrors the activation-conflict lint philosophy of PROP-003 §2.10.)

### 2.4 The edge model {#edges}

`req r1`

A **typed, directed property multigraph**:

- **Nodes:** `SpecUnit { uri, kind, r, content_hash }`, `CodeItem { symbol_path, item_kind, crate, content_hash }`, plus derived `Command`, `ErrorVariant` views.
- **Edges:** `(CodeItem) --implements/verifies/documents/deviates/informs--> (SpecUnit @ r)`, each with provenance (`authored` | `generated` | `proposed`) and, for `deviates`, the mandatory reason. *(Brownfield amendment:)* spec units additionally carry a lifecycle status (`ratified` | `planned` | `disputed` | `retired`), and a spec↔spec edge `conflicts_with` records detected contradictions; edges into `disputed` units are frozen — exempt from suspect-clearing — until adjudication. Coverage math reports `planned` scope separately and never penalizes it.
- **Direction of authority:** spec → code (the Red Book's top-down flow). The reverse direction is *computed* (the index inverts edges) plus one social channel: a `proposed` edge pool (§4, Phase 2) feeding the Sync-from-Code protocol when code grows meaning the spec lacks.

### 2.5 The index: `specmap.json` {#index}

`req r1`

**Decision.** A derived, deterministic, committed artefact, regenerated by `cargo xtask specmap` and gated by `cargo xtask specmap --check` in CI — the exact idiom `check-codegen` already established.

- Built by a source scanner (syn or tree-sitter over the workspace; markdown parser over `spec/**`). No macro expansion needed — `#[spec]` is read as text/AST, which also makes the JS/Python bindings (§2.9) uniform.
- Canonical JSON (stable ordering), schema under `schemas/specmap.jtd.json` → `vibe-wire` types via the existing codegen pipeline.
- Contents: all nodes (with content hashes and *current* file:line spans), all edges, plus computed tables: coverage per REQ (`{implemented, verified, documented}` bits), orphans (public items with no edge own-or-inherited), suspects (edges whose pinned `r` < unit's current `r`), unbumped-hash warnings.
- **Determinism is a tested property**, same as the resolver's (PROP-003 §3.3): index twice, assert byte-identical.

### 2.6 Query surface {#queries}

`req r1`

```
vibe trace coverage [--crate X] [--kind req]      # matrix: REQ × {impl, test, doc}
vibe trace impact <spec-uri>                      # all items/tests reachable from a unit
vibe trace orphans [--ratchet-file …]             # unjustified public items
vibe trace stale                                  # suspect edges + unbumped-hash warnings
vibe explain <command|symbol|spec-uri> [--json|--text|--prose]
```

- `--json` emits the raw subgraph (agent-friendly); `--text` a deterministic structured rendering; `--prose` an LLM rendering of the same subgraph. **The tool MUST be fully useful without an LLM** — `--prose` is a presentation layer, never the data layer.
- **Error provenance:** `vibe`'s error rendering looks up the failing error variant in the index and appends `violates spec://…#req-… (r2) — run: vibe explain <uri>`. This is the single highest-leverage consumer: every failure becomes a doorway into the metamodel.
- During the pilot these live behind `cargo xtask trace …` to avoid touching the CLI surface prematurely; promotion to `vibe trace` / `vibe explain` is a Phase 4 decision (§4).

### 2.7 The LLM boundary {#llm-boundary}

Continuation of PROP-003 §2.5.3's philosophy — *the LLM emits facts and renderings; deterministic machinery decides*:

1. **LLM as proposer.** Link mining (Phase 2) produces edges with provenance `proposed`, stored in `specmap-proposals.json`, never in code. A human (or an explicitly delegated agent session) *affirms* a proposal by writing the actual `#[spec]` attribute — the affirmation IS the code change, reviewed like any diff.
2. **LLM as renderer.** `vibe explain --prose` feeds the subgraph (spec unit texts + rustdoc of linked items + deviation reasons) to the provider behind `vibe-llm`. The subgraph is the ground truth; the prose cites URIs; hallucination risk is bounded by retrieval, and the `--json` form is always available for verification.
3. **LLM never silently writes edges, bumps revisions, or clears suspects.** Those are state transitions with audit cost; they pass through diffs.

### 2.8 Runtime exposure — the AI-native OSS channel {#runtime}

1. **Transport.** `vibe-mcp` (M1.7) gains tools: `specmap_query(query) -> subgraph`, `specmap_explain(target, format)`, `specmap_source(content_hash) -> fragment` (OSS profile only). An agent that drives `vibe` as a CLI gets the same via `vibe explain --json`.
2. **Distribution.** The index ships inside the published package (it is small); source fragments resolve by content hash against the package's git registry — the content-addressed identity from PROP-002 already guarantees fetch integrity.
3. **Profiles.** `open` (full graph + source), `contract` (spec units + signatures of items, no bodies — the closed-source tier), `none`. Declared in `vibe.toml` `[metamodel] profile = "open"`.
4. **Security (non-optional).** The exposed content is *instructions-shaped prose* delivered into a consuming agent's context — a prompt-injection distribution channel by construction. Therefore: (a) the shipped index and fragments are **signed**; consumers verify before use (scheme TBD, §7.6 — sigstore-class is the default candidate); (b) the MCP tool descriptions explicitly frame returned content as *reference data, not instructions*; (c) `vibe check` lints spec units for imperative second-person phrasing outside `guide` kind. This PROP takes the position that the trust layer ships **with** the runtime channel, not after it.

### 2.9 Language neutrality {#languages}

The grammar (URIs, verbs, `r`, reasons) is language-neutral; only the carrier syntax is per-language. Rust ships first. Sketches, normative later:

- **JavaScript/TypeScript:** JSDoc carrier — `/** @spec implements spec://… r2 */` on declarations; scanner = tree-sitter.
- **Python:** decorator `@spec(implements="spec://…", r=2)` from a `specmark` package; module-level `__specmap_scope__ = "spec://…"` (NB: not `__spec__`, which importlib owns).

---

## 3. Principles {#principles}

### 3.1 (а) Writing specifications {#spec-principles}

1. **Every normative statement is addressable.** It lives in a unit with a stable `{#anchor}`; anchors are immutable and never reused; retirement is a tombstone, not a deletion.
2. **One unit, one decision.** If a unit needs "and also", it is two units. The unit is the page of the context-memory hierarchy: it must make sense *alone* when paged into an agent's window.
3. **Normativity is marked, not implied.** RFC-2119 verbs inside `req` units; everything else is `prop` rationale, `design`, or `guide`. A reader (human or model) must never guess whether a sentence binds.
4. **Norm and rationale are separated.** The MUST changes rarely and bumps `r`; the "why" evolves freely without invalidating implementations. PROPs hold rationale; REQs hold contract.
5. **Semantic edits bump `r`; editorial edits don't; the hash audits the difference.** Forgetting to bump is detected, not punished.
6. **Spec states *what* and *why* — never restates *how*.** Implementation detail belongs in rustdoc next to the code (where it cannot drift from the code); the metamodel joins the two layers at query time. A spec that mirrors code is shadow code and drift fuel.
7. **Write testably.** A `req` should imply its verification; if you cannot imagine the `#[verifies]` test, it is `design`, not `req`.
8. **Deviations are first-class and honest.** When reality intentionally differs, the code says `deviates` + reason — the generalisation of the existing `<!-- REVIEW: … -->` discipline. An undocumented deviation found by audit is a defect.
9. **Cross-reference by URI only.** No "see above", no relative prose pointers — they don't survive paging or reorganisation.
10. **Units fit a page.** Soft target ≤ 120 lines per unit; `vibe check` warns beyond. Long units page badly and hash-churn often.

### 3.2 (б) Writing Rust under specmap {#rust-principles}

> Deliberately *not* a general style guide — only what traceability and the metamodel require. House rules (clippy `-D warnings`, `forbid(unsafe_code)`, etc.) stay where they are.

1. **The item is the unit of meaning.** Shape code so each public item serves few spec units (≤ 3 edges; lint beyond). If an item needs more, split the item or merge the units.
2. **Tags travel with code.** `#[spec]` on items; `scope!` per module for inheritance; private helpers inherit silently. Moving a function moves its link; that is the entire point.
3. **Typed verbs, no bare links.** `implements` ≠ `documents` ≠ `deviates`; the verb is what makes the graph queryable.
4. **Tests declare what they verify.** `#[verifies(uri, r)]` on the test, not a comment. Coverage = REQ × {impl, test} computed, not estimated.
5. **Rustdoc is the detail layer.** Every tagged public item's doc comment states the *practically important* behaviour — errors, edge cases, performance traps. `vibe explain` composes spec (contract) + rustdoc (detail); neither duplicates the other.
6. **No orphan public API.** Every `pub` item is reachable from an edge, own or inherited. Ratcheted: warn → error per crate as migration lands (§4).
7. **Generated code is excluded; its generator input is tagged.** Schema files and macro definitions carry the edges; expansion output is marked generated.
8. **Errors are contract surface.** Public error types/variants that signal a requirement carry its edge, enabling error-message provenance. An error no spec explains is an undocumented behaviour.

### 3.3 (в) Binding principles {#binding-principles}

Section 2 *is* deliverable (в). For reading convenience, the five load-bearing invariants: edges travel with artefacts (§2.3); two-tier revisions with asymmetric invalidation (§2.2); derived deterministic committed index with a CI gate (§2.5); the tool is fully functional without an LLM, and the LLM only proposes and renders (§2.6–2.7); the runtime channel ships signed or not at all (§2.8.4).

---

## 4. (г) Migration playbook — transforming vibevm with Claude Code {#migration}

**Strategy: easy wins first, ratchet always, never gate the whole repo on day one.** (The maximum-perfection horizon — full backfill of all 12 crates, JS/Py bindings, signed runtime channel — is Phase 5+, listed for honesty, not for scheduling.)

### Phase 0 — tooling skeleton (≈ half a day)

- `crates/specmark/`: the no-op attribute + `scope!` + `verifies` macros (syn parse of the grammar, rustdoc line injection, zero runtime cost).
- `xtask specmap` subcommand: markdown unit parser + syn-based item scanner + canonical JSON emitter; `--check` mode (regenerate-and-diff, the `check-codegen` idiom).
- `schemas/specmap.jtd.json` + codegen.
- Acceptance: index builds deterministically twice on the untouched repo (zero edges, full node inventory); CI job wired but non-blocking.

### Phase 1 — pilot: PROP-003 §2.6.1 × `vibe-resolver/src/conditional.rs`

The smallest real loop, chosen deliberately: fresh spec, ~130-line module, and it carries a live design question (the `not`/monotonicity issue) that becomes the first officially traceable REQ with a recorded `deviates`.

1. Unit-ify §2.6.1: add `req` markers + anchors for (i) the fixed-point/monotonicity contract, (ii) the predicate grammar, (iii) the host-invariance rule. *Anchors added, no prose rewritten* — additions only, owner-frozen text untouched pending sign-off.
2. Tag `conditional.rs`: `implements` on the enum and `parse`, `deviates` (+ reason) for unimplemented boolean composition, `verifies` on its tests.
3. **Drift drill (the acceptance that matters):** semantically edit the fixed-point unit, bump `r` → gate flags the suspect edges; re-affirm → gate clears. Then edit a typo *without* bumping → hash warning fires. Both behaviours demonstrated in one PR description.
4. `xtask trace explain conditional::ConditionalPredicate::parse --text` emits a correct subgraph.

### Phase 2 — backfill `vibe-resolver` with Claude Code

Two link sources, both flowing through the `proposed` pool (§2.7), affirmed by diff review:

**(a) Latent corpus mining** — the repo already cites `spec://` in commit bodies (Rule 2):

```
git log --all --pretty='%H %s' --grep='spec://' | …
```

Each (commit → files touched → URIs cited) triple seeds proposed edges with evidence pointers.

**(b) Crate sweep.** Claude Code prompt (guardrails included):

```text
Read spec/modules/vibe-resolver/*.md and crates/vibe-resolver/src/.
For every public item, propose at most 3 specmap edges using the
PROP-014 §2.3 grammar. For each proposal output: item path, verb,
spec URI + r, a one-line evidence quote from BOTH sides, and a
confidence (high/medium/low). Do NOT edit any file. Do NOT propose
edges where you cannot quote evidence from the spec side — mark the
item "candidate orphan" instead. Emit specmap-proposals.json only.
```

Affirmation session prompt:

```text
Take specmap-proposals.json entries marked APPROVED in the review
file. Write the corresponding #[spec]/#[verifies]/scope! annotations.
One commit per module, Conventional Commits, body citing the spec://
URIs added. Run `cargo xtask specmap --check` and `cargo test -p
vibe-resolver` before each commit. Touch nothing outside
crates/vibe-resolver and the proposals file.
```

- Acceptance: `vibe-resolver` coverage report ≥ 90% of `req` units implemented-and-verified; orphan list for the crate empty or dispositioned in AUDIT; gate flipped to blocking *for this crate only* (the ratchet file lists exempt crates).

### Phase 3 — expansion + metrics

- Crate-by-crate (suggested order: `vibe-core` → `vibe-install` → `vibe-registry` → CLI last), each flipping its ratchet entry.
- **Instrument the economics** — the empirical answer to "will this rot": stale-edge half-life after a normal refactor week; proposals-to-affirmation lag; % of PRs touching tagged items that also touch their pins. Targets set after two weeks of data, recorded in AUDIT.

### Phase 4 — surfaces

- Promote `xtask trace`/`explain` → `vibe trace` / `vibe explain` (`--json/--text/--prose`).
- Error provenance wiring in `vibe-cli` error rendering.
- `vibe-mcp` tools per §2.8 — **blocked on the signing decision (§7.6)**; ships signed or not at all.

---

## 5. Rejected alternatives {#rejected}

1. **External sidecar map only** (a `specmap.toml` maintained by hand or by tool, no in-source tags). Rots immediately: without a compiler regenerating it, every refactor silently invalidates spans and symbol paths. Kept only as the *derived* index (§2.5), where regeneration is the lifecycle.
2. **Line/range anchors** ("PROP-003 lines 410–462", `src/naive.rs:118-160`). Maximally precise and maximally fragile; every upstream edit shifts them. Spans are demoted to derived-index decoration.
3. **Embedding-similarity recovered links as ground truth.** Non-deterministic core, unexplainable diffs, silent drift. Allowed exactly once in the lifecycle: as a *proposer* in Phase 2, behind human affirmation.
4. **Literate programming / tangle** (spec is the single source; code is extracted). Inverts authority correctly but destroys the entire Rust toolchain experience (rust-analyzer, incremental compile, grep-ability) and forces the spec to carry *how*. The Red Book's layer model (spec=meaning, code=detail) is the opposite bet, deliberately.
5. **External requirements database** (DOORS/Doorstop-style items outside the repo). Violates "project facts live in the repo" (CLAUDE.md memory discipline) and splits the review surface. Everything here is files in git — the book's ch. 2 thesis.
6. **Full formal specification** (TLA+/Kani/Dafny for the contracts). Wrong genre for prose contracts and process disciplines; *complementary* for isolated algorithmic kernels — the conditional-deps fixed point is a natural first candidate if we ever want a machine-checked model, and the specmap edge type for it would be `verifies`.

## 6. Prior art and license posture {#prior-art}

Conventions and ideas are free; code is not. Per PROP-000 §3 (permissive only; GPL/AGPL/LGPL forbidden as dependencies), roles below are explicit. License fields to be re-verified before any code-level reuse.

| System | License (verify) | Role here |
|---|---|---|
| OpenFastTrace | GPL-3.0 | **Study only.** Borrowed *ideas*: artifact-type chains (req→dsn→impl→utest), `~rev` semantics, coverage states. No code, no linkage. |
| strictdoc | Apache-2.0 | Friendly. Grammar/UI patterns for requirement documents. |
| Doorstop | LGPL-3.0 | Wrapper-zone per policy if ever executed; borrowed *idea*: reviewed-hash stamps (our two-tier revisions). |
| Sphinx-needs | MIT | Friendly. Typed needs/links, filter queries. |
| DO-178C / DOORS culture | n/a (standards) | The cautionary tale §1.1 is built on: traceability that is audited but not load-bearing dies. |
| JS source maps / DWARF | n/a | The analogy and its precise failure point (§1.1). |
| syn / tree-sitter | MIT/Apache-2.0; MIT | Implementation dependencies for the scanner. |
| sigstore | Apache-2.0 | Default candidate for §2.8.4 signing. |

**Differentiators vs. classical requirements traceability:** (i) the map is consumed at *runtime by agents using the tool*, not only at audit time; (ii) an LLM participates — strictly as proposer and renderer behind a deterministic core; (iii) the map doubles as the context-paging table for agent sessions (PROP-009's intra-project counterpart); (iv) specs are package-distributed artefacts (vibevm itself), so tracing composes across the registry.

## 7. Open questions {#open}

1. **Cross-package URIs.** Group-qualified `spec://org.vibevm.world/wal/...` grammar and resolution against installed packages — after PROP-008 settles live.
2. **Inheritance merge.** v0.1: item tags replace `scope!` defaults. Is a `+implements` extend form needed? Decide on Phase 2 evidence.
3. **Unit moves across documents.** Anchor immutability covers renames-in-place; moving a unit between files needs either URI redirect stubs (PROP-012 flavour) or doc-path-free unit IDs. Lean: redirect stubs.
4. **Explanation caching.** `--prose` renderings keyed by (subgraph hash, model id) — where cached, when invalidated.
5. **Thresholds.** 3 edges/item, 120 lines/unit — placeholders until Phase 3 metrics.
6. **Signing scheme.** sigstore vs. minisign-class vs. registry-native git signatures; decide before Phase 4's MCP exposure; blocking for §2.8.
7. **Non-OSS `contract` profile.** Exactly which item metadata (signatures? doc comments?) is safe to ship; needs a real closed-source consumer to decide.
8. **Commit-message integration.** Rule 2 already cites `spec://`; should commits citing a REQ auto-link into the index as `informs` provenance? Cheap, probably yes; confirm noise level on Phase 2 history.

---

*This PROP is a design proposal. Ratification — and the `specmark`/xtask implementation start — happens through PR review against this document. Any mechanism specified here that is not exercised by the end of Phase 2 is removed from the spec rather than carried as aspirational documentation.*
