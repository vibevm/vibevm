# DRIFT-034 — a heading anchor may be written the way a fact id may {#root}

```
<status stage="impl" state="plan" ref="DRIFT-034"/>
```

**Status:** queued — **re-scoped 2026-07-26 after the first attempt stopped at
§8 and was right to.** The fold is gone; only the widening remains.
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** common (`core-ai-native-specmark-grammar`, `vibe-spec`)
**Finding:** F-085, second half.
**Owner ruling, 2026-07-26 (second):** «расширить, свёртку не делать.
Побайтовая проверка продолжает ловить каждый настоящий дубликат.»

**Release event (plan §5-D)** — the grammar crate is vendored; propagate with
`cargo xtask sync-engines` or the fix reaches one consumer and not the others.

## 1. Goal {#goal}

`is_valid_anchor` accepts the same character set `is_valid_fact_id` does, so a
heading anchor may be written any way a fact id may. **Nothing else changes** —
duplicate detection stays byte-exact and resolution stays byte-exact.

## 2. Why the fold is NOT part of this — read before touching anything {#no-fold}

The first version of this task required a case-insensitive duplicate check
alongside the widening, on the argument that the kebab-only law was a
*structural guarantee* that an `UPPER-SLUG` fact could not collide with a
heading anchor, and that widening removed it.

**That argument was wrong, and the corpus disproved it.** `##TWO-TREES` and
`{#two-trees}` differ **byte for byte**, so they were never a duplicate under
byte-exact detection — before or after any widening. What widening newly permits
is writing `{#TWO-TREES}` beside `##TWO-TREES`, and *that* is a byte-exact
duplicate the existing check already catches, unchanged.

The first attempt measured what the fold would have cost: **29 published anchor
pairs across 12 documents**, every one the house convention — a section heading
`{#kebab-slug}` with that section's lead normative fact `##KEBAB-SLUG` two lines
below. The register is what tells a reader which grain they are citing. Folding
case would flag the convention 29 times to guard against a hazard that does not
exist.

**So: do not make any duplicate check case-insensitive. Do not normalise case
anywhere.** If you find yourself wanting to, re-read this section — the
temptation is the defect.

## 3. Current state {#current}

Measured; **do not re-survey**, and do not trust a claim here you can check in
one command (five task files this campaign carried an error an executor found
that way).

- `is_valid_anchor` (grammar `lib.rs:105`) — `[a-z0-9]+(-[a-z0-9]+)*`, the
  heading law. **This is the only thing that changes.**
- `is_valid_fact_id` (`lib.rs:138`) — `[A-Za-z][A-Za-z0-9_-]*`. Unchanged.
- `parse_spec_uri` already validates with `is_valid_fact_id` (DRIFT-032,
  `c94b9b0e`). Do not undo it.
- The host twin `crates/vibe-spec/src/address.rs:244` was **already widened** by
  DRIFT-032. But its doc comment at `:236-238` still says «The kebab-only law
  still governs where a *heading* anchor is minted» — **that sentence becomes
  false with this change and must be corrected.**
- Heading-anchor validation call sites: `…/core-ai-native-specmap/src/mdspec.rs:103`
  and `:383`. They keep calling `is_valid_anchor`; its *meaning* widens under them.
- **The recorded reason for the kebab-only law**, verbatim from PROP-014:
  «`<anchor>` is the explicit `{#kebab-anchor}` **already used by every PROP
  heading**» — a description of practice, not a constraint. That is why it
  re-opened.
- **The widening is not purely additive.** Kebab admits a **digit head**, so
  `{#9lives}` goes accepted → rejected; trailing dashes go rejected → accepted.
  Measured across the whole tree: **727 files, 1 227 distinct heading anchors,
  zero** that fail `[A-Za-z][A-Za-z0-9_-]*`, **zero** digit-headed. Blast radius
  is empty — but the asymmetry is real and belongs in a test.

## 4. Required behavior {#behavior}

