# DRIFT-004 — specmap learns `##<ID>` fact anchors (owner commission) {#root}

<status stage="impl" state="done" ref="DRIFT-004"/>

**Status:** done — executed by Opus 2026-07-24, reviewed and accepted by
Fable the same day (diff read at the seam hunks: fact_anchor_at /
list_item_content mirror the host scanner without importing it,
segment_block_facts implements the span rule, dedup interleaves with
the heading walk in document order; 81 tests green incl. 13 new
fact-anchor cases; is_valid_anchor byte-identical; sync-engines
--check green — v0.8.0 is vendored nowhere; self-check all green,
exit 0 via sentinel). Accepted clarification: task §4.3's
`##not valid!` example was imprecise under first-token semantics (it
mints fact `not`); genuinely-invalid heads (`##9bad`, `##bad!`,
`##!`, `###`) are the tested ignore cases — matches the host
reference. No schema change was needed (SpecUnit already models
untyped units); the §8 stop rule did not fire.
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** package — `core-ai-native` v0.8.0 (the OPEN, unreleased line;
the host pins 0.7.0 and is untouched by this task)
**Unit-stability check (release precondition):** the governing contract
text (PROP-014 §2.1 fact-anchor amendment) landed 2026-07-24, same
session, before this task was authored.
**Origin:** owner commission, in session 2026-07-24: «научи specmark
`##<ID>` как first-token параграфа/элемента списка. Это должно работать
для всех поддерживаемых языков (rust, typescript, go)» — the language
families all vendor this one engine, so the neutral-core change IS the
all-languages change.

## 1. Goal {#goal}

The spec-side scanner (`core-ai-native-specmap`, `mdspec.rs`) extracts
`##<ID>` fact anchors as addressable spec units, so code in any language
can cite `spec://<pkg>/<doc>#<FACT-ID>` and the join/check machinery
resolves it; the anchor-id grammar crate gains the fact-id validator
while the heading kebab-only law stays untouched.

## 2. Contract {#contract}

> A `##<ID>` written as the **first token of a paragraph or a list item**
> (at any nesting depth, outside fenced/inline code) mints a **fact
> unit** — the finest addressable grain. `<ID>` follows
> `[A-Za-z][A-Za-z0-9_-]*` … Fact ids share **one address space with
> heading anchors per document** … a duplicate id (fact-vs-fact or
> fact-vs-heading) is an extraction warning. Heading anchors keep the
> kebab-only law; the wider id grammar applies to `##` ids only.
> — PROP-014 §2.1 (fact amendment, 2026-07-24),
> `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.xml`

> Fact units carry no `kind:` line (§2.2 typing stays a heading-unit
> discipline); their normativity signal is the id register. Edges cite
> them exactly as heading units.
> — PROP-014 §2.1, same amendment

## 3. Current state {#current}

- `…/v0.8.0/crates/core-ai-native-specmap/src/mdspec.rs` — `parse_units`
  extracts units from **anchored headings only** (`parse_heading`,
  span to next same-or-higher heading); `fence_mask` already exists and
  masks fenced code; per-doc `seen_anchors` dedup exists.
- `…/v0.8.0/crates/core-ai-native-specmark-grammar/src/lib.rs:105` —
  `is_valid_anchor` enforces kebab-case (correct for headings; would
  reject `##UPPER-SLUG` fact ids if reused as-is).
- The reference implementation of the fact-anchor recognition (id
  charset, first-token rule, list-marker handling, fence opacity) is the
  host's `crates/progress-core/src/parse/facts.rs::take_fact_id` +
  `blocks.rs`/`facts.rs` — READ them for semantics, do NOT import them
  (PROP-043 §2 separability: no code coupling between the two scanners;
  the convention is held by tests).

## 4. Change {#change}

1. **Grammar crate** (`core-ai-native-specmark-grammar`): add
   `pub fn is_valid_fact_id(id: &str) -> bool` implementing
   `[A-Za-z][A-Za-z0-9_-]*`. `is_valid_anchor` stays byte-identical.
