# Flow: Commit autonomy {#root}

<status stage="impl" state="done"/>

@fact:THE-DEFAULT-POSTURE-ON-THE-COMMIT-AND-PUSH-SURFACE The default posture for how much an agent decides on its own versus stops to ask, on the
**commit and push** surface. @status:impl/done

## Routine — proceed without asking {#routine}

@fact:ROUTINE-LARGE-CHANGES-PROCEED-WITHOUT-A-HANDSHAKE Routine large changes proceed and are committed/pushed **without** a confirmation handshake: @status:impl/done

- @fact:ROUTINE-IMPLEMENTING-A-PLANNED-MILESTONE implementing a planned milestone, @status:impl/done
- @fact:ROUTINE-FINISHING-A-FEATURE-SLICE finishing a feature slice, @status:impl/done
- @fact:ROUTINE-TOUCHING-MANY-FILES-FOR-ONE-REASON touching many files for one
  coherent reason. @status:impl/done

@fact:the-approval-was-given-upstream The approval was given upstream, when the work was authorised; a mid-work
"shall I proceed?" is overhead already paid for. @status:spec/done

## Non-routine — stop and ask first {#red-lines}

@fact:ASK-FIRST-FOR-ANYTHING-WHOSE-REVERSAL-COSTS-WORK Ask first for anything whose reversal costs work: @status:impl/done

- @fact:RED-LINE-REWRITING-PUBLISHED-HISTORY rewriting published history (rebase of pushed commits, `git commit --amend` on pushed work); @status:impl/done
- @fact:RED-LINE-FORCE-PUSH `git push --force` / `--force-with-lease`; @status:impl/done
- @fact:RED-LINE-LARGE-BINARY-BLOBS bringing in large binary blobs; @status:impl/done
- @fact:RED-LINE-CI-SIGNING-OR-SECRETS changing CI, signing, or secrets configuration; @status:impl/done
- @fact:RED-LINE-ANYTHING-WHOSE-REVERSAL-WOULD-COST-WORK **any operation whose reversal would cost work.** @status:impl/done

@fact:WHEN-UNCERTAIN-ASK **When uncertain, ask.** @status:impl/done

@fact:sibling-document-pointers Full protocol: @spec://org.vibevm.world/git-autonomy/flows/autonomy/AUTONOMY-PROTOCOL#root. @status:impl/done
