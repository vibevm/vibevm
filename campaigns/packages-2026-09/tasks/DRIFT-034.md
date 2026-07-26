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

### Executor, 2026-07-26 — STOPPED at §8, nothing landed {#log-exec}

**Verdict: §8's second stop fires, and it fires 29 times.** The corpus does not
carry a handful of case collisions to report and step around — the pattern the
new law would flag *is the house authoring idiom*, used deliberately in 12
published spec documents. Nothing was changed: the tree is byte-identical to the
base commit `f8f92f78`, and the only edit in this session is this log entry.
Landing the widening alone was not an option either — §2 forbids exactly that —
so the task returns whole rather than half-done.

**§8's FIRST stop does not fire — resolution does not fold case.** Confirmed,
not re-surveyed: zero `to_lowercase` / `to_ascii_lowercase` /
`eq_ignore_ascii_case` / `to_uppercase` occurrences in `crates/vibe-spec/`,
`crates/progress-core/`, or the authored
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/` (grep, exit 1 —
no matches), and no `unicase`-style dependency. Every anchor index is byte-exact.
So the widening could not have produced silent aliasing; the problem is the
other half.

**Duplicate detection, named with file:line before touching it (§3's ask).**
It is computed in **five** places, not one. Four are the "one address space per
document" law; the fifth is where the widening's own guarantee lived.

| # | Site | What it is | Reported? |
|---|------|-----------|-----------|
| 1 | `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/mdspec.rs:341` — `seen_anchors: Vec<String>`, checked `:274` (fact) and `:393` (heading), warning minted `:206` | PROP-014's «extraction warning» | yes, `duplicate-anchor` |
| 2 | `crates/progress-core/src/parse/anchors.rs:12` — `seen: HashMap<String, usize>`, checked `:22`, message `:27` | PROP-043 §3.8 anchor laws | yes, `IssueCode::DuplicateId`, `Severity::Error` — this is what `vibe progress check` fails on |
| 3 | `crates/vibe-spec/src/gate.rs:55` — `seen: HashMap<&str, NodeId>`, `first_duplicate` | PROP-035 §7.3 merged-view uniqueness | yes, a **build error** via `pipeline.rs:82` |
| 4 | `crates/vibe-spec/src/doctree.rs:71` — `duplicate_anchors: Vec<String>`, recorded `:175` (heading) and `:361` (fact) | the per-file record | no production consumer; only `doctree/tests.rs` reads it |
| 5 | `packages/…/core-ai-native-specmark-grammar/src/lib.rs:114` — `is_valid_anchor` | not a duplicate check — the *structural* guarantee §2 is about | n/a |

Site 4's map is also the **resolution** index (`find_by_anchor` `:218`,
`resolve_path` `:227`), so §4 step 5 constrains the implementation there: the
fold has to be a second, parallel key set, never the lookup key. Noted for
whoever picks this up.

**The finding — 29 case-differing collisions in the exact §6 corpus.** Measured
with the **real engine**, not a regex approximation: a temporary integration
test (`crates/progress-core/tests/tmp_drift034_survey.rs`) loaded
`progress.toml` through `progress_core::scope::{load_config, observed_files}`,
parsed every observed file through `progress_core::parse::parse_document`, and
applied the proposed law over the parser's own `units[].anchor` +
`blocks[].facts[].id` in document order. It reported `OBSERVED FILES: 264` —
§6's number exactly — and 29 collisions across 12 files. The test was deleted;
`git status` is clean.

*(A first regex pass over the tree found only 3 of these. It was wrong — it
skipped paragraph-lead facts, and an earlier version silently swallowed every
file behind a `read_to_string` error arm. The engine-run numbers below are the
ones to trust; the regex pass is recorded only so nobody repeats it.)*

Every one of the 29 is the **same shape**: a section heading `{#kebab-slug}`
and, two to four lines later, that section's own lead normative fact
`##KEBAB-SLUG` — the fact that states what the section is about.

```
spec/common/PROP-000.md                            setup-docs @284         SETUP-DOCS @286
spec/common/PROP-031-algorithmic-refactoring.md    three-tier @60          THREE-TIER @62
spec/common/PROP-031-algorithmic-refactoring.md    algebra @80             ALGEBRA @82
spec/design/action-system.md                       thesis @13              THESIS @15
spec/design/action-system.md                       contract-pointer @275   CONTRACT-POINTER @277
spec/modules/vibe-actions/PROP-039-action-system.md  address-grammar @43     ADDRESS-GRAMMAR @45
spec/modules/vibe-actions/PROP-039-action-system.md  address-uniqueness @56  ADDRESS-UNIQUENESS @58
spec/modules/vibe-cli/PROP-036-package-tree.md     static-decompile @144   STATIC-DECOMPILE @146
spec/modules/vibe-cli/PROP-037-tree-tui.md         component-strategy @111 COMPONENT-STRATEGY @113
spec/modules/vibe-cli/PROP-042-aiui-observation.md in-place-upgrade @207   IN-PLACE-UPGRADE @210
spec/modules/vibe-index/PROP-005-package-index.md  slice-1 @709            SLICE-1 @711
spec/modules/vibe-index/PROP-005-package-index.md  slice-2 @715            SLICE-2 @717
spec/modules/vibe-index/PROP-005-package-index.md  slice-3 @725            SLICE-3 @727
spec/modules/vibe-index/PROP-005-package-index.md  slice-4 @735            SLICE-4 @737
spec/modules/vibe-index/PROP-005-package-index.md  slice-5 @745            SLICE-5 @747
spec/modules/vibe-index/PROP-005-package-index.md  slice-6 @758            SLICE-6 @760
spec/modules/vibe-index/PROP-005-package-index.md  slice-7 @768            SLICE-7 @770
spec/modules/vibe-index/PROP-005-package-index.md  slice-8 @774            SLICE-8 @776
spec/modules/vibe-index/PROP-005-package-index.md  slice-9 @780            SLICE-9 @782
spec/modules/vibe-index/PROP-005-package-index.md  slice-10 @786           SLICE-10 @788
spec/modules/vibe-index/PROP-005-package-index.md  slice-11 @792           SLICE-11 @794
spec/modules/vibe-registry/PROP-001-git-backend.md registry-trait @167     REGISTRY-TRAIT @169
spec/modules/vibe-registry/PROP-001-git-backend.md cache-layout @192       CACHE-LAYOUT @194
spec/modules/vibe-resolver/PROP-017-resolvo-resolver.md determinism @297   DETERMINISM @299
spec/modules/vibe-workspace/PROP-007-workspace.md  path-source @103        PATH-SOURCE @107
spec/modules/vibe-workspace/PROP-007-workspace.md  impl-gates @327         IMPL-GATES @329
spec/modules/vibe-workspace/PROP-009-loading-model.md two-trees @32        TWO-TREES @34
spec/modules/vibe-workspace/PROP-009-loading-model.md effective-boot @43   EFFECTIVE-BOOT @45
spec/modules/vibe-workspace/PROP-009-loading-model.md inclusion-types @87  INCLUSION-TYPES @89
```

All 29 are in `spec/`; the `packages/**` half of the wave-2 corpus contributes
**zero**. The same sweep over specmap's corpus (`spec/**` + `VIBEVM-SPEC.md`)
adds nothing new: the same 29, plus 69 **byte-exact** duplicates that all live
in `spec/boot/STATIC.md` — a generated boot artifact that concatenates package
snippets, so its anchors legitimately repeat. Those 69 are caught by today's
byte-exact check already, are pre-existing at the base commit, and are out of
the Progress-Control corpus (`spec/boot/[0-9]*.md` does not match it). Not this
task's business, recorded so the next sweep does not mistake them for new.

**Why this is a stop and not a fix.** §8 says re-casing a published anchor is
the owner's call, and this is not three accidents — it is 29 deliberate pairs
written by the house convention the two registers were *designed* for: kebab
names the service unit (the section), UPPER names the normative fact inside it,
and PROP-014's own text calls the register "convention, not enforced". A
case-insensitive collision law makes that convention illegal retroactively. The
choices all belong to the owner, and each is a different task:

1. **Re-case the corpus** — 29 anchors change spelling. Breaks anchor
   immutability (PROP-014) and any `spec://…#<id>` citing them. Not an
   executor's call, and probably not the right one.
2. **Narrow the law** — a fact whose *enclosing section* carries the same
   case-folded anchor is a restatement, not a collision (`gate.rs:61` already
   has precedent for exempting a structurally-legitimate repeat: the `:add`
   heading-vs-heading artifact). This would clear all 29, because all 29 are
   exactly that shape. But it is a **third** law nothing keeps honest, and it
   would knowingly leave `#static-decompile` and `#STATIC-DECOMPILE` as two
   live addresses in one document — the aliasing §2 says the check exists to
   prevent. A spec question, not a code one.
3. **Keep the check byte-exact** and land the widening with a different
   guarantee — but §2 rules that out as stated.
4. **Fold only within a grain** (heading-vs-heading and fact-vs-fact
   case-insensitive, heading-vs-fact byte-exact). Clears all 29, since all 29
   are cross-grain. Also a spec question: it half-abandons "one address space".

Option 2 or 4 looks like what the owner's ruling («разреши всё и сделай проверку
дубликатов регистронезависимой») would have said had the 29 been on the table
when it was made. That is a guess, and guessing is what §8 exists to prevent.

