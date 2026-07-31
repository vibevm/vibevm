# Atomic Commits Protocol {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines *what* an atomic commit
is, *what* makes a sequence of commits well-formed, *when* a change
set must be split, and *why* this discipline matters more in a
human-AI team than in a traditional team. @impl/done

## What an atomic commit is {#what}

##AN-ATOMIC-COMMIT-CARRIES-EXACTLY-ONE-IDEA An atomic commit carries **exactly one idea**. @impl/done

##THE-DIFF-SHOWS-EVERY-CHANGE-AND-NOTHING-ELSE The diff shows every
change required to express that idea, and nothing else. @impl/done

##three-equivalent-framings-lead Three equivalent framings of the same rule: @impl/done

- ##FRAMING-ONE-LOGICAL-UNIT **One logical unit.** The smallest set of edits that, together,
  express a complete thought and leave the tree in a consistent state. @impl/done
- ##FRAMING-ONE-INTENT-PER-MESSAGE **One intent per message.** If the body needs to say "also, while
  I was in there, …", the "also" belongs in its own commit. @impl/done
- ##FRAMING-ONE-THING-TO-VERIFY **One thing to verify.** A reviewer should be able to answer "does
  this commit do what the subject claims?" without having to
  partition the diff first. @impl/done

##A-BUNDLED-COMMIT-IS-THREE-COMMITS-NOT-ONE A commit that bundles "fix typo in README" + "refactor planner" +
"update schema" is three commits, not one. @impl/done

## Why atomic commits matter more here {#why}

##in-a-pure-human-team-they-are-a-quality-of-life-feature In a pure-human team, atomic commits are a quality-of-life feature: @spec/done

- ##QOL-EASIER-REVIEW easier review, @spec/done
- ##QOL-CLEANER-BISECTS cleaner bisects, @spec/done
- ##QOL-VIABLE-CHERRY-PICKS viable cherry-picks. @spec/done

##teams-that-skip-the-discipline-ship-anyway Teams that skip
the discipline ship anyway. @spec/done

##in-a-human-ai-team-they-are-load-bearing In a human-AI team they are **load-bearing**. Three reasons: @spec/done

### Diff as verification channel {#why-diff}

##the-humans-primary-verification-mechanism-is-reading-the-diff The human's primary verification mechanism is reading the diff. @spec/done

##a-mixed-commit-forces-the-human-to-partition-it-first If
one commit mixes three concerns across eight files and ninety lines,
the human has to mentally partition the diff before assessing any
single piece. @spec/done

##the-partition-step-is-where-mistakes-slip-through That partition step is where mistakes slip through —
the human is now doing a task the commit structure should have done
for them. @spec/done

##ONE-CONCERN-PER-COMMIT-MAKES-THE-DIFF-VERIFIABLE-IN-ONE-PASS One concern per commit makes the diff directly verifiable
in one pass. @impl/done

### Rollback precision {#why-rollback}

##some-ai-authored-changes-will-turn-out-to-be-wrong Some AI-authored changes will turn out to be wrong. @spec/done

##REVERT-MUST-UNDO-THE-WRONG-THING-AND-NOTHING-ELSE When that
happens, `git revert <sha>` must undo the wrong thing without also
undoing three correct things that happened to ride in the same
commit. @impl/done

##THAT-IS-ONLY-POSSIBLE-IF-THEY-WERE-THREE-COMMITS-TO-BEGIN-WITH That is only possible if the three things were three
commits to begin with. @impl/done

### Commit log as decision history {#why-log}

##THE-MESSAGE-IS-THE-ONLY-PLACE-THE-WHY-IS-RECORDED-DURABLY The commit message is where the *why* of a **change** is recorded at
per-change granularity, bound to its diff and surviving spec prose
decay and WAL overwrites. The *why* of a **decision** has its own
durable home — the sibling `decision-records` flow puts it at the
governing spec anchor
(`spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root`)
— and neither substitutes for the other. @impl/done

##three-rationales-in-one-message-each-get-watered-down If one message has to carry three rationales, each one
gets watered down. @spec/done

##six-months-later-the-log-reads-refactored-stuff Six months later the log reads "refactored
stuff" instead of "replaced SHA-256 with blake3 because the vendor
library dropped SHA-256 support in 0.9". @spec/done

##ATOMIC-COMMITS-MAKE-THE-LOG-USABLE-AS-A-DECISION-ARCHIVE Atomic commits make the log
usable as a decision archive. @impl/done

## When to split {#splitting}

##A-WORKING-TREE-WITH-MIXED-CONCERNS-MUST-BE-SPLIT A working tree with mixed concerns must be split. @impl/done

##THE-TEST-IS-MECHANICAL-THE-WORD-ALSO The test is
one question: can the commit body be written without using the word
"also"? The word is the trigger; whether an "also" names a second
intent or is mere prose is the reader's call. @impl/done

##IF-NO-SPLIT If no, split. @impl/done

##common-cases-lead Common cases: @impl/done

- ##CASE-REFACTOR-PLUS-FEATURE **Refactor + feature.** Always separate. The refactor commit
  leaves behaviour unchanged; the feature commit leaves structure
  unchanged. One of each. @impl/done
- ##CASE-TESTS-PLUS-IMPLEMENTATION **Tests + implementation they verify.** Usually one commit — both
  halves of one idea. Exception: adding a battery of tests against
  pre-existing code is a test-only commit. @impl/done
- ##CASE-FORMAT-PLUS-SUBSTANCE **Format + substance.** Always separate. Whitespace-only commits
  are fine; mixing whitespace into a semantic change buries the
  semantic change in noise. @impl/done
