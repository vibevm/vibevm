# GUIDE — Authoring specifications with bidirectional binding, v0.1

**Status.** Beta. The working instruction that operationalizes PROP-014 (specmap) and Charter §3.1 for anyone — human or agent — writing or normalizing specifications. The spec is the *meaning and contract* layer; this guide is about making that layer addressable, revisable, and machine-bound to code.

---

## 1. The unit {#unit}

A specification is a set of **units**. A unit is the span from an anchored heading to the next same-or-higher heading. Every normative or load-bearing statement lives inside exactly one unit.

Worked example — the canonical shape:

```markdown
### Conditional dependencies resolve to a fixed point {#req-conditional-fixpoint}
`req r2`

Predicates are evaluated against resolved project state. Each resolution
pass MUST only add requirements (monotone); convergence in finite passes
follows. Negation over resolution-variant probes (`if_present`,
`if_provides`) MUST NOT appear in predicates; negation over
resolution-invariant probes (`if_os`, `if_env`, `if_command`, `if_files`)
MAY appear.

*Verification:* a solver test that injects a `not if_present` predicate
and asserts `PredicateError::Unsupported`.
```

Anatomy: heading (one sentence, declarative) + `{#anchor}` + kind/revision line + body + an explicit verification sketch for `req` units.

## 2. Anchors and URIs {#anchors}

- Anchor grammar: `{#<kind>-<topic>-<aspect>}` in kebab-case (`#req-conditional-fixpoint`, `#design-fact-store`). Short, specific, pronounceable.
- **Anchors are immutable once published and never reused.** Renaming the heading keeps the anchor. Retiring a unit leaves a tombstone: `<!-- RETIRED: superseded by #req-new-thing -->`.
- The unit's address everywhere (code tags, commits, conversations) is the URI: `spec://<package>/<doc-path>#<anchor>` — optionally pinned `…~r<N>`.
- Cross-reference **only** by URI. "See above", "as discussed earlier", relative pointers: forbidden — they do not survive paging or reorganization.

## 3. Kinds and normativity {#kinds}

| Kind line | Means | Binds code? |
|---|---|---|
| `req rN` | contract; RFC-2119 verbs (MUST/SHOULD/MAY) | yes — `implements` / `verifies` edges expected |
| `prop rN` | a decision and its rationale (the "why") | indirectly — REQs cite it |
| `design rN` | shape of a solution; non-binding | `informs` edges only |
| `guide rN` | usage documentation | `documents` edges |

Rules of voice: inside `req`, every MUST is testable — if you cannot imagine the `#[verifies]` test, demote the unit to `design`. Rationale lives in `prop` units, not inline in `req` (the MUST changes rarely; the why evolves freely without invalidating implementations).

### 3.1 Lifecycle status (brownfield amendment) {#status}

The kind line may carry a status: `req r1 planned` · `req r2 disputed(#other-anchor)` · default (no status) = ratified · retired = tombstone comment. Semantics: `planned` units are contracts not yet implemented — zero coverage is *expected* and reported separately, never as a defect; the first real `implements` edge flips the status in the same PR. `disputed` units are recorded contradictions (`conflicts_with` edge + a debt entry with evidence from both sides) — normalization never resolves disputes inline; adjudication (supersede / scope-split / stay-open) is an explicit owner act, and edges into disputed pairs are frozen until it happens. Full machinery: `BROWNFIELD-PROTOCOL-v0.1.md` §5.

## 4. The revision discipline {#revisions}

- `rN` is the **author-asserted semantic revision**. Bump on meaning change only. Typos and rewording do not bump.
- The indexer hashes the unit body. Hash changed while `r` did not → the tooling asks: *editorial, or forgot to bump?* Answer by either bumping or marking the commit body `spec-editorial: <anchor>`.
- Bumping `r` flips every edge pinned to the old revision to **suspect** — that is the feature, not a punishment. Re-affirm each edge consciously (update the pin in code) after checking the implementation still satisfies the new meaning.
- Never bump `r` "to be safe." A false bump taxes every implementer with re-affirmation work (violates A2's economics).

## 5. Style rules {#style}

1. **One unit, one decision.** "And also" means split.
2. **Page-sized.** Soft limit 120 lines per unit; a unit must make sense alone when paged into a model's context window.
3. **What and why, never how.** A spec restating code is shadow code — drift fuel. Implementation detail lives in doc comments next to the code; the metamodel joins the layers at query time.
4. **Self-contained.** No implicit context: define terms or link their defining unit by URI.
5. **Deviations are written down where they happen** — in code, via `deviates` + reason. The spec does not pre-authorize deviations; it can later absorb them (revision) or reject them (fix the code).
6. **Two readers, one text.** Write for the human reviewer and the model identically: deterministic structure, no rhetorical suspense, conclusions first.

## 6. Activation descriptions (for distributable rule/spec packages) {#activation}

Units packaged for lazy-push delivery carry a natural-language trigger. Style (inherits vibevm's subskill guidance): begin with *"When you …"*; specificity beats verbosity; concrete situations, not topics.

- Bad: `"about Rust error handling"` — fires on everything Rust-adjacent.
- Good: `"When you are adding or changing a public error type in a library crate and need the variant-to-requirement tagging and #[track_caller] conventions."`

Soft cap ~600 characters; the review tooling penalizes vague-when-long.

## 7. Normalizing legacy specifications (importer output contract) {#legacy}

Arbitrary prose (old specs, skills, design docs) enters the metamodel only through normalization:

1. **Segment** into candidate units; 2. **classify** (`req`/`design`/`guide`/noise) — LLM-proposed, cached; 3. **anchor** + mark normativity at `r1`; 4. **affirm** — a human accepts/edits; nothing unaffirmed enters; 5. dialect adapters where the source has structure (first-class case: agent skill files — name + activation description + body map ~1:1 onto a lazy-push subskill manifest).

Honesty clause: units that resist classification enter as `design` with a `confidence: low` mark. A wrong MUST is worse than an honest "unclear."

## 8. Checklists {#checklists}

**Author, before PR:** every new normative sentence sits in an anchored unit · kind/revision line present · MUSTs testable · no "see above" · units ≤ 120 lines · semantic edits bumped, editorial edits marked · tombstones for anything retired.

**Reviewer:** would I know how to `#[verifies]` each MUST? · does any unit restate code? (reject) · do bumped revisions list their now-suspect edges in the PR description? · do new anchors follow the grammar and collide with nothing (`trace check` is green)?

## 9. Anti-patterns {#anti-patterns}

Shadow-code specs (rewriting the implementation in prose) · mega-units (three decisions under one anchor) · unverifiable MUSTs ("the code MUST be clean") · revision inflation (bumping on typos) · anchor reuse after retirement (breaks every historical link) · rationale-in-contract (why-paragraphs inside `req` bodies) · prose cross-references.

---

*Any markup element defined here that the indexer does not parse by Playbook Phase 1 is removed from this guide rather than carried as aspiration.*