**The widening half was measured and is safe on its own** — recorded so a
re-dispatch does not pay for it twice. Across 727 markdown files (the tree
minus `vibedeps/`, `.vibe/cache/`, `refs/`, `**/vendor/`, `target/`): **1 227**
distinct `{#…}` heading anchors, of which **0** fail `[A-Za-z][A-Za-z0-9_-]*`,
**0** are digit-headed (so nothing moves from accepted to rejected), and **0**
are non-kebab (so nothing moves the other way either). DRIFT-032's 380/750
measurement stands; this is the same answer over a wider scan. The widening is
a pure no-op on today's corpus — which is precisely why it must not ship alone:
it costs nothing today and silently removes the guarantee that `##UPPER-SLUG`
can never collide with a heading anchor.

**Two slips in §6's own edge-case list, worth fixing before re-dispatch.**
`is_valid_anchor("9lives")` is listed as "still **false**" — it is **true**
today (kebab admits a digit head), and would *become* false under the widening.
The same slip appeared in DRIFT-032 §4 and its executor logged it; the end
state is right, only the word "still" is wrong. §3's line «heading anchors at
`:103` and `:383`, fact ids at `:197`» points at the `is_valid_anchor` /
`is_valid_fact_id` **call sites**, not at duplicate detection — the table above
is the real answer to §3's ask.