- ##CASE-CROSS-MODULE-CHANGE **Cross-module change touching ten files for one reason.** That
  is still one atomic commit. Do not over-split along file
  boundaries — the atomic unit is the idea, not the file. @impl/done

##sibling-document-pointers Mechanical procedure for producing the split:
[`splitting-large-changes.md`](splitting-large-changes.md). @impl/done

## When to batch {#batching}

##SOMETIMES-THREE-COUPLED-CHANGES-FORM-ONE-IDEA Sometimes three tightly coupled changes form one idea. @impl/done

##example-a-new-type-its-first-user-and-its-test Example:
introducing a new type, the first function that uses it, and the
test that verifies the function. @impl/done

##NONE-OF-THE-THREE-MAKES-SENSE-ALONE None of the three makes sense
alone; splitting them produces two intermediate commits where the
tree does not compile. @impl/done

##THE-ONE-IDEA-IS-ONE-ATOMIC-COMMIT The one idea is "introduce and test this
new type" — that is one atomic commit. @impl/done

##THE-RULE-IS-ONE-IDEA-NOT-ONE-HUNK The rule is: atomic means *one idea*, not *one hunk*. @impl/done

##USE-JUDGEMENT Use judgement. @impl/done

##THE-WORD-ALSO-IS-STILL-THE-TEST The word "also" is still the test — if the commit body would need
to say "introduces type X, also adds function Y using it, also
tests Y", the phrasing is wrong; if it naturally reads "introduces
and tests type X", it is one commit. @impl/done

### Milestone commits {#milestone}

##A-SEQUENCE-OF-ATOMIC-COMMITS-CAN-FORM-A-MILESTONE Some sessions produce a sequence of atomic commits that together
form a recognisable milestone — "implemented FEAT-007" or "M1.1
shipped". @impl/done

##THE-MILESTONE-FRAMING-LIVES-ABOVE-THE-COMMIT-LEVEL The individual commits stay atomic; the milestone framing
lives *above* the commit level, in a separate milestone commit (a
tag message, a PR description, or a dedicated `chore(release)`
commit that contains no code changes but narrates the set). @impl/done

##DO-NOT-RETRO-FIT-A-MILESTONE-INTO-ONE-GIANT-COMMIT Do not retro-fit a milestone narrative into a single giant commit. @impl/done

##THE-ATOMIC-COMMITS-ARE-THE-SOURCE-OF-TRUTH The atomic commits are the source of truth; the milestone is the
story told over them. @impl/done

## Pushed history is frozen {#pushed}

##once-a-commit-has-been-pushed-lead Once a commit has been pushed: @impl/done

- ##FROZEN-NEVER-AMEND **Never** `git commit --amend` without explicit human approval. @impl/done
- ##FROZEN-NEVER-REBASE-THE-PUSHED-RANGE **Never** `git rebase -i` the pushed range without explicit
  human approval. @impl/done
- ##FROZEN-NEVER-FORCE-PUSH-WITHOUT-APPROVAL **Never** `git push --force` or `--force-with-lease` without
  explicit human approval. @impl/done

##A-MISTAKE-IN-A-PUSHED-COMMIT-IS-FIXED-BY-A-NEW-COMMIT A mistake in a pushed commit is fixed by a new commit (type
`fix` or `revert`), not by rewriting history. @impl/done

##non-negotiable-because-others-may-already-have-pulled This is non-negotiable
because other agents and humans may already have pulled the pushed
commits; rewriting history under them corrupts their view of the
repository. @spec/done

## The AI advantage {#ai-advantage}

##humans-hate-splitting-commits Humans hate splitting commits. @spec/done

##slicing-a-messy-tree-is-cognitively-expensive After a long prototyping session the
working tree is a mess, and slicing it cleanly is cognitively
expensive. @spec/done

##most-humans-under-deadline-pressure-skip-the-discipline Most humans under deadline pressure skip the discipline. @spec/done

##ai-does-not-get-tired AI does not get tired. @spec/done

##ai-is-genuinely-happy-to-decompose-a-messy-tree AI is genuinely happy to read a messy
`git status`, propose a five-commit plan that decomposes the mess
along intent lines, and execute each commit one at a time under
human verification. @spec/done

##the-human-ai-team-is-strictly-faster-here This is one of the places where the human-AI
team is strictly faster than either participant alone — **delegate
the split**, verify the plan rather than each commit individually. @spec/done

##a-working-prompt-for-delegation-lead A working prompt for delegation: @impl/done

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

##THE-HUMAN-VERIFIES-THE-PLAN-NOT-EACH-COMMIT The human verifies the split *plan*, not each commit individually. @impl/done

## Summary {#summary}

- ##SUM-ONE-COMMIT-ONE-IDEA One commit, one idea. Not one file. @impl/done
- ##SUM-THE-NO-ALSO-TEST The "no 'also' in the body" test screens for violations; whether an "also" names a second intent is the reader's call. @impl/done
- ##SUM-WHAT-SEPARATES-AND-WHAT-COMBINES Refactor vs feature: always separate. Tests with their impl:
  usually together. Format vs substance: always separate. @impl/done
- ##SUM-PUSHED-HISTORY-IS-FROZEN Pushed history is frozen. Amend/force-push only with human
  approval. @impl/done
- ##SUM-DELEGATE-THE-SPLIT-AND-VERIFY-THE-PLAN Delegate the split of messy trees to the agent; verify the plan. @impl/done
