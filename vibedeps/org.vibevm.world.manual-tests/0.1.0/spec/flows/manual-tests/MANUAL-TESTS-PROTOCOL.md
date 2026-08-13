# Manual-Tests Protocol {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file defines *why* a project keeps a
second, human-run test tier alongside its automated suite, *what* a
manual test is and is not, *when* to run one, *who* runs it, and the
directory convention that keeps the tier discoverable. @status:impl/done

@fact:sibling-document-pointers The four rules
each test must satisfy live in [`authoring-rules.md`](authoring-rules.md);
the copy-ready skeleton lives in [`test-template.md`](test-template.md). @status:impl/done

## Why a second tier exists {#why-second-tier}

@fact:the-automated-suite-is-hermetic-because-it-lies-about-the-world The automated suite is fast and hermetic *because it lies about the
world*. @status:spec/done

@fact:to-give-the-same-answer-on-every-machine-it-substitutes To run in a second and give the same answer on every machine,
it substitutes fakes for real dependencies, temporary directories for
the real per-user layout, and local fixtures for real remotes. @status:spec/done

@fact:that-substitution-is-the-refactor-loop-and-the-blind-spot That
substitution is exactly what makes it a good refactor loop — and
exactly why it cannot prove the surfaces that only exist in the real
world. @status:spec/done

@fact:those-surfaces-are-enumerable-lead Those surfaces are enumerable: @status:impl/done

| Surface | What the automated tier uses | What only the real world has |
|---------|------------------------------|------------------------------|
| @fact:ROW-SURFACE-AUTHENTICATION Authentication @status:spec/done | An in-process fake or a skipped check @status:spec/done | Real SSH keys, OAuth flows, API tokens against a real endpoint @status:spec/done |
| @fact:ROW-SURFACE-PER-USER-STATE Per-user state @status:spec/done | A throwaway temp directory @status:spec/done | The actual on-disk layout the tool creates under the user's home @status:spec/done |
| @fact:ROW-SURFACE-ARTIFACTS-CONSUMERS-READ Artifacts consumers read @status:spec/done | An in-memory value @status:spec/done | A lockfile (or export, or manifest) byte-for-byte as a downstream consumer receives it @status:spec/done |
| @fact:ROW-SURFACE-HUMAN-INTENT Human intent @status:spec/done | An `assert_eq!` on a string @status:spec/done | A person reading the output and confirming it says what they meant @status:spec/done |

@fact:THE-MANUAL-TIER-IS-THAT-LAST-MILE The manual tier is that last mile. @status:impl/done

@fact:THE-MANUAL-TIER-COMPLEMENTS-AND-DOES-NOT-REPLACE It **complements** the automated
suite; it does **not** replace it. @status:impl/done

@fact:DELETING-AN-AUTOMATED-TEST-GETS-THE-TRADE-BACKWARDS Deleting an automated test because
"the manual walkthrough covers it" gets the trade exactly backwards —
the fast tier stays the refactor loop, and the manual tier is the
slower, higher-confidence pass laid over the top. @status:impl/done

## What a manual test is {#what}

@fact:A-MANUAL-TEST-IS-A-SELF-CONTAINED-MARKDOWN-WALKTHROUGH A manual test is a **self-contained markdown walkthrough that a human
executes top to bottom and finishes with no ambient state left
behind**. @status:impl/done

@fact:IT-READS-LIKE-A-RECIPE It reads like a recipe: preconditions, a clean-slate setup,
a numbered sequence of steps — each a command block plus an
"Expected" paragraph — a teardown, and a list of what to collect if a
step diverges. @status:impl/done

@fact:OPEN-IT-FOLLOW-IT-AND-KNOW-PRECISELY-WHICH-STEP-DIVERGED Open the file, follow it, and either every step
matches its Expected or one does not and you know precisely which. @status:impl/done

## What a manual test is NOT {#not}

- @fact:NOT-A-REPLACEMENT-FOR-THE-AUTOMATED-TIER **Not a replacement for the automated tier.** If a check *can* be
  made fast and hermetic, it belongs in the automated suite, where it
  runs on every change. The manual tier is for what genuinely cannot. @status:impl/done
- @fact:NOT-EXPLORATORY-TESTING **Not exploratory testing.** Exploration is unscripted, one-off,
  and discards its steps. A manual test is scripted, repeatable, and
  **versioned next to the code** — the same reader running it next
  quarter takes the same path and expects the same output. If the
  product changes, the walkthrough is edited, not improvised around. @status:impl/done

## When to run {#when}

