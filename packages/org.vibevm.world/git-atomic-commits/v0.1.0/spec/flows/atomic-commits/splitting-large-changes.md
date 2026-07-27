# Splitting a large working tree into atomic commits {#root}

<status stage="spec" state="done"/>

##a-long-session-leaves-the-tree-looking-like-a-weather-map A long prototyping session often leaves the working tree looking
like a weather map: changes everywhere, some connected, some not. @spec/done

##this-document-is-the-splitting-procedure This document is the procedure for turning that mess into a
sequence of atomic commits without losing or reordering work. @impl/done

## Precondition {#precondition}

##THE-TREE-MUST-BUILD-AND-IDEALLY-PASS-TESTS-BEFORE-STARTING Before starting, the working tree must build and (ideally) pass
tests. @impl/done

##STABILISE-TO-GREEN-BEFORE-SPLITTING-A-BROKEN-TREE If the tree is broken end to end, do not try to split yet —
stabilise to a green state first, on a single WIP commit if
necessary, then split from the green state. @impl/done

##a-broken-starting-point-means-a-moving-target A broken intermediate during a split is inevitable sometimes, but a
broken *starting* point means the split is working with a moving
target. @spec/done

## Step 1 — inventory {#inventory}

##run-lead Run: @impl/done

```
git status
git diff --stat HEAD
git diff HEAD
```

##READ-THE-OUTPUT-END-TO-END Read the output end to end. @impl/done

##NAME-EVERY-CHANGED-CHUNK-BY-INTENT-NOT-BY-FILE The goal is to name every changed
chunk by **intent**, not by file. @impl/done

##produce-a-list-lead Produce a list in a scratch
document (or a chat message to the agent): @impl/done

```
- Added retry helper in retry.rs
- Used retry helper in http_client.rs
- Added test for retry helper
- Renamed `conn` to `connection` in connection.rs
- Fixed typo in README
- Reformatted unrelated imports in lib.rs
```

##six-items-six-intents Six items. Six intents. @impl/done

##NOT-NECESSARILY-SIX-COMMITS Not necessarily six commits — some will
collapse together in the next step. @impl/done

## Step 2 — group {#group}

##collapse-items-that-express-the-same-idea-lead Collapse items that express the same idea into one commit: @impl/done

- ##GROUP-EXAMPLE-RETRY-HELPER "Added retry helper" + "Used retry helper" + "Added test for retry
  helper" → one commit: `feat(core): retry helper for transient
  failures`. These three changes are one idea. @impl/done
- ##GROUP-EXAMPLE-RENAME "Renamed `conn` to `connection`" → one commit:
  `refactor(core): rename conn to connection`. Mechanical rename,
  one concern. @impl/done
- ##GROUP-EXAMPLE-TYPO-FIX "Fixed typo in README" → one commit: `docs: fix typo in README`.
  Trivially one concern. @impl/done
- ##GROUP-EXAMPLE-REFORMATTED-IMPORTS "Reformatted unrelated imports" → one commit:
  `style(core): reformat imports`. Trivially one concern, but see
  the note below. @impl/done

##six-items-collapsed-to-four-commits Six items collapsed to four commits. @impl/done

##THAT-IS-THE-SPLIT-PLAN That is the split plan. @impl/done

### Note on accidental changes {#accidental}

##reformatted-imports-are-often-an-auto-format-footprint "Reformatted unrelated imports" is often the footprint of an
auto-format-on-save firing during a different task. @spec/done

##CONSIDER-REVERTING-AN-UNINTENTIONAL-REFORMAT If the reformat
was not intentional, consider reverting it rather than shipping a
`style` commit for it — tidy is not the same as intentional. @impl/done

##AN-INTENTIONAL-PASS-IS-A-STYLE-COMMIT-A-STRAY-ONE-IS-CRUFT An
intentional formatting pass is a style commit; a stray one is
cruft. @impl/done

## Step 3 — apply {#apply}

##USE-GIT-ADD-P-TO-STAGE-EXACTLY-THE-HUNKS-THAT-BELONG For each planned commit, use `git add -p` (patch mode) to stage
exactly the hunks that belong to it. @impl/done

##THEN-COMMIT-WITH-THE-APPROPRIATE-CONVENTIONAL-COMMITS-MESSAGE Then commit with the
appropriate Conventional Commits message (the format is the
`conventional-commits` flow: `spec://org.vibevm.world/git-conventional-commits/flows/conventional-commits/conventional-commits#root`). @impl/done

```
git add -p                # stage retry-feature hunks
git commit                # message: feat(core): retry helper ...

git add -p                # stage rename hunks
git commit                # message: refactor(core): rename conn ...

git add README.md
git commit                # message: docs: fix typo in README

git add -p                # stage formatting hunks (or skip if accidental)
git commit                # message: style(core): reformat imports
```

##RUN-GIT-STATUS-AFTER-EACH-COMMIT After each commit run `git status` to verify the remaining tree
contains only what has not yet been split — no stray staging, no
hidden files dragged in by a wildcard. @impl/done

### `git add -A` / `git add .` {#add-all}

