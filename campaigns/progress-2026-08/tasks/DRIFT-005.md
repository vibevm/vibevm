# DRIFT-005 — the spec compiler learns fact inheritance (R1–R4) {#root}

<status stage="impl" state="done" ref="DRIFT-005"/>

**Status:** done — executed by Opus 2026-07-24, reviewed and accepted by
Fable the same day (gate.rs read in full, override helper and diff stat
reviewed; 100 lib + 12 integration tests green; boundaries held —
vibe-workspace untouched, CompileError::DuplicateId surfaces through
the existing InlineCompile wrap; stop rule untouched; self-check all
green, exit 0). Accepted design judgment, enshrined as a PROP-035
S7.3 precision line: the gate flags a repeat only when at least one
occurrence is a fact leaf — a pure heading-vs-heading repeat is the
:add concatenation's own artifact, not a collision.
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** workspace (host `vibe-spec` — doctree / merge / resolver / embed)
**Unit-stability check (release precondition):** the governing contract
(PROP-035 §5 fact leaves + §7.3 fact-inheritance clause, owner-ratified
2026-07-24) landed the same session, before this task was authored.
**Origin:** owner ratification of the F-022 proposal («делай»), in session
2026-07-24.

## 1. Goal {#goal}

The live contract↔source machinery stops being fact-blind: the document
IR carries fact leaves, the `:add` merge performs per-fact override
instead of minting duplicates, the merged view re-gates id uniqueness as
a build error, and the resolver/`#embed` address fact units.

## 2. Contract {#contract}

> **Fact leaves** … A `##<ID>` first token of a paragraph or list item …
> is a **leaf node** of the IR: `kind = fact`, `id = <ID>`, body-span =
> the carrying paragraph or item with its continuation lines; its parent
> is the enclosing section node. Fact ids share the document's one anchor
> namespace. The resolver (§6) resolves a fact address like any node;
> `#embed` of a fact splices exactly its unit …; `#use` of a fact address
> pulls the top-level anchored ancestor of its **enclosing section**.
> — `vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml` §5 (fact amendment)

> 1. **Section fate by default.** Facts ride their section …
> 2. **Per-fact override.** Within a merged `:add` section, a source fact
>    redeclaring a contract fact's `##<ID>` **overrides** it: the contract
>    fact's span is dropped from the merged output and the source's is
>    canonical (last-wins in contract→source order) …
> 3. **The merged view holds uniqueness.** After merging, the compiler
>    re-runs the anchor-uniqueness check — fact and heading ids, one
>    namespace — over the merged document; a surviving duplicate … is a
>    **build error**, never a warning.
> — PROP-035 §7.3, fact-inheritance clause (owner-ratified 2026-07-24)

## 3. Current state {#current}

- `crates/vibe-spec/src/doctree.rs` — the heading-grain IR (nodes from
  `{#anchor}` headings; body spans by the §5 rule). No fact awareness.
- `crates/vibe-spec/src/merge.rs` — `MergeMode` (`Add`/`Replace`),
  `merge_contract_source`, `fold_source`: section-grain, concatenates
  bodies under `Add` — a fact id declared on both sides duplicates in
  the merged text today, and nothing checks the merged output.
- `crates/vibe-spec/src/resolver.rs` / `embed.rs` — anchor resolution and
  `#embed` splicing over the heading-grain tree; a fact address does not
  resolve.
- Fact-anchor recognition reference (do NOT import either; mirror the
  semantics in vibe-spec's own idiom — the convention is held by tests):
  the host scanner `crates/progress-core/src/parse/facts.rs`
  (`take_fact_id`, list markers `-`/`*`/`+`/`N.`/`N)` at any indent,
  whitespace/EOL terminator, fence opacity) and the package twin
  `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/mdspec.rs`
  (`fact_anchor_at`, `list_item_content`, `segment_block_facts`).

## 4. Required behavior {#behavior}

1. **IR fact leaves (R4a).** `doctree` learns fact units: within a
   section's body, a `##<ID>` first token of a paragraph or of a list
   item (any nesting depth, outside fenced code) becomes a child leaf —
   `kind = fact`, `id`, span = the segment's own lines (lead paragraph up
   to the next item/structural break, or the item plus its indented
   continuations). Ids validate as `[A-Za-z][A-Za-z0-9_-]*`; an invalid
   id after `##` is prose (no node, no warning). Fact ids join the same
   per-document id namespace as heading anchors.