@fact:three-triggers-lead Three triggers, each independent: @status:impl/done

| Trigger | Why it fires the tier |
|---------|-----------------------|
| @fact:ROW-TRIGGER-BEFORE-TAGGING-ANY-MILESTONE **Before tagging any milestone** @status:impl/done | The tag claims the shipped features work end to end. Walk every manual test the index marks required for those features first. @status:impl/done |
| @fact:ROW-TRIGGER-AFTER-AN-INTEGRATION-SURFACE-CHANGE **After a change to an integration surface** @status:impl/done | Auth, per-user layout, consumer-facing artifacts, network I/O — run the relevant walkthroughs *even when the automated suite stays green*. Green fakes do not prove a changed real surface. @status:impl/done |
| @fact:ROW-TRIGGER-WHEN-A-USER-FILES-AN-INTEGRATION-BUG **When a user files an integration bug** @status:impl/done | Capture their steps as a new manual test. It becomes both the reproducer that confirms the fix and the guard against regression. @status:impl/done |

## Who runs it {#who}

@fact:A-HUMAN-RUNS-IT-BECAUSE-THE-TIER-EXISTS-FOR-HUMAN-EYES A **human** runs it, because the reason the tier exists is human eyes
on real output. @status:impl/done

@fact:AN-AGENT-MAY-PRE-RUN-THE-WALKTHROUGH An **agent** may *pre-run* the walkthrough — execute
each step and flag any whose result diverges from its Expected
paragraph — and that triage is genuinely useful. @status:impl/done

@fact:THE-PRE-RUN-IS-NOT-THE-SIGN-OFF But the pre-run is
not the sign-off. @status:impl/done

@fact:only-a-person-can-judge-that-the-output-says-what-was-meant Only a person can read the tool's real output and
judge "yes, that is what I meant". @status:spec/done

@fact:RECORD-THE-PRE-RUN-AS-TRIAGE-THE-PASS-ONLY-OVER-A-HUMAN-SIGNATURE Record the agent's pre-run as
triage; record the pass only over a human signature. @status:impl/done

## The directory convention {#directory}

@fact:MANUAL-TESTS-LIVE-IN-A-DEDICATED-DIRECTORY-AT-THE-REPOSITORY-ROOT Manual tests live in a dedicated **`manual-tests/`** directory at the
repository root, separate from end-user documentation — this is a
contributor-facing checklist for how the product is verified, not how
it is used. @status:impl/done

- @fact:ONE-MARKDOWN-FILE-PER-SCENARIO **One markdown file per scenario**, named for the milestone or
  feature it covers with a short slug: `m1-first-run-smoke.md`,
  `auth-real-remote.md`. The filename is the index entry; there is no
  second registry to keep in sync. @status:impl/done
- @fact:AN-INDEX-README-IN-THE-DIRECTORY **An index `README.md`** in the directory: a table of the files,
  what each covers, and which milestone requires it. New test, new
  row. @status:impl/done
- @fact:KEEP-EACH-FILE-TO-ONE-SCENARIO **Keep each file to one scenario.** A walkthrough that has grown
  past a screen or two of steps is usually two scenarios wearing one
  filename — split it. @status:impl/done

## Re-derive for your project {#re-derive}

@fact:re-derive-lead Do not copy this protocol's surfaces verbatim — copy the *task*, and
let the agent enumerate the surfaces this project actually has: @status:impl/done

```
Read this flow's documents (your project installed them — typically `vibedeps/flow-manual-tests/<version>/spec/flows/manual-tests/`, check `vibe.lock`) in full, then adapt the tier to this
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

- @fact:SUM-THE-TWO-TIERS-ARE-COMPLEMENTARY The automated tier proves the logic on fakes; the manual tier
  proves the world. Complementary, never a substitute. @status:impl/done
- @fact:SUM-WHAT-A-MANUAL-TEST-IS A manual test is a self-contained walkthrough run top to bottom
  that leaves no ambient state — scripted and versioned, not
  exploratory. @status:impl/done
- @fact:SUM-THE-THREE-TRIGGERS Run it before a milestone tag, after any integration-surface
  change, and to reproduce a user's integration bug. @status:impl/done
- @fact:SUM-AGENT-PRE-RUNS-HUMAN-SIGNS-OFF An agent pre-runs and flags mismatches; a human signs off. @status:impl/done
- @fact:SUM-ONE-FILE-PER-SCENARIO-INDEXED-BY-A-README One file per scenario under `manual-tests/`, indexed by a README. @status:impl/done
