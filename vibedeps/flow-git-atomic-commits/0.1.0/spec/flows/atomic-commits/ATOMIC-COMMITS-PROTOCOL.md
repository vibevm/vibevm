# Atomic Commits Protocol {#root}

**Scope of this document.** This file defines *what* an atomic commit
is, *what* makes a sequence of commits well-formed, *when* a change
set must be split, and *why* this discipline matters more in a
human-AI team than in a traditional team.

## What an atomic commit is {#what}

An atomic commit carries **exactly one idea**. The diff shows every
change required to express that idea, and nothing else.

Three equivalent framings of the same rule:

- **One logical unit.** The smallest set of edits that, together,
  express a complete thought and leave the tree in a consistent state.
- **One intent per message.** If the body needs to say "also, while
  I was in there, …", the "also" belongs in its own commit.
- **One thing to verify.** A reviewer should be able to answer "does
  this commit do what the subject claims?" without having to
  partition the diff first.

A commit that bundles "fix typo in README" + "refactor planner" +
"update schema" is three commits, not one.

## Why atomic commits matter more here {#why}

In a pure-human team, atomic commits are a quality-of-life feature:
easier review, cleaner bisects, viable cherry-picks. Teams that skip
the discipline ship anyway.

In a human-AI team they are **load-bearing**. Three reasons:

### Diff as verification channel {#why-diff}

The human's primary verification mechanism is reading the diff. If
one commit mixes three concerns across eight files and ninety lines,
the human has to mentally partition the diff before assessing any
single piece. That partition step is where mistakes slip through —
the human is now doing a task the commit structure should have done
for them. One concern per commit makes the diff directly verifiable
in one pass.

### Rollback precision {#why-rollback}

Some AI-authored changes will turn out to be wrong. When that
happens, `git revert <sha>` must undo the wrong thing without also
undoing three correct things that happened to ride in the same
commit. That is only possible if the three things were three
commits to begin with.

### Commit log as decision history {#why-log}

The commit message is the only place where the *why* of a change is
recorded at a granularity that survives spec prose decay and WAL
overwrites. If one message has to carry three rationales, each one
gets watered down. Six months later the log reads "refactored
stuff" instead of "replaced SHA-256 with blake3 because the vendor
library dropped SHA-256 support in 0.9". Atomic commits make the log
usable as a decision archive.

## When to split {#splitting}

A working tree with mixed concerns must be split. The test is
mechanical: can the commit body be written without using the word
"also"? If no, split.

Common cases:

- **Refactor + feature.** Always separate. The refactor commit
  leaves behaviour unchanged; the feature commit leaves structure
  unchanged. One of each.
- **Tests + implementation they verify.** Usually one commit — both
  halves of one idea. Exception: adding a battery of tests against
  pre-existing code is a test-only commit.
- **Format + substance.** Always separate. Whitespace-only commits
  are fine; mixing whitespace into a semantic change buries the
  semantic change in noise.
- **Cross-module change touching ten files for one reason.** That
  is still one atomic commit. Do not over-split along file
  boundaries — the atomic unit is the idea, not the file.

Mechanical procedure for producing the split:
[`splitting-large-changes.md`](splitting-large-changes.md).

## When to batch {#batching}

Sometimes three tightly coupled changes form one idea. Example:
introducing a new type, the first function that uses it, and the
test that verifies the function. None of the three makes sense
alone; splitting them produces two intermediate commits where the
tree does not compile. The one idea is "introduce and test this
new type" — that is one atomic commit.

The rule is: atomic means *one idea*, not *one hunk*. Use judgement.
The word "also" is still the test — if the commit body would need
to say "introduces type X, also adds function Y using it, also
tests Y", the phrasing is wrong; if it naturally reads "introduces
and tests type X", it is one commit.

### Milestone commits {#milestone}

Some sessions produce a sequence of atomic commits that together
form a recognisable milestone — "implemented FEAT-007" or "M1.1
shipped". The individual commits stay atomic; the milestone framing
lives *above* the commit level, in a separate milestone commit (a
tag message, a PR description, or a dedicated `chore(release)`
commit that contains no code changes but narrates the set).

Do not retro-fit a milestone narrative into a single giant commit.
The atomic commits are the source of truth; the milestone is the
story told over them.

## Pushed history is frozen {#pushed}

Once a commit has been pushed:

- **Never** `git commit --amend`.
- **Never** `git rebase -i` the pushed range.
- **Never** `git push --force` or `--force-with-lease` without
  explicit human approval.

A mistake in a pushed commit is fixed by a new commit (type
`fix` or `revert`), not by rewriting history. This is non-negotiable
because other agents and humans may already have pulled the pushed
commits; rewriting history under them corrupts their view of the
repository.

## The AI advantage {#ai-advantage}

Humans hate splitting commits. After a long prototyping session the
working tree is a mess, and slicing it cleanly is cognitively
expensive. Most humans under deadline pressure skip the discipline.

AI does not get tired. AI is genuinely happy to read a messy
`git status`, propose a five-commit plan that decomposes the mess
along intent lines, and execute each commit one at a time under
human verification. This is one of the places where the human-AI
team is strictly faster than either participant alone — **delegate
the split**, verify the plan rather than each commit individually.

A working prompt for delegation:

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

The human verifies the split *plan*, not each commit individually.

## Summary {#summary}

- One commit, one idea. Not one file.
- The "no 'also' in the body" test catches violations mechanically.
- Refactor vs feature: always separate. Tests with their impl:
  usually together. Format vs substance: always separate.
- Pushed history is frozen. Amend/force-push only with human
  approval.
- Delegate the split of messy trees to the agent; verify the plan.