2. **mdspec** (`core-ai-native-specmap`): in `parse_units`, after the
   existing heading walk (or interleaved with it — executor's call),
   recognise fact anchors on non-fence lines:
   - a line whose first token is `##<ID>` (paragraph form), or a list
     item — `-` / `*` / `+` / `N.` / `N)` marker at any indent — whose
     first token after the marker is `##<ID>`;
   - `##` immediately followed by a valid fact id and then
     whitespace/EOL; anything else (e.g. `##!`, `###`, a heading line —
     headings start `#<space>`-style and are already consumed by the
     heading walk) is not a fact anchor;
   - emit a spec unit for each: anchor = `<ID>`, span = the carrying
     paragraph (to the next blank/structural line) or the item's own
     lines including indented continuations; untyped (no `kind:`
     machinery applies);
   - fact ids validate via `is_valid_fact_id`; an invalid id after `##`
     is skipped silently (it is prose, e.g. a Markdown `##`-run), NOT a
     warning — only a VALID id mints a unit;
   - fact ids join the same per-doc `seen_anchors` dedup — a duplicate
     (fact-vs-fact or fact-vs-heading) emits the existing
     `duplicate-anchor` warning shape.
3. **Tests/fixtures** in the package's own test surface (mirroring the
   host scanner's semantics — the convention-held-by-tests pattern):
   - paragraph fact anchor (kebab) and list-item fact anchor (UPPER)
     both extracted, addressable, spans correct;
   - nested list item's fact anchor is its own unit;
   - `##inside-fence` ignored; `## Heading`-style lines not treated as
     fact anchors; `##not valid!` (invalid id) ignored;
   - duplicate: a fact id equal to a heading anchor warns
     `duplicate-anchor`;
   - a join-level test if the crate has one cheaply reachable: a code
     tag citing `spec://…#FACT-ID` resolves against the extracted unit
     (skip if the join tests live elsewhere — say so in the report).

## 5. Boundaries {#boundaries}

- Touch ONLY
  `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmark-grammar/**`
  and
  `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/**`
  (source + their tests/fixtures).
- Do NOT touch: `v0.7.0/**` (published line), ANY `crates/vendor/**`
  copy anywhere (vendored engines are regenerated, never hand-edited),
  host `crates/**`, `spec/**`, `campaigns/**`, any manifest versions
  (no version bumps in this task — propagation to the language families
  is a separate, later minting).
- Never edit spec text or golden tests. Spec doubts → §8 stop rule.
- ≤600 lines per file after edits (the conform budget).

## 6. Acceptance {#acceptance}

- `cargo test --manifest-path packages/org.vibevm.ai-native/core-ai-native/v0.8.0/Cargo.toml -p core-ai-native-specmap -p core-ai-native-specmark-grammar`
  green, including every §4.3 case.
- `cargo clippy --manifest-path packages/org.vibevm.ai-native/core-ai-native/v0.8.0/Cargo.toml -p core-ai-native-specmap -p core-ai-native-specmark-grammar --all-targets -- -D warnings`
  clean; `cargo fmt` over that workspace clean.
- `is_valid_anchor` behavior byte-identical (existing grammar tests
  untouched and green).
- `bash tools/self-check.sh` green end to end (real exit code, not a
  piped tail).

## 7. Analogies {#analogies}

"Do it like X": the host's `crates/progress-core/src/parse/facts.rs`
(`take_fact_id`, list-item recognition) is the semantic reference —
reimplement the same recognition in mdspec's own idiom over its own
`fence_mask`; do not import or copy-paste across the separability seam.

## 8. Stop rule {#stop}

If the `SpecUnit` model cannot represent an untyped sub-heading unit
without a schema change to `specmap.json` (a field the join/check reads
would need to change shape), STOP and return with the exact field —
a schema bump is a decision above this task. Budget signal: past ~8
files / ~700 changed lines, stop and return with findings.

## 9. Log {#log}

queued 2026-07-24 (Fable, owner commission in session).
