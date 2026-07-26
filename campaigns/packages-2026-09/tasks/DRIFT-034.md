# DRIFT-034 — an anchor may be written any way, and a collision is caught {#root}

```
<status stage="impl" state="plan" ref="DRIFT-034"/>
```

**Status:** queued — **dispatch only after DRIFT-032 lands.** It edits the same
file (`…/specmark-grammar/src/lib.rs`); running both at once is a merge conflict
with extra steps.
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** common (`core-ai-native-specmark-grammar`, `vibe-spec`, `specmap`)
**Finding:** F-085, second half.
**Owner ruling, 2026-07-26:** «разреши всё и сделай проверку дубликатов
регистронезависимой.»

**Release event (plan §5-D)** — same propagation duty as DRIFT-032.

## 1. Goal {#goal}

A heading anchor may use the same character set a fact id may, and two ids that
differ only in case are reported as a duplicate instead of silently becoming two
addresses.

## 2. Contract {#contract}

```
> Fact ids share **one address space with heading anchors per document** — the
> same `spec://<package>/<doc-path>#<ID>` form cites either, and a duplicate id
> (fact-vs-fact or fact-vs-heading) is an extraction warning.
> — spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014-specmap-bidirectional-traceability#…
```

```
> `<ID>` is `[A-Za-z][A-Za-z0-9_-]*`
> — spec://vibevm/modules/vibe-progress/PROP-043#FACT-ID-GRAMMAR
```

**The two halves of this task are one idea.** Widening the heading law removes a
*structural* guarantee — today an `UPPER-SLUG` fact can never collide with a
heading anchor, because a heading anchor cannot be UPPER. The case-insensitive
duplicate check is what replaces that guarantee with an enforced one. Landing
the widening without the check would trade a guarantee for nothing, which is
the failure this task must not produce.

## 3. Current state {#current}

- `is_valid_anchor` (grammar `lib.rs:105`) — `[a-z0-9]+(-[a-z0-9]+)*`.
- `is_valid_fact_id` (`lib.rs:138`) — `[A-Za-z][A-Za-z0-9_-]*`, already the
  wider set. **Nothing about fact ids changes in this task.**
- The recorded reason for the kebab-only heading law is, verbatim from PROP-014:
  «`<anchor>` is the explicit `{#kebab-anchor}` **already used by every PROP
  heading**» — a description of practice, not a constraint. Confirmed
  2026-07-26; the owner re-opened it on that basis.
- Duplicate detection today: PROP-014 calls a duplicate id «an extraction
  warning». Find where it is computed — start at
  `…/core-ai-native-specmap/src/mdspec.rs` (heading anchors at `:103` and `:383`,
  fact ids at `:197`) — and name it in §9 with file:line **before** changing it.
- Heading-anchor validation sites: `mdspec.rs:103`, `mdspec.rs:383`. The host
  twin is `crates/vibe-spec/src/address.rs:217` (`is_valid_anchor_segment`),
  whose comment says it mirrors the vendored law — **it must move too, or the
  seam diverges**, exactly as DRIFT-031 found for `list_item_content`.

## 4. Required behavior {#behavior}

```
1. Widen is_valid_anchor to the same character set as is_valid_fact_id:
   `[A-Za-z][A-Za-z0-9_-]*`. Prefer having one validator and one law
   rather than two functions that now agree — two that agree today are
   the next thing nothing keeps honest. If you keep both names, make
   one call the other; do not copy the predicate.
2. Widen the host twin in vibe-spec/src/address.rs to match. The two
   must agree on the same input set.
3. Make duplicate detection CASE-INSENSITIVE across the one address
   space: `##FOO-BAR` and `{#foo-bar}` in the same document are a
   duplicate and are reported, not two addresses. Same for fact-vs-fact
   (`##FOO` / `##foo`) and heading-vs-heading.
4. The duplicate REPORT must name both spellings as written, not a
   normalised form — a message saying `foo-bar` twice is useless when
   the file contains `FOO-BAR` and `foo-bar`.
5. Resolution stays CASE-SENSITIVE. Widening is about what may be
   WRITTEN and what is flagged as a collision; `spec://…#FOO` must not
   silently resolve to a unit anchored `foo`. If you find resolution
   already normalises case, STOP and report — that is a different
   defect and changes this task's shape.
6. Propagate to every vendored copy with `cargo xtask sync-engines`.
```

Edge cases: an anchor that is now legal but was not (`{#Some_Anchor}`) parses
and resolves; the existing 275 kebab citations keep working unchanged — a
widening must never invalidate what was valid; a document with `##FOO` and
`{#foo}` reports exactly one duplicate, not two; a leading digit or a leading
`-`/`_` is still rejected.

Error paths: the anchor-rejection message changes text; the duplicate message
gains the second spelling. No new error kind.

## 5. Boundaries {#boundaries}

- **Do not edit `spec/**`.** PROP-014's anchor clause and PROP-043 both need a
  line recording the widened law and the case-insensitive collision rule — **the
  reviewer writes both.** A spec doubt is a §8 stop.
- **Do not hand-edit anything under `crates/vendor/`.** `sync-engines` writes it.
- **Do not rename or re-case any existing anchor anywhere in the corpus.**
  Anchors are immutable once published (PROP-014). This task changes what is
  *permitted*, never what is *written*.
- **Do not touch** `campaigns/**` except §9 of this file.

## 6. Acceptance {#acceptance}

```bash
cargo fmt --all
cargo test -p vibe-spec
cargo test --manifest-path packages/org.vibevm.ai-native/core-ai-native/v0.8.0/Cargo.toml --workspace
cargo xtask sync-engines --check
bash tools/self-check.sh ; echo "EXIT=$?"
cargo run -q -p vibe-cli --bin vibe -- progress check --no-cache --campaign campaigns/packages-2026-09
```

Read the floor's **real** exit code. Use `--no-cache` — a grammar change is
invisible to a warm parse cache.

- `is_valid_anchor("Some_Anchor")` → **true** (was false);
- `is_valid_anchor("9lives")`, `("-x")`, `("")` → still **false**;
- a fixture document carrying `##FOO-BAR` and a heading `{#foo-bar}` → **one**
  duplicate reported, naming **both spellings as written**;
- `progress check --no-cache` over the real corpus → **clean, 264 files**. If
  the new case-insensitive check finds a real collision in the corpus, that is
  a **finding, not a fix** — report it, do not re-case anything.

New tests: one asserting the widened heading set; one asserting fact-vs-heading
case collision is reported with both spellings; one asserting resolution stays
case-sensitive; one asserting the host twin and the package grammar accept the
same input set.

Discipline: `cargo fmt --all`, clippy clean, **no AI attribution anywhere**.
Commits: the widening and the collision check are **one** logical change (see
§2) — one commit, plus a second for `sync-engines` propagation if separable.

## 7. Analogies {#analogies}

DRIFT-032 has just done the smaller version of the same move — pointing a
validation site at the wider predicate without duplicating it. Read its diff
first; this task extends the same seam and must not undo it.

## 8. Stop rule {#stop}

- If resolution already normalises anchor case: **STOP and report.** Widening
  on top of a case-folding resolver produces silent aliasing.
- If the case-insensitive check finds collisions in the existing corpus:
  **report them and stop** before touching any of them. Re-casing a published
  anchor breaks immutability and is the owner's call.
- **Budget signal:** past **10 files / 250 lines** excluding vendored
  propagation, stop and return.

## 9. Log {#log}

*(appended by executor / reviewer)*
