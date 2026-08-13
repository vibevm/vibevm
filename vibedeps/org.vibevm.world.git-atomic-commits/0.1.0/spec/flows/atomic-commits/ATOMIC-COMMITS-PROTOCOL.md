# Atomic Commits Protocol {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file defines *what* an atomic commit
is, *what* makes a sequence of commits well-formed, *when* a change
set must be split, and *why* this discipline matters more in a
human-AI team than in a traditional team. @status:impl/done

## What an atomic commit is {#what}

@fact:AN-ATOMIC-COMMIT-CARRIES-EXACTLY-ONE-IDEA An atomic commit carries **exactly one idea**. @status:impl/done

@fact:THE-DIFF-SHOWS-EVERY-CHANGE-AND-NOTHING-ELSE The diff shows every
change required to express that idea, and nothing else. @status:impl/done

@fact:three-equivalent-framings-lead Three equivalent framings of the same rule: @status:impl/done

- @fact:FRAMING-ONE-LOGICAL-UNIT **One logical unit.** The smallest set of edits that, together,
  express a complete thought and leave the tree in a consistent state. @status:impl/done
- @fact:FRAMING-ONE-INTENT-PER-MESSAGE **One intent per message.** If the body needs to say "also, while
  I was in there, …", the "also" belongs in its own commit. @status:impl/done
- @fact:FRAMING-ONE-THING-TO-VERIFY **One thing to verify.** A reviewer should be able to answer "does
  this commit do what the subject claims?" without having to
  partition the diff first. @status:impl/done

@fact:A-BUNDLED-COMMIT-IS-THREE-COMMITS-NOT-ONE A commit that bundles "fix typo in README" + "refactor planner" +
"update schema" is three commits, not one. @status:impl/done

## Why atomic commits matter more here {#why}

@fact:in-a-pure-human-team-they-are-a-quality-of-life-feature In a pure-human team, atomic commits are a quality-of-life feature: @status:spec/done

- @fact:QOL-EASIER-REVIEW easier review, @status:spec/done
- @fact:QOL-CLEANER-BISECTS cleaner bisects, @status:spec/done
- @fact:QOL-VIABLE-CHERRY-PICKS viable cherry-picks. @status:spec/done

@fact:teams-that-skip-the-discipline-ship-anyway Teams that skip
the discipline ship anyway. @status:spec/done

@fact:in-a-human-ai-team-they-are-load-bearing In a human-AI team they are **load-bearing**. Three reasons: @status:spec/done

### Diff as verification channel {#why-diff}

@fact:the-humans-primary-verification-mechanism-is-reading-the-diff The human's primary verification mechanism is reading the diff. @status:spec/done

@fact:a-mixed-commit-forces-the-human-to-partition-it-first If
one commit mixes three concerns across eight files and ninety lines,
the human has to mentally partition the diff before assessing any
single piece. @status:spec/done

@fact:the-partition-step-is-where-mistakes-slip-through That partition step is where mistakes slip through —
the human is now doing a task the commit structure should have done
for them. @status:spec/done

@fact:ONE-CONCERN-PER-COMMIT-MAKES-THE-DIFF-VERIFIABLE-IN-ONE-PASS One concern per commit makes the diff directly verifiable
in one pass. @status:impl/done

### Rollback precision {#why-rollback}

@fact:some-ai-authored-changes-will-turn-out-to-be-wrong Some AI-authored changes will turn out to be wrong. @status:spec/done

@fact:REVERT-MUST-UNDO-THE-WRONG-THING-AND-NOTHING-ELSE When that
happens, `git revert <sha>` must undo the wrong thing without also
undoing three correct things that happened to ride in the same
commit. @status:impl/done

@fact:THAT-IS-ONLY-POSSIBLE-IF-THEY-WERE-THREE-COMMITS-TO-BEGIN-WITH That is only possible if the three things were three
commits to begin with. @status:impl/done

### Commit log as decision history {#why-log}

