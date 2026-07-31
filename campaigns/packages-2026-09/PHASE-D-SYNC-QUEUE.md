# Phase D — the sync-from-code queue {#root}

_Created 2026-07-31, wave 7. §4 of the [batch plan](PHASE-D-BATCH-PLAN.md#waves)
says D3's diffs are «prepared in advance, presented in batches, **approved one at
a time**». The preparation had nowhere to live until now; this is that place._

**Nothing here is applied.** Every entry is a correction written out and left
unapplied, because an edit on this route produces a spec diff and **the owner
approves every spec diff** ([§1.2](PHASE-D-BATCH-PLAN.md#routes)). The
re-verdicts that needed no edit have already landed and are not repeated here.

Regenerate the route's size at any time:

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py
```

---

## What wave 7 established before preparing anything {#reverify-first}

The route was **re-verified rather than executed**, and that changed its size
more than any edit would have. Across the batches that returned, of the verdicts
examined a large minority did not survive as stated — and **the dominant cause
was not mis-measurement**. It was one of four attribution or perimeter errors,
each now named in [§6.1](PHASE-D-BATCH-PLAN.md#delegation-lessons) and
[§3.7](PHASE-D-BATCH-PLAN.md#compliance-blindness):

- **a real defect convicting the wrong sentence** — a summary drifting over a
  body confirmed on the same measurement; a capability sentence convicted of a
  practice failure; a row convicted for its neighbour's rule;
- **the perimeter named a directory rather than a place a mechanism can live** —
  «the crate» is not one place when a package ships five and six siblings vendor
  copies;
- **the wrong consumer** — §3.8, for packages whose audience is external;
- **the string rather than the thing** — `grep '## Changelog'` returns 1 while
  **15 of 42 PROPs carry a dated per-document change record with 33 entries**
  under `## Version history`.

**So the first thing this queue asks is not approval — it is that the remaining
route be re-verified before its diffs are written.** Every correction below is
written against a verdict that survived that check.

---

## A. The closing-rule family — decide this one first {#closing-rules}

**One rule, written four times, in four documents of `core-ai-native`.** The
rule governs what a document does with content that is specified and unexercised.

**The ask is a choice between two coherent answers**, not an edit:

1. **Demote the markers** — the four anchors move to the honest state and the
   documents stop claiming the unexercised half is built; or
2. **Legalise the «Specified, not built» form** — the annotate-in-place
   convention Phase D has been landing since wave 5 becomes the sanctioned way a
   document carries an unexercised claim, and the closing rule is amended to say
   so.

**This blocks group B and must be answered first.** Three documents already say
unexercised content «is removed rather than carried as aspiration» and **carry
nine «Specified, not built» passages between them** — so the corpus currently
does both. The 23 corrections in group B are **all written in the form group A
currently forbids**; approving them under answer (1) would be approving text the
same package prohibits.

---

## B. Annotate-in-place corrections — 23, and they wait on A {#annotations}

Prepared per anchor in the wave-7 harvest records under
[`harvest/`](harvest/), each with the exact replacement text and the measurement
behind it. They are not restated here because the harvest is the evidence and a
second copy is a second writer for one fact — this campaign's most-repeated
finding.

Read them with `harvest/d7a-core-sync-reverify.md` open at the obligation id.

---

## C. Independent single-sentence corrections {#singles}

| what | where | why it is independent |
|---|---|---|
| the front door says «prompt content only» | `core-ai-native` | over five crates and 10 072 lines of the package's own Rust. One sentence, no dependency on A |
| `##SUM-EVERY-HOST-HOLDS-THE-FULL-HISTORY` says «the full history» unqualified | `source-mirrors/daily-loop.md` | `refs = ["main","tags"]` is the flow's own example twice, and 13 local branches are on no host. One approval covers the same clause in three places |
| `##SWEEP-FLIP-ONLY-AFTER-DRAIN` (`GUIDE-AI-NATIVE-GO.md:626`) says a package enters `gated_packages` | `go-ai-native-lang`, rides `F-166` | a two-word swap (`gated_packages` → `gated_crates`): the key exists in three documents and zero code; the shipped key is the shared top-level `gated_crates` (vendor `config.rs:44`). A wave-8/D9 false confirm re-judged `drift` verdict-first (2026-07-31) and clustered here; its SKILL and card siblings are already corrected, so this is the last of the three go-package copies — `conform-frontend-go.md`'s copy belongs to `F-185` in group B |

---

## D. The campaign's own unfinished repair {#our-own}

**Recorded here rather than quietly fixed, because it is ours.**

Wave 6 corrected `##CODE-MARKS-WHAT-IT-IMPLEMENTS-THE-SPEC-WHAT-VERIFIES-IT`
(commit `24c0629e`) to note that where a project mechanizes the graph, both
edges are authored on the code side. **The correction did not propagate to its
own summary.** Sixty-five lines below, in the same file,
`##SUM-THE-BIDIRECTIONAL-GRAPH` still reads:

> `Implements:` markers plus `Test:` lines form a bidirectional graph that pays
> off with zero tooling. @impl/done

`grep -rc '^Test: ' spec/` returns **0**. So the summary asserts a form with no
instances, in a document whose body anchor has already been corrected away from
it — **a `duplication` defect authored by the phase that exists to remove them**,
and a partial closure of exactly the kind §3.1 warns about.

It is filed rather than fixed because the fix is an edit on this route. Found by
a delegated worker re-verifying the boss's own output, which is the argument for
having the wave re-verify at all.

---

## What is being asked, in one screen {#ask}

1. **Group A first** — one ruling, two coherent answers, and it gates 23 rows.
2. **Group B after A**, in batches, approved one at a time per §1.2.
3. **Group C any time** — two sentences, independent of everything.
4. **Group D** — the same choice as A, applied to our own text.