**§4 step 2 is already done.** `crates/vibe-spec/src/address.rs:244`
(`is_valid_anchor_segment`) was widened to `[A-Za-z][A-Za-z0-9_-]*` by
DRIFT-032, so the host twin already accepts the id set. One doc-comment
sentence there («The kebab-only law still governs where a *heading* anchor is
minted; it is not this parser's business», `address.rs:236-238`) becomes false
the moment the heading law widens, and needs to move with it.

**§6 run on the untouched tree** — the base is genuinely green, so the finding
above is a property of the corpus and not of anything this session did.

```
cargo fmt --all                                   EXIT=0   (no file reformatted)
cargo test -p vibe-spec                           EXIT=0
cargo test --manifest-path …/core-ai-native/v0.8.0/Cargo.toml --workspace
                                                  EXIT=0   (11 suites, 0 failed)
cargo xtask sync-engines --check                  EXIT=0
    sync-engines --check: every vendored crate matches its authored source
    (33 pair(s) across 6 sync set(s)).
bash tools/self-check.sh ; echo "EXIT=$?"         EXIT=0
    self-check: all green
vibe progress check --no-cache --campaign campaigns/packages-2026-09
                                                  EXIT=0
    progress check: clean (264 files, 0 warning(s))
```

**Open items.**

1. The owner must rule between the four options above before this task can be
   re-dispatched. Whichever wins changes §4 step 3, so the task file needs the
   edit before an executor takes it again.
2. Nothing was committed beyond this log. `is_valid_anchor` is untouched, no
   `sync-engines` propagation was needed, and no anchor anywhere was renamed or
   re-cased.
3. Not verified, stated as an open item: whether the 29 anchors are actually
   *cited* from code or from other documents. It does not change the verdict —
   immutability applies to a published anchor whether or not something cites it
   today — but it would size option 1 if the owner picks it.
