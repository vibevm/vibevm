# Flow: Atomic Commits {#root}

This project uses the **atomic commits** discipline as its Git
contract. One commit carries exactly one idea, and the commit message
explains *why*.

## Core rule

**One commit = one logical change**, not one file changed.

A session that produces (a) a typo fix, (b) a refactor, and (c) a
schema update is **three** commits, not one. A feature that touches
fifteen files for one coherent reason is **one** commit, not fifteen.

## Format

Every commit follows [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): short imperative subject line

Longer body explaining WHY this change was made and what follows from
it. Cite spec://… URIs where relevant.
```

Subject ≤ 60 characters (hard limit 72). Body answers *why*, not *what*
— the diff already shows what changed. Full format in
[`spec/flows/atomic-commits/conventional-commits.md`](../flows/atomic-commits/conventional-commits.md).

## Session end

Before closing a session:

1. Run `git status` and `git diff HEAD`. Name every change by intent.
2. Group changes into atomic commits — one commit per intent, not per
   file.
3. Stage and commit each group separately with a well-formed message.
4. Do not amend or force-push commits that are already pushed; create a
   new commit instead, unless the human explicitly approves history
   rewriting.

Procedure for splitting a messy working tree:
[`spec/flows/atomic-commits/splitting-large-changes.md`](../flows/atomic-commits/splitting-large-changes.md).

## Why this matters in a human-AI team

- **Diff as verification.** Humans verify code by reading diffs. A
  diff that mixes three concerns is not verifiable in one pass.
- **Rollback precision.** `git revert <sha>` must undo the wrong
  thing without also undoing two correct things.
- **Commit log as decision record.** The message is the only place
  where *why* survives after the WAL and spec prose decay.

Full rationale: [`spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md`](../flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md).

## Never

- Never mix refactor + feature + docs + bugfix in one commit.
- Never write a subject that summarises *what* changed — the diff does
  that. Write *why*.
- Never `git commit --amend` on a pushed commit without explicit human
  approval. Same for `git push --force`.
