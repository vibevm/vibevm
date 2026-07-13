# Flow: Commit autonomy {#root}

The default posture for how much an agent decides on its own versus stops to ask, on the
**commit and push** surface.

## Routine — proceed without asking {#routine}

Routine large changes proceed and are committed/pushed **without** a confirmation handshake:
implementing a planned milestone, finishing a feature slice, touching many files for one
coherent reason. The approval was given upstream, when the work was authorised; a mid-work
"shall I proceed?" is overhead already paid for.

## Non-routine — stop and ask first {#red-lines}

Ask first for anything whose reversal costs work:

- rewriting published history (rebase of pushed commits, `git commit --amend` on pushed work);
- `git push --force` / `--force-with-lease`;
- bringing in large binary blobs;
- changing CI, signing, or secrets configuration;
- **any operation whose reversal would cost work.**

**When uncertain, ask.**

Full protocol: [`spec/flows/autonomy/AUTONOMY-PROTOCOL.md`](../flows/autonomy/AUTONOMY-PROTOCOL.md).
