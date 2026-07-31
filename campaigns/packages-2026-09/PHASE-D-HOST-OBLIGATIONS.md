# Phase D — what the host owes {#root}

_Written 2026-07-29, derived from
[`run/state/routing.json`](run/state/routing.json) and the registry. **Not
hand-maintained: regenerate the counts, do not re-type them.**_

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py
```

---

## Why this file exists {#why}

§3.6 routes most of this corpus away from the packages. Three waves examined
161 anchors and moved fifteen; the rest are **route (b)** — the rule is sound
and the consumer does not keep it — so the package does not move and *the host
owes the work*.

That determination is a finding, and a finding that lives only in a routing
record is a finding nobody acts on. **This file is the other half of the exit
gate.** The gate says «the ledger is empty or every survivor is an owner-ruled
deferral»; these are the survivors, and they need the owner's ruling to become
deferrals rather than silence.

**53 obligations · 142 anchors · nothing left owed to a package.**

| package | anchors | package | anchors |
|---|---:|---|---:|
| `campaign-plans` | 29 | `decision-records` | 9 |
| `comparative-research` | 24 | `addressable-specs` | 7 |
| `wal` | 22 | `spec-genres` | 7 |
| `health-audit` | 16 | `sync-from-code` | 5 |
| `manual-tests` | 11 | `conflict-protocol` | 1 |
| `operating-modes` | 9 | `two-process-model` | 1 |
| | | `wal-specspaces` | 1 |

By type: `reality-mismatch` 39 · `contradiction` 12 · `duplication` 1 ·
`relocation` 1.

---

## The three answers the owner can give {#answers}

Every one of the 53 takes exactly one of these, and none of them is «edit the
package», which is what routing them here already decided.

1. **The host adopts the practice.** The rule is sound, the host should keep
   it, and the work is a host task. `flow:campaign-plans`'
   `##COLD-A-LITERAL-QUICK-START-BLOCK` went this way on 2026-07-29 and is the
   worked precedent: the owner ruled the rule sound, both live plans gained the
   block, and the fact re-judged `confirmed` with no package edit at all.
2. **The host records a deliberate exception.** The rule is sound and the host
   chooses otherwise for a stated reason. Phase C's own ruling makes this a
   real closure and not a loophole: **a marked exception is not drift**, while
   an unmarked one is. The fact is then confirmed with the exception named.
3. **The obligation is deferred, with the reason on record.** Nothing is done
   now, and the exit gate counts it as an owner-ruled deferral rather than as
   work skipped.

**What is not on the list: softening the package.** That is the one answer
§3.6 forbids and the one the profanation of §0 consists of.

---

## Where the weight is {#weight}

**`campaign-plans` at 29 anchors is the largest, and it is one shape.** The
fifteen-section plan skeleton the flow defines is instantiated exactly once in
this repository and that instance is archived. The two live campaigns replaced
the one-file dialect with a zone directory and side documents, which the format
explicitly permits — but the sections went with the dialect: risks 16 archived /
0 live, non-goals 9 / 0, whole-campaign acceptance 8 / 0, execution ledger 8 / 0,
commit maps 3 / 0, safe stop 12 / 0, Phase 0 five archived and none live. **This
is adopt-then-drop, not non-adoption**, which is what makes it drift at all —
and it is therefore one ruling, not twenty-nine.

> **Re-measured 2026-07-31 over the whole tree, and the characterisation needs
> one correction that changes the ruling.** The ratios above were taken over
> `spec/terraforms/` and `legacy-spec/` — the same perimeter wave 6 proved blind,
> because it omits the `fractality` specspace, a **second project that adopted
> this flow** and boots it at slot 40 of its own generated `spec/boot/INDEX.md`.
> Counted by file across archived · host-live · fractality:
>
> | form | `legacy-spec/` | host live plans | `fractality` plans |
> |---|---:|---:|---:|
> | commit map | 4 | 0 | **3** |
> | safe stop | 12 | 0 | **3** |
> | whole-campaign acceptance | 9 | 0 | **2** |
> | non-goals | 9 | 0 | **3** |
> | risks | 16 | 0 | **3** |
> | Phase 0 | 16 | 0 | **2** |
>
> **The practice is not abandoned; the host's own two plans are the outlier.**
> That flips the ruling this section asks for. «Adopt-then-drop» invites «then
> let us formally drop it». «Live in the sibling project, absent in the host's
> two plans» invites the opposite — bring the host's plans into line — and it is
> the reading the measurement supports.
>
> *One trap, recorded because it nearly landed in this table.* A naive count
> shows one host-live hit for every form. Every one of them is inside
> `PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` — this campaign's own plan — and
> matches only because the §7 LOG entry written the day before **quotes these
> words in prose**. They are not sections. The host-live column is 0, and the
> campaign nearly measured its own footprint as evidence about its subject.

