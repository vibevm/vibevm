# Flow: Atomic Commits {#root}

<status stage="impl" state="done"/>

@fact:THE-PROJECT-USES-THE-ATOMIC-COMMITS-DISCIPLINE This project uses the **atomic commits** discipline as its Git
contract. @status:impl/done

@fact:ONE-COMMIT-ONE-IDEA-AND-THE-MESSAGE-EXPLAINS-WHY One commit carries exactly one idea, and the commit message
explains *why*. @status:impl/done

## Core rule {#core-rule}

@fact:ONE-COMMIT-EQUALS-ONE-LOGICAL-CHANGE **One commit = one logical change**, not one file changed. @status:impl/done

@fact:A-MIXED-SESSION-IS-THREE-COMMITS A session that produces (a) a typo fix, (b) a refactor, and (c) a
schema update is **three** commits, not one. @status:impl/done

@fact:A-COHERENT-FIFTEEN-FILE-CHANGE-IS-ONE-COMMIT A feature that touches
fifteen files for one coherent reason is **one** commit, not fifteen. @status:impl/done

## Message format {#message-format}

@fact:COMMIT-MESSAGES-FOLLOW-THE-CONVENTIONAL-COMMITS-FLOW Commit messages follow the **git-conventional-commits** flow — a sibling package:
`spec://org.vibevm.world/git-conventional-commits/flows/conventional-commits/conventional-commits#root`. @status:impl/done

@fact:CONVENTIONAL-COMMITS-IS-THE-FORMAT-THIS-FLOW-IS-THE-ATOMICITY Conventional Commits is the *format*; this flow is the *atomicity* (one commit, one idea). @status:impl/done

@fact:THE-TWO-ARE-DISTINCT-AND-RUN-TOGETHER The
two are distinct and run together — a `feat: add foo, bar, baz` message is valid Conventional
Commits and a violation of the atomic rule. @status:impl/done

## Session end {#session-end}

@fact:before-closing-a-session-lead Before closing a session: @status:impl/done

1. @fact:STEP-RUN-STATUS-AND-DIFF-AND-NAME-EVERY-CHANGE Run `git status` and `git diff HEAD`. Name every change by intent. @status:impl/done
2. @fact:STEP-GROUP-CHANGES-INTO-ATOMIC-COMMITS Group changes into atomic commits — one commit per intent, not per
   file. @status:impl/done
3. @fact:STEP-STAGE-AND-COMMIT-EACH-GROUP-SEPARATELY Stage and commit each group separately with a well-formed message. @status:impl/done
4. @fact:STEP-DO-NOT-AMEND-OR-FORCE-PUSH-PUSHED-COMMITS Do not amend or force-push commits that are already pushed; create a
   new commit instead, unless the human explicitly approves history
   rewriting. @status:impl/done

@fact:splitting-procedure-pointer Procedure for splitting a messy working tree:
@spec://org.vibevm.world/git-atomic-commits/flows/atomic-commits/splitting-large-changes#root. @status:impl/done

## Why this matters in a human-AI team {#why-human-ai-teams}

- @fact:WHY-DIFF-AS-VERIFICATION **Diff as verification.** Humans verify code by reading diffs. A
  diff that mixes three concerns is not verifiable in one pass. @status:spec/done
- @fact:WHY-ROLLBACK-PRECISION **Rollback precision.** `git revert <sha>` must undo the wrong
  thing without also undoing two correct things. @status:impl/done
- @fact:WHY-COMMIT-LOG-AS-DECISION-RECORD **Commit log as decision record.** The message is the only place
  where *why* survives after the WAL and spec prose decay. @status:impl/done

@fact:full-protocol-pointer Full rationale: @spec://org.vibevm.world/git-atomic-commits/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL#root. @status:impl/done

## Never {#never}

- @fact:NEVER-MIX-REFACTOR-FEATURE-DOCS-AND-BUGFIX Never mix refactor + feature + docs + bugfix in one commit. @status:impl/done
- @fact:NEVER-WRITE-A-SUBJECT-THAT-SUMMARISES-WHAT-CHANGED Never write a subject that summarises *what* changed — the diff does
  that. Write *why*. @status:impl/done
- @fact:NEVER-AMEND-A-PUSHED-COMMIT-WITHOUT-HUMAN-APPROVAL Never `git commit --amend` on a pushed commit without explicit human
  approval. Same for `git push --force`. @status:impl/done
