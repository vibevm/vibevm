# Splitting a large working tree into atomic commits {#root}

A long prototyping session often leaves the working tree looking
like a weather map: changes everywhere, some connected, some not.
This document is the procedure for turning that mess into a
sequence of atomic commits without losing or reordering work.

## Precondition {#precondition}

Before starting, the working tree must build and (ideally) pass
tests. If the tree is broken end to end, do not try to split yet —
stabilise to a green state first, on a single WIP commit if
necessary, then split from the green state.

A broken intermediate during a split is inevitable sometimes, but a
broken *starting* point means the split is working with a moving
target.

## Step 1 — inventory {#inventory}

Run:

```
git status
git diff --stat HEAD
git diff HEAD
```

Read the output end to end. The goal is to name every changed
chunk by **intent**, not by file. Produce a list in a scratch
document (or a chat message to the agent):

```
- Added retry helper in retry.rs
- Used retry helper in http_client.rs
- Added test for retry helper
- Renamed `conn` to `connection` in connection.rs
- Fixed typo in README
- Reformatted unrelated imports in lib.rs
```

Six items. Six intents. Not necessarily six commits — some will
collapse together in the next step.

## Step 2 — group {#group}

Collapse items that express the same idea into one commit:

- "Added retry helper" + "Used retry helper" + "Added test for retry
  helper" → one commit: `feat(core): retry helper for transient
  failures`. These three changes are one idea.
- "Renamed `conn` to `connection`" → one commit:
  `refactor(core): rename conn to connection`. Mechanical rename,
  one concern.
- "Fixed typo in README" → one commit: `docs: fix typo in README`.
  Trivially one concern.
- "Reformatted unrelated imports" → one commit:
  `style(core): reformat imports`. Trivially one concern, but see
  the note below.

Six items collapsed to four commits. That is the split plan.

### Note on accidental changes {#accidental}

"Reformatted unrelated imports" is often the footprint of an
auto-format-on-save firing during a different task. If the reformat
was not intentional, consider reverting it rather than shipping a
`style` commit for it — tidy is not the same as intentional. An
intentional formatting pass is a style commit; a stray one is
cruft.

## Step 3 — apply {#apply}

For each planned commit, use `git add -p` (patch mode) to stage
exactly the hunks that belong to it. Then commit with the
appropriate Conventional Commits message (the format is the
`conventional-commits` flow: `spec://org.vibevm.world/git-conventional-commits/flows/conventional-commits/conventional-commits#root`).

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

After each commit run `git status` to verify the remaining tree
contains only what has not yet been split — no stray staging, no
hidden files dragged in by a wildcard.

### `git add -A` / `git add .` {#add-all}

Avoid `git add -A` and `git add .` while splitting. They stage
*everything*, which defeats the patch-mode intent and may pull in
files you did not plan to commit (scratch files, credentials,
large binary outputs). Use explicit file names or `git add -p`.

## Step 4 — verify build between commits {#verify}

Whenever feasible, each intermediate commit should leave the tree
buildable. Run the build (and ideally tests) after each commit,
not only at the end:

```
cargo build    # or the local equivalent
cargo test     # where fast enough to keep in the loop
```

A split that produces two compilable commits plus two broken
intermediate commits is a bisect liability — `git bisect` will
hit the broken intermediates and mis-attribute failures.

If strict between-commit greenness is impractical (for example, a
rename that must touch six files atomically and cannot be usefully
decomposed further), squash the intermediates so at least the
boundary is clean.

## Step 5 — spot check the log {#spotcheck}

After the split:

```
git log --oneline -n <N>
```

Read the subject lines back. Each should make sense in isolation.
If you see `chore: more stuff`, `wip`, or `fix: address review` in
the list, stop — the split is incomplete or the message is
uninformative. Rewrite before moving on.

## Delegating to the agent {#delegate}

This is a highly mechanical task and the agent is genuinely better
at it than most humans under time pressure. A working prompt:

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

The human verifies the **split plan**, not each individual
`git commit` invocation. That is where the division of labour
actually pays off.

## Before pushing {#prepush}

After the split, before `git push`:

1. Re-read the log (`git log --oneline origin/<branch>..HEAD`).
2. Skim each diff (`git show <sha>`). Subject should match diff.
3. Check for secrets, large binaries, or scratch files that
   slipped in.
4. Only now push.

Pushed history is frozen (see
[`ATOMIC-COMMITS-PROTOCOL.md` §pushed](ATOMIC-COMMITS-PROTOCOL.md#pushed)).
Fixing a mistake after the push means a new `fix`/`revert` commit,
not a force-push.

## Summary {#summary}

- Stabilise to green first; split second.
- Inventory by intent, not by file. Group by intent.
- Use `git add -p` to stage hunks per commit. Avoid `git add -A`.
- Build between commits when feasible.
- Delegate the mechanical split to the agent; verify the plan,
  not the typing.
- Pre-push review is a separate, deliberate step.