@fact:THE-MESSAGE-IS-THE-ONLY-PLACE-THE-WHY-IS-RECORDED-DURABLY The commit message is where the *why* of a **change** is recorded at
per-change granularity, bound to its diff and surviving spec prose
decay and WAL overwrites. The *why* of a **decision** has its own
durable home — the sibling `decision-records` flow puts it at the
governing spec anchor
(`spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root`)
— and neither substitutes for the other. @status:impl/done

@fact:three-rationales-in-one-message-each-get-watered-down If one message has to carry three rationales, each one
gets watered down. @status:spec/done

@fact:six-months-later-the-log-reads-refactored-stuff Six months later the log reads "refactored
stuff" instead of "replaced SHA-256 with blake3 because the vendor
library dropped SHA-256 support in 0.9". @status:spec/done

@fact:ATOMIC-COMMITS-MAKE-THE-LOG-USABLE-AS-A-DECISION-ARCHIVE Atomic commits make the log
usable as a decision archive. @status:impl/done

## When to split {#splitting}

@fact:A-WORKING-TREE-WITH-MIXED-CONCERNS-MUST-BE-SPLIT A working tree with mixed concerns must be split. @status:impl/done

@fact:THE-TEST-IS-MECHANICAL-THE-WORD-ALSO The test is
one question: can the commit body be written without using the word
"also"? The word is the trigger; whether an "also" names a second
intent or is mere prose is the reader's call. @status:impl/done

@fact:IF-NO-SPLIT If no, split. @status:impl/done

@fact:common-cases-lead Common cases: @status:impl/done

- @fact:CASE-REFACTOR-PLUS-FEATURE **Refactor + feature.** Always separate. The refactor commit
  leaves behaviour unchanged; the feature commit leaves structure
  unchanged. One of each. @status:impl/done
- @fact:CASE-TESTS-PLUS-IMPLEMENTATION **Tests + implementation they verify.** Usually one commit — both
  halves of one idea. Exception: adding a battery of tests against
  pre-existing code is a test-only commit. @status:impl/done
- @fact:CASE-FORMAT-PLUS-SUBSTANCE **Format + substance.** Always separate. Whitespace-only commits
  are fine; mixing whitespace into a semantic change buries the
  semantic change in noise. @status:impl/done
- @fact:CASE-CROSS-MODULE-CHANGE **Cross-module change touching ten files for one reason.** That
  is still one atomic commit. Do not over-split along file
  boundaries — the atomic unit is the idea, not the file. @status:impl/done

@fact:sibling-document-pointers Mechanical procedure for producing the split:
[`splitting-large-changes.md`](splitting-large-changes.md). @status:impl/done

## When to batch {#batching}

@fact:SOMETIMES-THREE-COUPLED-CHANGES-FORM-ONE-IDEA Sometimes three tightly coupled changes form one idea. @status:impl/done

@fact:example-a-new-type-its-first-user-and-its-test Example:
introducing a new type, the first function that uses it, and the
test that verifies the function. @status:impl/done

@fact:NONE-OF-THE-THREE-MAKES-SENSE-ALONE None of the three makes sense
alone; splitting them produces two intermediate commits where the
tree does not compile. @status:impl/done

@fact:THE-ONE-IDEA-IS-ONE-ATOMIC-COMMIT The one idea is "introduce and test this
new type" — that is one atomic commit. @status:impl/done

@fact:THE-RULE-IS-ONE-IDEA-NOT-ONE-HUNK The rule is: atomic means *one idea*, not *one hunk*. @status:impl/done

@fact:USE-JUDGEMENT Use judgement. @status:impl/done

@fact:THE-WORD-ALSO-IS-STILL-THE-TEST The word "also" is still the test — if the commit body would need
to say "introduces type X, also adds function Y using it, also
tests Y", the phrasing is wrong; if it naturally reads "introduces
and tests type X", it is one commit. @status:impl/done

### Milestone commits {#milestone}

@fact:A-SEQUENCE-OF-ATOMIC-COMMITS-CAN-FORM-A-MILESTONE Some sessions produce a sequence of atomic commits that together
form a recognisable milestone — "implemented FEAT-007" or "M1.1
shipped". @status:impl/done

