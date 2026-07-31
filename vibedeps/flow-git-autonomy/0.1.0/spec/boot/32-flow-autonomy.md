# Flow: Commit autonomy {#root}

<status stage="impl" state="done"/>

##THE-DEFAULT-POSTURE-ON-THE-COMMIT-AND-PUSH-SURFACE The default posture for how much an agent decides on its own versus stops to ask, on the
**commit and push** surface. @impl/done

## Routine — proceed without asking {#routine}

##ROUTINE-LARGE-CHANGES-PROCEED-WITHOUT-A-HANDSHAKE Routine large changes proceed and are committed/pushed **without** a confirmation handshake: @impl/done

- ##ROUTINE-IMPLEMENTING-A-PLANNED-MILESTONE implementing a planned milestone, @impl/done
- ##ROUTINE-FINISHING-A-FEATURE-SLICE finishing a feature slice, @impl/done
- ##ROUTINE-TOUCHING-MANY-FILES-FOR-ONE-REASON touching many files for one
  coherent reason. @impl/done

##the-approval-was-given-upstream The approval was given upstream, when the work was authorised; a mid-work
"shall I proceed?" is overhead already paid for. @spec/done

## Non-routine — stop and ask first {#red-lines}

##ASK-FIRST-FOR-ANYTHING-WHOSE-REVERSAL-COSTS-WORK Ask first for anything whose reversal costs work: @impl/done

- ##RED-LINE-REWRITING-PUBLISHED-HISTORY rewriting published history (rebase of pushed commits, `git commit --amend` on pushed work); @impl/done
- ##RED-LINE-FORCE-PUSH `git push --force` / `--force-with-lease`; @impl/done
- ##RED-LINE-LARGE-BINARY-BLOBS bringing in large binary blobs; @impl/done
- ##RED-LINE-CI-SIGNING-OR-SECRETS changing CI, signing, or secrets configuration; @impl/done
- ##RED-LINE-ANYTHING-WHOSE-REVERSAL-WOULD-COST-WORK **any operation whose reversal would cost work.** @impl/done

##WHEN-UNCERTAIN-ASK **When uncertain, ask.** @impl/done

##sibling-document-pointers Full protocol: @spec://org.vibevm.world/git-autonomy/flows/autonomy/AUTONOMY-PROTOCOL#root. @impl/done