2. **Per-fact override in `Add` (R2).** `merge_contract_source` and
   `fold_source`: when both the contract and the source version of a
   section carry a fact with the same id, the contract fact's SPAN is
   omitted from the merged text; the source fact stays in its own
   position. Facts present on one side only, and all non-fact text, are
   carried exactly as today. `Replace` semantics are unchanged (R1).
3. **Merged-view uniqueness gate (R3).** After the merge produces the
   effective document, collect every id (heading + fact) and fail the
   build on any duplicate with a typed error naming the id and both
   occurrences (section/line). Wire it where the pipeline already
   surfaces build errors, so `vibe install`/compilation fails loud.
4. **Resolver + `#embed` (R4b).** A `spec://…#<FACT-ID>` resolves to the
   fact leaf; `#embed` of a fact splices exactly the fact's span; `#use`
   of a fact address maps to the top-level anchored ancestor of the
   enclosing section through the existing ancestor rule (no new rule —
   the fact's parent chain supplies it).
5. Tests (in the touched crates' own test surface):
   - doctree: paragraph fact, list-item fact, nested item, fence-opaque,
     invalid-id-is-prose, fact-vs-heading duplicate detected;
   - merge: `Add` with a redeclared id → override (contract span absent,
     source present, exactly one occurrence); `Add` without redeclaration
     → both sides' facts present; `Replace` → contract facts gone;
   - gate: a non-override duplicate (two different sections) → typed
     build error; clean merge → no error;
   - resolver/embed: a fact address resolves; `#embed` splices the unit.

## 5. Boundaries {#boundaries}

- Touch ONLY `crates/vibe-spec/src/**` (and, if the uniqueness gate's
  wiring point genuinely lives there, the narrowest call site in
  `crates/vibe-workspace/src/boot*` — say so in the report if used).
- Do NOT touch: `packages/**` (both the authored engines and vendored
  copies), `crates/progress-core/**`, any other host crate, `spec/**`,
  `campaigns/**`, manifests/versions.
- Never edit spec text or golden tests. Spec doubts → §8, not
  improvisation.
- ≤600 lines per file after edits (the conform budget); split along
  seams if a file would exceed it.

## 6. Acceptance {#acceptance}

- `cargo test -p vibe-spec` (and `-p vibe-workspace` if touched) green,
  including every §4.5 case.
- `cargo clippy -p vibe-spec --all-targets -- -D warnings` clean;
  `cargo fmt --all` then `--check` clean.
- Existing merge/doctree/resolver tests untouched and green (motion of
  behavior only where §4 names it).
- `bash tools/self-check.sh` green end to end — capture the REAL exit
  code (redirect output to a file; echo `$?` in the same command; never
  a piped tail).

## 7. Analogies {#analogies}

"Do it like X": the fact recognition mirrors
`progress-core/src/parse/facts.rs` and mdspec's `fact_anchor_at` /
`segment_block_facts` — same semantics, vibe-spec's own idiom, no
imports across the separability seams. The typed build error follows
whatever error enum `pipeline.rs` already raises for cycle/link faults.

## 8. Stop rule {#stop}

If per-fact override cannot be implemented without changing the shape of
`MergedSection` or any type a caller outside `vibe-spec` consumes — STOP
and return naming the type; a public-surface change is a decision above
this task. Budget signal: past ~8 files / ~800 changed lines, stop and
return with findings.

## 9. Log {#log}

queued 2026-07-24 (Fable, owner ratification «делай»).