@fact:THE-MILESTONE-FRAMING-LIVES-ABOVE-THE-COMMIT-LEVEL The individual commits stay atomic; the milestone framing
lives *above* the commit level, in a separate milestone commit (a
tag message, a PR description, or a dedicated `chore(release)`
commit that contains no code changes but narrates the set). @status:impl/done

@fact:DO-NOT-RETRO-FIT-A-MILESTONE-INTO-ONE-GIANT-COMMIT Do not retro-fit a milestone narrative into a single giant commit. @status:impl/done

@fact:THE-ATOMIC-COMMITS-ARE-THE-SOURCE-OF-TRUTH The atomic commits are the source of truth; the milestone is the
story told over them. @status:impl/done

## Pushed history is frozen {#pushed}

@fact:once-a-commit-has-been-pushed-lead Once a commit has been pushed: @status:impl/done

- @fact:FROZEN-NEVER-AMEND **Never** `git commit --amend` without explicit human approval. @status:impl/done
- @fact:FROZEN-NEVER-REBASE-THE-PUSHED-RANGE **Never** `git rebase -i` the pushed range without explicit
  human approval. @status:impl/done
- @fact:FROZEN-NEVER-FORCE-PUSH-WITHOUT-APPROVAL **Never** `git push --force` or `--force-with-lease` without
  explicit human approval. @status:impl/done

@fact:A-MISTAKE-IN-A-PUSHED-COMMIT-IS-FIXED-BY-A-NEW-COMMIT A mistake in a pushed commit is fixed by a new commit (type
`fix` or `revert`), not by rewriting history. @status:impl/done

@fact:non-negotiable-because-others-may-already-have-pulled This is non-negotiable
because other agents and humans may already have pulled the pushed
commits; rewriting history under them corrupts their view of the
repository. @status:spec/done

## The AI advantage {#ai-advantage}

@fact:humans-hate-splitting-commits Humans hate splitting commits. @status:spec/done

@fact:slicing-a-messy-tree-is-cognitively-expensive After a long prototyping session the
working tree is a mess, and slicing it cleanly is cognitively
expensive. @status:spec/done

@fact:most-humans-under-deadline-pressure-skip-the-discipline Most humans under deadline pressure skip the discipline. @status:spec/done

@fact:ai-does-not-get-tired AI does not get tired. @status:spec/done

@fact:ai-is-genuinely-happy-to-decompose-a-messy-tree AI is genuinely happy to read a messy
`git status`, propose a five-commit plan that decomposes the mess
along intent lines, and execute each commit one at a time under
human verification. @status:spec/done

@fact:the-human-ai-team-is-strictly-faster-here This is one of the places where the human-AI
team is strictly faster than either participant alone — **delegate
the split**, verify the plan rather than each commit individually. @status:spec/done

@fact:a-working-prompt-for-delegation-lead A working prompt for delegation: @status:impl/done

```
I have a dirty working tree. Before committing:
1. Run `git status` and `git diff HEAD`.
2. Name every change by intent.
3. Group intents into atomic commits.
4. Show me the proposed split as: commit number, subject line,
   list of files / hunks it will stage.
5. Do NOT run any git commands after the proposal until I approve.
On approval, execute the split one commit at a time, running the
local build between commits. Stop and surface any build failure
before continuing.
```

@fact:THE-HUMAN-VERIFIES-THE-PLAN-NOT-EACH-COMMIT The human verifies the split *plan*, not each commit individually. @status:impl/done

## Summary {#summary}

- @fact:SUM-ONE-COMMIT-ONE-IDEA One commit, one idea. Not one file. @status:impl/done
- @fact:SUM-THE-NO-ALSO-TEST The "no 'also' in the body" test screens for violations; whether an "also" names a second intent is the reader's call. @status:impl/done
- @fact:SUM-WHAT-SEPARATES-AND-WHAT-COMBINES Refactor vs feature: always separate. Tests with their impl:
  usually together. Format vs substance: always separate. @status:impl/done
- @fact:SUM-PUSHED-HISTORY-IS-FROZEN Pushed history is frozen. Amend/force-push only with human
  approval. @status:impl/done
- @fact:SUM-DELEGATE-THE-SPLIT-AND-VERIFY-THE-PLAN Delegate the split of messy trees to the agent; verify the plan. @status:impl/done
