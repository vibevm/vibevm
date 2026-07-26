# DRIFT-032 — a `spec://` URI can address a normative fact {#root}

```
<status stage="impl" state="plan" ref="DRIFT-032"/>
```

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** common (`core-ai-native-specmark-grammar`, `vibe-spec`)
**Finding:** F-085 (campaign LOG, 2026-07-26).
**Owner ruling, 2026-07-26:** «URI-парсер безусловно должен работать с
идентификаторами фактов, чтобы на них можно было ссылаться и из кода тоже.
Заголовочные якори пока не трогаем.»

**This is a release event (plan §5-D):** the grammar crate lives in
`core-ai-native` and is vendored into five other packages. A fix that is not
propagated ships to one consumer and not the others, and turns the floor red.

## 1. Goal {#goal}

`#[spec(implements = "spec://…#SOME-NORMATIVE-FACT")]` compiles, so code can
cite the `##UPPER-SLUG` anchors Phase B is minting. Heading anchors stay
kebab-only, unchanged.

## 2. Contract {#contract}

```
> `##UPPER-SLUG` names a **normative fact** (a law, rule, carrier, changelog
> entry — content with binding weight); `##kebab-case` names a **service unit**
> — spec://vibevm/modules/vibe-progress/PROP-043#DECISION-TWO-REGISTERS
```

```
> `<ID>` is `[A-Za-z][A-Za-z0-9_-]*`; the unit is then addressable as
> `spec://…/<doc>#<ID>`, sharing one address space with the heading
> `{#anchor}`s — a duplicate across both forms is a `check` error.
> — spec://vibevm/modules/vibe-progress/PROP-043#FACT-ID-GRAMMAR
```

`##FACT-ID-GRAMMAR` already states that a fact is addressable **as a
`spec://…#<ID>` URI**. The URI parser does not implement that sentence. This
task makes the code match a spec unit that already says so — it is not a new
decision, it is a missing implementation.

## 3. Current state {#current}

Verified 2026-07-26 by reading the grammar crate and measuring the corpus.
**Do not re-discover.**

- `is_valid_anchor` (grammar `lib.rs:105`) — kebab-only
  `[a-z0-9]+(-[a-z0-9]+)*`. **This is the heading-anchor law and it does not
  change.**
- `is_valid_fact_id` (`lib.rs:138`) — `[A-Za-z][A-Za-z0-9_-]*`, a **strict
  superset**; its own doc comment says both registers validate here.
- `parse_spec_uri` (`lib.rs:157`) validates the anchor at **`lib.rs:195`** with
  `is_valid_anchor` — the narrow one. That single line is the defect.
- **`is_valid_anchor` has exactly one enforcement site** in the crate — that
  line. Every other mention is a doc-test.
- Measured: of every `spec://…#anchor` cited from `crates/`, **275 are kebab
  and none is UPPER**. DRIFT-031 hit the wall, cited containing *section*
  anchors instead, and recorded the deviation.

**Three sites the fix must reach, and the second is the one that gets missed:**

1. `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmark-grammar/src/lib.rs:195`
   — the authored grammar.
2. **`crates/vibe-spec/src/address.rs:217`** — the HOST's own twin,
   `is_valid_anchor_segment`, whose comment at `:234` says it mirrors "the
   vendored `is_valid_anchor`". Fix only the package and host-side `spec://`
   parsing still rejects UPPER. This is the same separability-seam twin hazard
   DRIFT-031 reported for `list_item_content` — check for others before
   declaring done.
3. The **vendored copies** under `crates/vendor/` in the three `-lang` stacks
   and the `-mcp` packages. `cargo xtask sync-engines --check` is a floor step,
   so an unpropagated fix turns the floor red.

**A test pins the old behaviour and must flip.**
`…/specmark-grammar/src/lib/tests.rs:33` lists `"spec://vibevm/x#A-b"` among
the rejected URIs, commented `// uppercase anchor`. **Moving it to the accepted
set is the point of the task, not a golden being bent to pass** — the owner
ruled the behaviour changes. Say so in the commit body. The two assertions at
`tests.rs:163-166` (`is_valid_fact_id("FACT-A")` true, `is_valid_anchor("FACT-A")`
false) stay **true and unchanged** — a useful signal that the change is
correctly scoped.

## 4. Required behavior {#behavior}