**`wal` at 22 and `health-audit` at 16 are the same genre**: flows whose subject
is the host's own practice, measured against the host's own artefacts. Their
rulings are about what the host will actually keep doing, not about wording.

**Three carry a defect the routing record already names, and each is a
one-line host fix rather than a ruling:**

- `PROP-035`'s `##related` has no return leg to `spec/design/structural-loader.md`,
  which names it three times (from F-335).
- The `revisit-triggers` field definition and its own example library disagree
  about whether an event trigger is a legal trigger (from F-224) — and that one
  is `self`, so it is a package obligation the waves have not reached yet.
- The commit-subject grammar is stated three times in two packages (from
  F-340), which is a `duplication` and a §4.5 release event.

---

## The census that sizes the biggest ruling {#census}

Measured here rather than taken from anyone's report, over
`spec/common/*.md` + `spec/modules/*/*.md`:

```bash
grep -rc "\*\*Decision\.\*\*" spec/common/*.md spec/modules/*/*.md | awk -F: '{s+=$2} END{print s}'
```

**122 sections carry a `**Decision.**` line and 4 carry a `Revisit when`** —
and `**Considered and rejected` occurs **4 times in the whole tree, in exactly
two files**, `PROP-036` once and `PROP-043` three times.

*Two workers reported ~154 sections and 149 stubs; my count over the perimeter
stated above is 122 and 118. The gap is a perimeter difference, not a
disagreement about the finding — theirs presumably reaches `spec/design/` and
mine does not. **The number to act on is the one whose perimeter is written
down**, which is why this paragraph carries the command.*

`flow:decision-records` asks every reopenable choice to carry a record with its
alternatives and a revisit condition. The host writes the Decision line and
stops — 4 of 122 go further. That is not a wording problem in the flow and it is
not a small task; it is the single largest piece of work this phase has
surfaced, and it belongs on the owner's desk as a decision about *whether the
host adopts the practice* before anyone writes a hundred-odd records.

> **Re-measured 2026-07-31 over the whole tree, and this reframes the question
> from «whether to adopt» to «why the PROP tree is the outlier».** Counting
> sections that carry a bolded `Decision` label, against those carrying all four
> fields (`Decision` · `Why` · `Considered and rejected` · `Revisit when` /
> `When to revisit`):
>
> | perimeter | Decision-labelled | all four |
> |---|---:|---:|
> | `spec/common` + `spec/modules` — the perimeter above | 153 | **4** |
> | all of `spec/` | 157 | **7** |
> | `campaigns/` — *this campaign's own records* | 15 | **8** |
> | **the `fractality` specspace** | 34 | **14** |
>
> **The practice is adopted, and adopted well, in the sibling project: 14 of 34,
> about 41 %,** against roughly 4.6 % in the host's PROP/FEAT tree. It is also
> the form this campaign itself writes — every `Decision` / `Why` / `Considered
> and rejected` / `Revisit when` block in the batch plan is one. So the honest
> statement is not «the host does not do this». It is: **the host does it
> wherever it plans work, and does not do it in the document genre where the
> reopenable choices actually live.**
>
> That is a smaller and better-posed decision than «adopt a practice». It asks
> which PROP/FEAT decisions are genuinely reopenable — almost certainly far
> fewer than 153 — and whether the four-field form is owed to those rather than
> to every bolded `Decision` line.
>
> *Counted under [`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](PHASE-D-BATCH-PLAN.md#delegation-lessons):
> 8 of the complete records are this campaign's own, which is why the
> `campaigns/` row is broken out rather than folded into a host-wide total. A
> figure that silently included them would have reported the host's adoption as
> half again what it is.*