```
1. Widen is_valid_anchor to `[A-Za-z][A-Za-z0-9_-]*`. Do NOT copy the
   predicate — have it call is_valid_fact_id, so the two laws cannot
   drift apart later. Two functions that agree today are the class of
   defect this campaign has found five times.
2. Correct the now-false doc comments: is_valid_anchor's own, and
   vibe-spec/src/address.rs:236-238. A comment that states the old law
   is worse than no comment, because it reads as authoritative.
3. Duplicate detection is UNCHANGED and byte-exact. Resolution is
   UNCHANGED and byte-exact. Touch neither.
4. Propagate to every vendored copy with `cargo xtask sync-engines`.
```

Edge cases: `{#Some_Anchor}` and `{#a-}` become legal; `{#9lives}` becomes
illegal (measured: none exist); `{#-x}`, `{#_x}` and `{#}` stay illegal; the
existing 1 227 heading anchors and 275 code citations keep working.

Error paths: the anchor-rejection message text changes. No new error kind.

## 5. Boundaries {#boundaries}

- **Do not edit `spec/**`.** PROP-014's anchor clause records the kebab-only law
  and now needs a line — **the reviewer writes it.** A spec doubt is a §8 stop.
- **Do not hand-edit anything under `crates/vendor/`.** `sync-engines` writes it.
- **Do not rename or re-case any existing anchor.** Anchors are immutable once
  published. This task changes what is *permitted*, never what is *written* —
  and in particular **the 29 pairs in §2 are correct as they stand.**
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

Read the floor's **real** exit code. `--no-cache` — a grammar change is invisible
to a warm parse cache.

- `is_valid_anchor("Some_Anchor")` → **true** (was false);
- `is_valid_anchor("a-")` → **true** (was false);
- `is_valid_anchor("9lives")` → **false** (was **true** — the one regression
  direction, with an empty blast radius);
- `is_valid_anchor("-x")`, `("_x")`, `("")` → **false**, as before;
- `is_valid_anchor` and `is_valid_fact_id` agree on every input — assert it over
  a shared table rather than two lists;
- `progress check --no-cache` → **clean, 264 files**, and the 29 heading/fact
  pairs still pass, because nothing folds case.

New tests: one over a shared input table asserting the two validators agree; one
pinning the digit-head asymmetry in both directions; one asserting a document
carrying `{#two-trees}` and `##TWO-TREES` is **not** a duplicate.

Discipline: `cargo fmt --all`, clippy clean, **one commit** for the widening
(plus a second for `sync-engines` propagation if separable), **no AI attribution
anywhere**.

## 7. Analogies {#analogies}

DRIFT-032 (`c94b9b0e`) pointed one validation site at the wider predicate
without duplicating it, and left the other law alone. This is the same move on
the other law. Read that diff first; do not undo it.

## 8. Stop rule {#stop}

- If widening `is_valid_anchor` makes any existing corpus anchor invalid:
  **STOP and report.** §3 says the blast radius is empty; if it is not, the
  measurement was wrong and re-casing a published anchor is the owner's call.
- If something outside the two validators turns out to depend on heading
  anchors being lowercase: **STOP and report** with file:line.
- **Budget signal:** past **6 files / 150 lines** excluding vendored
  propagation, stop and return — this is a two-line change plus tests.

## 9. Log {#log}

### Executor, first attempt, 2026-07-26 — STOPPED at §8 {#log-exec-1}

Measured that a case-insensitive duplicate check flags **29 published anchor
pairs across 12 documents**, all the section-heading/lead-fact convention;
listed them; shipped nothing. Also established: duplicate detection lives in
**five** places (`mdspec.rs:341`, `progress-core/parse/anchors.rs:12`,
`vibe-spec/gate.rs:55`, `vibe-spec/doctree.rs:71`, and the validator), and
`doctree.rs:70`'s map **is** the resolution index — so any future fold would
have to be a second parallel key set, never the lookup key. Recorded that
**zero** case-folding calls exist anywhere in the relevant tree, and that 69
byte-exact duplicate anchors sit inside the generated `spec/boot/STATIC.xml`
(the F-078 duplication seen by another instrument).

**The stop was correct and the reviewer's argument was the thing that was
wrong** — see §2, rewritten. Owner ruled the fold out; the widening stands.

### Reviewer, 2026-07-26 — re-scoped {#log-review}

Task rewritten to the widening alone. §2 now carries the corrected reasoning so
no future reader inherits the original error.