```
1. parse_spec_uri validates the anchor position with is_valid_fact_id
   instead of is_valid_anchor. Update the error message: it currently
   says "must be kebab-case", which stops being the rule.
2. is_valid_anchor is NOT changed. Heading anchors stay kebab-only.
   mdspec.rs:103 and :383 validate heading anchors and keep calling it.
   mdspec.rs:197 already uses is_valid_fact_id; leave it.
3. Apply the same widening to the host twin in vibe-spec/src/address.rs,
   so a host-side spec:// URI accepts what the package's does. The two
   must agree; a divergence here is the defect, not the fix.
4. BEFORE declaring done, verify no downstream consumer normalises or
   lowercases an anchor, and no index is keyed case-insensitively. If
   one is, an UPPER anchor could collide with a kebab one and the
   duplicate-anchor check would miss it. Report what you found either
   way, with file:line. Do not assume.
5. Propagate to every vendored copy with `cargo xtask sync-engines`
   (not by hand), and prove it with `--check`.
```

Edge cases: `#A-b~r2` (revision pin with an UPPER anchor) parses; an anchor of
`_leading` or `9lives` still fails, since `is_valid_fact_id` requires an ASCII
letter first; an empty anchor still fails; a URI with no `#` still fails.

Error paths: the anchor-rejection message changes text. No new error kind.

## 5. Boundaries {#boundaries}

- **Do not edit `spec/**`.** PROP-014 / PROP-043 may want a line recording the
  widened URI grammar — **the reviewer writes it.**
- **Do not change `is_valid_anchor`.** The owner ruled heading anchors are not
  touched. A fix that widens *both* is out of scope and is a §8 stop.
- **Do not hand-edit anything under a `crates/vendor/` directory.** Those are
  copies; `sync-engines` writes them. Hand-editing a vendored file is the exact
  failure step 6 of the floor exists to catch.
- **Do not touch** `packages/**` beyond the grammar crate, and `campaigns/**`
  only in §9 of this file.

## 6. Acceptance {#acceptance}

```bash
cargo fmt --all
cargo test -p vibe-spec
cargo test --manifest-path packages/org.vibevm.ai-native/core-ai-native/v0.8.0/Cargo.toml --workspace
cargo xtask sync-engines --check
bash tools/self-check.sh ; echo "EXIT=$?"
```

Read the floor's **real** exit code; never judge it from a piped `tail`.

Then prove the goal end to end — add a temporary `#[spec(implements = …)]`
citing a real UPPER fact anchor minted by B1 (for example
`spec://org.vibevm.ai-native/core-ai-native/00-MANIFESTO#SINGLE-DESIGN-TARGET`),
confirm it **compiles**, then remove it and say in §9 that you did. A task that
widens a grammar and never cites one real anchor has not been tested.

- `parse_spec_uri("spec://vibevm/x#A-b")` → **Ok**, anchor `A-b`;
- `parse_spec_uri("spec://vibevm/x#A-b~r2")` → **Ok**, pin 2;
- `is_valid_anchor("FACT-A")` → **still false**;
- `cargo xtask sync-engines --check` → clean across all pairs.

New tests: one asserting an UPPER anchor round-trips through `parse_spec_uri`;
one asserting the heading-anchor law is unchanged; one in `vibe-spec` asserting
the host twin agrees with the package grammar on the same input set.

Discipline: `cargo fmt --all`, clippy clean, **no AI attribution anywhere**.
Commits: the grammar widening and the host-twin widening are one logical change
(they must agree or the seam is broken) — **one commit**, plus a second for the
`sync-engines` propagation if it produces a separable diff.

## 7. Analogies {#analogies}

`mdspec.rs:197` is the shape already doing the right thing: it validates a
`##ID` with `is_valid_fact_id` while its siblings at `:103` and `:383` validate
heading anchors with `is_valid_anchor`. The URI parser simply never joined that
pattern.

## 8. Stop rule {#stop}

- If a spec unit states the URI anchor grammar is kebab-only: **STOP**,
  `<!-- REVIEW: … -->`, question in §9, status `returned`. The owner's ruling
  changes the code; if a spec says otherwise, the spec edit is the reviewer's.
- If §4 step 4 finds a case-insensitive index or a normalisation step, **stop
  and report before widening** — collision behaviour is a design question.
- **Budget signal:** past **10 files / 250 lines** excluding vendored
  propagation, stop and return.

## 9. Log {#log}

*(appended by executor / reviewer)*
