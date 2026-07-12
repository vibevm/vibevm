# Manual-Tests Protocol {#root}

**Scope of this document.** This file defines *why* a project keeps a
second, human-run test tier alongside its automated suite, *what* a
manual test is and is not, *when* to run one, *who* runs it, and the
directory convention that keeps the tier discoverable. The four rules
each test must satisfy live in [`authoring-rules.md`](authoring-rules.md);
the copy-ready skeleton lives in [`test-template.md`](test-template.md).

## Why a second tier exists {#why-second-tier}

The automated suite is fast and hermetic *because it lies about the
world*. To run in a second and give the same answer on every machine,
it substitutes fakes for real dependencies, temporary directories for
the real per-user layout, and local fixtures for real remotes. That
substitution is exactly what makes it a good refactor loop — and
exactly why it cannot prove the surfaces that only exist in the real
world.

Those surfaces are enumerable:

| Surface | What the automated tier uses | What only the real world has |
|---------|------------------------------|------------------------------|
| Authentication | An in-process fake or a skipped check | Real SSH keys, OAuth flows, API tokens against a real endpoint |
| Per-user state | A throwaway temp directory | The actual on-disk layout the tool creates under the user's home |
| Artifacts consumers read | An in-memory value | A lockfile (or export, or manifest) byte-for-byte as a downstream consumer receives it |
| Human intent | An `assert_eq!` on a string | A person reading the output and confirming it says what they meant |

The manual tier is that last mile. It **complements** the automated
suite; it does **not** replace it. Deleting an automated test because
"the manual walkthrough covers it" gets the trade exactly backwards —
the fast tier stays the refactor loop, and the manual tier is the
slower, higher-confidence pass laid over the top.

## What a manual test is {#what}

A manual test is a **self-contained markdown walkthrough that a human
executes top to bottom and finishes with no ambient state left
behind**. It reads like a recipe: preconditions, a clean-slate setup,
a numbered sequence of steps — each a command block plus an
"Expected" paragraph — a teardown, and a list of what to collect if a
step diverges. Open the file, follow it, and either every step
matches its Expected or one does not and you know precisely which.

## What a manual test is NOT {#not}

- **Not a replacement for the automated tier.** If a check *can* be
  made fast and hermetic, it belongs in the automated suite, where it
  runs on every change. The manual tier is for what genuinely cannot.
- **Not exploratory testing.** Exploration is unscripted, one-off,
  and discards its steps. A manual test is scripted, repeatable, and
  **versioned next to the code** — the same reader running it next
  quarter takes the same path and expects the same output. If the
  product changes, the walkthrough is edited, not improvised around.

## When to run {#when}

Three triggers, each independent:

| Trigger | Why it fires the tier |
|---------|-----------------------|
| **Before tagging any milestone** | The tag claims the shipped features work end to end. Walk every manual test the index marks required for those features first. |
| **After a change to an integration surface** | Auth, per-user layout, consumer-facing artifacts, network I/O — run the relevant walkthroughs *even when the automated suite stays green*. Green fakes do not prove a changed real surface. |
| **When a user files an integration bug** | Capture their steps as a new manual test. It becomes both the reproducer that confirms the fix and the guard against regression. |

## Who runs it {#who}

A **human** runs it, because the reason the tier exists is human eyes
on real output. An **agent** may *pre-run* the walkthrough — execute
each step and flag any whose result diverges from its Expected
paragraph — and that triage is genuinely useful. But the pre-run is
not the sign-off. Only a person can read the tool's real output and
judge "yes, that is what I meant". Record the agent's pre-run as
triage; record the pass only over a human signature.

## The directory convention {#directory}

Manual tests live in a dedicated **`manual-tests/`** directory at the
repository root, separate from end-user documentation — this is a
contributor-facing checklist for how the product is verified, not how
it is used.

- **One markdown file per scenario**, named for the milestone or
  feature it covers with a short slug: `m1-first-run-smoke.md`,
  `auth-real-remote.md`. The filename is the index entry; there is no
  second registry to keep in sync.
- **An index `README.md`** in the directory: a table of the files,
  what each covers, and which milestone requires it. New test, new
  row.
- **Keep each file to one scenario.** A walkthrough that has grown
  past a screen or two of steps is usually two scenarios wearing one
  filename — split it.

## Re-derive for your project {#re-derive}

Do not copy this protocol's surfaces verbatim — copy the *task*, and
let the agent enumerate the surfaces this project actually has:

```
Read spec/flows/manual-tests/ in full, then adapt the tier to this
project:
1. List every integration surface the automated suite fakes, mocks,
   or skips — real auth, the per-user state directory, remote I/O,
   consumer-facing artifacts, any "does this output read right?"
   check. Name the file or module that fakes each.
2. For each surface, say whether a fast hermetic test could cover it
   instead. If yes, that is an automated-suite gap, not a manual
   test — flag it separately.
3. For the genuine remainder, propose a manual-tests/ walkthrough
   per scenario: milestone-slug filename, one line on what it proves.
4. Draft the index README table for those files.
5. Show me the list and the drafts. Write nothing until I approve.
```

## Summary {#summary}

- The automated tier proves the logic on fakes; the manual tier
  proves the world. Complementary, never a substitute.
- A manual test is a self-contained walkthrough run top to bottom
  that leaves no ambient state — scripted and versioned, not
  exploratory.
- Run it before a milestone tag, after any integration-surface
  change, and to reproduce a user's integration bug.
- An agent pre-runs and flags mismatches; a human signs off.
- One file per scenario under `manual-tests/`, indexed by a README.