##AVOID-GIT-ADD-ALL-WHILE-SPLITTING Avoid `git add -A` and `git add .` while splitting. @impl/done

##THEY-STAGE-EVERYTHING-AND-DEFEAT-THE-PATCH-MODE-INTENT They stage
*everything*, which defeats the patch-mode intent and may pull in
files you did not plan to commit (scratch files, credentials,
large binary outputs). @impl/done

##USE-EXPLICIT-FILE-NAMES-OR-GIT-ADD-P Use explicit file names or `git add -p`. @impl/done

## Step 4 — verify build between commits {#verify}

##EACH-INTERMEDIATE-COMMIT-SHOULD-LEAVE-THE-TREE-BUILDABLE Whenever feasible, each intermediate commit should leave the tree
buildable. @impl/done

##run-the-build-after-each-commit-lead Run the build (and ideally tests) after each commit,
not only at the end: @impl/done

```
cargo build    # or the local equivalent
cargo test     # where fast enough to keep in the loop
```

##BROKEN-INTERMEDIATES-ARE-A-BISECT-LIABILITY A split that produces two compilable commits plus two broken
intermediate commits is a bisect liability — `git bisect` will
hit the broken intermediates and mis-attribute failures. @impl/done

##SQUASH-THE-INTERMEDIATES-WHEN-GREENNESS-IS-IMPRACTICAL If strict between-commit greenness is impractical (for example, a
rename that must touch six files atomically and cannot be usefully
decomposed further), squash the intermediates so at least the
boundary is clean. @impl/done

## Step 5 — spot check the log {#spotcheck}

##after-the-split-lead After the split: @impl/done

```
git log --oneline -n <N>
```

##READ-THE-SUBJECT-LINES-BACK Read the subject lines back. @impl/done

##EACH-SUBJECT-SHOULD-MAKE-SENSE-IN-ISOLATION Each should make sense in isolation. @impl/done

##STOP-ON-AN-UNINFORMATIVE-SUBJECT-LINE If you see `chore: more stuff`, `wip`, or `fix: address review` in
the list, stop — the split is incomplete or the message is
uninformative. @impl/done

##REWRITE-BEFORE-MOVING-ON Rewrite before moving on. @impl/done

## Delegating to the agent {#delegate}

##the-agent-is-better-at-this-than-most-humans-under-pressure This is a highly mechanical task and the agent is genuinely better
at it than most humans under time pressure. A working prompt: @spec/done

```
I have a dirty working tree. Before committing:
1. Run `git status` and `git diff HEAD`.
2. Name every change by intent (not by file).
3. Group intents into atomic commits.
4. Show me the proposed split as: commit number, subject line,
   list of files / hunks it will stage.
5. Do NOT run any git commands after the proposal until I approve.

On approval:
- Execute the split one commit at a time.
- Run `cargo build` (or the local equivalent) between commits.
- Stop and surface any build failure before continuing.
- Do NOT push. That is a separate, explicit step.
```

##THE-HUMAN-VERIFIES-THE-SPLIT-PLAN The human verifies the **split plan**, not each individual
`git commit` invocation. @impl/done

##that-is-where-the-division-of-labour-pays-off That is where the division of labour
actually pays off. @spec/done

## Before pushing {#prepush}

##after-the-split-before-pushing-lead After the split, before `git push`: @impl/done

1. ##PREPUSH-RE-READ-THE-LOG Re-read the log (`git log --oneline origin/<branch>..HEAD`). @impl/done
2. ##PREPUSH-SKIM-EACH-DIFF Skim each diff (`git show <sha>`). Subject should match diff. @impl/done
3. ##PREPUSH-CHECK-FOR-SECRETS-AND-STRAYS Check for secrets, large binaries, or scratch files that
   slipped in. @impl/done
4. ##PREPUSH-ONLY-NOW-PUSH Only now push. @impl/done

##PUSHED-HISTORY-IS-FROZEN Pushed history is frozen (see
[`ATOMIC-COMMITS-PROTOCOL.md` §pushed](ATOMIC-COMMITS-PROTOCOL.md#pushed)). @impl/done

##A-MISTAKE-AFTER-THE-PUSH-MEANS-A-NEW-COMMIT Fixing a mistake after the push means a new `fix`/`revert` commit,
not a force-push. @impl/done

## Summary {#summary}

- ##SUM-STABILISE-THEN-SPLIT Stabilise to green first; split second. @impl/done
- ##SUM-INVENTORY-AND-GROUP-BY-INTENT Inventory by intent, not by file. Group by intent. @impl/done
- ##SUM-STAGE-HUNKS-WITH-GIT-ADD-P Use `git add -p` to stage hunks per commit. Avoid `git add -A`. @impl/done
- ##SUM-BUILD-BETWEEN-COMMITS Build between commits when feasible. @impl/done
- ##SUM-DELEGATE-AND-VERIFY-THE-PLAN Delegate the mechanical split to the agent; verify the plan,
  not the typing. @impl/done
- ##SUM-PRE-PUSH-REVIEW-IS-SEPARATE Pre-push review is a separate, deliberate step. @impl/done
